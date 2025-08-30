use serde::{Deserialize, Serialize};

use crate::cards::Card;
use crate::player::PlayerAction;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Street { Preflop, Flop, Turn, River }

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub player_id: usize,
    pub street: Street,
    pub action: PlayerAction,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HandRecord {
    pub hand_id: String,
    pub seed: Option<u64>,
    pub actions: Vec<ActionRecord>,
    pub board: Vec<Card>,
    pub result: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

pub fn format_hand_id(yyyymmdd: &str, seq: u32) -> String {
    format!("{}-{:06}", yyyymmdd, seq)
}

use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;
use chrono::{Utc, SecondsFormat};

pub struct HandLogger {
    writer: Option<BufWriter<File>>,
    date: String,
    seq: u32,
}

impl HandLogger {
    pub fn create<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        if let Some(parent) = path.as_ref().parent() { if !parent.as_os_str().is_empty() { let _ = create_dir_all(parent); } }
        let f = File::create(path)?;
        Ok(Self { writer: Some(BufWriter::new(f)), date: "19700101".to_string(), seq: 0 })
    }

    pub fn with_seq_for_test(date: &str) -> Self {
        Self { writer: None, date: date.to_string(), seq: 0 }
    }

    pub fn next_id(&mut self) -> String {
        self.seq += 1;
        format_hand_id(&self.date, self.seq)
    }

    pub fn write(&mut self, record: &HandRecord) -> std::io::Result<()> {
        // inject timestamp if missing
        let mut rec = record.clone();
        if rec.ts.is_none() {
            rec.ts = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
        }
        let line = serde_json::to_string(&rec).expect("serialize");
        if let Some(w) = &mut self.writer { w.write_all(line.as_bytes())?; w.write_all(b"\n")?; w.flush()?; }
        Ok(())
    }
}
