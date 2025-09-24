// Root integration test crate that wires submodules

mod helpers;
mod integration {
    // groups files under tests/integration/
    mod assertions_basic; // rust/cli/tests/integration/assertions_basic.rs (2.3)
    mod cli_basic; // rust/cli/tests/integration/cli_basic.rs
    mod config_precedence; // rust/cli/tests/integration/config_precedence.rs (3.2)
    mod evaluation_basic; // rust/cli/tests/integration/evaluation_basic.rs (6.2)
    mod file_corruption_recovery; // rust/cli/tests/integration/file_corruption_recovery.rs (5.2)
    mod file_dir_processing; // rust/cli/tests/integration/file_dir_processing.rs (5.3)
    mod file_io_basic; // rust/cli/tests/integration/file_io_basic.rs (5.1)
    mod game_logic;
    mod helpers_temp_files; // rust/cli/tests/integration/helpers_temp_files.rs (2.2 Red)
    mod simulation_basic; // rust/cli/tests/integration/simulation_basic.rs (6.1)
                          // rust/cli/tests/integration/game_logic.rs (B 4.2)

    mod data_format {
        use crate::helpers::cli_runner::CliRunner;
        use crate::helpers::temp_files::TempFileManager;

        #[test]
        fn k1_dataset_rejects_schema_mismatch() {
            let tfm = TempFileManager::new().expect("temp dir");
            let input = tfm
                .create_file(
                    "invalid.jsonl",
                    "{\"hand_id\":\"20250102-000001\",\"seed\":1,\"actions\":\"oops\",\"board\":[],\"result\":null,\"ts\":null,\"meta\":null}\n",
                )
                .expect("write invalid jsonl");
            let outdir = tfm.create_directory("out").expect("create output dir");
            let cli = CliRunner::new().expect("cli runner");
            let input_str = input.to_string_lossy().to_string();
            let outdir_str = outdir.to_string_lossy().to_string();
            let args = [
                "dataset",
                "--input",
                input_str.as_str(),
                "--outdir",
                outdir_str.as_str(),
            ];
            let res = cli.run(&args);
            assert_ne!(res.exit_code, 0, "dataset should fail for schema mismatch");
            assert!(
                res.stderr.contains("Invalid record"),
                "expected schema error, stderr={}",
                res.stderr
            );
        }

        #[test]
        fn k2_export_reports_lock_conflict() {
            use rusqlite::Connection;

            let tfm = TempFileManager::new().expect("temp dir");
            let input = tfm
                .create_file(
                    "valid.jsonl",
                    "{\"hand_id\":\"20250102-000001\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":null,\"ts\":null,\"meta\":null}\n",
                )
                .expect("write input jsonl");
            let sqlite_dir = tfm.create_directory("sqlite").expect("create sqlite dir");
            let db_path = sqlite_dir.join("locked.sqlite");

            let conn = Connection::open(&db_path).expect("open sqlite");
            conn.execute("BEGIN IMMEDIATE", []).expect("lock sqlite");

            let cli = CliRunner::new().expect("cli runner");
            let res = cli.run(&[
                "export",
                "--input",
                input.to_string_lossy().as_ref(),
                "--format",
                "sqlite",
                "--output",
                db_path.to_string_lossy().as_ref(),
            ]);

            assert_eq!(
                res.exit_code, 2,
                "export should fail while database is locked"
            );
            assert!(
                res.stderr.contains("SQLite busy"),
                "expected lock retry message, stderr={}",
                res.stderr
            );

            let _ = conn.execute("ROLLBACK", []);
        }

        #[test]
        fn k3_dataset_streams_large_input() {
            let tfm = TempFileManager::new().expect("temp dir");
            let mut content = String::new();
            for i in 0..32 {
                content.push_str(&format!(
                    "{{\"hand_id\":\"20250102-{idx:06}\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":null,\"ts\":null,\"meta\":null}}\n",
                    idx = i + 1
                ));
            }
            let input = tfm
                .create_file("bulk.jsonl", &content)
                .expect("write bulk input");
            let outdir = tfm.create_directory("dataset").expect("create dataset dir");
            let cli = CliRunner::new().expect("cli runner");
            let res = cli.run_with_env(
                &[
                    "dataset",
                    "--input",
                    input.to_string_lossy().as_ref(),
                    "--outdir",
                    outdir.to_string_lossy().as_ref(),
                ],
                &[
                    ("AXM_DATASET_STREAM_THRESHOLD", "5"),
                    ("AXM_DATASET_STREAM_TRACE", "1"),
                ],
            );

            assert_eq!(res.exit_code, 0, "dataset should succeed in streaming mode");
            assert!(
                res.stderr.contains("Streaming dataset input"),
                "expected streaming trace message, stderr={}",
                res.stderr
            );
        }
    }
}
