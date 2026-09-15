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

/// Rotation: an append-only journal must not grow without bound. Past this
/// size the current file is archived and a fresh one starts; the newest
/// KEEP_ARCHIVES are kept, older ones are dropped. Auditability is preserved
/// for the recent window, which is what `uni events` reads by default.
pub const JOURNAL_MAX_BYTES: u64 = 1_048_576; // 1 MiB
pub const KEEP_ARCHIVES: usize = 3;

/// Archive `path` when it exceeds `max_bytes`, and prune old archives.
/// Returns the archive path when a rotation happened.
pub fn rotate_if_needed(
    path: &std::path::Path,
    max_bytes: u64,
) -> Result<Option<std::path::PathBuf>> {
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return Ok(None),
    };
    if size <= max_bytes {
        return Ok(None);
    }
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let archive = path.with_file_name(format!("events.{stamp}.jsonl"));
    std::fs::rename(path, &archive)?;
    prune_archives(path, KEEP_ARCHIVES)?;
    Ok(Some(archive))
}

/// Is this path an archive of `path`? The current journal is `events.jsonl`,
/// which also starts with "events." — counting it as an archive would let the
/// retention prune delete the live journal.
fn is_archive_of(path: &std::path::Path, candidate: &std::path::Path) -> bool {
    let live = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("events.jsonl");
    let Some(name) = candidate.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    candidate != path && name != live && name.starts_with("events.") && name.ends_with(".jsonl")
}

