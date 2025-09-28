use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::io::IsTerminal;
use std::io::Write;
mod config;
pub mod ui;
use axm_engine::engine::Engine;
use rand::{seq::SliceRandom, RngCore, SeedableRng};

use std::collections::HashSet;

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

        if !starting_stacks.contains_key(&player_id) {
            return Err(format!(
                "Unknown player {} at hand {} (action #{})",
                player_id,
                hand_index,
                idx + 1
            ));
        }

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

fn validate_dealing_meta(
    meta: &serde_json::Map<String, serde_json::Value>,
    button: Option<&str>,
    starting_stacks: &HashMap<String, i64>,
    hand_index: u64,
) -> Result<(), String> {
    if starting_stacks.is_empty() {
        return Ok(());
    }
    let player_count = starting_stacks.len();
    let rounds = 2; // Texas Hold'em: two hole cards per player
    let sb = meta.get("small_blind").and_then(|v| v.as_str());
    let bb = meta.get("big_blind").and_then(|v| v.as_str());
    if let Some(sb_id) = sb {
        if !starting_stacks.contains_key(sb_id) {
            return Err(format!(
                "Invalid dealing order at hand {}: unknown small blind {}",
                hand_index, sb_id
            ));
        }
    }
    if let Some(bb_id) = bb {
        if !starting_stacks.contains_key(bb_id) {
            return Err(format!(
                "Invalid dealing order at hand {}: unknown big blind {}",
                hand_index, bb_id
            ));
        }
    }
    if let (Some(btn), Some(sb_id)) = (button, sb) {
        if sb_id != btn {
            return Err(format!(
                "Invalid dealing order at hand {}: button {} must match small blind {}",
                hand_index, btn, sb_id
            ));
        }
    }
    if let (Some(sb_id), Some(bb_id)) = (sb, bb) {
        if sb_id == bb_id {
            return Err(format!(
                "Invalid dealing order at hand {}: small blind and big blind must differ",
                hand_index
            ));
        }
        if player_count == 2 {
            if let Some(expected_bb) = starting_stacks
                .keys()
                .find(|id| id.as_str() != sb_id)
                .map(|s| s.as_str())
            {
                if bb_id != expected_bb {
                    return Err(format!(
                        "Invalid dealing order at hand {}: expected big blind {} but found {}",
                        hand_index, expected_bb, bb_id
                    ));
                }
            }
        }
    }
    if let Some(seq_val) = meta.get("deal_sequence") {
        let seq = seq_val.as_array().ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: deal_sequence must be an array",
                hand_index
            )
        })?;
        let seq_ids: Option<Vec<&str>> = seq.iter().map(|v| v.as_str()).collect();
        let seq_ids = seq_ids.ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: deal_sequence must contain player identifiers",
                hand_index
            )
        })?;
        let expected_len = player_count * rounds;
        if seq_ids.len() != expected_len {
            return Err(format!(
                "Invalid dealing order at hand {}: expected {} entries in deal_sequence but found {}",
                hand_index,
                expected_len,
                seq_ids.len()
            ));
        }
        let known: HashSet<&str> = starting_stacks.keys().map(|k| k.as_str()).collect();
        if seq_ids.iter().any(|id| !known.contains(id)) {
            return Err(format!(
                "Invalid dealing order at hand {}: deal_sequence references unknown player",
                hand_index
            ));
        }
        let first_round = &seq_ids[..player_count];
        if let Some(sb_id) = sb {
            if first_round.first().copied() != Some(sb_id) {
                return Err(format!(
                    "Invalid dealing order at hand {}: expected {} to receive the first card",
                    hand_index, sb_id
                ));
            }
        }
        if let Some(bb_id) = bb {
            if player_count >= 2 {
                if first_round.get(1).copied() != Some(bb_id) {
                    return Err(format!(
                        "Invalid dealing order at hand {}: expected {} to receive the second card",
                        hand_index, bb_id
                    ));
                }
            }
        }
        let first_round_set: HashSet<&str> = first_round.iter().copied().collect();
        if first_round_set.len() != player_count {
            return Err(format!(
                "Invalid dealing order at hand {}: duplicate players in first deal round",
                hand_index
            ));
        }
        for round_idx in 1..rounds {
            let chunk = &seq_ids[round_idx * player_count..(round_idx + 1) * player_count];
            if chunk != first_round {
                return Err(format!(
                    "Invalid dealing order at hand {}: inconsistent card distribution order",
                    hand_index
                ));
            }
        }
    }
    if let Some(burn_val) = meta.get("burn_positions") {
        let burn_arr = burn_val.as_array().ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: burn_positions must be an array",
                hand_index
            )
        })?;
        let burn_positions: Option<Vec<i64>> = burn_arr.iter().map(|v| v.as_i64()).collect();
        let burn_positions = burn_positions.ok_or_else(|| {
            format!(
                "Invalid dealing order at hand {}: burn_positions must contain integers",
                hand_index
            )
        })?;
        if burn_positions.len() != 3 {
            return Err(format!(
                "Invalid dealing order at hand {}: expected 3 burn positions",
                hand_index
            ));
        }
        let player_count_i64 = player_count as i64;
        if player_count_i64 >= 2 {
            let hole_cards = player_count_i64 * 2;
            let expected = vec![
                hole_cards + 1,
                hole_cards + 1 + 3 + 1,
                hole_cards + 1 + 3 + 1 + 1 + 1,
            ];
            if burn_positions != expected {
                return Err(format!(
                    "Invalid dealing order at hand {}: expected burn positions {:?} but found {:?}",
                    hand_index, expected, burn_positions
                ));
            }
        }
    }
    Ok(())
}

