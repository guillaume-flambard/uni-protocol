//! Identity adapters: the one place a proof's actor identity is upgraded from
//! "self-declared" (A3-D) to "externally verified" (A3).
//!
//! Trust root: `.uni/config.toml` `[identities]`. A token is verified only
//! against key material pinned there (a JWKS document named by `jwks_file`),
//! never against an issuer fetched at runtime. A token whose issuer the
//! registry does not declare is refused, not trusted: the registry is the only
//! root of trust, and a URL is not one.
//!
//! Verification is offline and deterministic given the pinned JWKS. It reads the
//! clock for one thing only, `exp`: a token past its expiry is refused. That is
//! a validity check on the proof of identity, not a decision predicate.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use uni_evidence::Actor;

const DEFAULT_ALGORITHMS: [&str; 2] = ["RS256", "ES256"];

/// A trusted issuer, pinned by the registry.
#[derive(Debug, Clone)]
pub struct IdentityProvider {
    /// The `iss` value a token must carry (for SPIFFE JWT-SVID, the trust domain).
    pub issuer: String,
    /// `oidc` | `entra` | `spiffe`; recorded as the actor source.
    pub source: String,
    /// Workspace-absolute path to the pinned JWKS document.
    pub jwks_file: std::path::PathBuf,
    /// When non-empty, the token's `aud` must intersect this list. Empty means
    /// audience is not enforced for this issuer.
    pub audiences: Vec<String>,
    /// Signing algorithms accepted from this issuer.
    pub algorithms: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IdentityRegistry {
    /// Keyed by issuer (`iss` claim / trust domain).
    pub providers: std::collections::BTreeMap<String, IdentityProvider>,
}

impl IdentityRegistry {
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub fn issuers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

/// Parse `[identities]` from the registry. A malformed entry is a hard error:
/// an identity entry that cannot be used must never be silently dropped, or a
/// typo would look like "no issuer declares this token".
pub fn load_identities(
    dot_uni: &std::path::Path,
    workspace: &std::path::Path,
) -> Result<IdentityRegistry> {
    let path = dot_uni.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(IdentityRegistry::default());
    };
    let val: toml::Value = text
        .parse()
        .map_err(|e| anyhow!("cannot parse {}: {e}", path.display()))?;
    let Some(table) = val.get("identities").and_then(|v| v.as_table()) else {
        return Ok(IdentityRegistry::default());
    };
    let mut providers = std::collections::BTreeMap::new();
    for (issuer, v) in table {
        let tbl = v.as_table().ok_or_else(|| {
            anyhow!("[identities.\"{issuer}\"] must be a table declaring jwks_file")
        })?;
        let source = tbl
            .get("source")
            .and_then(|s| s.as_str())
            .unwrap_or("oidc")
            .to_string();
        if !matches!(source.as_str(), "oidc" | "entra" | "spiffe") {
            return Err(anyhow!(
                "identity '{issuer}': unknown source '{source}' (known: oidc, entra, spiffe)"
            ));
        }
        let jwks = tbl.get("jwks_file").and_then(|s| s.as_str()).ok_or_else(|| {
            anyhow!("identity '{issuer}' has no jwks_file: an identity without pinned key material is not trusted")
        })?;
        let jwks_file = if std::path::Path::new(jwks).is_absolute() {
            std::path::PathBuf::from(jwks)
        } else {
            workspace.join(jwks)
        };
        let audiences = tbl
            .get("audiences")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let algorithms = tbl
            .get("algorithms")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_ALGORITHMS.iter().map(|s| s.to_string()).collect());
        providers.insert(
            issuer.clone(),
            IdentityProvider {
                issuer: issuer.clone(),
                source,
                jwks_file,
                audiences,
                algorithms,
            },
        );
    }
    Ok(IdentityRegistry { providers })
}

/// Only the subject is read here: `iss`, `aud` and `exp` are enforced by the
/// validator against the pinned issuer, not by this struct.
#[derive(Debug, Deserialize)]
struct Claims {
    #[serde(default)]
    sub: String,
}

