//! VerifierBinding (v0.2): the authorized resolution of a claim's verification
//! requirement to a concrete verifier. Claim -> requirement -> binding ->
//! evidence. Only a human `uni bind` act creates one; the verify path only
//! reads. AI may propose bindings; trusted configuration authorizes them.

use super::{save_json, sha256_hex};
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

pub fn binding_path(dot_uni: &std::path::Path, claim_id: &str) -> std::path::PathBuf {
    bindings_dir(dot_uni).join(format!("{claim_id}.json"))
}

pub fn load_binding(dot_uni: &std::path::Path, claim_id: &str) -> Option<VerifierBinding> {
    super::load_json(&binding_path(dot_uni, claim_id))
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
    std::fs::create_dir_all(bindings_dir(dot_uni))?;
    save_json(&binding_path(dot_uni, claim_id), &binding)?;
    Ok(binding)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(b.binding_hash, binding_hash("c", "v", "behaves", Some("test_a")));
        let back = load_binding(&du, "c").unwrap();
        assert_eq!(back, b);
        // Re-authorization replaces (explicit human act, never merged).
        let b2 = authorize(&du, "c", "v2", "behaves", None, "local:memo").unwrap();
        assert_ne!(b2.binding_hash, b.binding_hash);
        assert_eq!(load_binding(&du, "c").unwrap().verifier_ref, "v2");
    }
}
