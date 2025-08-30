use std::fs;
use std::path::PathBuf;
use axm_cli::run;
use axm_engine::cards::{Card, Rank as R, Suit as S};
use axm_engine::logger::{HandRecord, ActionRecord, Street};
use axm_engine::player::PlayerAction as A;

fn tmp_jsonl(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
    p
}

#[test]
fn verify_checks_records() {
    let path = tmp_jsonl("verify");
    // valid completed board (5 cards)
    let rec = HandRecord {
        hand_id: "20250102-000001".into(), seed: Some(1), actions: vec![ActionRecord{player_id:0,street:Street::River,action:A::Check}],
        board: vec![
            Card{suit:S::Clubs,rank:R::Ace},Card{suit:S::Diamonds,rank:R::Two},Card{suit:S::Hearts,rank:R::Three},
            Card{suit:S::Spades,rank:R::Four},Card{suit:S::Clubs,rank:R::Five}
        ], result: Some("p0".into()), ts: None, meta: None, showdown: None
    };
    let mut s = String::new(); s.push_str(&serde_json::to_string(&rec).unwrap()); s.push('\n');
    fs::write(&path, s).unwrap();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","verify","--input", path.to_string_lossy().as_ref()], &mut out, &mut err);
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Verify: OK"));
    assert!(stdout.contains("hands=1"));
}

#[test]
fn doctor_reports_ok() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","doctor"], &mut out, &mut err);
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Doctor: OK"));
}

#[test]
fn bench_runs_quickly() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(["axm","bench"], &mut out, &mut err);
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Benchmark:"));
}
