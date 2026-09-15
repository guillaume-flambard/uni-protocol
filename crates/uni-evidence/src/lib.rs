use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod binding;

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
    /// Per-file hashes of the watched subject, so a stale proof can name the
    /// files that changed instead of only saying that something did.
    #[serde(default)]
    pub artifact_files: std::collections::BTreeMap<String, String>,
    /// sha256(ref|run|expect|expect_not|files): isolates the cache per verifier spec.
    /// Without it, two contracts sharing a claim id could reuse each other's evidence.
    #[serde(default)]
    pub fingerprint: String,
    /// Verification Context (v0.2): validity dimensions recorded at run time.
    /// Any drift on registry/contract/platform/artifact/commit forces re-run;
    /// policy drift forces decision recompute (verify always re-decides).
    #[serde(default)]
    pub registry_hash: String,
    #[serde(default)]
    pub policy_hash: String,
    #[serde(default)]
    pub contract_hash: String,
    /// "<os>-<arch>" at run time; cross-platform reuse is never trusted.
    #[serde(default)]
    pub platform: String,
    /// Who produced this proof (the verifier side). Defaults to anonymous for
    /// pre-B3 files, which can therefore never satisfy independence.
    #[serde(default)]
    pub actor: Actor,
    /// Who launched the verified work (defaults to the local user).
    #[serde(default)]
    pub executor: Actor,
    /// sha256(claim|verifier|requirement) of the authorizing VerifierBinding
    /// (empty when the verification carries no requirement).
    #[serde(default)]
    pub binding_hash: String,
    /// Time dimension: when this proof stops being valid (None = no expiry).
    /// Stored on the evidence so the expiry travels with the proof, and a
    /// registry change cannot silently extend it.
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

/// Current verification context, computed fresh on every verify run.
#[derive(Debug, Clone)]
pub struct EvidenceContext {
    pub fingerprint: String,
    pub commit_sha: String,
    pub workspace_dirty: bool,
    pub artifact_hash: Option<String>,
    /// Per-file hashes of the watched subject, when the verifier declares one.
    pub artifact_files: std::collections::BTreeMap<String, String>,
    pub registry_hash: String,
    pub contract_hash: String,
    pub platform: String,
    /// Expected binding hash when the verification carries a requirement
    /// (None = unconditional verification; a stored non-empty binding_hash
    /// then means the contract dropped its REQUIRE -> stale via contract_hash
    /// anyway, see cmd_verify).
    pub binding_hash: Option<String>,
}

pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
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
    // Dirtiness means "someone changed tracked content since the recorded
    // revision". Two exclusions matter:
    //  - .uni/ is UNI's own runtime state; trust-relevant parts (registry,
    //    policies) are covered by their own hashes.
    //  - untracked files are build outputs and scratch, not code: a verifier
    //    that compiles (target/, Cargo.lock, node_modules) must not stale its
    //    own fresh evidence. Watched-file drift is caught by artifact_hash.
    let dirty = std::process::Command::new("git")
        .args([
            "status",
            "--porcelain",
            "--untracked-files=no",
            "--",
            ".",
            ":!.uni",
        ])
        .current_dir(workspace)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    (sha, dirty)
}

