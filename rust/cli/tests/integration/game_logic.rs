use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;
use serde_json::{json, Value};
use std::path::PathBuf;

fn standard_board() -> Vec<Value> {
    vec![
        json!({"rank": "Ace", "suit": "Hearts"}),
        json!({"rank": "King", "suit": "Diamonds"}),
        json!({"rank": "Queen", "suit": "Spades"}),
        json!({"rank": "Jack", "suit": "Clubs"}),
        json!({"rank": "Ten", "suit": "Hearts"}),
    ]
}

fn write_records(tfm: &TempFileManager, name: &str, records: &[Value]) -> PathBuf {
    let serialized = records
        .iter()
        .map(|rec| serde_json::to_string(rec).expect("serialize record"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut content = serialized;
    content.push('\n');
    tfm.create_file(name, &content).expect("create file")
}

#[test]
fn b1_verify_rejects_additional_hands_after_bust() {
    let tfm = TempFileManager::new().expect("temp dir");
    let bust_hand = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": -100, "p1": 100},
        "end_reason": "player_bust",
        "ts": "2025-01-01T00:00:00Z"
    });
    let continued_hand = json!({
        "hand_id": "19700101-000002",
        "seed": 2,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 0},
            {"id": "p1", "stack_start": 200}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": 0, "p1": 0},
        "end_reason": "continue",
        "ts": "2025-01-01T00:02:00Z"
    });
    let path = write_records(&tfm, "stack_zero.jsonl", &[bust_hand, continued_hand]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when hands continue after bust"
    );
    assert!(
        res.stderr.to_lowercase().contains("stack"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn b2_verify_chip_conservation_passes_when_sum_zero() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 50, "p1": -50},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "ok.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_eq!(res.exit_code, 0, "verify should pass: {}", res.stderr);
}

#[test]
fn b2_verify_chip_conservation_fails_when_sum_nonzero() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 50, "p1": -40},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "bad.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(res.exit_code, 0);
    assert!(
        res.stderr.to_lowercase().contains("chip"),
        "stderr: {}",
        res.stderr
    );
}
#[test]
fn b3_verify_rejects_invalid_chip_unit_bet() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 100},
            {"id": "p1", "stack_start": 100}
        ],
        "actions": [
            {"player_id": 0, "street": "Preflop", "action": {"Bet": 30}}
        ],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 50, "p1": -50},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "invalid_bet.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(res.exit_code, 0, "verify should fail on invalid bet size");
    assert!(
        res.stderr.to_lowercase().contains("bet"),
        "stderr: {}",
        res.stderr
    );
}
#[test]
fn b4_verify_rejects_under_minimum_raise_delta() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 1000}
        ],
        "actions": [
            {"player_id": 0, "street": "Preflop", "action": {"Bet": 100}},
            {"player_id": 1, "street": "Preflop", "action": {"Raise": 50}}
        ],
        "board": standard_board(),
        "result": "p1",
        "showdown": null,
        "net_result": {"p0": -100, "p1": 100},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:00:00Z"
    });
    let path = write_records(&tfm, "invalid_raise.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(res.exit_code, 0, "verify should fail on short raise");
    assert!(
        res.stderr.to_lowercase().contains("raise"),
        "stderr: {}",
        res.stderr
    );
}
#[test]
fn b5_verify_flags_raise_after_short_all_in() {
    let tfm = TempFileManager::new().expect("temp dir");
    let record = json!({
        "hand_id": "19700101-000001",
        "seed": 1,
        "level": 1,
        "blinds": {"sb": 50, "bb": 100},
        "button": "BTN",
        "players": [
            {"id": "p0", "stack_start": 1000},
            {"id": "p1", "stack_start": 250}
        ],
        "actions": [
            {"player_id": 0, "street": "Preflop", "action": {"Bet": 200}},
            {"player_id": 1, "street": "Preflop", "action": "AllIn"},
            {"player_id": 0, "street": "Preflop", "action": {"Raise": 200}}
        ],
        "board": standard_board(),
        "result": "p0",
        "showdown": null,
        "net_result": {"p0": 250, "p1": -250},
        "end_reason": "showdown",
        "ts": "2025-01-01T00:05:00Z"
    });
    let path = write_records(&tfm, "reopen_after_allin.jsonl", &[record]);

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["verify", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "verify should fail when betting reopens after short all-in"
    );
    assert!(
        res.stderr.to_lowercase().contains("all-in") || res.stderr.to_lowercase().contains("raise"),
        "stderr: {}",
        res.stderr
    );
}
