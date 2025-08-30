use std::fs;
use std::path::PathBuf;
use axm_cli::run;
use axm_engine::cards::{Card, Rank as R, Suit as S};
use axm_engine::logger::{HandRecord, ActionRecord, Street};
use axm_engine::player::PlayerAction as A;

fn mk_jsonl(name: &str, n: usize) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
    let base = HandRecord { hand_id: "20250102-000001".into(), seed: Some(1), actions: vec![ActionRecord{player_id:0,street:Street::Preflop,action:A::Bet(10)}], board: vec![Card{suit:S::Clubs,rank:R::Ace}], result: Some("p0".into()), ts: None, meta: None, showdown: None };
    let mut s = String::new();
    for i in 0..n { let mut r = base.clone(); r.hand_id = format!("20250102-{:06}", i+1); if i%3==0 { r.result = Some("p1".into()); } s.push_str(&serde_json::to_string(&r).unwrap()); s.push('\n'); }
    fs::write(&p, s).unwrap();
    p
}

#[test]
fn dataset_random_split_creates_files() {
    let input = mk_jsonl("dataset_in", 10);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outdir = PathBuf::from("target").join(format!("ds_{}", std::process::id()));
    let code = run(["axm","dataset","--input", input.to_string_lossy().as_ref(), "--outdir", outdir.to_string_lossy().as_ref(), "--train","0.7","--val","0.2","--test","0.1","--seed","7"], &mut out, &mut err);
    assert_eq!(code, 0);
    let t = fs::read_to_string(outdir.join("train.jsonl")).unwrap();
    let v = fs::read_to_string(outdir.join("val.jsonl")).unwrap();
    let te = fs::read_to_string(outdir.join("test.jsonl")).unwrap();
    let cnt = t.lines().count() + v.lines().count() + te.lines().count();
    assert_eq!(cnt, 10);
}

#[test]
fn dataset_default_split() {
    let input = mk_jsonl("dataset_in_default", 5);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outdir = PathBuf::from("target").join(format!("dsd_{}", std::process::id()));
    let code = run(["axm","dataset","--input", input.to_string_lossy().as_ref(), "--outdir", outdir.to_string_lossy().as_ref()], &mut out, &mut err);
    assert_eq!(code, 0);
    let t = fs::read_to_string(outdir.join("train.jsonl")).unwrap();
    assert!(t.lines().count() >= 3); // 80%
}