pub fn evidence_path(
    dot_uni: &std::path::Path,
    claim_id: &str,
    fingerprint: &str,
) -> std::path::PathBuf {
    dot_uni
        .join("evidence")
        .join(format!("{claim_id}__{fingerprint}.json"))
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
                if best
                    .as_ref()
                    .map(|b: &Evidence| ev.created_at > b.created_at)
                    .unwrap_or(true)
                {
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
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
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

/// An identity attached to evidence (B3). Independence (who) and identity
/// assurance (how strongly proven) are DISTINCT properties: a self-declared
/// `--actor someone-else` is independent on paper but proves nothing about
/// identity. Only externally verified sources count toward real A3.
#[derive(Debug, Clone, serde::Serialize, Deserialize, PartialEq)]
pub struct Actor {
    /// e.g. "local:alice", "ci:build-12", "spiffe://acme/verifier/build-12"
    pub id: String,
    /// "local" | "cli" | "spiffe" | "entra" | "oidc" | ...
    pub source: String,
    /// "self-declared" | "verified". Only the identity adapter
    /// (`uni_verify::identity::verify_token`) may set "verified".
    pub assurance: String,
}

impl Default for Actor {
    fn default() -> Self {
        Actor {
            id: String::new(),
            source: "local".into(),
            assurance: "self-declared".into(),
        }
    }
}

impl Actor {
    pub fn local() -> Self {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".into());
        Actor {
            id: format!("local:{user}"),
            source: "local".into(),
            assurance: "self-declared".into(),
        }
    }

    /// Parse a `--actor` flag value. Scheme-prefixed ids keep their scheme as
    /// source but stay self-declared: a prefix is a claim about who you are,
    /// not a proof of it. Verification happens in the identity adapter, which
    /// builds a `verified` actor from a token instead.
    pub fn declared(id: &str) -> Self {
        let source = if let Some((scheme, _)) = id.split_once("://") {
            match scheme {
                "spiffe" | "entra" | "oidc" => scheme.to_string(),
                _ => "cli".to_string(),
            }
        } else {
            "cli".to_string()
        };
        Actor {
            id: id.to_string(),
            source,
            assurance: "self-declared".into(),
        }
    }

    pub fn is_anonymous(&self) -> bool {
        self.id.trim().is_empty() || self.id == "anonymous"
    }

    /// An actor whose identity was verified externally (A3). Only an identity
    /// adapter may construct one: a prefix or a flag never does.
    pub fn verified(id: &str, source: &str) -> Self {
        Actor {
            id: id.to_string(),
            source: source.to_string(),
            assurance: "verified".into(),
        }
    }

    pub fn is_verified(&self) -> bool {
        self.assurance == "verified"
    }
}

pub fn load_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

/// Cache outcome with the reason for a miss, so callers can tell "never
/// proven" (Miss) from "proven once, context drifted" (Stale). The latter
/// carries escalation weight under policy (escalate_on_stale).
#[derive(Debug, Clone)]
pub enum CacheOutcome {
    /// Boxed: `Evidence` is close to 600 bytes, and the two other variants are
    /// tiny, so an unboxed `Hit` would make every `Miss` cost 600 bytes.
    Hit(Box<Evidence>),
    /// The proof exists but no longer applies, and here is why. This is the
    /// product's central message, so it travels as data, not as a boolean.
    Stale(Vec<StaleReason>),
    Miss,
}

/// One concrete reason a previous proof stopped covering what is being
/// delivered. Every variant names the dimension of the Verification Context
/// that drifted, and carries what a human needs to act.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "dimension", rename_all = "snake_case")]
pub enum StaleReason {
    /// The artifact under proof is a different revision now.
    CommitChanged { from: String, to: String },
    /// Tracked content changed without a commit.
    UncommittedChanges { workspace_dirty: bool },
    /// Watched files changed, with the files named.
    SubjectChanged {
        changed: Vec<String>,
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// The contract that defined the claim changed.
    ContractChanged { from: String, to: String },
    /// The trusted registry changed, and the diff is named by the caller.
    VerifierConfigChanged { from: String, to: String },
    /// The evidence was produced on another platform.
    PlatformChanged { from: String, to: String },
    /// The authorized resolution changed (re-bound, or a new selector).
    AuthorizationChanged { from: String, to: String },
    /// The proof outlived its declared window.
    Expired { at: String },
}

impl StaleReason {
    /// Short label for logs, journal attributes and CI annotations.
    pub fn label(&self) -> &'static str {
        match self {
            StaleReason::CommitChanged { .. } => "commit_changed",
            StaleReason::UncommittedChanges { .. } => "uncommitted_changes",
            StaleReason::SubjectChanged { .. } => "subject_changed",
            StaleReason::ContractChanged { .. } => "contract_changed",
            StaleReason::VerifierConfigChanged { .. } => "verifier_config_changed",
            StaleReason::PlatformChanged { .. } => "platform_changed",
            StaleReason::AuthorizationChanged { .. } => "authorization_changed",
            StaleReason::Expired { .. } => "expired",
        }
    }

    /// One line, human-facing, for `uni explain` and the GitHub check body.
    pub fn describe(&self) -> String {
        match self {
            StaleReason::CommitChanged { from, to } => format!(
                "artifact moved from commit {} to {}",
                short(from),
                short(to)
            ),
            StaleReason::UncommittedChanges { .. } => {
                "tracked files were modified after the proof".to_string()
            }
            StaleReason::SubjectChanged {
                changed,
                added,
                removed,
            } => {
                let mut parts = vec![];
                if !changed.is_empty() {
                    parts.push(format!("changed: {}", changed.join(", ")));
                }
                if !added.is_empty() {
                    parts.push(format!("added: {}", added.join(", ")));
                }
                if !removed.is_empty() {
                    parts.push(format!("removed: {}", removed.join(", ")));
                }
                format!("watched subject moved ({})", parts.join("; "))
            }
            StaleReason::ContractChanged { .. } => "the contract changed".to_string(),
            StaleReason::VerifierConfigChanged { .. } => {
                "the trusted verifier registry changed".to_string()
            }
            StaleReason::PlatformChanged { from, to } => {
                format!("proof made on {from}, running on {to}")
            }
            StaleReason::AuthorizationChanged { .. } => {
                "the authorized verifier binding changed".to_string()
            }
            StaleReason::Expired { at } => format!("proof expired at {at}"),
        }
    }
}

fn short(sha: &str) -> String {
    sha.chars().take(8).collect()
}

/// Load persisted evidence for a claim; re-validate against the full
/// Verification Context. Stale/absent evidence is NOT trusted: caller must
/// re-run the verifier. Policy drift does NOT force a re-run: the decision
/// is recomputed from stored evidence with the current policy on every run.
pub fn load_valid_for_claim(
    dot_uni: &std::path::Path,
    claim_id: &str,
    ctx: &EvidenceContext,
) -> CacheOutcome {
    let path = evidence_path(dot_uni, claim_id, &ctx.fingerprint);
    let Some(mut ev): Option<Evidence> = load_json(&path) else {
        return CacheOutcome::Miss;
    };
    if ev.fingerprint != ctx.fingerprint {
        return CacheOutcome::Miss; // foreign or pre-fingerprint artifact
    }
    // Collect every drifted dimension, not just the fact of drift.
    let mut reasons: Vec<StaleReason> = vec![];
    if ev.commit_sha != ctx.commit_sha {
        reasons.push(StaleReason::CommitChanged {
            from: ev.commit_sha.clone(),
            to: ctx.commit_sha.clone(),
        });
    }
    if ev.workspace_dirty != ctx.workspace_dirty {
        reasons.push(StaleReason::UncommittedChanges {
            workspace_dirty: ctx.workspace_dirty,
        });
    }
    if !ev.artifact_hash.is_empty()
        && ctx.artifact_hash.as_deref() != Some(ev.artifact_hash.as_str())
    {
        // Name the files when the proof carries per-file hashes.
        let mut changed = vec![];
        let mut added = vec![];
        let mut removed = vec![];
        for (path, old) in &ev.artifact_files {
            match ctx.artifact_files.get(path) {
                Some(new) if new != old => changed.push(path.clone()),
                Some(_) => {}
                None => removed.push(path.clone()),
            }
        }
        for path in ctx.artifact_files.keys() {
            if !ev.artifact_files.contains_key(path) {
                added.push(path.clone());
            }
        }
        reasons.push(StaleReason::SubjectChanged {
            changed,
            added,
            removed,
        });
    }
    if !ev.contract_hash.is_empty() && ev.contract_hash != ctx.contract_hash {
        reasons.push(StaleReason::ContractChanged {
            from: ev.contract_hash.clone(),
            to: ctx.contract_hash.clone(),
        });
    }
    if !ev.registry_hash.is_empty() && ev.registry_hash != ctx.registry_hash {
        reasons.push(StaleReason::VerifierConfigChanged {
            from: ev.registry_hash.clone(),
            to: ctx.registry_hash.clone(),
        });
    }
    if !ev.platform.is_empty() && ev.platform != ctx.platform {
        reasons.push(StaleReason::PlatformChanged {
            from: ev.platform.clone(),
            to: ctx.platform.clone(),
        });
    }
    if !ev.binding_hash.is_empty() && ctx.binding_hash.as_deref() != Some(ev.binding_hash.as_str())
    {
        reasons.push(StaleReason::AuthorizationChanged {
            from: ev.binding_hash.clone(),
            to: ctx.binding_hash.clone().unwrap_or_default(),
        });
    }
    // Expired evidence is stale, not missing: the proof existed and decayed.
    if let Some(at) = ev.expires_at.filter(|t| *t <= chrono::Utc::now()) {
        reasons.push(StaleReason::Expired {
            at: at.to_rfc3339(),
        });
    }
    if !reasons.is_empty() {
        ev.state = EvidenceState::Stale;
        return CacheOutcome::Stale(reasons);
    }
    if ev.state == EvidenceState::Invalid {
        return CacheOutcome::Miss;
    }
    CacheOutcome::Hit(Box::new(ev))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(suffix: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "uni-ev-{suffix}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(d.join(".uni/evidence")).unwrap();
        d
    }

    fn ev() -> Evidence {
        Evidence {
            id: "c-abc".into(),
            claim_id: "c".into(),
            producer: "shell-verifier".into(),
            command: "true".into(),
            exit_code: 0,
            output_hash: "h".into(),
            output_excerpt: "".into(),
            commit_sha: "sha1".into(),
            workspace_dirty: false,
            state: EvidenceState::Valid,
            created_at: Utc::now(),
            duration_ms: 1,
            artifact_hash: String::new(),
            artifact_files: Default::default(),
            fingerprint: "fp1".into(),
            expires_at: None,
            registry_hash: "reg1".into(),
            policy_hash: "pol1".into(),
            contract_hash: "con1".into(),
            platform: "linux-x86_64".into(),
            actor: Actor::local(),
            executor: Actor::local(),
            binding_hash: String::new(),
        }
    }

    fn ctx() -> EvidenceContext {
        EvidenceContext {
            fingerprint: "fp1".into(),
            commit_sha: "sha1".into(),
            workspace_dirty: false,
            artifact_hash: None,
            artifact_files: Default::default(),
            registry_hash: "reg1".into(),
            contract_hash: "con1".into(),
            platform: "linux-x86_64".into(),
            binding_hash: None,
        }
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(sha256_hex(b"a").len(), 64);
    }

    #[test]
    fn evidence_path_isolates_fingerprint() {
        let a = evidence_path(std::path::Path::new(".uni"), "x", "fp1");
        let b = evidence_path(std::path::Path::new(".uni"), "x", "fp2");
        assert_ne!(a, b);
        assert!(a.to_string_lossy().contains("x__fp1"));
    }

    #[test]
    fn is_stale_matrix() {
        let e = ev();
        assert!(!is_stale(&e, "sha1", false));
        assert!(is_stale(&e, "sha2", false));
        assert!(is_stale(&e, "sha1", true));
    }

    #[test]
    fn load_valid_for_claim_checks_everything() {
        use CacheOutcome::*;
        let d = tmp("load");
        let du = d.join(".uni");
        let mut e = ev();
        save_json(&evidence_path(&du, "c", "fp1"), &e).unwrap();
        assert!(matches!(load_valid_for_claim(&du, "c", &ctx()), Hit(_)));
        let mut wrong_fp = ctx();
        wrong_fp.fingerprint = "WRONG".into();
        assert!(matches!(load_valid_for_claim(&du, "c", &wrong_fp), Miss));
        let mut other_sha = ctx();
        other_sha.commit_sha = "other".into();
        assert!(matches!(
            load_valid_for_claim(&du, "c", &other_sha),
            Stale(_)
        ));
        e.state = EvidenceState::Invalid;
        save_json(&evidence_path(&du, "c", "fp1"), &e).unwrap();
        assert!(matches!(load_valid_for_claim(&du, "c", &ctx()), Miss));
    }

    #[test]
    fn load_valid_for_claim_enforces_artifact_hash() {
        use CacheOutcome::*;
        let d = tmp("ah");
        let du = d.join(".uni");
        let mut e = ev();
        e.artifact_hash = "aaa".into();
        save_json(&evidence_path(&du, "c", "fp1"), &e).unwrap();
        let mut ok = ctx();
        ok.artifact_hash = Some("aaa".into());
        assert!(matches!(load_valid_for_claim(&du, "c", &ok), Hit(_)));
        let mut bad = ctx();
        bad.artifact_hash = Some("bbb".into());
        assert!(matches!(load_valid_for_claim(&du, "c", &bad), Stale(_)));
        assert!(matches!(load_valid_for_claim(&du, "c", &ctx()), Stale(_)));
    }

    #[test]
    fn verification_context_drift_invalidates() {
        use CacheOutcome::*;
        // B1: contract, registry, or platform drift forces re-run.
        let d = tmp("ctx");
        let du = d.join(".uni");
        save_json(&evidence_path(&du, "c", "fp1"), &ev()).unwrap();
        let mut drift = ctx();
        drift.contract_hash = "con2".into();
        assert!(matches!(load_valid_for_claim(&du, "c", &drift), Stale(_)));
        let mut drift = ctx();
        drift.registry_hash = "reg2".into();
        assert!(matches!(load_valid_for_claim(&du, "c", &drift), Stale(_)));
        let mut drift = ctx();
        drift.platform = "darwin-arm64".into();
        assert!(matches!(load_valid_for_claim(&du, "c", &drift), Stale(_)));
    }

    #[test]
    fn expired_evidence_is_stale_not_missing() {
        use CacheOutcome::*;
        let d = tmp("expiry");
        let du = d.join(".uni");
        let mut e = ev();
        e.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        save_json(&evidence_path(&du, "c", "fp1"), &e).unwrap();
        assert!(matches!(load_valid_for_claim(&du, "c", &ctx()), Stale(_)));

        // Still valid while inside its window.
        e.expires_at = Some(Utc::now() + chrono::Duration::hours(1));
        save_json(&evidence_path(&du, "c", "fp1"), &e).unwrap();
        assert!(matches!(load_valid_for_claim(&du, "c", &ctx()), Hit(_)));

        // No expiry set: never expires (backward compatible with old files).
        e.expires_at = None;
        save_json(&evidence_path(&du, "c", "fp1"), &e).unwrap();
        assert!(matches!(load_valid_for_claim(&du, "c", &ctx()), Hit(_)));
    }

    #[test]
    fn legacy_evidence_without_context_still_loads() {
        use CacheOutcome::*;
        // v0.1.0 files carry empty context hashes: accepted (backward compat),
        // but any drift on the new dimensions of a v0.2 file invalidates.
        let d = tmp("legacy");
        let du = d.join(".uni");
        let mut e = ev();
        e.registry_hash.clear();
        e.contract_hash.clear();
        e.platform.clear();
        save_json(&evidence_path(&du, "c", "fp1"), &e).unwrap();
        assert!(matches!(load_valid_for_claim(&du, "c", &ctx()), Hit(_)));
    }

    #[test]
    fn lock_excludes_second_holder_and_releases() {
        let d = tmp("lock");
        let du = d.join(".uni");
        let l1 = acquire_lock_with(&du, 3).unwrap();
        assert!(acquire_lock_with(&du, 2).is_err());
        drop(l1);
        assert!(acquire_lock_with(&du, 2).is_ok());
    }

    #[test]
    fn save_json_round_trip() {
        let d = tmp("save");
        let p = d.join(".uni/evidence/c__fp1.json");
        save_json(&p, &ev()).unwrap();
        let back: Evidence = load_json(&p).unwrap();
        assert_eq!(back.claim_id, "c");
    }
}
