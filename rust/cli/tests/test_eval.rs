use axm_cli::run;

#[test]
fn eval_reports_summary_for_two_ais() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","eval","--ai-a","random","--ai-b","random","--hands","10","--seed","2"], &mut out, &mut err);
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Eval: hands=10"));
    assert!(s.contains("A:"));
    assert!(s.contains("B:"));
}

