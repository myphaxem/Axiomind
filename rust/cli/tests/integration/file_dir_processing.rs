use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

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
