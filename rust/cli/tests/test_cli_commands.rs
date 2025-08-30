use axm_cli::run;

#[test]
fn help_lists_expected_commands() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    // Expectation: --help shows all top-level subcommands per spec
    let _code = run(["axm", "--help"], &mut out, &mut err);
    let stdout = String::from_utf8_lossy(&out);
    for cmd in [
        "play", "replay", "stats", "verify", "deal", "bench",
        "sim", "eval", "export", "dataset", "cfg", "doctor", "rng",
    ] {
        assert!(stdout.contains(cmd), "help should list subcommand `{}`", cmd);
    }
}

#[test]
fn cfg_shows_default_settings() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    // Expectation: `axm cfg` prints defaults including starting_stack=20000 and level=1
    let _code = run(["axm", "cfg"], &mut out, &mut err);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"starting_stack\": 20000"));
    assert!(stdout.contains("\"level\": 1"));
}

