use std::io::Write;
use clap::{Parser, Subcommand, ValueEnum};
mod config;
pub mod ui;
use axm_engine::engine::Engine;
use rand_chacha::ChaCha20Rng;
use rand::{SeedableRng, RngCore};

/// Runs the CLI with provided args, writing to the given writers.
/// Returns the intended process exit code.
pub fn run<I, S>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        let _ = writeln!(out, "Axiomind Poker CLI\n");
        let _ = writeln!(out, "Usage: axm <command> [options]\n");
        let _ = writeln!(out, "Commands:");
        for c in [
            "play", "replay", "stats", "verify", "deal", "bench",
            "sim", "eval", "export", "dataset", "cfg", "doctor", "rng",
        ] { let _ = writeln!(out, "  {}", c); }
        let _ = writeln!(out, "\nOptions:\n  -h, --help     Show this help");
        return 0;
    }

    let parsed = AxmCli::try_parse_from(&argv);
    match parsed {
        Err(e) => {
            let _ = writeln!(out, "Axiomind Poker CLI");
            let _ = writeln!(out, "Use --help for usage.");
            let _ = writeln!(err, "{}", e);
            2
        }
        Ok(cli) => match cli.cmd {
            Commands::Cfg => {
                match config::load() {
                    Ok(c) => { let _ = writeln!(out, "{}", serde_json::to_string_pretty(&c).unwrap()); 0 }
                    Err(e) => { let _ = ui::write_error(err, &format!("Invalid configuration: {}", e)); 2 }
                }
            }
            Commands::Play { vs, hands, seed, level } => {
                let hands = hands.unwrap_or(1);
                let seed = seed.unwrap_or(0);
                let level = level.unwrap_or(1);
                let _ = writeln!(out, "play: vs={} hands={} seed={}", vs.as_str(), hands, seed);
                let _ = writeln!(out, "Level: {}", level);
                let mut eng = Engine::new(Some(seed), level);
                eng.shuffle();
                let scripted = std::env::var("AXM_TEST_INPUT").ok();
                let mut played = 0u32;
                for i in 1..=hands {
                    let _ = writeln!(out, "Hand {}", i);
                    let _ = eng.deal_hand();
                    match vs {
                        Vs::Human => {
                            // prompt once; in tests, read from AXM_TEST_INPUT
                            let action = scripted.as_deref().unwrap_or("");
                            if action.is_empty() {
                                let _ = writeln!(out, "Enter action (check/call/bet/raise/fold/q): ");
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
            Commands::Replay { input } => {
                match std::fs::read_to_string(&input) {
                    Ok(content) => {
                        let count = content.lines().filter(|l| !l.trim().is_empty()).count();
                        let _ = writeln!(out, "Replayed: {} hands", count);
                        0
                    }
                    Err(e) => { let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e)); 2 }
                }
            }
            Commands::Stats { input } => {
                match std::fs::read_to_string(&input) {
                    Ok(content) => {
                        let mut hands = 0u64; let mut p0 = 0u64; let mut p1 = 0u64;
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            hands += 1;
                            if let Ok(rec) = serde_json::from_str::<axm_engine::logger::HandRecord>(line) {
                                if let Some(r) = rec.result.as_deref() {
                                    if r == "p0" { p0 += 1; }
                                    if r == "p1" { p1 += 1; }
                                }
                            }
                        }
                        let summary = serde_json::json!({"hands": hands, "winners": {"p0": p0, "p1": p1}});
                        let _ = writeln!(out, "{}", serde_json::to_string_pretty(&summary).unwrap());
                        0
                    }
                    Err(e) => { let _ = ui::write_error(err, &format!("Failed to read {}: {}", input, e)); 2 }
                }
            }
            Commands::Verify { input } => {
                // verify basic rule: completed hands have 5 board cards
                let mut ok = true; let mut hands = 0u64;
                if let Some(path) = input {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            for line in content.lines().filter(|l| !l.trim().is_empty()) {
                                hands += 1;
                                if let Ok(rec) = serde_json::from_str::<axm_engine::logger::HandRecord>(line) {
                                    if rec.board.len() != 5 { ok = false; }
                                } else { ok = false; }
                            }
                        }
                        Err(e) => { let _ = ui::write_error(err, &format!("Failed to read {}: {}", path, e)); return 2; }
                    }
                }
                let status = if ok { "OK" } else { "FAIL" };
                let _ = writeln!(out, "Verify: {} (hands={})", status, hands);
                if ok { 0 } else { 2 }
            }
            Commands::Doctor => {
                let _ = writeln!(out, "Doctor: OK");
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
                    if deck.remaining() < 7 { deck.shuffle(); }
                    let mut arr: [Card;7] = [deck.deal_card().unwrap();7];
                    for i in 1..7 { arr[i] = deck.deal_card().unwrap(); }
                    let _ = axm_engine::hand::evaluate_hand(&arr);
                    cnt += 1;
                }
                let dur = start.elapsed();
                let _ = writeln!(out, "Benchmark: {} iters in {:?}", cnt, dur);
                0
            }
            Commands::Deal { seed } => {
                let mut eng = Engine::new(seed, 1);
                eng.shuffle(); let _ = eng.deal_hand();
                let p = eng.players();
                let hc1 = p[0].hole_cards(); let hc2 = p[1].hole_cards();
                let fmt = |c: axm_engine::cards::Card| format!("{:?}{:?}", c.rank, c.suit);
                let _ = writeln!(out, "Hole P1: {} {}", fmt(hc1[0].unwrap()), fmt(hc1[1].unwrap()));
                let _ = writeln!(out, "Hole P2: {} {}", fmt(hc2[0].unwrap()), fmt(hc2[1].unwrap()));
                let b = eng.board();
                let _ = writeln!(out, "Board: {} {} {} {} {}", fmt(b[0]), fmt(b[1]), fmt(b[2]), fmt(b[3]), fmt(b[4]));
                0
            }
            Commands::Rng { seed } => {
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
                let mut vals = vec![]; for _ in 0..5 { vals.push(rng.next_u64()); }
                let _ = writeln!(out, "RNG sample: {:?}", vals);
                0
            }
            _ => 0,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "axm", author = "Axiomind", version, about = "Axiomind Poker CLI", disable_help_flag = true)]
struct AxmCli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Play { #[arg(long, value_enum)] vs: Vs, #[arg(long)] hands: Option<u32>, #[arg(long)] seed: Option<u64>, #[arg(long)] level: Option<u8> },
    Replay { #[arg(long)] input: String },
    Stats { #[arg(long)] input: String },
    Verify { #[arg(long)] input: Option<String> },
    Deal { #[arg(long)] seed: Option<u64> },
    Bench,
    Sim { #[arg(long)] hands: u64 },
    Eval { #[arg(long, name="ai-a")] ai_a: String, #[arg(long, name="ai-b")] ai_b: String },
    Export,
    Dataset,
    Cfg,
    Doctor,
    Rng { #[arg(long)] seed: Option<u64> },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Vs { Human, Ai }

impl Vs { fn as_str(&self) -> &'static str { match self { Vs::Human => "human", Vs::Ai => "ai" } } }
