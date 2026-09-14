use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contract {
    pub version: String,
    pub domain: String,
    pub intent: String,
    pub goal: String,
    pub claims: Vec<Claim>,
    pub verifications: Vec<Verification>,
    pub acceptance: Acceptance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claim {
    pub id: String,
    pub kind: ClaimKind,
    pub required: bool,
    pub critical: bool,
    pub ensure: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClaimKind {
    Claim,
    Invariant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Verification {
    pub claim_id: String,
    pub verifier_ref: String,
    pub inline_shell: Option<String>,
    pub line: usize,
    /// v0.2: optional resolution requirement, e.g. behavior("clamps above").
    /// Must immediately follow the VERIFY/USING block; authorizes via `uni bind`.
    #[serde(default)]
    pub requirement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Acceptance {
    /// v0.1: always true. The grammar enforces the only supported acceptance
    /// semantics (`required_claims == VERIFIED`); there is no configurable value.
    pub require_verified: bool,
}

/// Clauses the v0.1 grammar accepts inside ACCEPT WHEN. Anything else is a
/// hard error: the spec must never promise inert semantics.
fn validate_accept_condition(text: &str, line_no: usize) -> Result<()> {
    let parts: Vec<&str> = text
        .split("AND")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(anyhow!(
            "line {line_no}: ACCEPT WHEN needs a condition (v0.1 supports only 'required_claims == VERIFIED AND critical_failures == 0')"
        ));
    }
    let mut seen_required = false;
    for part in &parts {
        match *part {
            "required_claims == VERIFIED" => {
                if seen_required {
                    return Err(anyhow!("line {line_no}: duplicate clause 'required_claims == VERIFIED'"));
                }
                seen_required = true;
            }
            "critical_failures == 0" => {}
            other => {
                return Err(anyhow!(
                    "line {line_no}: unsupported ACCEPT WHEN clause '{other}' (v0.1 supports only 'required_claims == VERIFIED AND critical_failures == 0')"
                ));
            }
        }
    }
    if !seen_required {
        return Err(anyhow!(
            "line {line_no}: ACCEPT WHEN must include 'required_claims == VERIFIED'"
        ));
    }
    Ok(())
}

fn push_verification(
    claim_id: &str,
    using_raw: &str,
    vline: usize,
    line_no: usize,
    verifications: &mut Vec<Verification>,
) -> Result<()> {
    let (verifier_ref, inline_shell) = if let Some(q) = using_raw.strip_prefix("shell") {
        let cmd = q.trim().trim_matches('"').to_string();
        ("shell".to_string(), Some(cmd))
    } else {
        let mut tokens = using_raw.splitn(2, '"');
        let head = tokens.next().unwrap_or("").trim().to_string();
        let quoted = tokens.next().map(|s| s.trim_end_matches('"').to_string());
        (head, quoted)
    };
    if claim_id.is_empty() || verifier_ref.is_empty() {
        return Err(anyhow!("line {line_no}: VERIFY needs claim id and verifier ref"));
    }
    verifications.push(Verification {
        claim_id: claim_id.to_string(),
        verifier_ref,
        inline_shell,
        line: vline,
        requirement: None,
    });
    Ok(())
}

pub fn parse(source: &str) -> Result<Contract> {
    let mut version: Option<String> = None;
    let mut domain: Option<String> = None;
    let mut intent: Option<String> = None;
    let mut goal = String::new();
    let mut in_goal = false;
    let mut claims: Vec<Claim> = vec![];
    let mut verifications: Vec<Verification> = vec![];
    let acceptance = Acceptance {
        require_verified: true,
    };

    let mut pending_verify: Option<(String, usize)> = None;
    let mut pending_forbid: Option<usize> = None;
    // Line of the last completed VERIFY/USING block; a REQUIRE line may only
    // attach to it across blank/comment lines (v0.2 VerifierBinding).
    let mut last_using_line: Option<usize> = None;
    // ACCEPT WHEN continuations are consumed by lookahead so no flag is needed.
    let raw_lines: Vec<&str> = source.lines().collect();
    let mut skip_until = 0usize;
    for (idx, raw) in raw_lines.iter().enumerate() {
        if idx < skip_until {
            continue;
        }
        let line_no = idx + 1;
        let line = raw.trim();
        if line.starts_with("USING") {
            let (claim_id, vline) = pending_verify.take().ok_or_else(|| {
                anyhow!("line {line_no}: USING without preceding VERIFY")
            })?;
            push_verification(
                &claim_id,
                line.strip_prefix("USING").unwrap().trim(),
                vline,
                line_no,
                &mut verifications,
            )?;
            last_using_line = Some(line_no);
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            if in_goal && line.is_empty() {
                in_goal = false;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("VERSION") {
            version = Some(rest.trim().to_string());
            in_goal = false;
            continue;
        }
        if let Some(rest) = line.strip_prefix("DOMAIN") {
            domain = Some(rest.trim().to_string());
            in_goal = false;
            continue;
        }
        if let Some(rest) = line.strip_prefix("INTENT") {
            intent = Some(rest.trim().to_string());
            in_goal = false;
            continue;
        }
        if line == "GOAL" {
            in_goal = true;
            continue;
        }
        if in_goal {
            if line.starts_with("CLAIM")
                || line.starts_with("INVARIANT")
                || line.starts_with("FORBID")
                || line.starts_with("VERIFY")
                || line.starts_with("ACCEPT")
            {
                in_goal = false;
            } else {
                if !goal.is_empty() {
                    goal.push(' ');
                }
                goal.push_str(line);
                continue;
            }
        }
        if let Some(rest) = line.strip_prefix("CLAIM") {
            // CLAIM <id> REQUIRED | OPTIONAL
            let parts: Vec<&str> = rest.trim().split_whitespace().collect();
            if parts.is_empty() {
                return Err(anyhow!("line {line_no}: CLAIM needs an id"));
            }
            let id = parts[0].to_string();
            let required = !parts.iter().any(|p| *p == "OPTIONAL");
            // next line(s) ENSURE ... — handled in second pass? read inline: ENSURE may be same line after?
            let _ = required;
            claims.push(Claim {
                id,
                kind: ClaimKind::Claim,
                required,
                critical: false,
                ensure: String::new(),
                line: line_no,
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("INVARIANT") {
            let parts: Vec<&str> = rest.trim().split_whitespace().collect();
            if parts.is_empty() {
                return Err(anyhow!("line {line_no}: INVARIANT needs an id"));
            }
            let id = parts[0].to_string();
            let critical = parts.iter().any(|p| *p == "CRITICAL");
            claims.push(Claim {
                id,
                kind: ClaimKind::Invariant,
                required: true,
                critical,
                ensure: String::new(),
                line: line_no,
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("ENSURE") {
            let text = rest.trim().to_string();
            match claims.last_mut() {
                Some(c) => c.ensure = text,
                None => return Err(anyhow!("line {line_no}: ENSURE without preceding CLAIM/INVARIANT")),
            }
            continue;
        }
        if line.starts_with("FORBID") && line.trim() != "FORBID" {
            let expr = line.strip_prefix("FORBID").unwrap().trim().to_string();
            claims.push(Claim {
                id: format!("forbid-{}", claims.len() + 1),
                kind: ClaimKind::Invariant,
                required: true,
                critical: true,
                ensure: format!("FORBID {expr}"),
                line: line_no,
            });
            continue;
        }
        if line == "FORBID" {
            pending_forbid = Some(line_no);
            continue;
        }
        if pending_forbid.is_some() {
            let vline = pending_forbid.take().unwrap();
            claims.push(Claim {
                id: format!("forbid-{}", claims.len() + 1),
                kind: ClaimKind::Invariant,
                required: true,
                critical: true,
                ensure: format!("FORBID {line}"),
                line: vline,
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("VERIFY") {
            // forms: VERIFY id USING <ref> (same line) | VERIFY id (USING on next line)
            let parts = rest.trim();
            if let Some((claim_part, using_part)) = parts.split_once("USING") {
                let claim_id = claim_part.trim().to_string();
                push_verification(
                    &claim_id,
                    using_part.trim(),
                    line_no,
                    line_no,
                    &mut verifications,
                )?;
                last_using_line = Some(line_no);
            } else {
                let claim_id = parts.to_string();
                if claim_id.is_empty() {
                    return Err(anyhow!("line {line_no}: VERIFY needs a claim id"));
                }
                pending_verify = Some((claim_id, line_no));
            }
            continue;
        }
        if line.starts_with("ACCEPT WHEN") {
            let mut text = line.strip_prefix("ACCEPT WHEN").unwrap().trim().to_string();
            let mut j = idx + 1;
            while j < raw_lines.len() {
                let nx = raw_lines[j].trim();
                if nx.starts_with("required_claims")
                    || nx.starts_with("critical_failures")
                    || nx.starts_with("AND ")
                    || nx == "AND"
                {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(nx);
                    j += 1;
                } else {
                    break;
                }
            }
            skip_until = j;
            validate_accept_condition(&text, line_no)?;
            continue;
        }
        if line.starts_with("REJECT WHEN") {
            return Err(anyhow!(
                "line {line_no}: REJECT WHEN is reserved for v0.2 (Verification Context); remove it or see docs/specification.md"
            ));
        }
        if line.starts_with("ESCALATE WHEN") {
            return Err(anyhow!(
                "line {line_no}: ESCALATE WHEN is reserved for v0.2 (Verification Context); remove it or see docs/specification.md"
            ));
        }
        if line.starts_with("REQUIRE") {
            let expr = line.strip_prefix("REQUIRE").unwrap().trim().to_string();
            if expr.is_empty() {
                return Err(anyhow!("line {line_no}: REQUIRE needs an expression, e.g. REQUIRE behavior(\"...\")"));
            }
            if pending_verify.is_some() {
                return Err(anyhow!(
                    "line {line_no}: REQUIRE without completed VERIFY/USING block (expected USING first)"
                ));
            }
            let vline = last_using_line.ok_or_else(|| {
                anyhow!("line {line_no}: REQUIRE must immediately follow a VERIFY/USING block (standalone REQUIRE is reserved)")
            })?;
            // Only blank/comment lines may sit between USING and REQUIRE.
            for k in vline..line_no - 1 {
                let gap = raw_lines.get(k).map(|s| s.trim()).unwrap_or("");
                if !(gap.is_empty() || gap.starts_with('#')) {
                    return Err(anyhow!(
                        "line {line_no}: REQUIRE must immediately follow a VERIFY/USING block (found {gap:?} in between)"
                    ));
                }
            }
            let target = verifications.last_mut().ok_or_else(|| {
                anyhow!("line {line_no}: REQUIRE without preceding VERIFY")
            })?;
            if target.requirement.is_some() {
                return Err(anyhow!(
                    "line {line_no}: duplicate REQUIRE for claim '{}'",
                    target.claim_id
                ));
            }
            target.requirement = Some(expr);
            continue;
        }
        return Err(anyhow!("line {line_no}: unknown directive: {line}"));
    }

    let version = version.ok_or_else(|| anyhow!("missing VERSION"))?;
    let domain = domain.ok_or_else(|| anyhow!("missing DOMAIN"))?;
    let intent = intent.ok_or_else(|| anyhow!("missing INTENT"))?;
    if claims.is_empty() {
        return Err(anyhow!("contract has no CLAIM/INVARIANT"));
    }
    // link check
    for v in &verifications {
        if !claims.iter().any(|c| c.id == v.claim_id) {
            return Err(anyhow!(
                "line {}: VERIFY references unknown claim '{}'",
                v.line,
                v.claim_id
            ));
        }
    }
    Ok(Contract {
        version,
        domain,
        intent,
        goal,
        claims,
        verifications,
        acceptance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_hello() {
        let src = r#"VERSION 0.1
DOMAIN software
INTENT hello
GOAL
  Binary builds.
CLAIM binary-builds REQUIRED
  ENSURE binary builds
VERIFY binary-builds
  USING shell "cargo check"
ACCEPT WHEN
  required_claims == VERIFIED
  AND critical_failures == 0
"#;
        let c = parse(src).unwrap();
        assert_eq!(c.intent, "hello");
        assert_eq!(c.claims.len(), 1);
    }
}
