use std::io::Write;

/// Runs the CLI with provided args, writing to the given writers.
/// Returns the intended process exit code.
pub fn run<I, S>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    let cmd = argv.get(1).map(String::as_str);

    match cmd {
        None | Some("--help") | Some("-h") => {
            let _ = writeln!(out, "Axiomind Poker CLI");
            let _ = writeln!(out, "");
            let _ = writeln!(out, "Usage: axm <command> [options]");
            let _ = writeln!(out, "");
            let _ = writeln!(out, "Commands:");
            for c in [
                "play", "replay", "stats", "verify", "deal", "bench",
                "sim", "eval", "export", "dataset", "cfg", "doctor", "rng",
            ] {
                let _ = writeln!(out, "  {}", c);
            }
            let _ = writeln!(out, "");
            let _ = writeln!(out, "Options:\n  -h, --help     Show this help");
            0
        }
        Some("cfg") => {
            // Default settings per requirements: starting stacks 20000, level=1
            let _ = writeln!(
                out,
                "{{\"starting_stack\": 20000, \"level\": 1, \"seed\": null}}"
            );
            0
        }
        Some(other) => {
            let _ = writeln!(err, "Unknown command: {}", other);
            2
        }
    }
}
