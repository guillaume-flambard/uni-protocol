use uni_parser::parse;

fn fixture(name: &str) -> String {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    std::fs::read_to_string(format!("{base}/{name}")).unwrap()
}

#[test]
fn missing_version_errors() {
    assert!(parse(&fixture("missing_version.uni")).is_err());
}

#[test]
fn bad_directive_has_line() {
    let err = parse(&fixture("bad_directive.uni")).unwrap_err().to_string();
    assert!(err.contains("line 4"), "got: {err}");
}

#[test]
fn verify_unknown_claim_rejected() {
    let err = parse(&fixture("orphan_verify.uni")).unwrap_err().to_string();
    assert!(err.contains("unknown claim 'ghost'"), "got: {err}");
}

#[test]
fn ensure_without_claim_rejected() {
    let err = parse(&fixture("no_ensure.diag.uni")).unwrap_err().to_string();
    assert!(err.contains("ENSURE without preceding"), "got: {err}");
}

#[test]
fn accepted_when_is_optional_but_claims_required() {
    let src = "VERSION 0.1
DOMAIN software
INTENT x
CLAIM a REQUIRED
  ENSURE ok
VERIFY a
  USING project.check
";
    let c = parse(src).unwrap();
    assert_eq!(c.claims.len(), 1);
    assert!(c.acceptance.require_verified); // default per v0.1
}
