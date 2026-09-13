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

pub fn evidence_path(dot_uni: &std::path::Path, claim_id: &str) -> std::path::PathBuf {
    dot_uni.join("evidence").join(format!("{claim_id}.json"))
}

pub fn save_json(path: &std::path::Path, value: &impl Serialize) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
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
    cur_sha: &str,
    cur_dirty: bool,
) -> Option<Evidence> {
    let path = evidence_path(dot_uni, claim_id);
    let mut ev: Evidence = load_json(&path)?;
    if is_stale(&ev, cur_sha, cur_dirty) {
        ev.state = EvidenceState::Stale;
        return None;
    }
    if ev.state == EvidenceState::Invalid {
        return None;
    }
    Some(ev)
}
