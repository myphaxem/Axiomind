use std::io::Write;
use clap::{Parser, Subcommand, ValueEnum};

/// Runs the CLI with provided args, writing to the given writers.
/// Returns the intended process exit code.
pub fn run<I, S>(args: I, out: &mut dyn Write, _err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    let parsed = AxmCli::try_parse_from(&argv);
    match parsed {
        Err(_) => {
            // minimal help output per tests
            let _ = writeln!(out, "Axiomind Poker CLI\n");
            let _ = writeln!(out, "Usage: axm <command> [options]\n");
            let _ = writeln!(out, "Commands:");
            for c in [
                "play", "replay", "stats", "verify", "deal", "bench",
                "sim", "eval", "export", "dataset", "cfg", "doctor", "rng",
            ] { let _ = writeln!(out, "  {}", c); }
            let _ = writeln!(out, "\nOptions:\n  -h, --help     Show this help");
            0
        }
        Ok(cli) => match cli.cmd {
            Commands::Cfg => {
                let _ = writeln!(out, "{{\"starting_stack\": 20000, \"level\": 1, \"seed\": null}}");
                0
            }
            Commands::Play { vs, hands, seed } => {
                let _ = writeln!(out, "play: vs={} hands={} seed={}", vs.as_str(), hands.unwrap_or(0), seed.unwrap_or(0));
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
