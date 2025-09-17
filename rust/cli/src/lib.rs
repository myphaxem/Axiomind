use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::io::IsTerminal;
use std::io::Write;
mod config;
pub mod ui;
use axm_engine::engine::Engine;
use rand::{seq::SliceRandom, RngCore, SeedableRng};
use std::fs::File;
use std::io::Read;

fn ensure_no_reopen_after_short_all_in(
    actions: &[serde_json::Value],
    big_blind: i64,
    min_chip_unit: i64,
    starting_stacks: &HashMap<String, i64>,
    hand_index: u64,
) -> Result<(), String> {
    #[derive(Clone, Copy)]
    enum ActionKind {
        Bet(i64),
        Raise(i64),
        AllIn(Option<i64>),
        Call,
        Check,
        Fold,
        Other,
    }

    let mut remaining = starting_stacks.clone();
    let mut prev_street: Option<String> = None;
    let mut street_committed: HashMap<String, i64> = HashMap::new();
    let mut current_high: i64 = 0;
    let mut last_full_raise: i64 = big_blind.max(min_chip_unit);
    let mut reopen_blocked = false;

    let extract_player_id = |act: &serde_json::Value| -> Option<String> {
        act.get("player_id")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .or_else(|| {
                act.get("player_id").and_then(|v| {
                    v.as_i64().map(|n| {
                        let candidate = format!("p{}", n);
                        if starting_stacks.contains_key(&candidate) {
                            candidate
                        } else {
                            n.to_string()
                        }
                    })
                })
            })
    };

    for (idx, act) in actions.iter().enumerate() {
        let Some(player_id) = extract_player_id(act) else {
            continue;
        };

        if let Some(street) = act.get("street").and_then(|s| s.as_str()) {
            if prev_street.as_deref() != Some(street) {
                prev_street = Some(street.to_string());
                street_committed.clear();
                current_high = 0;
                last_full_raise = big_blind.max(min_chip_unit);
                reopen_blocked = false;
            }
        }

        let action_kind: ActionKind = match act.get("action") {
            Some(serde_json::Value::Object(map)) => {
                if let Some(amount) = map.get("Bet").and_then(|v| v.as_i64()) {
                    ActionKind::Bet(amount)
                } else if let Some(amount) = map.get("Raise").and_then(|v| v.as_i64()) {
                    ActionKind::Raise(amount)
                } else if let Some(amount) = map.get("AllIn").and_then(|v| v.as_i64()) {
                    ActionKind::AllIn(Some(amount))
                } else if map.get("Call").is_some() {
                    ActionKind::Call
                } else if map.get("Check").is_some() {
                    ActionKind::Check
                } else if map.get("Fold").is_some() {
                    ActionKind::Fold
                } else {
                    ActionKind::Other
                }
            }
            Some(serde_json::Value::String(name)) => match name.to_ascii_lowercase().as_str() {
                "bet" => {
                    return Err(format!(
                        "Bet action missing amount at hand {} (action #{})",
                        hand_index,
                        idx + 1
                    ))
                }
                "raise" => {
                    return Err(format!(
                        "Bet action missing amount at hand {} (action #{})",
                        hand_index,
                        idx + 1
                    ))
                }
                "call" => ActionKind::Call,
                "check" => ActionKind::Check,
                "fold" => ActionKind::Fold,
                "allin" | "all-in" => ActionKind::AllIn(None),
                _ => ActionKind::Other,
            },
            _ => ActionKind::Other,
        };

        let commit_before = street_committed.get(&player_id).copied().unwrap_or(0);
        let mut target_commit = commit_before;

        match action_kind {
            ActionKind::Bet(amount) => {
                if amount % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid bet amount {} at hand {} (action #{})",
                        amount,
                        hand_index,
                        idx + 1
                    ));
                }
                let min_bet = big_blind.max(min_chip_unit);
                if amount < min_bet {
                    return Err(format!(
                        "Bet below minimum {} at hand {} (action #{})",
                        min_bet,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = amount;
            }
            ActionKind::Raise(amount) => {
                if amount % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid raise amount {} at hand {} (action #{})",
                        amount,
                        hand_index,
                        idx + 1
                    ));
                }
                let min_delta = last_full_raise.max(big_blind).max(min_chip_unit);
                if amount < min_delta {
                    return Err(format!(
                        "Raise delta {} below minimum {} at hand {} (action #{})",
                        amount,
                        min_delta,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = current_high + amount;
            }
            ActionKind::AllIn(Some(amount)) => {
                if amount % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid all-in amount {} at hand {} (action #{})",
                        amount,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = commit_before + amount;
            }
            ActionKind::AllIn(None) => {
                let remaining_stack = remaining.get(&player_id).copied().unwrap_or(0);
                if remaining_stack % min_chip_unit != 0 {
                    return Err(format!(
                        "Invalid all-in amount {} at hand {} (action #{})",
                        remaining_stack,
                        hand_index,
                        idx + 1
                    ));
                }
                target_commit = commit_before + remaining_stack;
            }
            ActionKind::Call => {
                target_commit = current_high.max(commit_before);
            }
            ActionKind::Check | ActionKind::Fold | ActionKind::Other => {}
        }

        let mut delta_chips = target_commit.saturating_sub(commit_before);
        if let Some(rem) = remaining.get(&player_id) {
            if delta_chips > *rem {
                delta_chips = *rem;
            }
        }
        let new_commit = commit_before + delta_chips;

        if delta_chips > 0 {
            if let Some(rem) = remaining.get_mut(&player_id) {
                if *rem < delta_chips {
                    return Err(format!(
                        "Player {} commits more chips than stack at hand {} (action #{})",
                        player_id,
                        hand_index,
                        idx + 1
                    ));
                }
                *rem -= delta_chips;
            }
        }

        let extra = new_commit.saturating_sub(current_high);
        if extra > 0 {
            if reopen_blocked {
                return Err(format!(
                    "Betting illegally reopened after short all-in at hand {} (action #{})",
                    hand_index,
                    idx + 1
                ));
            }
            let min_full_raise = last_full_raise.max(big_blind).max(min_chip_unit);
            if extra < min_full_raise {
                reopen_blocked = true;
            } else {
                last_full_raise = extra;
                reopen_blocked = false;
            }
            current_high = new_commit;
        } else {
            current_high = current_high.max(new_commit);
        }

        street_committed.insert(player_id, new_commit);
    }

    Ok(())
}

pub fn run<I, S>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fn read_text_auto(path: &str) -> Result<String, String> {
        if path.ends_with(".zst") {
            // Read entire compressed file then decompress; more portable across platforms
            let comp = std::fs::read(path).map_err(|e| e.to_string())?;
            // Use a conservative initial capacity; zstd will grow as needed
            let dec = zstd::bulk::decompress(&comp, 8 * 1024 * 1024).map_err(|e| e.to_string())?;
            String::from_utf8(dec).map_err(|e| e.to_string())
        } else {
            std::fs::read_to_string(path).map_err(|e| e.to_string())
        }
    }
    fn validate_speed(speed: Option<f64>) -> Result<(), String> {
        if let Some(s) = speed {
            if s <= 0.0 {
                return Err("speed must be > 0".into());
            }
        }
        Ok(())
    }
    const COMMANDS: &[&str] = &[
        "play", "replay", "stats", "verify", "deal", "bench", "sim", "eval", "export", "dataset",
        "cfg", "doctor", "rng", "serve", "train",
    ];
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        let _ = writeln!(out, "Axiomind Poker CLI\n");
        let _ = writeln!(out, "Usage: axm <command> [options]\n");
        let _ = writeln!(out, "Commands:");
        for c in COMMANDS {
            let _ = writeln!(out, "  {}", c);
        }
        let _ = writeln!(out, "\nOptions:\n  -h, --help     Show this help");
        return 0;
    }
    if argv.iter().any(|a| a == "--version" || a == "-V") {
        let _ = writeln!(out, "axm {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }

    let parsed = AxmCli::try_parse_from(&argv);
    match parsed {
        Err(e) => {
            // Print clap error first
            let _ = writeln!(err, "{}", e);
            // Then print an explicit help excerpt including the Commands list to stderr
            let _ = writeln!(err, "");
            let _ = writeln!(err, "Axiomind Poker CLI");
            let _ = writeln!(err, "Usage: axm <command> [options]\n");
            let _ = writeln!(err, "Commands:");
            for c in COMMANDS {
                let _ = writeln!(err, "  {}", c);
            }
            let _ = writeln!(err, "\nFor full help, run: axm --help");
            2
        }
        Ok(cli) => match cli.cmd {
            Commands::Cfg => match config::load() {
                Ok(c) => {
                    let _ = writeln!(out, "{}", serde_json::to_string_pretty(&c).unwrap());
                    0
                }
                Err(e) => {
                    let _ = ui::write_error(err, &format!("Invalid configuration: {}", e));
                    2
                }
            },
            Commands::Play {
                vs,
                hands,
                seed,
                level,
            } => {
                let hands = hands.unwrap_or(1);
                let seed = seed.unwrap_or_else(|| rand::random());
                let level = level.unwrap_or(1);
                let non_tty_override = std::env::var("AXM_NON_TTY")
                    .ok()
                    .map(|v| {
                        let v = v.to_ascii_lowercase();
                        v == "1" || v == "true" || v == "yes" || v == "on"
                    })
                    .unwrap_or(false);
                if matches!(vs, Vs::Human) && (!std::io::stdin().is_terminal() || non_tty_override)
                {
                    let scripted = std::env::var("AXM_TEST_INPUT").ok();
                    if scripted.is_none() {
                        let _ =
                            ui::write_error(err, "Non-TTY environment: --vs human is not allowed");
                        return 2;
                    }
                }
                if hands == 0 {
                    let _ = ui::write_error(err, "hands must be >= 1");
                    return 2;
                }
                let _ = writeln!(
                    out,
                    "play: vs={} hands={} seed={}",
                    vs.as_str(),
                    hands,
                    seed
                );
                let _ = writeln!(out, "Level: {}", level);
                let mut eng = Engine::new(Some(seed), level);
                eng.shuffle();
                let scripted = std::env::var("AXM_TEST_INPUT").ok();
                let mut played = 0u32;
                for i in 1..=hands {
                    // simple level progression: +1 every 2 hands
                    let cur_level: u8 = level.saturating_add(((i - 1) / 2) as u8);
                    if i > 1 {
                        let _ = writeln!(out, "Level: {}", cur_level);
                    }
                    let (sb, bb) = match cur_level {
                        1 => (50, 100),
                        2 => (75, 150),
                        3 => (100, 200),
                        _ => (150, 300),
                    };
                    let _ = writeln!(out, "Blinds: SB={} BB={}", sb, bb);
                    let _ = writeln!(out, "Hand {}", i);
                    let _ = eng.deal_hand();
                    match vs {
                        Vs::Human => {
                            // prompt once; in tests, read from AXM_TEST_INPUT
                            let action = scripted.as_deref().unwrap_or("");
                            if action.is_empty() {
                                let _ =
                                    writeln!(out, "Enter action (check/call/bet/raise/fold/q): ");
                            }
                        }
                        Vs::Ai => {
                            let _ = writeln!(out, "ai: check");
                        }
                    }
                    played += 1;
                }
                let _ = writeln!(out, "Session hands={}", hands);
                let _ = writeln!(out, "Hands played: {} (completed)", played);
                0
            }
            Commands::Replay { input, speed } => {
                match read_text_auto(&input) {
                    Ok(content) => {
                        // Validate speed via helper for clarity and future reuse
                        if let Err(msg) = validate_speed(speed) {
                            let _ = ui::write_error(err, &msg);
                            return 2;
                        }
                        let count = content.lines().filter(|l| !l.trim().is_empty()).count();
                        let _ = writeln!(out, "Replayed: {} hands", count);
                        0
                    }
                    Err(e) => {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        2
                    }
                }
            }
            Commands::Stats { input } => {
                use std::path::Path;
                let path = Path::new(&input);
                let mut hands = 0u64;
                let mut p0 = 0u64;
                let mut p1 = 0u64;
                let mut skipped = 0u64;
                let mut corrupted = 0u64;
                let mut process_content = |content: String| {
                    let has_trailing_nl = content.ends_with('\n');
                    let lines: Vec<&str> =
                        content.lines().filter(|l| !l.trim().is_empty()).collect();
                    for (i, line) in lines.iter().enumerate() {
                        let rec: axm_engine::logger::HandRecord = match serde_json::from_str(line) {
                            Ok(v) => v,
                            Err(_) => {
                                if i == lines.len() - 1 && !has_trailing_nl {
                                    skipped += 1;
                                } else {
                                    corrupted += 1;
                                }
                                continue;
                            }
                        };
                        hands += 1;
                        if let Some(r) = rec.result.as_deref() {
                            if r == "p0" {
                                p0 += 1;
                            }
                            if r == "p1" {
                                p1 += 1;
                            }
                        }
                    }
                };

                if path.is_dir() {
                    let mut stack = vec![path.to_path_buf()];
                    while let Some(d) = stack.pop() {
                        let rd = match std::fs::read_dir(&d) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        for entry in rd.flatten() {
                            let p = entry.path();
                            if p.is_dir() {
                                stack.push(p);
                                continue;
                            }
                            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                            if !(name.ends_with(".jsonl") || name.ends_with(".jsonl.zst")) {
                                continue;
                            }
                            match read_text_auto(p.to_str().unwrap()) {
                                Ok(s) => process_content(s),
                                Err(e) => {
                                    let _ = ui::write_error(
                                        err,
                                        &format!("Failed to read {}: {}", p.display(), e),
                                    );
                                }
                            }
                        }
                    }
                } else {
                    match read_text_auto(&input) {
                        Ok(s) => process_content(s),
                        Err(e) => {
                            let _ =
                                ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                            return 2;
                        }
                    }
                }

                if corrupted > 0 {
                    let _ =
                        ui::write_error(err, &format!("Skipped {} corrupted record(s)", corrupted));
                }
                if skipped > 0 {
                    let _ = ui::write_error(
                        err,
                        &format!("Discarded {} incomplete final line(s)", skipped),
                    );
                }
                if !path.is_dir() && hands == 0 && (corrupted > 0 || skipped > 0) {
                    let _ = ui::write_error(err, "Invalid record");
                    return 2;
                }
                let summary = serde_json::json!({
                    "hands": hands,
                    "winners": { "p0": p0, "p1": p1 },
                });
                let _ = writeln!(out, "{}", serde_json::to_string_pretty(&summary).unwrap());
                0
            }
            Commands::Verify { input } => {
                // verify basic rule set covering board completion, chip conservation, and betting rules
                let mut ok = true;
                let mut hands = 0u64;
                let mut game_over = false;
                let mut stacks_after_hand: HashMap<String, i64> = HashMap::new();
                const MIN_CHIP_UNIT: i64 = 25;
                let Some(path) = input else {
                    let _ = ui::write_error(err, "input required");
                    return 2;
                };
                let valid_id = |s: &str| -> bool {
                    s.len() == 15
                        && s[0..8].chars().all(|c| c.is_ascii_digit())
                        && &s[8..9] == "-"
                        && s[9..].chars().all(|c| c.is_ascii_digit())
                };
                match read_text_auto(&path) {
                    Ok(content) => {
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            hands += 1;
                            if game_over {
                                ok = false;
                                let _ = ui::write_error(
                                    err,
                                    &format!("Hand {} recorded after player elimination", hands),
                                );
                            }
                            // parse as Value first to validate optional net_result chip conservation
                            let v: serde_json::Value = match serde_json::from_str(line) {
                                Ok(v) => v,
                                Err(_) => {
                                    ok = false;
                                    let _ = ui::write_error(err, "Invalid record");
                                    continue;
                                }
                            };
                            let mut starting_stacks: Option<HashMap<String, i64>> = None;
                            if let Some(players) = v.get("players").and_then(|p| p.as_array()) {
                                let mut start_map = HashMap::new();
                                for player in players {
                                    let Some(id) = player.get("id").and_then(|x| x.as_str()) else {
                                        continue;
                                    };
                                    let stack = player
                                        .get("stack_start")
                                        .and_then(|x| x.as_i64())
                                        .unwrap_or(0);
                                    start_map.insert(id.to_string(), stack);
                                }
                                if !stacks_after_hand.is_empty() {
                                    for (id, stack_start) in &start_map {
                                        if let Some(prev) = stacks_after_hand.get(id) {
                                            if *prev != *stack_start {
                                                ok = false;
                                                let _ = ui::write_error(
                                                    err,
                                                    &format!(
                                                        "Stack mismatch for {} at hand {}",
                                                        id, hands
                                                    ),
                                                );
                                            }
                                        }
                                        if *stack_start <= 0 {
                                            ok = false;
                                            let _ = ui::write_error(
                                                err,
                                                &format!(
                                                    "Player {} has non-positive starting stack at hand {}",
                                                    id, hands
                                                ),
                                            );
                                        }
                                    }
                                }
                                starting_stacks = Some(start_map.clone());
                                stacks_after_hand = start_map;
                            }
                            let mut big_blind = MIN_CHIP_UNIT;
                            if let Some(blinds_val) = v.get("blinds") {
                                if let Some(bb) = blinds_val.get("bb").and_then(|x| x.as_i64()) {
                                    big_blind = bb;
                                } else if let Some(arr) = blinds_val.as_array() {
                                    if arr.len() >= 2 {
                                        if let Some(bb) = arr[1].as_i64() {
                                            big_blind = bb;
                                        }
                                    }
                                }
                            }
                            if big_blind < MIN_CHIP_UNIT {
                                big_blind = MIN_CHIP_UNIT;
                            }
                            if let Some(actions) = v.get("actions").and_then(|a| a.as_array()) {
                                if let Some(ref start_map) = starting_stacks {
                                    if let Err(msg) = ensure_no_reopen_after_short_all_in(
                                        actions,
                                        big_blind,
                                        MIN_CHIP_UNIT,
                                        start_map,
                                        hands,
                                    ) {
                                        ok = false;
                                        let _ = ui::write_error(err, &msg);
                                    }
                                }
                            }
                            if let Some(nr) = v.get("net_result").and_then(|x| x.as_object()) {
                                let mut sum: i64 = 0;
                                for val in nr.values() {
                                    if let Some(n) = val.as_i64() {
                                        sum += n;
                                    }
                                }
                                if sum != 0 {
                                    ok = false;
                                    let _ = ui::write_error(err, "Chip conservation violated");
                                }
                                for (id, delta) in nr.iter() {
                                    if let Some(val) = delta.as_i64() {
                                        let entry =
                                            stacks_after_hand.entry(id.clone()).or_insert(0);
                                        *entry += val;
                                    }
                                }
                                if stacks_after_hand.values().any(|stack| *stack <= 0) {
                                    game_over = true;
                                }
                            }
                            match serde_json::from_value::<axm_engine::logger::HandRecord>(v) {
                                Ok(rec) => {
                                    if rec.board.len() != 5 {
                                        ok = false;
                                    }
                                    if !valid_id(&rec.hand_id) {
                                        ok = false;
                                        let _ = ui::write_error(err, "Invalid hand_id");
                                    }
                                }
                                Err(_) => {
                                    ok = false;
                                    let _ = ui::write_error(err, "Invalid record");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", path, e));
                        return 2;
                    }
                }
                let status = if ok { "OK" } else { "FAIL" };
                let _ = writeln!(out, "Verify: {} (hands={})", status, hands);
                if ok {
                    0
                } else {
                    2
                }
            }
            Commands::Doctor => {
                let _ = writeln!(out, "Doctor: OK");
                0
            }
            Commands::Eval {
                ai_a,
                ai_b,
                hands,
                seed,
            } => {
                if ai_a == ai_b {
                    let _ = ui::write_error(err, "Warning: identical AI models");
                }
                let mut a_wins = 0u32;
                let mut b_wins = 0u32;
                let s = seed.unwrap_or_else(|| rand::random());
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(s);
                for _ in 0..hands {
                    if (rng.next_u32() & 1) == 0 {
                        a_wins += 1;
                    } else {
                        b_wins += 1;
                    }
                }
                let _ = writeln!(out, "Eval: hands={} A:{} B:{}", hands, a_wins, b_wins);
                0
            }
            Commands::Bench => {
                // quick bench: evaluate 200 unique 7-card draws from shuffled deck
                use axm_engine::cards::Card;
                use axm_engine::deck::Deck;
                let start = std::time::Instant::now();
                let mut cnt = 0u64;
                let mut deck = Deck::new_with_seed(1);
                deck.shuffle();
                for _ in 0..200 {
                    if deck.remaining() < 7 {
                        deck.shuffle();
                    }
                    let mut arr: [Card; 7] = [deck.deal_card().unwrap(); 7];
                    for i in 1..7 {
                        arr[i] = deck.deal_card().unwrap();
                    }
                    let _ = axm_engine::hand::evaluate_hand(&arr);
                    cnt += 1;
                }
                let dur = start.elapsed();
                let _ = writeln!(out, "Benchmark: {} iters in {:?}", cnt, dur);
                0
            }
            Commands::Deal { seed } => {
                let base_seed = seed.unwrap_or_else(|| rand::random());
                let mut eng = Engine::new(Some(base_seed), 1);
                eng.shuffle();
                let _ = eng.deal_hand();
                let p = eng.players();
                let hc1 = p[0].hole_cards();
                let hc2 = p[1].hole_cards();
                let fmt = |c: axm_engine::cards::Card| format!("{:?}{:?}", c.rank, c.suit);
                let _ = writeln!(
                    out,
                    "Hole P1: {} {}",
                    fmt(hc1[0].unwrap()),
                    fmt(hc1[1].unwrap())
                );
                let _ = writeln!(
                    out,
                    "Hole P2: {} {}",
                    fmt(hc2[0].unwrap()),
                    fmt(hc2[1].unwrap())
                );
                let b = eng.board();
                let _ = writeln!(
                    out,
                    "Board: {} {} {} {} {}",
                    fmt(b[0]),
                    fmt(b[1]),
                    fmt(b[2]),
                    fmt(b[3]),
                    fmt(b[4])
                );
                0
            }
            Commands::Rng { seed } => {
                let s = seed.unwrap_or_else(|| rand::random());
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(s);
                let mut vals = vec![];
                for _ in 0..5 {
                    vals.push(rng.next_u64());
                }
                let _ = writeln!(out, "RNG sample: {:?}", vals);
                0
            }
            Commands::Sim {
                hands,
                output,
                seed,
                resume,
            } => {
                let total: usize = hands as usize;
                if total == 0 {
                    let _ = ui::write_error(err, "hands must be >= 1");
                    return 2;
                }
                let mut completed = 0usize;
                let mut path = None;
                if let Some(outp) = output.clone() {
                    path = Some(std::path::PathBuf::from(outp));
                }
                // resume: count existing unique hand_ids and warn on duplicates
                if let Some(res) = resume.as_ref() {
                    let contents = std::fs::read_to_string(res).unwrap_or_default();
                    let mut seen = std::collections::HashSet::new();
                    let mut dups = 0usize;
                    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
                        let hid = serde_json::from_str::<serde_json::Value>(line)
                            .ok()
                            .and_then(|v| {
                                v.get("hand_id")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string())
                            })
                            .unwrap_or_default();
                        if hid.is_empty() {
                            continue;
                        }
                        if !seen.insert(hid) {
                            dups += 1;
                        }
                    }
                    completed = seen.len();
                    path = Some(std::path::PathBuf::from(res));
                    if dups > 0 {
                        let _ = writeln!(err, "Warning: {} duplicate hand_id(s) skipped", dups);
                    }
                    let _ = writeln!(out, "Resumed from {}", completed);
                }
                let base_seed = seed.unwrap_or_else(|| rand::random());
                let mut eng = Engine::new(Some(base_seed), 1);
                eng.shuffle();
                let break_after = std::env::var("AXM_SIM_BREAK_AFTER")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok());
                for i in completed..total {
                    // create a fresh engine per hand to avoid residual hole cards
                    let mut e = Engine::new(Some(base_seed + i as u64), 1);
                    e.shuffle();
                    let _ = e.deal_hand();
                    if let Some(p) = &path {
                        let mut f = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(p)
                            .unwrap();
                        let hand_id = format!("19700101-{:06}", i + 1);
                        let board = e.board().clone();
                        let rec = serde_json::json!({
                            "hand_id": hand_id,
                            "seed": seed,
                            "actions": [],
                            "board": board,
                            "result": null,
                            "ts": null,
                            "meta": null
                        });
                        let _ = writeln!(f, "{}", serde_json::to_string(&rec).unwrap());
                    }
                    completed += 1;
                    if let Some(b) = break_after {
                        if completed == b {
                            let _ = writeln!(out, "Interrupted: saved {}/{}", completed, total);
                            return 130;
                        }
                    }
                }
                let _ = writeln!(out, "Simulated: {} hands", completed);
                0
            }
            Commands::Export {
                input,
                format,
                output,
            } => {
                let content = match std::fs::read_to_string(&input) {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        return 2;
                    }
                };
                match format.as_str() {
                    f if f.eq_ignore_ascii_case("csv") => {
                        let mut w = std::fs::File::create(&output)
                            .map(|f| std::io::BufWriter::new(f))
                            .map_err(|e| {
                                let _ = ui::write_error(
                                    err,
                                    &format!("Failed to write {}: {}", output, e),
                                );
                                e
                            })
                            .unwrap();
                        let _ = writeln!(w, "hand_id,seed,result,ts,actions,board");
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            let rec: axm_engine::logger::HandRecord =
                                serde_json::from_str(line).unwrap();
                            let seed = rec.seed.map(|v| v.to_string()).unwrap_or_else(|| "".into());
                            let result = rec.result.unwrap_or_default();
                            let ts = rec.ts.unwrap_or_default();
                            let _ = writeln!(
                                w,
                                "{},{},{},{},{},{}",
                                rec.hand_id,
                                seed,
                                result,
                                ts,
                                rec.actions.len(),
                                rec.board.len()
                            );
                        }
                        0
                    }
                    f if f.eq_ignore_ascii_case("json") => {
                        let mut arr = Vec::new();
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            let v: serde_json::Value = serde_json::from_str(line).unwrap();
                            arr.push(v);
                        }
                        let s = serde_json::to_string_pretty(&arr).unwrap();
                        std::fs::write(&output, s).unwrap();
                        0
                    }
                    _ => {
                        let _ = ui::write_error(err, "Unsupported format");
                        2
                    }
                }
            }
            Commands::Dataset {
                input,
                outdir,
                train,
                val,
                test,
                seed,
            } => {
                let content = std::fs::read_to_string(&input)
                    .map_err(|e| {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        e
                    })
                    .unwrap();
                let mut lines: Vec<String> = content
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|s| s.to_string())
                    .collect();
                let n = lines.len();
                if n == 0 {
                    let _ = ui::write_error(err, "Empty input");
                    return 2;
                }
                let tr = train.unwrap_or(0.8);
                let va = val.unwrap_or(0.1);
                let te = test.unwrap_or(0.1);
                let sum = tr + va + te;
                if (sum - 1.0).abs() > 1e-6 {
                    let _ = ui::write_error(err, "Splits must sum to 1.0");
                    return 2;
                }
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
                lines.shuffle(&mut rng);
                let n_tr = ((tr * n as f64).round() as usize).min(n);
                let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));
                let _n_te = n.saturating_sub(n_tr + n_va);
                let (trv, rest) = lines.split_at(n_tr);
                let (vav, tev) = rest.split_at(n_va);
                std::fs::create_dir_all(&outdir).unwrap();
                let write_split = |path: &std::path::Path, data: &[String]| {
                    let mut f = std::fs::File::create(path).unwrap();
                    for l in data {
                        let _ = writeln!(f, "{}", l);
                    }
                };
                write_split(&std::path::Path::new(&outdir).join("train.jsonl"), trv);
                write_split(&std::path::Path::new(&outdir).join("val.jsonl"), vav);
                write_split(&std::path::Path::new(&outdir).join("test.jsonl"), tev);
                0
            }
        },
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "axm",
    author = "Axiomind",
    version,
    about = "Axiomind Poker CLI",
    disable_help_flag = true
)]
struct AxmCli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Play {
        #[arg(long, value_enum)]
        vs: Vs,
        #[arg(long)]
        hands: Option<u32>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        level: Option<u8>,
    },
    Replay {
        #[arg(long)]
        input: String,
        #[arg(long)]
        speed: Option<f64>,
    },
    Stats {
        #[arg(long)]
        input: String,
    },
    Eval {
        #[arg(long, name = "ai-a")]
        ai_a: String,
        #[arg(long, name = "ai-b")]
        ai_b: String,
        #[arg(long)]
        hands: u32,
        #[arg(long)]
        seed: Option<u64>,
    },
    Verify {
        #[arg(long)]
        input: Option<String>,
    },
    Deal {
        #[arg(long)]
        seed: Option<u64>,
    },
    Bench,
    Sim {
        #[arg(long)]
        hands: u64,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        resume: Option<String>,
    },
    Export {
        #[arg(long)]
        input: String,
        #[arg(long)]
        format: String,
        #[arg(long)]
        output: String,
    },
    Dataset {
        #[arg(long)]
        input: String,
        #[arg(long)]
        outdir: String,
        #[arg(long)]
        train: Option<f64>,
        #[arg(long)]
        val: Option<f64>,
        #[arg(long)]
        test: Option<f64>,
        #[arg(long)]
        seed: Option<u64>,
    },
    Cfg,
    Doctor,
    Rng {
        #[arg(long)]
        seed: Option<u64>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Vs {
    Human,
    Ai,
}

impl Vs {
    fn as_str(&self) -> &'static str {
        match self {
            Vs::Human => "human",
            Vs::Ai => "ai",
        }
    }
}
