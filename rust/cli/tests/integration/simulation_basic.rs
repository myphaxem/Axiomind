use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

#[test]
fn d1_sim_requires_hands() {
    let cli = CliRunner::new().expect("init");
    let res = cli.run(&["sim"]);
    assert_ne!(res.exit_code, 0);
    assert!(
        res.stderr.to_lowercase().contains("required"),
        "stderr should mention required argument: {}",
        res.stderr
    );
}

#[test]
fn d2_sim_rejects_zero_hands() {
    let cli = CliRunner::new().expect("init");
    let res = cli.run(&["sim", "--hands", "0"]);
    assert_ne!(res.exit_code, 0);
    assert!(res.stderr.to_lowercase().contains("hands"));
}

#[test]
fn d3_resume_duplicate_hand_ids_are_detected_and_skipped() {
    // Prepare a resume file with duplicate hand_id entries
    let tfm = TempFileManager::new().expect("tfm");
    let path = tfm
        .create_file(
            "resume.jsonl",
            "{\"hand_id\":\"19700101-000001\"}\n{\"hand_id\":\"19700101-000001\"}\n",
        )
        .unwrap();
    let cli = CliRunner::new().expect("init");
    let res = cli.run(&["sim", "--hands", "3", "--resume", &path.to_string_lossy()]);
    // Should resume from 1 unique line, warn about 1 duplicate
    assert_eq!(res.exit_code, 0);
    assert!(
        res.stdout.contains("Resumed from 1"),
        "stdout: {}",
        res.stdout
    );
    assert!(
        res.stderr.to_lowercase().contains("duplicate"),
        "stderr should warn about duplicates: {}",
        res.stderr
    );
}

#[test]
fn d4_sim_outputs_identical_for_same_seed_and_level() {
    let tfm = TempFileManager::new().expect("tfm");
    let path1 = tfm
        .create_file("sim/run1.jsonl", "")
        .expect("run1");
    let path2 = tfm
        .create_file("sim/run2.jsonl", "")
        .expect("run2");

    let cli = CliRunner::new().expect("cli");
    let out1 = path1.to_string_lossy().to_string();
    let out2 = path2.to_string_lossy().to_string();

    let res1 = cli.run(&[
        "sim",
        "--hands",
        "3",
        "--seed",
        "7",
        "--level",
        "2",
        "--output",
        &out1,
    ]);
    assert_eq!(res1.exit_code, 0, "first sim failed: {}", res1.stderr);

    let res2 = cli.run(&[
        "sim",
        "--hands",
        "3",
        "--seed",
        "7",
        "--level",
        "2",
        "--output",
        &out2,
    ]);
    assert_eq!(res2.exit_code, 0, "second sim failed: {}", res2.stderr);

    assert_eq!(res1.stdout, res2.stdout, "stdout mismatch");

    let content1 = std::fs::read_to_string(&path1).expect("read run1");
    let content2 = std::fs::read_to_string(&path2).expect("read run2");
    assert_eq!(content1, content2, "JSONL outputs differ");
    assert!(
        content1.contains("\"level\":2"),
        "expected level field in JSONL: {}",
        content1
    );
}