fn validate_roster_state(
    prev: Option<&HashMap<String, i64>>,
    current: &HashMap<String, i64>,
    hands: u64,
    err: &mut dyn Write,
    ok: &mut bool,
) {
    if let Some(prev_map) = prev {
        for (id, stack_start) in current {
            if let Some(prev_stack) = prev_map.get(id) {
                if *prev_stack != *stack_start {
                    *ok = false;
                    let _ = ui::write_error(
                        err,
                        &format!("Stack mismatch for {} at hand {}", id, hands),
                    );
                }
                if *prev_stack <= 0 {
                    *ok = false;
                    let _ = ui::write_error(
                        err,
                        &format!(
                            "Player {} reappeared after elimination at hand {}",
                            id, hands
                        ),
                    );
                }
            } else {
                *ok = false;
                let _ =
                    ui::write_error(err, &format!("Unexpected player {} at hand {}", id, hands));
            }
        }
        for (id, prev_stack) in prev_map {
            if !current.contains_key(id) && *prev_stack > 0 {
                *ok = false;
                let _ = ui::write_error(err, &format!("Missing player {} at hand {}", id, hands));
            }
        }
    }
    for (id, stack_start) in current {
        if *stack_start <= 0 {
            *ok = false;
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

pub fn run<I, S>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fn strip_utf8_bom(s: &mut String) {
        const UTF8_BOM: &str = "\u{feff}";
        if s.starts_with(UTF8_BOM) {
            s.drain(..UTF8_BOM.len());
        }
    }

    fn read_text_auto(path: &str) -> Result<String, String> {
        let mut content = if path.ends_with(".zst") {
            // Read entire compressed file then decompress; more portable across platforms
            let comp = std::fs::read(path).map_err(|e| e.to_string())?;
            // Use a conservative initial capacity; zstd will grow as needed
            let dec = zstd::bulk::decompress(&comp, 8 * 1024 * 1024).map_err(|e| e.to_string())?;
            String::from_utf8(dec).map_err(|e| e.to_string())?
        } else {
            std::fs::read_to_string(path).map_err(|e| e.to_string())?
        };
        strip_utf8_bom(&mut content);
        Ok(content)
    }
    fn validate_speed(speed: Option<f64>) -> Result<(), String> {
        if let Some(s) = speed {
            if s <= 0.0 {
                return Err("speed must be > 0".into());
            }
        }
        Ok(())
    }

    fn compute_splits(
        train: Option<f64>,
        val: Option<f64>,
        test: Option<f64>,
    ) -> Result<[f64; 3], String> {
        const DEFAULTS: [f64; 3] = [0.8, 0.1, 0.1];
        let mut splits = [0.0; 3];
        for (idx, opt) in [train, val, test].into_iter().enumerate() {
            splits[idx] = match opt {
                Some(v) if v.is_sign_negative() => {
                    return Err("Splits must be non-negative".into());
                }
                Some(v) if v > 1.0 + 1e-6 => v / 100.0,
                Some(v) => v,
                None => DEFAULTS[idx],
            };
        }
        let sum: f64 = splits.iter().sum();
        if (sum - 1.0).abs() > 1e-6 {
            return Err("Splits must sum to 100% (1.0 total)".into());
        }
        Ok(splits)
    }

    fn dataset_stream_if_needed(
        input: &str,
        outdir: &str,
        train: Option<f64>,
        val: Option<f64>,
        test: Option<f64>,
        seed: Option<u64>,
        err: &mut dyn Write,
    ) -> Option<i32> {
        use std::io::{BufRead, BufReader, BufWriter};

        let threshold = std::env::var("AXM_DATASET_STREAM_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10_000);
        if threshold == 0 {
            return None;
        }

        let trace_stream = std::env::var("AXM_DATASET_STREAM_TRACE")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        let count_file = match std::fs::File::open(input) {
            Ok(f) => f,
            Err(e) => {
                let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                return Some(2);
            }
        };

        let mut record_count = 0usize;
        {
            let reader = BufReader::new(count_file);
            let mut first_line = true;
            for line in reader.lines() {
                match line {
                    Ok(mut line) => {
                        if first_line {
                            strip_utf8_bom(&mut line);
                            first_line = false;
                        }
                        if !line.trim().is_empty() {
                            record_count += 1;
                        }
                    }
                    Err(e) => {
                        let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                        return Some(2);
                    }
                }
            }
        }

        if record_count == 0 {
            let _ = ui::write_error(err, "Empty input");
            return Some(2);
        }

        if record_count <= threshold {
            return None;
        }

        let splits = match compute_splits(train, val, test) {
            Ok(v) => v,
            Err(msg) => {
                let _ = ui::write_error(err, &msg);
                return Some(2);
            }
        };

        let tr = splits[0];
        let va = splits[1];
        let n = record_count;
        let n_tr = ((tr * n as f64).round() as usize).min(n);
        let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));

        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
        let mut indices: Vec<usize> = (0..record_count).collect();
        indices.shuffle(&mut rng);

        #[derive(Clone, Copy)]
        enum SplitSlot {
            Train,
            Val,
            Test,
        }

        let mut assignments = vec![SplitSlot::Test; record_count];
        for &idx in indices.iter().take(n_tr) {
            assignments[idx] = SplitSlot::Train;
        }
        for &idx in indices.iter().skip(n_tr).take(n_va) {
            assignments[idx] = SplitSlot::Val;
        }

        if let Err(e) = std::fs::create_dir_all(outdir) {
            let _ = ui::write_error(
                err,
                &format!("Failed to create directory {}: {}", outdir, e),
            );
            return Some(2);
        }

        if trace_stream {
            let _ = ui::write_error(
                err,
                &format!("Streaming dataset input (records={})", record_count),
            );
        }

        let out_root = std::path::Path::new(outdir);
        let mut train_writer =
            BufWriter::new(std::fs::File::create(out_root.join("train.jsonl")).unwrap());
        let mut val_writer =
            BufWriter::new(std::fs::File::create(out_root.join("val.jsonl")).unwrap());
        let mut test_writer =
            BufWriter::new(std::fs::File::create(out_root.join("test.jsonl")).unwrap());

        let data_file = match std::fs::File::open(input) {
            Ok(f) => f,
            Err(e) => {
                let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                return Some(2);
            }
        };
        let reader = BufReader::new(data_file);
        let mut record_idx = 0usize;
        let mut first_line = true;

        for (line_idx, line_res) in reader.lines().enumerate() {
            let mut line = match line_res {
                Ok(line) => line,
                Err(e) => {
                    let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                    return Some(2);
                }
            };
            if first_line {
                strip_utf8_bom(&mut line);
                first_line = false;
            }
            if line.trim().is_empty() {
                continue;
            }
            if let Err(e) = serde_json::from_str::<axm_engine::logger::HandRecord>(&line) {
                let _ = ui::write_error(
                    err,
                    &format!("Invalid record at line {}: {}", line_idx + 1, e),
                );
                return Some(2);
            }
            let bucket = assignments
                .get(record_idx)
                .copied()
                .unwrap_or(SplitSlot::Test);
            record_idx += 1;
            match bucket {
                SplitSlot::Train => {
                    let _ = writeln!(train_writer, "{}", line);
                }
                SplitSlot::Val => {
                    let _ = writeln!(val_writer, "{}", line);
                }
                SplitSlot::Test => {
                    let _ = writeln!(test_writer, "{}", line);
                }
            }
        }

        Some(0)
    }

    fn run_stats(input: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
        use std::path::Path;

        struct StatsState {
            hands: u64,
            p0: u64,
            p1: u64,
            skipped: u64,
            corrupted: u64,
            stats_ok: bool,
        }

        fn consume_stats_content(content: String, state: &mut StatsState, err: &mut dyn Write) {
            let has_trailing_nl = content.ends_with('\n');
            let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
            for (i, line) in lines.iter().enumerate() {
                let parsed: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => {
                        if i == lines.len() - 1 && !has_trailing_nl {
                            state.skipped += 1;
                        } else {
                            state.corrupted += 1;
                        }
                        continue;
                    }
                };

                let rec: axm_engine::logger::HandRecord =
                    match serde_json::from_value(parsed.clone()) {
                        Ok(v) => v,
                        Err(_) => {
                            state.corrupted += 1;
                            continue;
                        }
                    };

                if let Some(net_obj) = parsed.get("net_result").and_then(|v| v.as_object()) {
                    let mut sum = 0i64;
                    let mut invalid = false;
                    for (player, val) in net_obj {
                        if let Some(n) = val.as_i64() {
                            sum += n;
                        } else {
                            invalid = true;
                            state.stats_ok = false;
                            let _ = ui::write_error(
                                err,
                                &format!(
                                    "Invalid net_result value for {} at hand {}",
                                    player, rec.hand_id
                                ),
                            );
                        }
                    }
                    if sum != 0 {
                        state.stats_ok = false;
                        let _ = ui::write_error(
                            err,
                            &format!("Chip conservation violated at hand {}", rec.hand_id),
                        );
                    }
                    if invalid {
                        continue;
                    }
                }

                state.hands += 1;
                if let Some(r) = rec.result.as_deref() {
                    if r == "p0" {
                        state.p0 += 1;
                    }
                    if r == "p1" {
                        state.p1 += 1;
                    }
                }
            }
        }

        let path = Path::new(input);
        let mut state = StatsState {
            hands: 0,
            p0: 0,
            p1: 0,
            skipped: 0,
            corrupted: 0,
            stats_ok: true,
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
                        Ok(s) => consume_stats_content(s, &mut state, err),
                        Err(e) => {
                            let _ = ui::write_error(
                                err,
                                &format!("Failed to read {}: {}", p.display(), e),
                            );
                            state.stats_ok = false;
                        }
                    }
                }
            }
        } else {
            match read_text_auto(input) {
                Ok(s) => consume_stats_content(s, &mut state, err),
                Err(e) => {
                    let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e));
                    return 2;
                }
            }
        }

        if state.corrupted > 0 {
            let _ = ui::write_error(
                err,
                &format!("Skipped {} corrupted record(s)", state.corrupted),
            );
        }
        if state.skipped > 0 {
            let _ = ui::write_error(
                err,
                &format!("Discarded {} incomplete final line(s)", state.skipped),
            );
        }
        if !path.is_dir() && state.hands == 0 && (state.corrupted > 0 || state.skipped > 0) {
            let _ = ui::write_error(err, "Invalid record");
            return 2;
        }

        let summary = serde_json::json!({
            "hands": state.hands,
            "winners": { "p0": state.p0, "p1": state.p1 },
        });
        let _ = writeln!(out, "{}", serde_json::to_string_pretty(&summary).unwrap());
        if state.stats_ok {
            0
        } else {
            2
        }
    }

    fn export_sqlite(content: &str, output: &str, err: &mut dyn Write) -> i32 {
        enum ExportAttemptError {
            Busy(String),
            Fatal(String),
        }

        fn sqlite_busy(err: &rusqlite::Error) -> bool {
            matches!(
                err,
                rusqlite::Error::SqliteFailure(info, _)
                    if matches!(
                        info.code,
                        rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                    )
            )
        }

        fn export_sqlite_attempt(content: &str, output: &str) -> Result<(), ExportAttemptError> {
            let output_path = std::path::Path::new(output);
            if let Some(parent) = output_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ExportAttemptError::Fatal(format!(
                            "Failed to create directory {}: {}",
                            parent.display(),
                            e
                        ))
                    })?;
                }
            }

            let mut conn = rusqlite::Connection::open(output).map_err(|e| {
                if sqlite_busy(&e) {
                    ExportAttemptError::Busy(format!("open {}: {}", output, e))
                } else {
                    ExportAttemptError::Fatal(format!("Failed to open {}: {}", output, e))
                }
            })?;

            let tx = conn.transaction().map_err(|e| {
                if sqlite_busy(&e) {
                    ExportAttemptError::Busy(format!("start transaction: {}", e))
                } else {
                    ExportAttemptError::Fatal(format!("Failed to start transaction: {}", e))
                }
            })?;

            tx.execute("DROP TABLE IF EXISTS hands", []).map_err(|e| {
                if sqlite_busy(&e) {
                    ExportAttemptError::Busy(format!("reset schema: {}", e))
                } else {
                    ExportAttemptError::Fatal(format!("Failed to reset schema: {}", e))
                }
            })?;

            tx.execute(
                "CREATE TABLE hands (
                hand_id TEXT PRIMARY KEY NOT NULL,
                seed INTEGER,
                result TEXT,
                ts TEXT,
                actions INTEGER NOT NULL,
                board INTEGER NOT NULL,
                raw_json TEXT NOT NULL
            )",
                [],
            )
            .map_err(|e| {
                if sqlite_busy(&e) {
                    ExportAttemptError::Busy(format!("create schema: {}", e))
                } else {
                    ExportAttemptError::Fatal(format!("Failed to create schema: {}", e))
                }
            })?;

            let mut stmt = tx
                .prepare(
                    "INSERT INTO hands (hand_id, seed, result, ts, actions, board, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .map_err(|e| {
                    if sqlite_busy(&e) {
                        ExportAttemptError::Busy(format!("prepare insert: {}", e))
                    } else {
                        ExportAttemptError::Fatal(format!("Failed to prepare insert: {}", e))
                    }
                })?;

            for (line_idx, line) in content.lines().enumerate() {
                let raw = line.trim();
                if raw.is_empty() {
                    continue;
                }

                let record: axm_engine::logger::HandRecord = serde_json::from_str(raw)
                    .map_err(|e| ExportAttemptError::Fatal(format!("Invalid record: {}", e)))?;

                let axm_engine::logger::HandRecord {
                    hand_id,
                    seed,
                    actions,
                    board,
                    result,
                    ts,
                    ..
                } = record;

                let actions_len = actions.len() as i64;
                let board_len = board.len() as i64;

                let seed_val = match seed {
                    Some(v) if v > i64::MAX as u64 => {
                        return Err(ExportAttemptError::Fatal(format!(
                            "Seed {} exceeds supported range",
                            v
                        )));
                    }
                    Some(v) => Some(v as i64),
                    None => None,
                };

                stmt.execute(rusqlite::params![
                    hand_id,
                    seed_val,
                    result,
                    ts,
                    actions_len,
                    board_len,
                    raw,
                ])
                .map_err(|e| {
                    if sqlite_busy(&e) {
                        ExportAttemptError::Busy(format!(
                            "insert record at line {}: {}",
                            line_idx + 1,
                            e
                        ))
                    } else {
                        ExportAttemptError::Fatal(format!("Failed to insert record: {}", e))
                    }
                })?;
            }

            drop(stmt);

            tx.commit().map_err(|e| {
                if sqlite_busy(&e) {
                    ExportAttemptError::Busy(format!("commit export: {}", e))
                } else {
                    ExportAttemptError::Fatal(format!("Failed to commit export: {}", e))
                }
            })?;

            Ok(())
        }

        let max_attempts = std::env::var("AXM_EXPORT_SQLITE_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(3);
        let backoff_ms = std::env::var("AXM_EXPORT_SQLITE_RETRY_SLEEP_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(50);

        for attempt in 1..=max_attempts {
            match export_sqlite_attempt(content, output) {
                Ok(()) => return 0,
                Err(ExportAttemptError::Busy(msg)) => {
                    if attempt == max_attempts {
                        let _ = ui::write_error(
                            err,
                            &format!("SQLite busy after {} attempt(s): {}", attempt, msg),
                        );
                        return 2;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(
                        backoff_ms * attempt as u64,
                    ));
                }
                Err(ExportAttemptError::Fatal(msg)) => {
                    let _ = ui::write_error(err, &msg);
                    return 2;
                }
            }
        }

        2
    }

    fn run_doctor(out: &mut dyn Write, err: &mut dyn Write) -> i32 {
        use std::env;
        use std::path::{Path, PathBuf};
        use std::time::{SystemTime, UNIX_EPOCH};

        struct DoctorCheck {
            name: &'static str,
            ok: bool,
            detail: String,
            error: Option<String>,
        }

        impl DoctorCheck {
            fn ok(name: &'static str, detail: impl Into<String>) -> Self {
                DoctorCheck {
                    name,
                    ok: true,
                    detail: detail.into(),
                    error: None,
                }
            }

            fn fail(
                name: &'static str,
                detail: impl Into<String>,
                error: impl Into<String>,
            ) -> Self {
                DoctorCheck {
                    name,
                    ok: false,
                    detail: detail.into(),
                    error: Some(error.into()),
                }
            }

            fn to_value(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                map.insert(
                    "status".into(),
                    serde_json::Value::String(if self.ok { "ok" } else { "fail" }.into()),
                );
                map.insert(
                    "detail".into(),
                    serde_json::Value::String(self.detail.clone()),
                );
                if let Some(err) = &self.error {
                    map.insert("error".into(), serde_json::Value::String(err.clone()));
                }
                serde_json::Value::Object(map)
            }
        }

        fn unique_suffix() -> u128 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros()
        }

        fn check_sqlite(dir: &Path) -> DoctorCheck {
            if !dir.exists() {
                return DoctorCheck::fail(
                    "sqlite",
                    format!("SQLite check looked for {}", dir.display()),
                    format!(
                        "SQLite check failed: directory {} does not exist",
                        dir.display()
                    ),
                );
            }
            if !dir.is_dir() {
                return DoctorCheck::fail(
                    "sqlite",
                    format!("SQLite check attempted in {}", dir.display()),
                    format!("SQLite check failed: {} is not a directory", dir.display()),
                );
            }
            let candidate = dir.join(format!("axm-doctor-{}.sqlite", unique_suffix()));
            match rusqlite::Connection::open(&candidate) {
                Ok(conn) => {
                    let pragma = conn.execute("PRAGMA user_version = 1", []);
                    drop(conn);
                    if pragma.is_err() {
                        let _ = std::fs::remove_file(&candidate);
                        return DoctorCheck::fail(
                            "sqlite",
                            format!("SQLite write attempt in {}", dir.display()),
                            format!(
                                "SQLite check failed: unable to write to {}",
                                candidate.display()
                            ),
                        );
                    }
                    let _ = std::fs::remove_file(&candidate);
                    DoctorCheck::ok(
                        "sqlite",
                        format!("SQLite write test passed in {}", dir.display()),
                    )
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&candidate);
                    DoctorCheck::fail(
                        "sqlite",
                        format!("SQLite write attempt in {}", dir.display()),
                        format!("SQLite check failed: {}", e),
                    )
                }
            }
        }

        fn check_data_dir(path: &Path) -> DoctorCheck {
            if !path.exists() {
                return DoctorCheck::fail(
                    "data_dir",
                    format!("Data directory probe at {}", path.display()),
                    format!(
                        "Data directory check failed: {} does not exist",
                        path.display()
                    ),
                );
            }
            if !path.is_dir() {
                return DoctorCheck::fail(
                    "data_dir",
                    format!("Data directory probe at {}", path.display()),
                    format!(
                        "Data directory check failed: {} is not a directory",
                        path.display()
                    ),
                );
            }
            let probe = path.join("axm-doctor-write.tmp");
            match std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&probe)
            {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(b"ok") {
                        let _ = std::fs::remove_file(&probe);
                        return DoctorCheck::fail(
                            "data_dir",
                            format!("Data directory write attempt in {}", path.display()),
                            format!("Data directory check failed: {}", e),
                        );
                    }
                    drop(file);
                    let _ = std::fs::remove_file(&probe);
                    DoctorCheck::ok(
                        "data_dir",
                        format!("Data directory '{}' is writable", path.display()),
                    )
                }
                Err(e) => DoctorCheck::fail(
                    "data_dir",
                    format!("Data directory write attempt in {}", path.display()),
                    format!("Data directory check failed: {}", e),
                ),
            }
        }

        fn evaluate_locale(source: &str, value: String) -> DoctorCheck {
            let lowered = value.to_ascii_lowercase();
            let display = value.clone();
            if lowered.contains("utf-8") || lowered.contains("utf8") {
                DoctorCheck::ok(
                    "locale",
                    format!("{} reports UTF-8 locale ({})", source, display),
                )
            } else {
                DoctorCheck::fail(
                    "locale",
                    format!("{} reports non-UTF-8 locale ({})", source, display.clone()),
                    format!("Locale check failed: {}={} is not UTF-8", source, display),
                )
            }
        }

        fn check_locale(override_val: Option<String>) -> DoctorCheck {
            if let Some(val) = override_val {
                return evaluate_locale("AXM_DOCTOR_LOCALE_OVERRIDE", val);
            }
            for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
                if let Ok(val) = std::env::var(key) {
                    return evaluate_locale(key, val);
                }
            }
            let candidate =
                std::env::temp_dir().join(format!("axm-doctor-診断-{}.txt", unique_suffix()));
            match std::fs::File::create(&candidate) {
                Ok(mut file) => {
                    if let Err(e) = file.write_all("✓".as_bytes()) {
                        let _ = std::fs::remove_file(&candidate);
                        return DoctorCheck::fail(
                            "locale",
                            "UTF-8 filesystem probe failed",
                            format!("Locale check failed: {}", e),
                        );
                    }
                    drop(file);
                    let _ = std::fs::remove_file(&candidate);
                    DoctorCheck::ok(
                        "locale",
                        "UTF-8 filesystem probe succeeded (fallback)".to_string(),
                    )
                }
                Err(e) => DoctorCheck::fail(
                    "locale",
                    "UTF-8 filesystem probe failed",
                    format!("Locale check failed: {}", e),
                ),
            }
        }

        let sqlite_dir = env::var("AXM_DOCTOR_SQLITE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::temp_dir());
        let data_dir = env::var("AXM_DOCTOR_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));
        let locale_override = env::var("AXM_DOCTOR_LOCALE_OVERRIDE").ok();

        let checks = vec![
            check_sqlite(&sqlite_dir),
            check_data_dir(&data_dir),
            check_locale(locale_override),
        ];

        let mut report = serde_json::Map::new();
        let mut ok_all = true;
        for check in checks {
            if !check.ok {
                ok_all = false;
                if let Some(msg) = &check.error {
                    let _ = ui::write_error(err, msg);
                }
            }
            report.insert(check.name.to_string(), check.to_value());
        }

        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&serde_json::Value::Object(report)).unwrap()
        );

        if ok_all {
            0
        } else {
            2
        }
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
            Commands::Cfg => match config::load_with_sources() {
                Ok(resolved) => {
                    let config::ConfigResolved { config, sources } = resolved;
                    let display = serde_json::json!({
                        "starting_stack": {
                            "value": config.starting_stack,
                            "source": sources.starting_stack,
                        },
                        "level": {
                            "value": config.level,
                            "source": sources.level,
                        },
                        "seed": {
                            "value": config.seed,
                            "source": sources.seed,
                        },
                        "adaptive": {
                            "value": config.adaptive,
                            "source": sources.adaptive,
                        },
                        "ai_version": {
                            "value": config.ai_version,
                            "source": sources.ai_version,
                        }
                    });
                    let _ = writeln!(out, "{}", serde_json::to_string_pretty(&display).unwrap());
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
            Commands::Stats { input } => run_stats(&input, out, err),
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
                                let prev_state = if stacks_after_hand.is_empty() {
                                    None
                                } else {
                                    Some(&stacks_after_hand)
                                };
                                validate_roster_state(prev_state, &start_map, hands, err, &mut ok);
                                starting_stacks = Some(start_map.clone());
                                stacks_after_hand = start_map;
                                if let Some(nr_obj) =
                                    v.get("net_result").and_then(|x| x.as_object())
                                {
                                    for id in nr_obj.keys() {
                                        if !stacks_after_hand.contains_key(id) {
                                            ok = false;
                                            let _ = ui::write_error(
                                                err,
                                                &format!(
                                                    "Unknown player {} in net_result at hand {}",
                                                    id, hands
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                            if let (Some(ref start_map), Some(meta_obj)) = (
                                starting_stacks.as_ref(),
                                v.get("meta").and_then(|m| m.as_object()),
                            ) {
                                let button_id = v.get("button").and_then(|b| b.as_str());
                                if let Err(msg) =
                                    validate_dealing_meta(meta_obj, button_id, start_map, hands)
                                {
                                    ok = false;
                                    let _ = ui::write_error(err, &msg);
                                }
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
                            match serde_json::from_value::<axm_engine::logger::HandRecord>(
                                v.clone(),
                            ) {
                                Ok(rec) => {
                                    if rec.board.len() != 5 {
                                        ok = false;
                                        let _ = ui::write_error(
                                            err,
                                            &format!(
                                                "Invalid board length at hand {}: expected 5 cards but found {}",
                                                hands,
                                                rec.board.len()
                                            ),
                                        );
                                    }

                                    let mut seen_cards: HashSet<axm_engine::cards::Card> =
                                        HashSet::new();
                                    let mut duplicate_cards: HashSet<axm_engine::cards::Card> =
                                        HashSet::new();
                                    {
                                        let mut record_card = |card: axm_engine::cards::Card| {
                                            if !seen_cards.insert(card) {
                                                duplicate_cards.insert(card);
                                            }
                                        };
                                        for card in &rec.board {
                                            record_card(*card);
                                        }
                                        if let Some(players) =
                                            v.get("players").and_then(|p| p.as_array())
                                        {
                                            for player in players {
                                                let pid = player
                                                    .get("id")
                                                    .and_then(|x| x.as_str())
                                                    .unwrap_or("unknown");
                                                if let Some(hole_cards) = player
                                                    .get("hole_cards")
                                                    .and_then(|h| h.as_array())
                                                {
                                                    for card_val in hole_cards {
                                                        match serde_json::from_value::<
                                                            axm_engine::cards::Card,
                                                        >(
                                                            card_val.clone()
                                                        ) {
                                                            Ok(card) => record_card(card),
                                                            Err(_) => {
                                                                ok = false;
                                                                let _ = ui::write_error(
                                                                    err,
                                                                    &format!(
                                                                        "Invalid card specification for {} at hand {}",
                                                                        pid,
                                                                        hands
                                                                    ),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if !duplicate_cards.is_empty() {
                                        ok = false;
                                        let mut cards: Vec<String> = duplicate_cards
                                            .iter()
                                            .map(|card| format!("{:?} {:?}", card.rank, card.suit))
                                            .collect();
                                        cards.sort();
                                        let _ = ui::write_error(
                                            err,
                                            &format!(
                                                "Duplicate card(s) detected at hand {}: {}",
                                                hands,
                                                cards.join(", ")
                                            ),
                                        );
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
            Commands::Doctor => run_doctor(out, err),
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
                level,
                resume,
            } => {
                let total: usize = hands as usize;
                if total == 0 {
                    let _ = ui::write_error(err, "hands must be >= 1");
                    return 2;
                }
                let level = level.unwrap_or(1);
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
                let mut eng = Engine::new(Some(base_seed), level);
                eng.shuffle();
                let break_after = std::env::var("AXM_SIM_BREAK_AFTER")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok());
                for i in completed..total {
                    // create a fresh engine per hand to avoid residual hole cards
                    let mut e = Engine::new(Some(base_seed + i as u64), level);
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
                            "level": level,
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
                    f if f.eq_ignore_ascii_case("sqlite") => export_sqlite(&content, &output, err),
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
                if let Some(code) =
                    dataset_stream_if_needed(&input, &outdir, train, val, test, seed, err)
                {
                    return code;
                }
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
                let splits = match compute_splits(train, val, test) {
                    Ok(v) => v,
                    Err(msg) => {
                        let _ = ui::write_error(err, &msg);
                        return 2;
                    }
                };
                let tr = splits[0];
                let va = splits[1];
                let te = splits[2];
                let sum = tr + va + te;
                if (sum - 1.0).abs() > 1e-6 {
                    let _ = ui::write_error(err, "Splits must sum to 100% (1.0 total)");
                    return 2;
                }
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
                lines.shuffle(&mut rng);
                let n_tr = ((tr * n as f64).round() as usize).min(n);
                let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));
                let _n_te = n.saturating_sub(n_tr + n_va);
                for (idx, raw) in lines.iter().enumerate() {
                    let trimmed = raw.trim();
                    if let Err(e) = serde_json::from_str::<axm_engine::logger::HandRecord>(trimmed)
                    {
                        let _ = ui::write_error(
                            err,
                            &format!("Invalid record at line {}: {}", idx + 1, e),
                        );
                        return 2;
                    }
                }
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
        level: Option<u8>,
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
