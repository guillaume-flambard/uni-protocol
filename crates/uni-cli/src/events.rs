//! Append-only event journal (blueprint §16) with uni.* attribute keys.
//! Filesystem-only: `.uni/events.jsonl`, one JSON object per line.
use anyhow::Result;

pub struct Event {
    pub name: &'static str,
    pub attrs: Vec<(String, String)>,
}

impl Event {
    pub fn to_json(&self, ts: &str) -> String {
        let mut obj = serde_json::Map::new();
        obj.insert("event".into(), serde_json::json!(self.name));
        obj.insert("timestamp".into(), serde_json::json!(ts));
        let mut attrs = serde_json::Map::new();
        for (k, v) in &self.attrs {
            attrs.insert(k.clone(), serde_json::json!(v));
        }
        obj.insert("attributes".into(), serde_json::Value::Object(attrs));
        serde_json::Value::Object(obj).to_string()
    }
}

pub fn journal_path() -> std::path::PathBuf {
    crate::dot_uni().join("events.jsonl")
}

pub fn append(events: &[Event]) -> Result<usize> {
    let path = journal_path();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let ts = chrono::Utc::now().to_rfc3339();
    let mut out = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    use std::io::Write;
    for e in events {
        writeln!(out, "{}", e.to_json(&ts))?;
    }
    Ok(events.len())
}

use serde::Deserialize;

#[derive(Clone, serde::Serialize, Deserialize)]
pub struct JournalEvent {
    pub event: String,
    pub timestamp: String,
    pub attributes: std::collections::BTreeMap<String, serde_json::Value>,
}

pub fn read_all() -> Result<Vec<JournalEvent>> {
    use std::io::BufRead;
    let mut out = vec![];
    if !journal_path().exists() {
        return Ok(out);
    }
    let f = std::fs::File::open(journal_path())?;
    for line in std::io::BufReader::new(f).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<JournalEvent>(&line) else {
            continue;
        };
        out.push(v);
    }
    Ok(out)
}
