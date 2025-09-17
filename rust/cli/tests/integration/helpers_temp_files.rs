// 2.2: Temporary file management utilities (Red)
// This test assumes helpers::temp_files::TempFileManager exists.

use crate::helpers;

#[test]
fn tfm_creates_files_and_dirs() {
    let tfm =
        helpers::temp_files::TempFileManager::new().expect("TempFileManager::new should succeed");
    let dir = tfm.create_directory("work").expect("create_directory");
    assert!(dir.exists(), "work dir should exist");

    let f = tfm
        .create_file("work/sample.txt", "hello")
        .expect("create_file");
    assert!(f.exists(), "file should exist");

    let content = std::fs::read_to_string(&f).expect("read");
    assert_eq!(content, "hello");
}

#[test]
fn tfm_creates_unique_directories_under_load() {
    use std::collections::HashSet;
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;

    const WORKERS: usize = 32;
    let barrier = Arc::new(Barrier::new(WORKERS));
    let seen = Arc::new(Mutex::new(HashSet::new()));
    let mut handles = Vec::with_capacity(WORKERS);

    for _ in 0..WORKERS {
        let barrier = Arc::clone(&barrier);
        let seen = Arc::clone(&seen);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let tfm = helpers::temp_files::TempFileManager::new().expect("TempFileManager::new");
            let marker = tfm
                .create_directory("marker")
                .expect("create_directory marker");
            let base = marker.parent().expect("marker parent").to_path_buf();
            let mut guard = seen.lock().expect("lock seen");
            assert!(
                guard.insert(base.clone()),
                "duplicate TempFileManager base dir: {:?}",
                base
            );
        }));
    }

    for handle in handles {
        handle.join().expect("thread join");
    }
}
