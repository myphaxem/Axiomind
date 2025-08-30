use std::fs;
use std::path::PathBuf;

use axm_engine::cards::{Card, Rank as R, Suit as S};
use axm_engine::logger::{HandRecord, ActionRecord, Street, HandLogger};
use axm_engine::player::PlayerAction;

fn tmp_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    p
}

#[test]
fn writes_jsonl_with_lf_only() {
    let path = tmp_path("handlog");
    let mut logger = HandLogger::create(&path).expect("create logger");
    let rec = HandRecord {
        hand_id: "20250102-000001".to_string(),
        seed: Some(1),
        actions: vec![ActionRecord { player_id: 0, street: Street::Preflop, action: PlayerAction::Check }],
        board: vec![Card { suit: S::Clubs, rank: R::Ace }],
        result: Some("p0".to_string()),
    };
    logger.write(&rec).expect("write");
    let bytes = fs::read(&path).expect("read file");
    assert!(bytes.ends_with(b"\n"));
    assert!(!bytes.contains(&b'\r'));
}

#[test]
fn sequential_ids_increment() {
    let mut logger = HandLogger::with_seq_for_test("20251231");
    assert_eq!(logger.next_id(), "20251231-000001");
    assert_eq!(logger.next_id(), "20251231-000002");
}

