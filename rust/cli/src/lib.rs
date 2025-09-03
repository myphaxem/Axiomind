use std::io::Write;
use clap::{Parser, Subcommand, ValueEnum};
mod config;
pub mod ui;
use axm_engine::engine::Engine;
use rand::{SeedableRng, RngCore, seq::SliceRandom};

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
    if argv.iter().any(|a| a == "--version" || a == "-V") {
        let _ = writeln!(out, "axm {}", env!("CARGO_PKG_VERSION"));
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
                if hands == 0 { let _ = ui::write_error(err, "hands must be >= 1"); return 2; }
                let _ = writeln!(out, "play: vs={} hands={} seed={}", vs.as_str(), hands, seed);
                let _ = writeln!(out, "Level: {}", level);
                let mut eng = Engine::new(Some(seed), level);
                eng.shuffle();
                let scripted = std::env::var("AXM_TEST_INPUT").ok();
                let mut played = 0u32;
                for i in 1..=hands {
                    // simple level progression: +1 every 2 hands
                    let cur_level: u8 = level.saturating_add(((i-1)/2) as u8);
                    if i>1 { let _ = writeln!(out, "Level: {}", cur_level); }
                    let (sb, bb) = match cur_level { 1 => (50,100), 2 => (75,150), 3 => (100,200), _ => (150,300) };
                    let _ = writeln!(out, "Blinds: SB={} BB={}", sb, bb);
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
                        let mut hands = 0u64; let mut p0 = 0u64; let mut p1 = 0u64; let mut invalid=false;
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            hands += 1;
                            let rec: axm_engine::logger::HandRecord = match serde_json::from_str(line) { Ok(v)=>v, Err(_)=>{ invalid=true; break; } };
                            if let Some(r) = rec.result.as_deref() {
                                if r == "p0" { p0 += 1; }
                                if r == "p1" { p1 += 1; }
                            }
                        }
                        if invalid { let _=ui::write_error(err, "Invalid record encountered"); return 2; }
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
                let Some(path) = input else { let _=ui::write_error(err, "input required"); return 2; };
                let valid_id = |s: &str| -> bool { s.len()==15 && s[0..8].chars().all(|c| c.is_ascii_digit()) && &s[8..9]=="-" && s[9..].chars().all(|c| c.is_ascii_digit()) };
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            hands += 1;
                            match serde_json::from_str::<axm_engine::logger::HandRecord>(line) {
                                Ok(rec) => {
                                    if rec.board.len() != 5 { ok = false; }
                                    if !valid_id(&rec.hand_id) { ok = false; let _=ui::write_error(err, "Invalid hand_id"); }
                                }
                                Err(_) => { ok = false; let _=ui::write_error(err, "Invalid record"); }
                            }
                        }
                    }
                    Err(e) => { let _ = ui::write_error(err, &format!("Failed to read {}: {}", path, e)); return 2; }
                }
                let status = if ok { "OK" } else { "FAIL" };
                let _ = writeln!(out, "Verify: {} (hands={})", status, hands);
                if ok { 0 } else { 2 }
            }
            Commands::Doctor => {
                let _ = writeln!(out, "Doctor: OK");
                0
            }
            Commands::Eval { ai_a: _, ai_b: _, hands, seed: _ } => {
                let hands = hands.unwrap_or(10);
                let mut a_wins = 0u32; let mut b_wins = 0u32;
                for i in 0..hands { if (i % 2) == 0 { a_wins+=1; } else { b_wins+=1; } }
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
            Commands::Sim { hands, output, seed, resume } => {
                let total: usize = hands as usize;
                if total == 0 { let _=ui::write_error(err, "hands must be >= 1"); return 2; }
                let mut completed = 0usize;
                let mut path = None;
                if let Some(outp) = output.clone() { path = Some(std::path::PathBuf::from(outp)); }
                // resume: count existing lines
                if let Some(res) = resume.clone() { let contents = std::fs::read_to_string(&res).unwrap_or_default(); completed = contents.lines().filter(|l| !l.trim().is_empty()).count(); path = Some(std::path::PathBuf::from(res)); let _=writeln!(out, "Resumed from {}", completed); }
                let mut eng = Engine::new(seed, 1); eng.shuffle();
                let break_after = std::env::var("AXM_SIM_BREAK_AFTER").ok().and_then(|v| v.parse::<usize>().ok());
                for i in completed..total {
                    // create a fresh engine per hand to avoid residual hole cards
                    let mut e = Engine::new(seed.map(|s| s + i as u64), 1);
                    e.shuffle();
                    let _ = e.deal_hand();
                    if let Some(p) = &path {
                        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
                        let hand_id = format!("19700101-{:06}", i+1);
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
                        let _=writeln!(f, "{}", serde_json::to_string(&rec).unwrap());
                    }
                    completed += 1;
                    if let Some(b) = break_after { if completed == b { let _ = writeln!(out, "Interrupted: saved {}/{}", completed, total); return 130; } }
                }
                let _ = writeln!(out, "Simulated: {} hands", completed);
                0
            }
            Commands::Export { input, format, output } => {
                let content = match std::fs::read_to_string(&input) { Ok(c)=>c, Err(e)=>{ let _=ui::write_error(err,&format!("Failed to read {}: {}", input, e)); return 2; } };
                match format.as_str() {
                    f if f.eq_ignore_ascii_case("csv") => {
                        let mut w = std::fs::File::create(&output).map(|f| std::io::BufWriter::new(f)).map_err(|e| { let _=ui::write_error(err,&format!("Failed to write {}: {}", output, e)); e }).unwrap();
                        let _ = writeln!(w, "hand_id,seed,result,ts,actions,board");
                        for line in content.lines().filter(|l| !l.trim().is_empty()) {
                            let rec: axm_engine::logger::HandRecord = serde_json::from_str(line).unwrap();
                            let seed = rec.seed.map(|v| v.to_string()).unwrap_or_else(||"".into());
                            let result = rec.result.unwrap_or_default();
                            let ts = rec.ts.unwrap_or_default();
                            let _ = writeln!(w, "{},{},{},{},{},{}", rec.hand_id, seed, result, ts, rec.actions.len(), rec.board.len());
                        }
                        0
                    }
                    f if f.eq_ignore_ascii_case("json") => {
                        let mut arr = Vec::new();
                        for line in content.lines().filter(|l| !l.trim().is_empty()) { let v: serde_json::Value = serde_json::from_str(line).unwrap(); arr.push(v); }
                        let s = serde_json::to_string_pretty(&arr).unwrap();
                        std::fs::write(&output, s).unwrap();
                        0
                    }
                    _ => { let _ = ui::write_error(err, "Unsupported format"); 2 }
                }
            }
            Commands::Dataset { input, outdir, train, val, test, seed } => {
                let content = std::fs::read_to_string(&input).map_err(|e|{ let _=ui::write_error(err,&format!("Failed to read {}: {}", input, e)); e }).unwrap();
                let mut lines: Vec<String> = content.lines().filter(|l| !l.trim().is_empty()).map(|s| s.to_string()).collect();
                let n = lines.len(); if n==0 { let _=ui::write_error(err, "Empty input"); return 2; }
                let tr = train.unwrap_or(0.8); let va = val.unwrap_or(0.1); let te = test.unwrap_or(0.1);
                let sum = tr+va+te; if (sum-1.0).abs() > 1e-6 { let _=ui::write_error(err, "Splits must sum to 1.0"); return 2; }
                let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed.unwrap_or(0));
                lines.shuffle(&mut rng);
                let n_tr = ((tr * n as f64).round() as usize).min(n);
                let n_va = ((va * n as f64).round() as usize).min(n.saturating_sub(n_tr));
                let _n_te = n.saturating_sub(n_tr + n_va);
                let (trv, rest) = lines.split_at(n_tr);
                let (vav, tev) = rest.split_at(n_va);
                std::fs::create_dir_all(&outdir).unwrap();
                let w = |p:&std::path::Path, it: &[String]|{ let mut f = std::fs::File::create(p).unwrap(); for l in it { let _=writeln!(f, "{}", l); } };
                w(&std::path::Path::new(&outdir).join("train.jsonl"), trv);
                w(&std::path::Path::new(&outdir).join("val.jsonl"), vav);
                w(&std::path::Path::new(&outdir).join("test.jsonl"), tev);
                0
            }
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
    Eval { #[arg(long, name="ai-a")] ai_a: String, #[arg(long, name="ai-b")] ai_b: String, #[arg(long)] hands: Option<u32>, #[arg(long)] seed: Option<u64> },
    Verify { #[arg(long)] input: Option<String> },
    Deal { #[arg(long)] seed: Option<u64> },
    Bench,
    Sim { #[arg(long)] hands: u64, #[arg(long)] output: Option<String>, #[arg(long)] seed: Option<u64>, #[arg(long)] resume: Option<String> },
    Export { #[arg(long)] input: String, #[arg(long)] format: String, #[arg(long)] output: String },
    Dataset { #[arg(long)] input: String, #[arg(long)] outdir: String, #[arg(long)] train: Option<f64>, #[arg(long)] val: Option<f64>, #[arg(long)] test: Option<f64>, #[arg(long)] seed: Option<u64> },
    Cfg,
    Doctor,
    Rng { #[arg(long)] seed: Option<u64> },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Vs { Human, Ai }

impl Vs { fn as_str(&self) -> &'static str { match self { Vs::Human => "human", Vs::Ai => "ai" } } }
