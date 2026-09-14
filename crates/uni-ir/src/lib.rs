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
