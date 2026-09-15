//! VerifierBinding (v0.2): the authorized resolution of a claim's verification
//! requirement to a concrete verifier. Claim -> requirement -> binding ->
//! evidence. Only a human `uni bind` act creates one; the verify path only
//! reads. AI may propose bindings; trusted configuration authorizes them.

use super::sha256_hex;
use anyhow::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VerifierBinding {
    pub claim_id: String,
    pub verifier_ref: String,
    pub requirement: String,
    /// Concrete test selector for `{{selector}}` template verifiers. The
    /// worker chooses the test name; the human authorizes which test counts.
    #[serde(default)]
    pub selector: Option<String>,
    pub authorized_by: String,
    pub authorized_at: String,
    pub binding_hash: String,
}

pub fn binding_hash(
    claim_id: &str,
    verifier_ref: &str,
    requirement: &str,
    selector: Option<&str>,
) -> String {
    sha256_hex(
        format!(
            "{claim_id}|{verifier_ref}|{requirement}|selector:{}",
            selector.unwrap_or("")
        )
        .as_bytes(),
    )[..12]
        .to_string()
}

pub fn bindings_dir(dot_uni: &std::path::Path) -> std::path::PathBuf {
    dot_uni.join("bindings")
}

/// The single reviewed bindings file. One entry per claim, and the whole file
/// is authorized in one act: fifty claims no longer mean fifty files, and a
/// re-authorization is a reviewable diff rather than a new blob.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BindingFile {
    #[serde(default)]
    pub bindings: std::collections::BTreeMap<String, BindingEntry>,
}

/// What the human writes; `authorized_*` and the hash are stamped by the act.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BindingEntry {
    pub verifier: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub requirement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default)]
    pub authorized_by: String,
    #[serde(default)]
    pub authorized_at: String,
    #[serde(default)]
    pub binding_hash: String,
}

pub fn bindings_file_path(dot_uni: &std::path::Path) -> std::path::PathBuf {
    dot_uni.join("bindings.toml")
}

fn to_binding(claim_id: &str, e: &BindingEntry) -> VerifierBinding {
    VerifierBinding {
        claim_id: claim_id.to_string(),
        verifier_ref: e.verifier.clone(),
        requirement: e.requirement.clone(),
        selector: e.selector.clone(),
        authorized_by: e.authorized_by.clone(),
        authorized_at: e.authorized_at.clone(),
        binding_hash: e.binding_hash.clone(),
    }
}

fn to_entry(b: &VerifierBinding) -> BindingEntry {
    BindingEntry {
        verifier: b.verifier_ref.clone(),
        requirement: b.requirement.clone(),
        selector: b.selector.clone(),
        authorized_by: b.authorized_by.clone(),
        authorized_at: b.authorized_at.clone(),
        binding_hash: b.binding_hash.clone(),
    }
}

fn read_file(path: &std::path::Path) -> BindingFile {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| toml::from_str::<BindingFile>(&t).ok())
        .unwrap_or_default()
}

fn write_file(path: &std::path::Path, file: &BindingFile) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let text = toml::to_string_pretty(file)?;
    // Atomic, like every other UNI write.
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Every authorized binding: the reviewed file first, then legacy per-claim
/// files for claims it does not mention (pre-v0.7 repositories).
pub fn load_all(dot_uni: &std::path::Path) -> std::collections::BTreeMap<String, VerifierBinding> {
    let mut out: std::collections::BTreeMap<String, VerifierBinding> =
        read_file(&bindings_file_path(dot_uni))
            .bindings
            .iter()
            .map(|(claim, e)| (claim.clone(), to_binding(claim, e)))
            .collect();
    if let Ok(rd) = std::fs::read_dir(bindings_dir(dot_uni)) {
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().map(|x| x == "json").unwrap_or(false) {
                if let Some(claim) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(b) = super::load_json::<VerifierBinding>(&path) {
                        out.entry(claim.to_string()).or_insert(b);
                    }
                }
            }
        }
    }
    out
}

pub fn binding_path(dot_uni: &std::path::Path, claim_id: &str) -> std::path::PathBuf {
    bindings_dir(dot_uni).join(format!("{claim_id}.json"))
}

pub fn load_binding(dot_uni: &std::path::Path, claim_id: &str) -> Option<VerifierBinding> {
    load_all(dot_uni).remove(claim_id)
}

/// Human authorization act: bind a claim's requirement to a verifier.
/// Overwrites any previous binding for the claim (re-authorization is
/// explicit; the old binding_hash stops matching and bound evidence stales).
pub fn authorize(
    dot_uni: &std::path::Path,
    claim_id: &str,
    verifier_ref: &str,
    requirement: &str,
    selector: Option<&str>,
    by: &str,
) -> Result<VerifierBinding> {
    let binding = VerifierBinding {
        claim_id: claim_id.to_string(),
        verifier_ref: verifier_ref.to_string(),
        requirement: requirement.to_string(),
        selector: selector.map(|s| s.to_string()),
        authorized_by: by.to_string(),
        authorized_at: chrono::Utc::now().to_rfc3339(),
        binding_hash: binding_hash(claim_id, verifier_ref, requirement, selector),
    };
    let path = bindings_file_path(dot_uni);
    let mut file = read_file(&path);
    file.bindings
        .insert(claim_id.to_string(), to_entry(&binding));
    write_file(&path, &file)?;
    Ok(binding)
}

