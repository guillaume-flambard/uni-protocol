//! A3 end-to-end: a JWT presented in UNI_IDENTITY_TOKEN, verified against a
//! pinned issuer, is what lifts an independent proof from A3-D to A3. A token
//! that cannot be verified is refused, never silently downgraded to declared.

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use std::path::PathBuf;
use std::process::Command;

const TEST_PRIVATE_PEM: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/identity/test.key.pem"
));
const TEST_JWKS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/identity/test.jwks.json"
));

const CONTRACT: &str = "VERSION 0.1
DOMAIN software
INTENT identity-demo
GOAL
  prove who produced the proof
CLAIM x REQUIRED
  ENSURE true
VERIFY x
  USING pass
ACCEPT WHEN
  required_claims == VERIFIED
";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn mk_repo(tag: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!(
        "uni-identity-{tag}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers]\n\"pass\" = \"true\"\n\n\
         [identities.\"https://issuer.example\"]\n\
         source = \"oidc\"\n\
         jwks_file = \"issuer.jwks.json\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("issuer.jwks.json"), TEST_JWKS).unwrap();
    std::fs::write(dir.join("c.uni"), CONTRACT).unwrap();
    dir
}

fn token(sub: &str, iss: &str, exp_secs: i64) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("uni-test-key".into());
    let claims = serde_json::json!({
        "sub": sub,
        "iss": iss,
        "exp": chrono::Utc::now().timestamp() + exp_secs,
    });
    let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_PEM.as_bytes()).unwrap();
    encode(&header, &claims, &key).unwrap()
}

fn run(dir: &PathBuf, args: &[&str], token: Option<&str>) -> (i32, String) {
    let mut c = Command::new(bin());
    c.args(args).current_dir(dir);
    if let Some(t) = token {
        c.env("UNI_IDENTITY_TOKEN", t);
    }
    let o = c.output().unwrap();
    (
        o.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

fn report(dir: &PathBuf) -> serde_json::Value {
    let (code, out) = run(dir, &["report", "--json"], None);
    assert_eq!(code, 0, "report failed:\n{out}");
    serde_json::from_str(&out).unwrap()
}

#[test]
fn a_verified_token_reaches_a3() {
    let dir = mk_repo("a3");
    let t = token("alice", "https://issuer.example", 300);
    let (code, out) = run(&dir, &["verify", "c.uni"], Some(&t));
    assert_eq!(code, 0, "{out}");
    let r = report(&dir);
    assert_eq!(r["decision"], "Accepted");
    assert_eq!(r["assurance"], "A3", "{r}");
    assert_eq!(r["independent_actor"], true, "{r}");
    assert_eq!(r["identity_assurance"], "VERIFIED", "{r}");
}

#[test]
fn a_declared_actor_reaches_only_a3_d() {
    let dir = mk_repo("a3d");
    let (code, out) = run(&dir, &["verify", "c.uni", "--actor", "ci:build-12"], None);
    assert_eq!(code, 0, "{out}");
    let r = report(&dir);
    assert_eq!(r["assurance"], "A3-D", "{r}");
    assert_eq!(r["identity_assurance"], "SELF-DECLARED", "{r}");
}

#[test]
fn no_identity_stays_at_a2() {
    let dir = mk_repo("a2");
    let (code, out) = run(&dir, &["verify", "c.uni"], None);
    assert_eq!(code, 0, "{out}");
    let r = report(&dir);
    assert_eq!(r["assurance"], "A2", "{r}");
    assert_eq!(r["independent_actor"], false, "{r}");
}

#[test]
fn token_and_actor_are_mutually_exclusive() {
    let dir = mk_repo("conflict");
    let t = token("alice", "https://issuer.example", 300);
    let (code, out) = run(
        &dir,
        &["verify", "c.uni", "--actor", "ci:build-12"],
        Some(&t),
    );
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("mutually exclusive"), "{out}");
}

#[test]
fn an_expired_token_is_refused_not_downgraded() {
    let dir = mk_repo("expired");
    let t = token("alice", "https://issuer.example", -3600);
    let (code, out) = run(&dir, &["verify", "c.uni"], Some(&t));
    assert_ne!(code, 0, "an invalid token must fail the run:\n{out}");
    assert!(out.contains("rejected by every trusted issuer"), "{out}");
}

#[test]
fn an_untrusted_issuer_is_refused() {
    let dir = mk_repo("untrusted");
    let t = token("alice", "https://evil.example", 300);
    let (code, out) = run(&dir, &["verify", "c.uni"], Some(&t));
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("rejected by every trusted issuer"), "{out}");
}

#[test]
fn adding_a_trusted_issuer_is_a_named_trust_boundary_change() {
    let dir = mk_repo("boundary");
    let t = token("alice", "https://issuer.example", 300);
    let (code, out) = run(&dir, &["verify", "c.uni"], Some(&t));
    assert_eq!(code, 0, "{out}");
    // The only edit touches who may be believed, not what may run: it still has
    // to be named, or REGISTRY_CHANGED would report an empty diff.
    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers]\n\"pass\" = \"true\"\n\n\
         [identities.\"https://issuer.example\"]\nsource = \"oidc\"\njwks_file = \"issuer.jwks.json\"\n\n\
         [identities.\"https://other.example\"]\nsource = \"oidc\"\njwks_file = \"other.jwks.json\"\n",
    )
    .unwrap();
    let (code, out) = run(&dir, &["verify", "c.uni"], Some(&t));
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("REGISTRY_CHANGED"), "{out}");
    assert!(out.contains("identity:"), "{out}");
}
