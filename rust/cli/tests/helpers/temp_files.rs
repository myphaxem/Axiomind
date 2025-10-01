use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TempFileManager {
    base_dir: PathBuf,
}

impl TempFileManager {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, String> {
        let pid = std::process::id();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut base = PathBuf::from("target");
        base.push(format!("ds_{}_{}_{}", pid, ts, unique));
        fs::create_dir_all(&base).map_err(|e| format!("create_dir_all: {}", e))?;
        Ok(Self { base_dir: base })
    }

    #[allow(dead_code)]
    pub fn create_directory<P: AsRef<Path>>(&self, name: P) -> Result<PathBuf, String> {
        let p = self.base_dir.join(name);
        fs::create_dir_all(&p).map_err(|e| format!("create_dir_all: {}", e))?;
        Ok(p)
    }

    #[allow(dead_code)]
    pub fn create_file<P: AsRef<Path>>(&self, name: P, content: &str) -> Result<PathBuf, String> {
        let p = self.base_dir.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("parent dir: {}", e))?;
        }
        let mut f = File::create(&p).map_err(|e| format!("create: {}", e))?;
        f.write_all(content.as_bytes())
            .map_err(|e| format!("write: {}", e))?;
        Ok(p)
    }
}

impl Drop for TempFileManager {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base_dir);
    }
}
