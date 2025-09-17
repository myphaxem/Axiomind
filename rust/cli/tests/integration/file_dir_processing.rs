use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

use serde_json::json;

#[test]
fn c8_stats_scans_directory_recursively_and_reports_warnings() {
    let tfm = TempFileManager::new().unwrap();
    let dir = tfm.create_directory("data").unwrap();
    // data/f1.jsonl -> 2 valid
    let f1 = dir.join("f1.jsonl");
    std::fs::write(&f1, b"{\"hand_id\":\"19700101-000001\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":\"p0\",\"ts\":null,\"meta\":null}\n{\"hand_id\":\"19700101-000002\",\"seed\":2,\"actions\":[],\"board\":[],\"result\":\"p1\",\"ts\":null,\"meta\":null}\n").unwrap();
    // (omit f2: focus on incomplete-final-line aggregation in directory)
    // data/nested/f3.jsonl -> 1 valid + incomplete final line
    let nested = dir.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let f3 = nested.join("f3.jsonl");
    std::fs::write(&f3, b"{\"hand_id\":\"19700101-000004\",\"seed\":4,\"actions\":[],\"board\":[],\"result\":\"p1\",\"ts\":null,\"meta\":null}\n{\"hand_id\":\"19700101-000005\"").unwrap();

    let cli = CliRunner::new().unwrap();
    let res = cli.run(&["stats", "--input", &dir.to_string_lossy()]);
    assert_eq!(
        res.exit_code, 0,
        "stats on directory should succeed, stderr: {}",
        res.stderr
    );
    // Expect 3 valid hands counted
    assert!(
        res.stdout.contains("\"hands\": 3"),
        "stdout: {}",
        res.stdout
    );
    // Warnings aggregated
    let err = res.stderr.to_lowercase();
    assert!(
        err.contains("discarded 1 incomplete"),
        "stderr should report incomplete final line: {}",
        res.stderr
    );
}

#[test]
fn f1_stats_flags_chip_conservation_violation() {
    let tfm = TempFileManager::new().expect("temp dir");
    let bad_record = json!({
        "hand_id": "19700101-000010",
        "seed": 10,
        "actions": [],
        "board": [],
        "result": "p0",
        "net_result": {"p0": 75, "p1": -50},
        "ts": null,
        "meta": null
    });
    let path = tfm
        .create_file(
            "stats/bad.jsonl",
            format!("{}\n", serde_json::to_string(&bad_record).unwrap()).as_str(),
        )
        .expect("bad file");

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["stats", "--input", &path.to_string_lossy()]);
    assert_ne!(
        res.exit_code, 0,
        "stats must fail on chip conservation violation"
    );
    assert!(
        res.stderr.to_lowercase().contains("chip"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn f2_stats_accepts_chip_conserving_records() {
    let tfm = TempFileManager::new().expect("temp dir");
    let rec1 = json!({
        "hand_id": "19700101-000011",
        "seed": 11,
        "actions": [],
        "board": [],
        "result": "p0",
        "net_result": {"p0": 50, "p1": -50},
        "ts": null,
        "meta": null
    });
    let rec2 = json!({
        "hand_id": "19700101-000012",
        "seed": 12,
        "actions": [],
        "board": [],
        "result": "p1",
        "net_result": {"p0": -30, "p1": 30},
        "ts": null,
        "meta": null
    });
    let path = tfm
        .create_file(
            "stats/good.jsonl",
            format!(
                "{}\n{}\n",
                serde_json::to_string(&rec1).unwrap(),
                serde_json::to_string(&rec2).unwrap()
            )
            .as_str(),
        )
        .expect("good file");

    let cli = CliRunner::new().expect("cli runner");
    let res = cli.run(&["stats", "--input", &path.to_string_lossy()]);
    assert_eq!(
        res.exit_code, 0,
        "stats should pass for chip-conserving records"
    );
    assert!(
        res.stdout.contains("\"hands\": 2"),
        "stdout: {}",
        res.stdout
    );
}
