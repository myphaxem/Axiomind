use axm_cli::run;
use axm_engine::cards::{Card, Rank as R, Suit as S};
use axm_engine::logger::{ActionRecord, HandRecord, Street};
use axm_engine::player::PlayerAction as A;
use std::fs;
use std::path::PathBuf;

fn mk_jsonl(name: &str, n: usize) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let base = HandRecord {
        hand_id: "20250102-000001".into(),
        seed: Some(1),
        actions: vec![ActionRecord {
            player_id: 0,
            street: Street::Preflop,
            action: A::Bet(10),
        }],
        board: vec![Card {
            suit: S::Clubs,
            rank: R::Ace,
        }],
        result: Some("p0".into()),
        ts: None,
        meta: None,
        showdown: None,
    };
    let mut s = String::new();
    for i in 0..n {
        let mut r = base.clone();
        r.hand_id = format!("20250102-{:06}", i + 1);
        s.push_str(&serde_json::to_string(&r).unwrap());
        s.push('\n');
    }
    fs::write(&p, s).unwrap();
    p
}

#[test]
fn export_to_csv() {
    let input = mk_jsonl("export_in", 3);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let output = input.with_extension("csv");
    let code = run(
        [
            "axm",
            "export",
            "--input",
            input.to_string_lossy().as_ref(),
            "--format",
            "csv",
            "--output",
            output.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let csv = fs::read_to_string(&output).unwrap();
    let mut lines = csv.lines();
    let header = lines.next().unwrap();
    assert!(header.contains("hand_id"));
    assert_eq!(lines.count(), 3);
}

#[test]
fn export_to_json_array() {
    let input = mk_jsonl("export_in_json", 2);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let output = input.with_extension("json");
    let code = run(
        [
            "axm",
            "export",
            "--input",
            input.to_string_lossy().as_ref(),
            "--format",
            "json",
            "--output",
            output.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let txt = fs::read_to_string(&output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    assert!(v.is_array());
    assert_eq!(v.as_array().unwrap().len(), 2);
}
