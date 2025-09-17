// 2.3: Poker-specific assertion helpers (Red)

use crate::helpers::assertions;
use crate::helpers::assertions::PokerAssertions;
use crate::helpers::cli_runner::CliRunner;

#[test]
fn assertions_help_contains_all_commands() {
    let cli = CliRunner::new().expect("CliRunner init");
    let res = cli.run(&["--help"]);
    assert_eq!(res.exit_code, 0);
    assertions::asserter().assert_help_contains_commands(&res.stdout);
}

#[test]
fn assertions_jsonl_required_fields_and_chip_conservation() {
    // Two JSONL records with net_result summing to zero
    let jsonl = r#"{"hand_id":"20250101-000001","seed":123,"level":1,"blinds":[50,100],"button":"BTN","players":["p0","p1"],"actions":[],"board":["Ah","Kd","7s","2c","9h"],"showdown":null,"net_result":{"p0":100,"p1":-100},"end_reason":"fold","timestamp":"2025-01-01T00:00:00Z"}
{"hand_id":"20250101-000002","seed":124,"level":1,"blinds":[50,100],"button":"BTN","players":["p0","p1"],"actions":[],"board":["As","Ks","Qs","Js","Ts"],"showdown":null,"net_result":{"p0":-50,"p1":50},"end_reason":"showdown","timestamp":"2025-01-01T00:01:00Z"}"#;

    let a = assertions::asserter();
    a.assert_jsonl_format(jsonl);
    a.assert_required_fields(
        jsonl,
        &[
            "hand_id",
            "seed",
            "level",
            "blinds",
            "button",
            "players",
            "actions",
            "board",
            "showdown",
            "net_result",
            "end_reason",
            "timestamp",
        ],
    );
    a.assert_chip_conservation(jsonl);
    a.assert_valid_hand_id("20250101-000001");
}

#[test]
fn assertions_deterministic_output() {
    let a = assertions::asserter();
    a.assert_deterministic_output(42, "OUTPUT", "OUTPUT");
}
