use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

#[test]
fn c4_incomplete_final_line_is_discarded_in_stats() {
    let tfm = TempFileManager::new().unwrap();
    let p = tfm.create_file("data.jsonl", "{\"hand_id\":\"19700101-000001\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":\"p0\",\"ts\":null,\"meta\":null}\n{\"hand_id\":\"19700101-000002\",\"seed\":2,\"actions\":[],\"board\":[],\"result\":\"p1\",\"ts\":null,\"meta\":null}\n{\"hand_id\":\"19700101-000003\"" ).unwrap();
    let cli = CliRunner::new().unwrap();
    let res = cli.run(&["stats", "--input", &p.to_string_lossy()]);
    assert_eq!(res.exit_code, 0, "stats should succeed");
    assert!(
        res.stdout.contains("\"hands\": 2"),
        "should count only valid lines: {}",
        res.stdout
    );
}

#[test]
fn c5_crlf_mixed_content_is_parsed() {
    let tfm = TempFileManager::new().unwrap();
    let crlf = "{\"hand_id\":\"19700101-000001\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":\"p0\",\"ts\":null,\"meta\":null}\r\n{\"hand_id\":\"19700101-000002\",\"seed\":2,\"actions\":[],\"board\":[],\"result\":\"p1\",\"ts\":null,\"meta\":null}\r\n";
    let p = tfm.create_file("crlf.jsonl", crlf).unwrap();
    let cli = CliRunner::new().unwrap();
    let res = cli.run(&["stats", "--input", &p.to_string_lossy()]);
    assert_eq!(res.exit_code, 0);
    assert!(res.stdout.contains("\"hands\": 2"));
}

#[test]
fn c6_non_utf8_reports_clear_error() {
    use std::fs;
    let tfm = TempFileManager::new().unwrap();
    let dir = tfm.create_directory("raw").unwrap();
    let path = dir.join("bad.jsonl");
    fs::write(&path, vec![0xff, 0xfe, 0xfd, b'\n']).unwrap();
    let cli = CliRunner::new().unwrap();
    let res = cli.run(&["stats", "--input", &path.to_string_lossy()]);
    assert_ne!(res.exit_code, 0);
    assert!(
        res.stderr.to_lowercase().contains("failed to read")
            || res.stderr.to_lowercase().contains("utf"),
        "stderr: {}",
        res.stderr
    );
}

#[test]
fn c7_zstd_compressed_jsonl_is_supported() {
    let tfm = TempFileManager::new().unwrap();
    let raw = "{\"hand_id\":\"19700101-000001\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":\"p0\",\"ts\":null,\"meta\":null}\n{\"hand_id\":\"19700101-000002\",\"seed\":2,\"actions\":[],\"board\":[],\"result\":\"p1\",\"ts\":null,\"meta\":null}\n";
    let dir = tfm.create_directory("zstd").unwrap();
    let path = dir.join("data.jsonl.zst");
    {
        let mut enc =
            zstd::stream::write::Encoder::new(std::fs::File::create(&path).unwrap(), 0).unwrap();
        use std::io::Write as _;
        enc.write_all(raw.as_bytes()).unwrap();
        enc.finish().unwrap();
    }
    let cli = CliRunner::new().unwrap();
    let res = cli.run(&["stats", "--input", &path.to_string_lossy()]);
    assert_eq!(res.exit_code, 0, "stats should succeed on zstd");
    assert!(
        res.stdout.contains("\"hands\": 2"),
        "stdout: {}",
        res.stdout
    );
}
