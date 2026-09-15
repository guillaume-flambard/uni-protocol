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
    let err = parse(&fixture("bad_directive.uni"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("line 4"), "got: {err}");
}

#[test]
fn verify_unknown_claim_rejected() {
    let err = parse(&fixture("orphan_verify.uni"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown claim 'ghost'"), "got: {err}");
}

#[test]
fn ensure_without_claim_rejected() {
    let err = parse(&fixture("no_ensure.diag.uni"))
        .unwrap_err()
        .to_string();
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
    let err = parse(&fixture("reserved_require.uni"))
        .unwrap_err()
        .to_string();
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
    assert_eq!(
        c.verifications[0].requirement.as_deref(),
        Some("behavior(\"checks\")")
    );
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

// --- the parts of the closed vocabulary the fixtures above do not reach ---

fn head(body: &str) -> String {
    format!("VERSION 0.1\nDOMAIN software\nINTENT x\nGOAL\n  n\n{body}ACCEPT WHEN\n  required_claims == VERIFIED\n")
}

/// The GOAL block is free-form prose, and it used to end only on a hand-picked
/// few directives. A reserved keyword sitting right after GOAL was therefore
/// absorbed as goal text: the hard error the vocabulary promises never fired.
/// These two are the regression, and the reason the terminator list is the
/// whole vocabulary rather than a subset.
#[test]
fn a_reserved_directive_right_after_goal_still_errors() {
    for keyword in ["REJECT WHEN", "ESCALATE WHEN"] {
        let src = head(&format!("{keyword}\n  x\n"));
        let err = parse(&src).unwrap_err().to_string();
        assert!(
            err.contains("reserved for v0.2"),
            "{keyword} was swallowed: {err}"
        );
        assert!(
            err.contains("line 6"),
            "{keyword} must name its line: {err}"
        );
    }
}

#[test]
fn a_stray_ensure_after_goal_is_not_swallowed_as_prose() {
    let err = parse(&head("ENSURE orphan\n")).unwrap_err().to_string();
    assert!(err.contains("ENSURE without preceding"), "got: {err}");
}

#[test]
fn domain_and_intent_are_required_too() {
    let body = "CLAIM a REQUIRED\n  ENSURE ok\nVERIFY a\n  USING k\nACCEPT WHEN\n  required_claims == VERIFIED\n";
    for (line, needle) in [
        ("VERSION 0.1\n", "missing VERSION"),
        ("DOMAIN software\n", "missing DOMAIN"),
        ("INTENT x\n", "missing INTENT"),
    ] {
        let src =
            format!("VERSION 0.1\nDOMAIN software\nINTENT x\nGOAL\n  n\n{body}").replace(line, "");
        let err = parse(&src).unwrap_err().to_string();
        assert!(err.contains(needle), "expected {needle:?}, got: {err}");
    }
}

#[test]
fn a_contract_needs_at_least_one_claim() {
    let src = "VERSION 0.1\nDOMAIN software\nINTENT x\nGOAL\n  n\nVERIFY a\n  USING k\n";
    let err = parse(src).unwrap_err().to_string();
    assert!(err.contains("no CLAIM/INVARIANT"), "got: {err}");
}

#[test]
fn claim_and_invariant_need_an_id() {
    let e = parse(&head("CLAIM\n  ENSURE ok\nVERIFY a\n  USING k\n"))
        .unwrap_err()
        .to_string();
    assert!(e.contains("CLAIM needs an id"), "got: {e}");

    let e = parse(&head("INVARIANT\n  ENSURE ok\nVERIFY a\n  USING k\n"))
        .unwrap_err()
        .to_string();
    assert!(e.contains("INVARIANT needs an id"), "got: {e}");
}

/// The closed vocabulary extends to modifiers. `CLAIM x BANANA` used to
/// compile, and `CLAIM x CRITICAL` was accepted and discarded, which turned a
/// critically-failing verifier into EVIDENCE_REQUIRED instead of REJECTED.
#[test]
fn claim_and_invariant_modifiers_are_validated() {
    let e = parse(&head(
        "CLAIM x CRITICAL\n  ENSURE ok\nVERIFY x\n  USING k\n",
    ))
    .unwrap_err()
    .to_string();
    assert!(e.contains("unknown CLAIM modifier 'CRITICAL'"), "got: {e}");
    assert!(
        e.contains("INVARIANT"),
        "the hint must name the right keyword: {e}"
    );
    assert!(e.contains("line 6"), "the error must name its line: {e}");

    let e = parse(&head("CLAIM x BANANA\n  ENSURE ok\nVERIFY x\n  USING k\n"))
        .unwrap_err()
        .to_string();
    assert!(e.contains("unknown CLAIM modifier 'BANANA'"), "got: {e}");

    let e = parse(&head(
        "INVARIANT x REQUIRED\n  ENSURE ok\nVERIFY x\n  USING k\n",
    ))
    .unwrap_err()
    .to_string();
    assert!(
        e.contains("unknown INVARIANT modifier 'REQUIRED'"),
        "got: {e}"
    );

    // The legal spellings, including the bare form, still parse.
    for body in [
        "CLAIM x REQUIRED\n  ENSURE ok\nVERIFY x\n  USING k\n",
        "CLAIM x OPTIONAL\n  ENSURE ok\nVERIFY x\n  USING k\n",
        "CLAIM x\n  ENSURE ok\nVERIFY x\n  USING k\n",
    ] {
        assert!(parse(&head(body)).is_ok(), "should parse: {body}");
    }
    let c = parse(&head(
        "INVARIANT x CRITICAL\n  ENSURE ok\nVERIFY x\n  USING k\n",
    ))
    .unwrap();
    assert!(c.claims[0].critical, "CRITICAL on an INVARIANT must stick");
}

/// A FORBID claim's id is numbered on FORBID alone. Numbering it from
/// `claims.len()` moved the id when an unrelated CLAIM was inserted above it,
/// which silently invalidated that claim's binding and evidence.
#[test]
fn forbid_ids_do_not_move_when_other_claims_are_inserted() {
    let first = parse(&head(
        "FORBID\n  direct_write(\"ledger\")\nVERIFY forbid-1\n  USING k\n",
    ))
    .unwrap();
    assert_eq!(first.claims[0].id, "forbid-1");

    let after_a_claim = parse(&head(
        "CLAIM a REQUIRED\n  ENSURE a\nFORBID\n  direct_write(\"ledger\")\nVERIFY a\n  USING k\nVERIFY forbid-1\n  USING k\n",
    ))
    .unwrap();
    assert_eq!(
        after_a_claim
            .claims
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "forbid-1"],
        "inserting a claim must not renumber the forbid claim"
    );
}

/// The other three ways an ACCEPT WHEN can be something v0.1 cannot honour:
/// a repeated clause, no required clause at all, and an empty condition.
#[test]
fn every_accept_when_v0_1_cannot_honour_is_an_error() {
    let body = "CLAIM a REQUIRED\n  ENSURE ok\nVERIFY a\n  USING k\n";
    for (tail, needle) in [
        (
            "ACCEPT WHEN\n  required_claims == VERIFIED\n  AND required_claims == VERIFIED\n",
            "duplicate clause",
        ),
        (
            "ACCEPT WHEN\n  critical_failures == 0\n",
            "must include 'required_claims == VERIFIED'",
        ),
        ("ACCEPT WHEN\n", "needs a condition"),
    ] {
        let err = parse(&format!(
            "VERSION 0.1\nDOMAIN software\nINTENT x\nGOAL\n  n\n{body}{tail}"
        ))
        .unwrap_err()
        .to_string();
        assert!(err.contains(needle), "expected {needle:?}, got: {err}");
    }
}

/// `shell` is the inline verifier as a whole word. As a bare prefix it hijacked
/// any registry key starting with those letters — `USING shellcheck` parsed as
/// `shell` running `check` — which turns a plain registry reference into an
/// inline command. That is the escape the trusted registry exists to prevent.
#[test]
fn shell_is_a_token_not_a_prefix() {
    let inline = parse(&head(
        "CLAIM a REQUIRED\n  ENSURE ok\nVERIFY a\n  USING shell \"cargo test\"\n",
    ))
    .unwrap();
    assert_eq!(inline.verifications[0].verifier_ref, "shell");
    assert_eq!(
        inline.verifications[0].inline_shell.as_deref(),
        Some("cargo test")
    );

    for key in ["shellcheck", "shell-runner", "shells"] {
        let c = parse(&head(&format!(
            "CLAIM a REQUIRED\n  ENSURE ok\nVERIFY a\n  USING {key}\n"
        )))
        .unwrap();
        assert_eq!(
            c.verifications[0].verifier_ref, key,
            "{key} must stay a registry reference"
        );
        assert_eq!(
            c.verifications[0].inline_shell, None,
            "{key} must not carry an inline command"
        );
    }
}

#[test]
fn a_quoted_argument_is_captured_without_its_quotes() {
    let c = parse(&head(
        "CLAIM a REQUIRED\n  ENSURE ok\nVERIFY a\n  USING suite \"clamps above\"\n",
    ))
    .unwrap();
    assert_eq!(c.verifications[0].verifier_ref, "suite");
    assert_eq!(
        c.verifications[0].inline_shell.as_deref(),
        Some("clamps above")
    );
}
