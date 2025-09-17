use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

// C-series 5.1: input validation and error handling (Red)

#[test]
fn c1_replay_requires_input_arg() {
    let cli = CliRunner::new().expect("CliRunner init");
    let res = cli.run(&["replay"]); // missing --input
    assert_ne!(res.exit_code, 0);
    let err = res.stderr.to_lowercase();
    assert!(
        err.contains("required") || err.contains("usage"),
        "stderr should indicate missing required arg: {}",
        res.stderr
    );
}

#[test]
fn c2_replay_speed_validation() {
    let tfm = TempFileManager::new().expect("tfm");
    let path = tfm.create_file("in.jsonl", "").expect("file");
    let cli = CliRunner::new().expect("CliRunner init");
    let res = cli.run(&["replay", "--input", &path.to_string_lossy(), "--speed", "0"]);
    assert_ne!(res.exit_code, 0);
    assert!(
        res.stderr.to_lowercase().contains("speed"),
        "stderr should mention speed violation: {}",
        res.stderr
    );
}

#[test]
fn c3_play_vs_human_requires_tty() {
    let cli = CliRunner::new().expect("CliRunner init");
    // ensure scripted input flag does not bypass TTY check
    std::env::remove_var("AXM_TEST_INPUT");
    // force non-tty for deterministic behavior across environments
    std::env::set_var("AXM_NON_TTY", "1");
    let res = cli.run(&["play", "--vs", "human", "--hands", "1"]);
    assert_ne!(res.exit_code, 0);
    let err = res.stderr.to_lowercase();
    assert!(
        err.contains("tty") || err.contains("refuse"),
        "stderr should warn about non-tty: {}",
        res.stderr
    );
}
