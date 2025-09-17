use axm_cli::run;

#[test]
fn help_lists_expected_commands() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    // Expectation: --help shows all top-level subcommands per spec
    let _code = run(["axm", "--help"], &mut out, &mut err);
    let stdout = String::from_utf8_lossy(&out);
    for cmd in [
        "play", "replay", "stats", "verify", "deal", "bench", "sim", "eval", "export", "dataset",
        "cfg", "doctor", "rng",
    ] {
        assert!(
            stdout.contains(cmd),
            "help should list subcommand `{}`",
            cmd
        );
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

#[test]
fn play_parses_args() {
    // In non-TTY test environment, use AI opponent to validate arg parsing
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let _code = run(
        ["axm", "play", "--vs", "ai", "--hands", "3", "--seed", "42"],
        &mut out,
        &mut err,
    );
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("play: vs=ai hands=3 seed=42"));
}

#[test]
fn invalid_vs_value_shows_error() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm", "play", "--vs", "robot"], &mut out, &mut err);
    assert_ne!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    // clap error message should mention invalid value
    assert!(stderr.to_lowercase().contains("invalid value"));
}

#[test]
fn cfg_reads_env_and_file_with_validation() {
    use std::fs;
    use std::path::PathBuf;
    // Prepare config file
    let mut p = PathBuf::from("target");
    p.push(format!("axm_cfg_{}.toml", std::process::id()));
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    fs::write(&p, "seed = 456\nlevel = 3\n").unwrap();
    std::env::set_var("AXM_CONFIG", &p);
    std::env::set_var("AXM_SEED", "123"); // env should override file

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm", "cfg"], &mut out, &mut err);
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"seed\": 123"));
    assert!(stdout.contains("\"level\": 3"));

    // invalid level -> non-zero and error message
    std::env::set_var("AXM_LEVEL", "0");
    let mut out2: Vec<u8> = Vec::new();
    let mut err2: Vec<u8> = Vec::new();
    let code2 = run(["axm", "cfg"], &mut out2, &mut err2);
    assert_ne!(code2, 0);
    let stderr = String::from_utf8_lossy(&err2);
    assert!(stderr.contains("Invalid configuration"));

    std::env::remove_var("AXM_CONFIG");
    std::env::remove_var("AXM_SEED");
    std::env::remove_var("AXM_LEVEL");
}