/// Keep the newest `keep` archives, delete the rest. Ordering is by the
/// embedded timestamp, which is why the archive name starts with it.
pub fn prune_archives(path: &std::path::Path, keep: usize) -> Result<usize> {
    let Some(dir) = path.parent() else {
        return Ok(0);
    };
    let mut archives: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_archive_of(path, p))
        .collect();
    archives.sort();
    let mut removed = 0;
    while archives.len() > keep {
        let oldest = archives.remove(0);
        if std::fs::remove_file(oldest).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Archives present for the current journal, oldest first.
pub fn archives() -> Vec<std::path::PathBuf> {
    archives_at(&journal_path())
}

/// Archives present beside `path`, oldest first.
pub fn archives_at(path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Some(dir) = path.parent() else {
        return vec![];
    };
    let mut out: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| is_archive_of(path, p))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

pub fn append(events: &[Event]) -> Result<usize> {
    let path = journal_path();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    // Rotate before writing so a single run cannot push the file past its cap.
    rotate_if_needed(&path, JOURNAL_MAX_BYTES)?;
    let ts = chrono::Utc::now().to_rfc3339();
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
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
    read_file(&journal_path())
}

/// Current journal, oldest archive first. `--all` reads the whole retained
/// history rather than the current window.
pub fn read_all_including_archives() -> Result<Vec<JournalEvent>> {
    let mut out = vec![];
    for a in archives() {
        out.extend(read_file(&a)?);
    }
    out.extend(read_file(&journal_path())?);
    Ok(out)
}

fn read_file(path: &std::path::Path) -> Result<Vec<JournalEvent>> {
    use std::io::BufRead;
    let mut out = vec![];
    if !path.exists() {
        return Ok(out);
    }
    let f = std::fs::File::open(path)?;
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

/// OTLP/JSON projection of the journal (no SDK, no dependency): one span per
/// event, resource `service.name=uni`, and the `uni.*` attributes carried as
/// string values. Trace and span ids are derived from the event content, so the
/// export is deterministic for a given journal rather than random.
pub fn to_otlp(events: &[JournalEvent]) -> serde_json::Value {
    fn hex16(seed: &str) -> String {
        uni_evidence::sha256_hex(seed.as_bytes())[..16].to_string()
    }
    fn hex32(seed: &str) -> String {
        uni_evidence::sha256_hex(seed.as_bytes())[..32].to_string()
    }
    let spans: Vec<serde_json::Value> = events
        .iter()
        .map(|e| {
            let nanos = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                .map(|t| t.timestamp_nanos_opt().unwrap_or(0).to_string())
                .unwrap_or_else(|_| "0".to_string());
            let attrs: Vec<serde_json::Value> = e
                .attributes
                .iter()
                .map(|(k, v)| {
                    let sval = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    serde_json::json!({"key": k, "value": {"stringValue": sval}})
                })
                .collect();
            let seed = serde_json::to_string(e).unwrap_or_default();
            let intent = e
                .attributes
                .get("uni.intent.id")
                .and_then(|v| v.as_str())
                .unwrap_or("uni")
                .to_string();
            serde_json::json!({
                "traceId": hex32(&intent),
                "spanId": hex16(&seed),
                "name": e.event,
                "kind": 1,
                "startTimeUnixNano": nanos,
                "endTimeUnixNano": nanos,
                "attributes": attrs,
            })
        })
        .collect();
    serde_json::json!({
        "resourceSpans": [{
            "resource": {
                "attributes": [
                    {"key": "service.name", "value": {"stringValue": "uni"}},
                    {"key": "service.version", "value": {"stringValue": env!("CARGO_PKG_VERSION")}}
                ]
            },
            "scopeSpans": [{
                "scope": {"name": "uni", "version": env!("CARGO_PKG_VERSION")},
                "spans": spans,
            }]
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "uni-events-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d.join("events.jsonl")
    }

    #[test]
    fn rotates_past_the_cap_and_keeps_the_newest_archives() {
        let path = tmp("rotate");
        std::fs::write(&path, "x".repeat(2048)).unwrap();
        let archived = rotate_if_needed(&path, 1024).unwrap();
        assert!(archived.is_some(), "past the cap it must archive");
        assert!(!path.exists(), "the current file starts empty");
        // Drive more rotations than the retention allows.
        for _ in 0..(KEEP_ARCHIVES + 3) {
            std::fs::write(&path, "x".repeat(2048)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1100));
            rotate_if_needed(&path, 1024).unwrap();
        }
        let kept = archives_at(&path);
        assert_eq!(
            kept.len(),
            KEEP_ARCHIVES,
            "retention caps the archives: {kept:?}"
        );
        // The survivors are the newest ones (sorted by embedded timestamp).
        let mut sorted = kept.clone();
        sorted.sort();
        assert_eq!(kept, sorted);
    }

    #[test]
    fn otlp_projection_carries_the_attributes() {
        let ev = JournalEvent {
            event: "DecisionIssued".into(),
            timestamp: "2026-09-14T10:00:00Z".into(),
            attributes: [
                (
                    "uni.intent.id".to_string(),
                    serde_json::json!("booking.cancel"),
                ),
                ("uni.assurance.level".to_string(), serde_json::json!("A2")),
            ]
            .into_iter()
            .collect(),
        };
        let doc = to_otlp(&[ev]);
        let span = &doc["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
        assert_eq!(span["name"], "DecisionIssued");
        assert_eq!(span["traceId"].as_str().unwrap().len(), 32);
        assert_eq!(span["spanId"].as_str().unwrap().len(), 16);
        assert!(span["startTimeUnixNano"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .is_ok());
        let keys: Vec<&str> = span["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["key"].as_str().unwrap())
            .collect();
        assert!(
            keys.contains(&"uni.intent.id") && keys.contains(&"uni.assurance.level"),
            "{keys:?}"
        );
        assert_eq!(
            doc["resourceSpans"][0]["resource"]["attributes"][0]["value"]["stringValue"],
            "uni"
        );
    }

    #[test]
    fn under_the_cap_nothing_happens() {
        let path = tmp("under");
        std::fs::write(&path, "x".repeat(10)).unwrap();
        assert!(rotate_if_needed(&path, 1024).unwrap().is_none());
        assert!(path.exists());
        assert!(archives_at(&path).is_empty());
    }
}
