use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvidenceState {
    Valid,
    Invalid,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub claim_id: String,
    pub producer: String,
    pub command: String,
    pub exit_code: i32,
    pub output_hash: String,
    pub output_excerpt: String,
    pub commit_sha: String,
    pub workspace_dirty: bool,
    pub state: EvidenceState,
    pub created_at: DateTime<Utc>,
    pub duration_ms: u128,
    /// sha256 over watched files content (empty = not content-bound)
    #[serde(default)]
    pub artifact_hash: String,
    /// sha256(ref|run|expect|expect_not|files): isolates the cache per verifier spec.
    /// Without it, two contracts sharing a claim id could reuse each other's evidence.
    #[serde(default)]
    pub fingerprint: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex_encode(h.finalize().as_slice())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn git_info(workspace: &std::path::Path) -> (String, bool) {
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "no-git".into());
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workspace)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    (sha, dirty)
}

pub fn evidence_path(dot_uni: &std::path::Path, claim_id: &str, fingerprint: &str) -> std::path::PathBuf {
    dot_uni.join("evidence").join(format!("{claim_id}__{fingerprint}.json"))
}

/// Find any evidence file for a claim (newest first) - for explain/detail views.
pub fn latest_for_claim(dot_uni: &std::path::Path, claim_id: &str) -> Option<Evidence> {
    let dir = dot_uni.join("evidence");
    let rd = std::fs::read_dir(dir).ok()?;
    let mut best: Option<Evidence> = None;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with(&format!("{claim_id}__")) && name.ends_with(".json") {
            if let Some(ev) = load_json::<Evidence>(&e.path()) {
                if best.as_ref().map(|b: &Evidence| ev.created_at > b.created_at).unwrap_or(true) {
                    best = Some(ev);
                }
            }
        }
    }
    best
}

pub fn save_json(path: &std::path::Path, value: &impl Serialize) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let text = serde_json::to_string_pretty(value)?;
    // Atomic write: tmp sibling + rename, so a concurrent or killed writer
    // can never leave a half-written JSON behind.
    let tmp = path.with_extension(format!(
        "tmp-{}",
        std::process::id()
    ));
    std::fs::write(&tmp, text)?;
    #[cfg(windows)]
    let _ = std::fs::remove_file(path);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Exclusive, process-safe lock for the persist phase (`.uni/.lock`).
/// Std-only (create_new is atomic): no new dependency, works on Windows.
/// A lock older than STALE_AFTER is treated as orphaned (crashed holder) and reaped.
pub struct DirLock {
    path: std::path::PathBuf,
}

const STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(600);
const ACQUIRE_RETRIES: u32 = 100;
const ACQUIRE_WAIT: std::time::Duration = std::time::Duration::from_millis(50);

pub fn acquire_lock(dot_uni: &std::path::Path) -> Result<DirLock> {
    acquire_lock_with(dot_uni, ACQUIRE_RETRIES)
}

pub fn acquire_lock_with(dot_uni: &std::path::Path, retries: u32) -> Result<DirLock> {
    std::fs::create_dir_all(dot_uni)?;
    let path = dot_uni.join(".lock");
    for _ in 0..retries.max(1) {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(DirLock { path }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(mtime) = meta.modified() {
                        if mtime.elapsed().unwrap_or_default() > STALE_AFTER {
                            let _ = std::fs::remove_file(&path);
                            continue;
                        }
                    }
                }
                std::thread::sleep(ACQUIRE_WAIT);
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(anyhow::anyhow!(
        "another uni process holds {} (stale locks reap after 10min)",
        path.display()
    ))
}

impl Drop for DirLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn is_stale(ev: &Evidence, current_sha: &str, current_dirty: bool) -> bool {
    // Any commit change or dirty transition invalidates bound evidence (FR-013 minimal).
    ev.commit_sha != current_sha || ev.workspace_dirty != current_dirty
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub by_claim: Vec<Evidence>,
}

pub fn load_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok())
}

/// Load persisted evidence for a claim; re-validate against current git state.
/// Stale/absent evidence is NOT trusted: caller must re-run the verifier.
pub fn load_valid_for_claim(
    dot_uni: &std::path::Path,
    claim_id: &str,
    fingerprint: &str,
    cur_sha: &str,
    cur_dirty: bool,
    current_artifact_hash: Option<&str>,
) -> Option<Evidence> {
    let path = evidence_path(dot_uni, claim_id, fingerprint);
    let mut ev: Evidence = load_json(&path)?;
    if ev.fingerprint != fingerprint {
        return None; // foreign or pre-fingerprint artifact
    }
    if is_stale(&ev, cur_sha, cur_dirty) {
        ev.state = EvidenceState::Stale;
        return None;
    }
    // content-bound evidence must match the watched files' current content (FR-010/FR-013)
    if !ev.artifact_hash.is_empty() && current_artifact_hash != Some(ev.artifact_hash.as_str()) {
        ev.state = EvidenceState::Stale;
        return None;
    }
    if ev.state == EvidenceState::Invalid {
        return None;
    }
    Some(ev)
}
