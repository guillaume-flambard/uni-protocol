use anyhow::Result;
use serde::{Deserialize, Serialize};
use uni_parser::Contract;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ir {
    pub uni_version: String,
    pub intent: IntentIr,
    pub claims: Vec<ClaimIr>,
    pub verification: Vec<VerificationIr>,
    pub acceptance: AcceptanceIr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentIr {
    pub id: String,
    pub domain: String,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimIr {
    pub id: String,
    pub kind: String,
    pub required: bool,
    pub critical: bool,
    pub ensure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIr {
    pub claim_id: String,
    pub verifier_ref: String,
    pub inline_shell: Option<String>,
    /// v0.2 resolution requirement; executable only via an authorized VerifierBinding.
    #[serde(default)]
    pub requirement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceIr {
    /// v0.1: always true; the grammar enforces the only supported semantics.
    pub require_verified: bool,
}

pub fn compile(c: &Contract) -> Result<Ir> {
    Ok(Ir {
        uni_version: c.version.clone(),
        intent: IntentIr {
            id: c.intent.clone(),
            domain: c.domain.clone(),
            goal: c.goal.clone(),
        },
        claims: c
            .claims
            .iter()
            .map(|k| ClaimIr {
                id: k.id.clone(),
                kind: match k.kind {
                    uni_parser::ClaimKind::Claim => "claim".into(),
                    uni_parser::ClaimKind::Invariant => "invariant".into(),
                },
                required: k.required,
                critical: k.critical,
                ensure: k.ensure.clone(),
            })
            .collect(),
        verification: c
            .verifications
            .iter()
            .map(|v| VerificationIr {
                claim_id: v.claim_id.clone(),
                verifier_ref: v.verifier_ref.clone(),
                inline_shell: v.inline_shell.clone(),
                requirement: v.requirement.clone(),
            })
            .collect(),
        acceptance: AcceptanceIr {
            require_verified: c.acceptance.require_verified,
        },
    })
}

pub fn to_json(ir: &Ir) -> Result<String> {
    Ok(serde_json::to_string_pretty(ir)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Contract {
        uni_parser::parse(
            "VERSION 0.1\nDOMAIN software\nINTENT demo\nGOAL\n ship it\nCLAIM a REQUIRED\n  ENSURE x\nINVARIANT b CRITICAL\n  ENSURE y\nVERIFY a\n  USING project.tests\nVERIFY b\n  USING project.tests\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n",
        )
        .unwrap()
    }

    #[test]
    fn compile_maps_everything() {
        let ir = compile(&sample()).unwrap();
        assert_eq!(ir.uni_version, "0.1");
        assert_eq!(ir.intent.id, "demo");
        assert_eq!(ir.claims.len(), 2);
        assert_eq!(ir.claims[0].kind, "claim");
        assert!(!ir.claims[0].critical);
        assert_eq!(ir.claims[1].kind, "invariant");
        assert!(ir.claims[1].critical);
        assert_eq!(ir.verification.len(), 2);
        assert!(ir.acceptance.require_verified);
    }

    #[test]
    fn to_json_round_trips_schema_keys() {
        let ir = compile(&sample()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&to_json(&ir).unwrap()).unwrap();
        for key in [
            "uni_version",
            "intent",
            "claims",
            "verification",
            "acceptance",
        ] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        assert!(
            v.get("constraints").is_none(),
            "constraints must be gone (A2)"
        );
    }
}
