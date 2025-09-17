use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

#[test]
fn i1_cfg_shows_defaults_for_adaptive_and_ai_version() {
    std::env::remove_var("AXM_CONFIG");
    std::env::remove_var("AXM_ADAPTIVE");
    std::env::remove_var("AXM_AI_VERSION");
    let cli = CliRunner::new().expect("init");
    let res = cli.run(&["cfg"]);
    assert_eq!(res.exit_code, 0);
    let out = res.stdout;
    assert!(
        out.contains("\"adaptive\": true"),
        "defaults should set adaptive=true: {}",
        out
    );
    assert!(
        out.contains("\"ai_version\": \"latest\""),
        "defaults should set ai_version=latest: {}",
        out
    );
}

#[test]
fn i2_precedence_cli_over_env_over_file_for_seed_and_ai() {
    let tfm = TempFileManager::new().unwrap();
    let cfg_path = tfm
        .create_file(
            "axm.toml",
            "seed = 456\nai_version = \"v1\"\nadaptive = false\n",
        )
        .unwrap();
    std::env::set_var("AXM_CONFIG", &cfg_path);
    // First, file-only precedence
    let cli = CliRunner::new().expect("init");
    let cfg1 = cli.run(&["cfg"]);
    assert_eq!(cfg1.exit_code, 0);
    assert!(cfg1.stdout.contains("\"seed\": 456"));
    assert!(cfg1.stdout.contains("\"ai_version\": \"v1\""));
    assert!(cfg1.stdout.contains("\"adaptive\": false"));

    // Next, env overrides file
    std::env::set_var("AXM_SEED", "123");
    std::env::set_var("AXM_AI_VERSION", "v2");
    std::env::set_var("AXM_ADAPTIVE", "on");
    let cfg2 = cli.run(&["cfg"]);
    assert!(cfg2.stdout.contains("\"seed\": 123"));
    assert!(cfg2.stdout.contains("\"ai_version\": \"v2\""));
    assert!(cfg2.stdout.contains("\"adaptive\": true"));

    // Finally, CLI (rng --seed) overrides env for seed determinism
    let r1 = cli.run(&["rng", "--seed", "42"]);
    let r2 = cli.run(&["rng", "--seed", "42"]);
    assert_eq!(
        r1.stdout, r2.stdout,
        "same seed should produce identical RNG output"
    );

    std::env::remove_var("AXM_CONFIG");
    std::env::remove_var("AXM_SEED");
    std::env::remove_var("AXM_AI_VERSION");
    std::env::remove_var("AXM_ADAPTIVE");
}

#[test]
fn i3_seed_default_is_non_deterministic() {
    let cli = CliRunner::new().expect("init");
    let a = cli.run(&["rng"]);
    let b = cli.run(&["rng"]);
    assert_ne!(
        a.stdout, b.stdout,
        "rng without --seed should be non-deterministic"
    );
}
