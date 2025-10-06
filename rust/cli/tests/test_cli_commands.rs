use axm_cli::run;

use once_cell::sync::Lazy;
use std::sync::Mutex;

static ENV_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[test]
fn help_lists_expected_commands() {
    let _env = ENV_GUARD.lock().unwrap();

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
    let _env = ENV_GUARD.lock().unwrap();

    for key in [
        "AXM_CONFIG",
        "AXM_SEED",
        "AXM_LEVEL",
        "AXM_ADAPTIVE",
        "AXM_AI_VERSION",
    ] {
        std::env::remove_var(key);
    }

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();

    let code = run(["axm", "cfg"], &mut out, &mut err);
    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));

    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let stack = &json["starting_stack"];
    assert_eq!(stack["value"].as_u64(), Some(20_000));
    assert_eq!(stack["source"].as_str(), Some("default"));

    let level = &json["level"];
    assert_eq!(level["value"].as_u64(), Some(1));
    assert_eq!(level["source"].as_str(), Some("default"));

    let seed = &json["seed"];
    assert!(seed["value"].is_null());
    assert_eq!(seed["source"].as_str(), Some("default"));

    let adaptive = &json["adaptive"];
    assert_eq!(adaptive["value"].as_bool(), Some(true));
    assert_eq!(adaptive["source"].as_str(), Some("default"));

    let ai_version = &json["ai_version"];
    assert_eq!(ai_version["value"].as_str(), Some("latest"));
    assert_eq!(ai_version["source"].as_str(), Some("default"));
}

#[test]
fn play_parses_args() {
    let _env = ENV_GUARD.lock().unwrap();

    // In non-TTY test environment, use AI opponent to validate arg parsing
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axm", "play", "--vs", "ai", "--hands", "2", "--seed", "42", "--level", "3",
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("play: vs=ai hands=2 seed=42"));
    assert!(stdout.contains("Level: 3"));
}

#[test]
fn invalid_vs_value_shows_error() {
    let _env = ENV_GUARD.lock().unwrap();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        ["axm", "play", "--vs", "human", "--hands", "1"],
        &mut out,
        &mut err,
    );
    assert_ne!(code, 0);
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("Non-TTY environment"));
}

#[test]
fn cfg_reads_env_and_file_with_validation() {
    let _env = ENV_GUARD.lock().unwrap();

    use std::fs;
    use std::path::PathBuf;

    let mut p = PathBuf::from("target");
    p.push(format!("axm_cfg_{}.toml", std::process::id()));
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        &p,
        "starting_stack = 25000\nlevel = 3\nadaptive = false\nai_version = \"v1\"\nseed = 456\n",
    )
    .unwrap();

    std::env::set_var("AXM_CONFIG", &p);
    std::env::set_var("AXM_SEED", "123");
    std::env::set_var("AXM_LEVEL", "4");
    std::env::set_var("AXM_ADAPTIVE", "on");
    std::env::set_var("AXM_AI_VERSION", "v2");

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm", "cfg"], &mut out, &mut err);
    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));
    let stdout = serde_json::from_slice::<serde_json::Value>(&out).unwrap();

    assert_eq!(stdout["starting_stack"]["value"].as_u64(), Some(25_000));
    assert_eq!(stdout["starting_stack"]["source"].as_str(), Some("file"));

    assert_eq!(stdout["level"]["value"].as_u64(), Some(4));
    assert_eq!(stdout["level"]["source"].as_str(), Some("env"));

    assert_eq!(stdout["seed"]["value"].as_u64(), Some(123));
    assert_eq!(stdout["seed"]["source"].as_str(), Some("env"));

    assert_eq!(stdout["adaptive"]["value"].as_bool(), Some(true));
    assert_eq!(stdout["adaptive"]["source"].as_str(), Some("env"));

    assert_eq!(stdout["ai_version"]["value"].as_str(), Some("v2"));
    assert_eq!(stdout["ai_version"]["source"].as_str(), Some("env"));

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
    std::env::remove_var("AXM_ADAPTIVE");
    std::env::remove_var("AXM_AI_VERSION");
    let _ = fs::remove_file(&p);
}