/// Verify a presented JWT against the pinned issuers. On success the Actor
/// carries assurance `verified`, which is what lets the decision engine reach
/// A3. An empty registry, an unparseable token, or a signature that no pinned
/// key accepts is a hard error: a presented token is never silently downgraded.
pub fn verify_token(token: &str, reg: &IdentityRegistry) -> Result<Actor> {
    let header = jsonwebtoken::decode_header(token)
        .map_err(|e| anyhow!("identity token is not a well-formed JWT: {e}"))?;
    if reg.is_empty() {
        return Err(anyhow!(
            "an identity token was presented but .uni/config.toml declares no [identities] issuer to trust it"
        ));
    }
    let alg = format!("{:?}", header.alg);
    let mut attempts: Vec<String> = vec![];
    for p in reg.providers.values() {
        if !p.algorithms.iter().any(|a| a == &alg) {
            attempts.push(format!("{}: algorithm {alg} not allowed", p.issuer));
            continue;
        }
        let text = match std::fs::read_to_string(&p.jwks_file) {
            Ok(t) => t,
            Err(e) => {
                attempts.push(format!(
                    "{}: cannot read {} ({e})",
                    p.issuer,
                    p.jwks_file.display()
                ));
                continue;
            }
        };
        let set: jsonwebtoken::jwk::JwkSet = match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                attempts.push(format!("{}: JWKS is not a valid key set ({e})", p.issuer));
                continue;
            }
        };
        // Select the key the token names. With no kid, only an unambiguous
        // single-key set is usable: guessing across keys puts resolution back
        // in the trust path.
        let jwk = match (&header.kid, set.keys.len()) {
            (Some(kid), _) => set
                .keys
                .iter()
                .find(|k| k.common.key_id.as_deref() == Some(kid.as_str())),
            (None, 1) => set.keys.first(),
            (None, _) => None,
        };
        let Some(jwk) = jwk else {
            attempts.push(format!("{}: no key for kid {:?}", p.issuer, header.kid));
            continue;
        };
        let key = match jsonwebtoken::DecodingKey::from_jwk(jwk) {
            Ok(k) => k,
            Err(e) => {
                attempts.push(format!("{}: unusable key ({e})", p.issuer));
                continue;
            }
        };
        let mut v = jsonwebtoken::Validation::new(header.alg);
        v.set_issuer(&[p.issuer.as_str()]);
        if p.audiences.is_empty() {
            // No audience pinned for this issuer: do not enforce one.
            v.validate_aud = false;
        } else {
            v.set_audience(&p.audiences);
        }
        v.set_required_spec_claims(&["exp", "iss", "sub"]);
        v.leeway = 60;
        match jsonwebtoken::decode::<Claims>(token, &key, &v) {
            Ok(data) => {
                let subject = data.claims.sub.trim().to_string();
                let (source, id) = if p.source == "spiffe" {
                    if !subject.starts_with("spiffe://") {
                        return Err(anyhow!(
                            "identity token from '{}' is source spiffe but its subject '{subject}' is not a spiffe:// id",
                            p.issuer
                        ));
                    }
                    ("spiffe".to_string(), subject)
                } else {
                    // A single colon: the issuer already carries `https://`, so
                    // `oidc://https://...` would nest two schemes in one id.
                    let source = p.source.clone();
                    (source.clone(), format!("{source}:{}#{subject}", p.issuer))
                };
                return Ok(Actor::verified(&id, &source));
            }
            Err(e) => attempts.push(format!("{}: {e}", p.issuer)),
        }
    }
    Err(anyhow!(
        "identity token rejected by every trusted issuer ({}): {}",
        reg.issuers().join(", "),
        attempts.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

    // A throwaway RSA keypair used only by these tests. It guards nothing.
    const TEST_PRIVATE_PEM: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/identity/test.key.pem"
    ));
    const TEST_JWKS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/identity/test.jwks.json"
    ));

    fn tmp(suffix: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "uni-id-{suffix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_jwks(dir: &std::path::Path) -> std::path::PathBuf {
        let p = dir.join("issuer.jwks.json");
        std::fs::write(&p, TEST_JWKS).unwrap();
        p
    }

    fn registry_with(
        issuer: &str,
        source: &str,
        jwks: std::path::PathBuf,
        audiences: &[&str],
    ) -> IdentityRegistry {
        let mut providers = std::collections::BTreeMap::new();
        providers.insert(
            issuer.to_string(),
            IdentityProvider {
                issuer: issuer.to_string(),
                source: source.to_string(),
                jwks_file: jwks,
                audiences: audiences.iter().map(|s| s.to_string()).collect(),
                algorithms: DEFAULT_ALGORITHMS.iter().map(|s| s.to_string()).collect(),
            },
        );
        IdentityRegistry { providers }
    }

    fn sign(sub: &str, iss: &str, aud: Option<&str>, exp_secs: i64) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("uni-test-key".into());
        let mut claims = serde_json::json!({
            "sub": sub,
            "iss": iss,
            "exp": (chrono::Utc::now().timestamp() + exp_secs),
        });
        if let Some(a) = aud {
            claims["aud"] = serde_json::json!(a);
        }
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_PEM.as_bytes()).unwrap();
        encode(&header, &claims, &key).unwrap()
    }

    #[test]
    fn verified_token_yields_a_verified_actor() {
        let d = tmp("ok");
        let jwks = write_jwks(&d);
        let reg = registry_with("https://issuer.example", "oidc", jwks, &[]);
        let token = sign("alice", "https://issuer.example", None, 300);
        let actor = verify_token(&token, &reg).unwrap();
        assert_eq!(actor.assurance, "verified");
        assert_eq!(actor.source, "oidc");
        assert_eq!(actor.id, "oidc:https://issuer.example#alice");
        assert!(actor.is_verified());
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let d = tmp("iss");
        let jwks = write_jwks(&d);
        let reg = registry_with("https://issuer.example", "oidc", jwks, &[]);
        // Same key, but the token claims a different issuer.
        let token = sign("alice", "https://evil.example", None, 300);
        assert!(verify_token(&token, &reg).is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        let d = tmp("exp");
        let jwks = write_jwks(&d);
        let reg = registry_with("https://issuer.example", "oidc", jwks, &[]);
        let token = sign("alice", "https://issuer.example", None, -3600);
        let err = verify_token(&token, &reg).unwrap_err().to_string();
        assert!(err.contains("rejected by every trusted issuer"), "{err}");
    }

    #[test]
    fn tampered_token_is_rejected() {
        let d = tmp("tamper");
        let jwks = write_jwks(&d);
        let reg = registry_with("https://issuer.example", "oidc", jwks, &[]);
        let token = sign("alice", "https://issuer.example", None, 300);
        // Flip the last character of the signature segment.
        let mut chars: Vec<char> = token.chars().collect();
        let last = chars.len() - 1;
        chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
        let tampered: String = chars.into_iter().collect();
        assert!(verify_token(&tampered, &reg).is_err());
    }

    #[test]
    fn empty_registry_refuses_a_presented_token() {
        let reg = IdentityRegistry::default();
        let token = sign("alice", "https://issuer.example", None, 300);
        let err = verify_token(&token, &reg).unwrap_err().to_string();
        assert!(err.contains("no [identities] issuer"), "{err}");
    }

    #[test]
    fn audience_is_enforced_when_pinned() {
        let d = tmp("aud");
        let jwks = write_jwks(&d);
        let reg = registry_with("https://issuer.example", "oidc", jwks, &["uni-cli"]);
        let good = sign("alice", "https://issuer.example", Some("uni-cli"), 300);
        assert!(verify_token(&good, &reg).is_ok());
        let bad = sign("alice", "https://issuer.example", Some("other"), 300);
        assert!(verify_token(&bad, &reg).is_err());
    }

    #[test]
    fn spiffe_subject_must_be_a_spiffe_id() {
        let d = tmp("spiffe");
        let jwks = write_jwks(&d);
        let reg = registry_with("acme.example", "spiffe", jwks, &[]);
        let good = sign(
            "spiffe://acme.example/verifier/build-12",
            "acme.example",
            None,
            300,
        );
        let actor = verify_token(&good, &reg).unwrap();
        assert_eq!(actor.source, "spiffe");
        assert_eq!(actor.id, "spiffe://acme.example/verifier/build-12");
        let bad = sign("alice", "acme.example", None, 300);
        assert!(verify_token(&bad, &reg).is_err());
    }

    #[test]
    fn load_identities_parses_and_refuses_half_wired_entries() {
        let d = tmp("load");
        std::fs::write(
            d.join("config.toml"),
            "[identities.\"https://issuer.example\"]\njwks_file = \"issuer.jwks.json\"\naudiences = [\"uni-cli\"]\nalgorithms = [\"RS256\"]\n",
        )
        .unwrap();
        let reg = load_identities(&d, &d).unwrap();
        let p = &reg.providers["https://issuer.example"];
        assert_eq!(p.source, "oidc");
        assert_eq!(p.audiences, vec!["uni-cli".to_string()]);
        assert_eq!(p.jwks_file, d.join("issuer.jwks.json"));

        // Missing jwks_file: hard error, never a silently dropped identity.
        std::fs::write(
            d.join("config.toml"),
            "[identities.\"https://issuer.example\"]\naudiences = [\"x\"]\n",
        )
        .unwrap();
        assert!(load_identities(&d, &d).is_err());

        // Unknown source: hard error.
        std::fs::write(
            d.join("config.toml"),
            "[identities.\"x\"]\nsource = \"ldap\"\njwks_file = \"j.json\"\n",
        )
        .unwrap();
        assert!(load_identities(&d, &d).is_err());
    }
}
