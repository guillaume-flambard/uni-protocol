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

#[test]
fn standalone_require_is_hard_error() {
    // v0.2 implements REQUIRE only as a VERIFY-attached resolution requirement.
    let err = parse(&fixture("reserved_require.uni")).unwrap_err().to_string();
    assert!(err.contains("must immediately follow"), "got: {err}");
}

#[test]
fn reserved_reject_when_is_hard_error() {
    let err = parse(&fixture("reserved_reject_when.uni"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("reserved for v0.2"), "got: {err}");
}

#[test]
fn accept_when_variant_is_hard_error() {
    let err = parse(&fixture("accept_variant_rejected.uni"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported ACCEPT WHEN clause"), "got: {err}");
}

#[test]
fn canonical_accept_when_parses() {
    let src = "VERSION 0.1
DOMAIN software
INTENT x
CLAIM a REQUIRED
  ENSURE ok
VERIFY a
  USING project.check
ACCEPT WHEN
  required_claims == VERIFIED
  AND critical_failures == 0
";
    let c = parse(src).unwrap();
    assert!(c.acceptance.require_verified);
}

#[test]
fn require_attaches_to_preceding_verify() {
    let src = "VERSION 0.1\nDOMAIN software\nINTENT x\nCLAIM a REQUIRED\n  ENSURE ok\nVERIFY a\n  USING project.check\n  REQUIRE behavior(\"checks\")\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n";
    let c = parse(src).unwrap();
    assert_eq!(c.verifications.len(), 1);
    assert_eq!(c.verifications[0].requirement.as_deref(), Some("behavior(\"checks\")"));
}

#[test]
fn require_without_verify_is_hard_error() {
    let src = "VERSION 0.1\nDOMAIN software\nINTENT x\nCLAIM a REQUIRED\n  ENSURE ok\nREQUIRE behavior(\"x\")\nVERIFY a\n  USING project.check\n";
    let err = parse(src).unwrap_err().to_string();
    assert!(err.contains("must immediately follow"), "got: {err}");
}

#[test]
fn require_after_other_block_is_hard_error() {
    let src = "VERSION 0.1\nDOMAIN software\nINTENT x\nCLAIM a REQUIRED\n  ENSURE ok\nVERIFY a\n  USING project.check\nCLAIM b REQUIRED\n  ENSURE ok2\nREQUIRE behavior(\"x\")\nVERIFY b\n  USING project.check\n";
    let err = parse(src).unwrap_err().to_string();
    assert!(err.contains("must immediately follow"), "got: {err}");
}
