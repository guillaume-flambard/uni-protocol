//! `uni brief` (v0.5): the deterministic work order handed to an implementing
//! agent. The contract already knows what evidence must exist to be accepted;
//! withholding that from the agent is what produced the study's one false
//! rejection (a correct fix whose test was named differently from the claim's
//! pinned verifier).
//!
//! This is guidance, not authority: nothing here executes, and the registry
//! remains the only place a command may come from. Output is byte-stable so it
//! can be committed, reviewed, and diffed like the contract itself.

use anyhow::Result;
use uni_ir::Ir;
use uni_verify::VerifierSpec;

/// Resolved evidence requirement for one claim.
pub struct ClaimBrief<'a> {
    pub claim: &'a uni_ir::ClaimIr,
    pub verifier_ref: Option<String>,
    pub spec: Option<VerifierSpec>,
    /// Why the requirement cannot execute as declared (unknown ref, etc).
    pub problem: Option<String>,
}

/// Deterministic extraction of the test selector a registry command pins, so
/// the work order can name the exact test the agent must create. Returns None
/// when the command does not look like a test invocation: guessing there would
/// be worse than saying nothing.
pub fn test_selector(command: &str) -> Option<String> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    // cargo test <name> [-- --exact]
    if let Some(i) = tokens.iter().position(|t| *t == "test") {
        if let Some(candidate) = tokens.get(i + 1) {
            if !candidate.starts_with('-') && *candidate != "--" && !candidate.starts_with("tests")
            {
                return Some((*candidate).to_string());
            }
        }
    }
    // node --test --test-name-pattern "<pattern>" | -k "<expr>"
    // The pattern may contain spaces, so parse the raw remainder, not tokens.
    for flag in ["--test-name-pattern", "-k"] {
        if let Some(pos) = command.find(flag) {
            let rest = command[pos + flag.len()..].trim_start();
            let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();
            if let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
                if let Some(end) = rest[1..].find(quote) {
                    return Some(rest[1..1 + end].to_string());
                }
            }
            let value: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    // python -m unittest module.Class.test
    if command.contains("-m unittest") {
        if let Some(after) = command.split("-m unittest").nth(1) {
            let module = after.split_whitespace().next().unwrap_or("");
            if let Some(last) = module.split('.').next_back() {
                if !last.is_empty() && module != "discover" {
                    return Some(last.to_string());
                }
            }
        }
    }
    None
}

pub fn build<'a>(
    ir: &'a Ir,
    registry: &std::collections::HashMap<String, VerifierSpec>,
) -> (Vec<ClaimBrief<'a>>, Vec<String>) {
    let mut out = vec![];
    let mut problems = vec![];
    for c in &ir.claims {
        let verification = ir.verification.iter().find(|v| v.claim_id == c.id);
        let (verifier_ref, spec, problem) = match verification {
            None => (
                None,
                None,
                Some("no VERIFY declared for this claim".to_string()),
            ),
            Some(v) => match registry.get(&v.verifier_ref) {
                Some(spec) => (Some(v.verifier_ref.clone()), Some(spec.clone()), None),
                None => {
                    // Inline shell is only legal while the registry is empty.
                    let msg = if registry.is_empty() {
                        None
                    } else {
                        Some(format!(
                            "verifier '{}' is not in the trusted registry",
                            v.verifier_ref
                        ))
                    };
                    (Some(v.verifier_ref.clone()), None, msg)
                }
            },
        };
        if c.required && verification.is_none() {
            problems.push(format!("claim '{}' has no VERIFY", c.id));
        }
        if let Some(p) = &problem {
            if registry
                .get(verifier_ref.as_deref().unwrap_or(""))
                .is_none()
            {
                problems.push(format!("claim '{}': {p}", c.id));
            }
        }
        out.push(ClaimBrief {
            claim: c,
            verifier_ref,
            spec,
            problem,
        });
    }
    (out, problems)
}

pub fn to_json(
    ir: &Ir,
    claims: &[ClaimBrief],
    problems: &[String],
    registry_hash: &str,
) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "intent": ir.intent.id,
        "domain": ir.intent.domain,
        "goal": ir.intent.goal,
        "acceptance": {
            "required_claims_verified": true,
            "critical_failures": 0,
        },
        "claims": claims.iter().map(|b| serde_json::json!({
            "id": b.claim.id,
            "kind": b.claim.kind,
            "required": b.claim.required,
            "critical": b.claim.critical,
            "ensure": b.claim.ensure,
            "verifier": b.verifier_ref,
            "evidence": b.spec.as_ref().map(|s| serde_json::json!({
                "kind": s.kind,
                // The token is never handed to the worker: it is replaced by the
                // placeholder an authorizing human will fill in.
                "command": s.run.replace(uni_verify::SELECTOR_TOKEN, "<your-test-name>"),
                "expect": s.expect,
                "expect_not": s.expect_not,
                "files": s.files,
                "timeout_secs": s.timeout,
            })),
            "problem": b.problem,
            "selector_template": b
                .spec
                .as_ref()
                .map(|s| s.run.contains(uni_verify::SELECTOR_TOKEN))
                .unwrap_or(false),
        })).collect::<Vec<_>>(),
        "registry_hash": registry_hash,
        "problems": problems,
        "note": "Guidance only. Commands run exclusively from the trusted registry; the agent's own report is never evidence.",
    }))
}