/// Authorize a reviewed file in one human act: every entry it lists is stamped
/// and merged into `.uni/bindings.toml`. This is the scale answer: review fifty
/// claims in one diff, authorize them with one command, and the journal records
/// the act once per entry.
pub fn authorize_from_file(
    dot_uni: &std::path::Path,
    source: &std::path::Path,
    by: &str,
) -> Result<Vec<VerifierBinding>> {
    let incoming = read_file(source);
    if incoming.bindings.is_empty() {
        anyhow::bail!("{} lists no [bindings] entries", source.display());
    }
    let path = bindings_file_path(dot_uni);
    let mut file = read_file(&path);
    let now = chrono::Utc::now().to_rfc3339();
    let mut out = vec![];
    for (claim, entry) in &incoming.bindings {
        let binding = VerifierBinding {
            claim_id: claim.clone(),
            verifier_ref: entry.verifier.clone(),
            requirement: entry.requirement.clone(),
            selector: entry.selector.clone(),
            authorized_by: by.to_string(),
            authorized_at: now.clone(),
            binding_hash: binding_hash(
                claim,
                &entry.verifier,
                &entry.requirement,
                entry.selector.as_deref(),
            ),
        };
        file.bindings.insert(claim.clone(), to_entry(&binding));
        out.push(binding);
    }
    write_file(&path, &file)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_json;

    #[test]
    fn binding_hash_stable_and_discriminating() {
        let a = binding_hash("c", "v", "r", None);
        assert_eq!(a, binding_hash("c", "v", "r", None));
        assert_eq!(a.len(), 12);
        assert_ne!(a, binding_hash("c", "v", "other", None));
        assert_ne!(a, binding_hash("c", "w", "r", None));
        // the selector is part of the authorization: changing it invalidates
        assert_ne!(a, binding_hash("c", "v", "r", Some("test_a")));
        assert_ne!(
            binding_hash("c", "v", "r", Some("test_a")),
            binding_hash("c", "v", "r", Some("test_b"))
        );
    }

    #[test]
    fn bulk_file_round_trip_and_legacy_precedence() {
        let dir = std::env::temp_dir().join(format!(
            "uni-bindset-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let du = dir.join(".uni");

        // A reviewed file with two entries, authorized in one act.
        let reviewed = dir.join("reviewed.toml");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            &reviewed,
            "[bindings.\"a\"]\nverifier = \"suite\"\nselector = \"test_a\"\nrequirement = \"behaves\"\n\n[bindings.\"b\"]\nverifier = \"suite\"\nselector = \"test_b\"\n",
        )
        .unwrap();
        let batch = authorize_from_file(&du, &reviewed, "local:memo").unwrap();
        assert_eq!(batch.len(), 2);
        let all = load_all(&du);
        assert_eq!(all.len(), 2);
        assert_eq!(all["a"].selector.as_deref(), Some("test_a"));
        assert_eq!(all["a"].requirement, "behaves");
        assert_eq!(all["a"].authorized_by, "local:memo");
        assert!(!all["a"].binding_hash.is_empty(), "the act stamps the hash");

        // A legacy per-claim file still loads, and the reviewed file wins on
        // a claim it also mentions.
        let legacy = binding_path(&du, "c");
        save_json(
            &legacy,
            &VerifierBinding {
                claim_id: "c".into(),
                verifier_ref: "old".into(),
                requirement: String::new(),
                selector: None,
                authorized_by: "local:legacy".into(),
                authorized_at: "2026-01-01T00:00:00Z".into(),
                binding_hash: binding_hash("c", "old", "", None),
            },
        )
        .unwrap();
        assert_eq!(load_all(&du)["c"].verifier_ref, "old");
        // Re-authorizing through the file replaces the legacy view.
        authorize(&du, "c", "new", "", None, "local:memo").unwrap();
        assert_eq!(load_all(&du)["c"].verifier_ref, "new");
    }

    #[test]
    fn empty_reviewed_file_is_refused() {
        let dir = std::env::temp_dir().join(format!(
            "uni-bindempty-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("empty.toml");
        std::fs::write(&f, "# nothing here\n").unwrap();
        assert!(authorize_from_file(&dir.join(".uni"), &f, "local:memo").is_err());
    }

    #[test]
    fn authorize_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "uni-bind-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let du = dir.join(".uni");
        let b = authorize(&du, "c", "v", "behaves", Some("test_a"), "local:memo").unwrap();
        assert_eq!(
            b.binding_hash,
            binding_hash("c", "v", "behaves", Some("test_a"))
        );
        let back = load_binding(&du, "c").unwrap();
        assert_eq!(back, b);
        // Re-authorization replaces (explicit human act, never merged).
        let b2 = authorize(&du, "c", "v2", "behaves", None, "local:memo").unwrap();
        assert_ne!(b2.binding_hash, b.binding_hash);
        assert_eq!(load_binding(&du, "c").unwrap().verifier_ref, "v2");
    }
}
