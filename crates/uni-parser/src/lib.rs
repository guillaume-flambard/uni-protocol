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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Acceptance {
    pub require_verified: bool,
    pub allow_critical_failures: u32,
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
    let mut acceptance = Acceptance {
        require_verified: true,
        allow_critical_failures: 0,
    };

    let mut pending_verify: Option<(String, usize)> = None;
    for (idx, raw) in source.lines().enumerate() {
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
        if line.starts_with("FORBID") {
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
            let rest = line.strip_prefix("ACCEPT WHEN").unwrap();
            if rest.contains("required_claims == VERIFIED") {
                acceptance.require_verified = true;
            }
            if rest.contains("critical_failures == 0") {
                acceptance.allow_critical_failures = 0;
            }
            continue;
        }
        if line.starts_with("required_claims")
            || line.starts_with("critical_failures")
            || line.starts_with("AND ")
            || line == "AND"
        {
            if line.contains("required_claims == VERIFIED") {
                acceptance.require_verified = true;
            }
            continue;
        }
        if line.starts_with("REJECT") || line.starts_with("ESCALATE") || line.starts_with("REQUIRE") {
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
