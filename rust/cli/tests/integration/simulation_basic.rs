use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

#[test]
fn d1_sim_requires_hands() {
    let cli = CliRunner::new().expect("init");
    let res = cli.run(&["sim"]);
    assert_ne!(res.exit_code, 0);
    assert!(res.stderr.to_lowercase().contains("required"), "stderr should mention required argument: {}", res.stderr);
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
    let path = tfm.create_file("resume.jsonl", "{\"hand_id\":\"19700101-000001\"}\n{\"hand_id\":\"19700101-000001\"}\n").unwrap();
    let cli = CliRunner::new().expect("init");
    let res = cli.run(&["sim", "--hands", "3", "--resume", &path.to_string_lossy()]);
    // Should resume from 1 unique line, warn about 1 duplicate
    assert_eq!(res.exit_code, 0);
    assert!(res.stdout.contains("Resumed from 1"), "stdout: {}", res.stdout);
    assert!(res.stderr.to_lowercase().contains("duplicate"), "stderr should warn about duplicates: {}", res.stderr);
}