pub fn to_markdown(ir: &Ir, claims: &[ClaimBrief], problems: &[String]) -> String {
    let mut md = String::new();
    md.push_str(&format!("# Work order: {}\n\n", ir.intent.id));
    if !ir.intent.goal.is_empty() {
        md.push_str(&format!("Goal: {}\n\n", ir.intent.goal));
    }
    md.push_str(
        "Acceptance is decided from EVIDENCE, not from a report of completion.\n\
         A claim is accepted only when its verifier produces green evidence that is\n\
         still valid for the current commit; your own \"done\" message is never evidence.\n\n",
    );

    md.push_str("## Claims to satisfy\n\n");
    for b in claims {
        let critical = if b.claim.critical { " CRITICAL" } else { "" };
        let required = if b.claim.required {
            "required"
        } else {
            "optional"
        };
        md.push_str(&format!(
            "### {} ({}, {}{})\n",
            b.claim.id, b.claim.kind, required, critical
        ));
        if !b.claim.ensure.is_empty() {
            md.push_str(&format!("Must hold: {}\n", b.claim.ensure));
        }
        match (&b.verifier_ref, &b.spec) {
            (Some(r), Some(spec)) => {
                md.push_str(&format!("\nEvidence required (verifier `{r}`):\n"));
                if spec.kind == "file-hash" {
                    md.push_str("- type: file-hash (no command runs)\n");
                    if !spec.files.is_empty() {
                        md.push_str(&format!("- watched files: {}\n", spec.files.join(", ")));
                    }
                    if !spec.expect_sha256.is_empty() {
                        md.push_str("- expected sha256:\n");
                        for (p, h) in &spec.expect_sha256 {
                            md.push_str(&format!("  - `{p}` = `{h}`\n"));
                        }
                    }
                } else if spec.run.contains(uni_verify::SELECTOR_TOKEN) {
                    // Selector template: the worker names the test, a human
                    // authorizes which name counts. Say exactly that.
                    md.push_str(
                        "- write the test; choose a clear name; a human then authorizes\n  that exact name, and only then does it count as evidence:\n",
                    );
                    md.push_str(&format!(
                        "  `uni bind --claim {} --verifier {} --selector <your-test-name>`\n",
                        b.claim.id,
                        b.verifier_ref.as_deref().unwrap_or("?")
                    ));
                    md.push_str(&format!(
                        "- it will run as: `{}`\n",
                        spec.run
                            .replace(uni_verify::SELECTOR_TOKEN, "<your-test-name>")
                    ));
                    if !spec.expect.is_empty() {
                        md.push_str(&format!("- required in output: `{}`\n", spec.expect));
                    }
                } else {
                    md.push_str(&format!("- command: `{}`\n", spec.run));
                    if !spec.expect.is_empty() {
                        md.push_str(&format!("- required in output: `{}`\n", spec.expect));
                    }
                    if !spec.expect_not.is_empty() {
                        md.push_str(&format!("- forbidden in output: `{}`\n", spec.expect_not));
                    }
                    if !spec.files.is_empty() {
                        md.push_str(&format!(
                            "- content-bound files: {}\n",
                            spec.files.join(", ")
                        ));
                    }
                    if let Some(sel) = test_selector(&spec.run) {
                        md.push_str(&format!(
                            "\nThis command selects the test `{sel}`. That test must exist under\n\
                             that exact name; a differently named test proves nothing here.\n"
                        ));
                    }
                }
            }
            (Some(r), None) => {
                md.push_str(&format!(
                    "\nEvidence required (verifier `{r}`): NOT RESOLVABLE from the registry.\n"
                ));
            }
            _ => {
                md.push_str(
                    "\nEvidence required: nothing declared. This claim cannot be accepted.\n",
                );
            }
        }
        md.push('\n');
    }

    if !problems.is_empty() {
        md.push_str("## Problems in the contract\n\n");
        for p in problems {
            md.push_str(&format!("- {p}\n"));
        }
        md.push('\n');
    }
    md.push_str(
        "## Rules\n\n\
         - Commands run only from `.uni/config.toml [verifiers]`; do not invent commands.\n\
         - Do not edit the contract, the registry, or previously passing tests to make evidence green.\n\
         - Report what you actually did; UNI decides acceptance, not you.\n",
    );
    md
}

/// Stable digest of the registry, so a brief is bound to the trust boundary it
/// was generated against (a registry change invalidates it).
pub fn registry_hash(registry_text: &str) -> String {
    uni_evidence::sha256_hex(registry_text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_extraction_is_conservative() {
        assert_eq!(
            test_selector("cargo test clamp_upper_works -- --exact").as_deref(),
            Some("clamp_upper_works")
        );
        assert_eq!(test_selector("cargo test").as_deref(), None);
        assert_eq!(test_selector("cargo check"), None);
        assert_eq!(test_selector("true"), None);
        assert_eq!(
            test_selector("node --test --test-name-pattern \"spaces become dashes\" x.mjs")
                .as_deref(),
            Some("spaces become dashes")
        );
        assert_eq!(
            test_selector("pytest -k bump_floor").as_deref(),
            Some("bump_floor")
        );
        assert_eq!(
            test_selector("python3 -m unittest test_pricing.Pricing.test_floor").as_deref(),
            Some("test_floor")
        );
        assert_eq!(
            test_selector("python3 -m unittest discover -p 'test_*.py'").as_deref(),
            None
        );
    }
}
