use std::io::Write;
use clap::{Parser, Subcommand, ValueEnum};
mod config;
pub mod ui;
use axm_engine::engine::Engine;
use rand_chacha::ChaCha20Rng;
use rand::SeedableRng;

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
            Commands::Play { vs, hands, seed } => {
                let hands = hands.unwrap_or(1);
                let seed = seed.unwrap_or(0);
                let _ = writeln!(out, "play: vs={} hands={} seed={}", vs.as_str(), hands, seed);
                let mut eng = Engine::new(Some(seed), 1);
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
                let _ = writeln!(out, "Hands played: {} (completed)", played);
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
    Play { #[arg(long, value_enum)] vs: Vs, #[arg(long)] hands: Option<u32>, #[arg(long)] seed: Option<u64> },
    Replay { #[arg(long)] input: String },
    Stats { #[arg(long)] input: String },
    Verify,
    Deal,
    Bench,
    Sim { #[arg(long)] hands: u64 },
    Eval { #[arg(long, name="ai-a")] ai_a: String, #[arg(long, name="ai-b")] ai_b: String },
    Export,
    Dataset,
    Cfg,
    Doctor,
    Rng,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Vs { Human, Ai }

impl Vs { fn as_str(&self) -> &'static str { match self { Vs::Human => "human", Vs::Ai => "ai" } } }
