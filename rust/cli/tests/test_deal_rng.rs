use axm_cli::run;

#[test]
fn deal_prints_board_and_holes() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","deal","--seed","1"], &mut out, &mut err);
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Hole P1:"));
    assert!(s.contains("Board:"));
}

#[test]
fn rng_prints_sample() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","rng","--seed","2"], &mut out, &mut err);
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("RNG sample:"));
}

