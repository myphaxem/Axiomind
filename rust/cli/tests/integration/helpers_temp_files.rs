// 2.2: Temporary file management utilities (Red)
// This test assumes helpers::temp_files::TempFileManager exists.

use crate::helpers;

#[test]
fn tfm_creates_files_and_dirs() {
    let tfm = helpers::temp_files::TempFileManager::new().expect("TempFileManager::new should succeed");
    let dir = tfm.create_directory("work").expect("create_directory");
    assert!(dir.exists(), "work dir should exist");

    let f = tfm.create_file("work/sample.txt", "hello").expect("create_file");
    assert!(f.exists(), "file should exist");

    let content = std::fs::read_to_string(&f).expect("read");
    assert_eq!(content, "hello");
}

