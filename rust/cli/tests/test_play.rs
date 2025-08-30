use std::env;
use axm_cli::run;

#[test]
fn human_quick_quit_via_test_input() {
    env::set_var("AXM_TEST_INPUT", "q\n");
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","play","--vs","human","--hands","1","--seed","42"], &mut out, &mut err);
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Hand 1"));
    assert!(stdout.to_lowercase().contains("completed"));
}

#[test]
fn ai_mode_runs_noninteractive() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","play","--vs","ai","--hands","2","--seed","7"], &mut out, &mut err);
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Hands played: 2"));
}

