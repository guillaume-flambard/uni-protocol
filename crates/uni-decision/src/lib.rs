use serde::{Deserialize, Serialize};
use uni_evidence::{Evidence, EvidenceState};
use uni_ir::Ir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Decision {
    Accepted,
    Rejected,
    EvidenceRequired,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimResult {
    pub claim_id: String,
    pub required: bool,
    pub critical: bool,
    pub state: EvidenceState,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub decision: Decision,
    pub claims: Vec<ClaimResult>,
    pub reason: String,
}

/// Deterministic truth table (PRD §11, §14):
/// - any critical Invalid => Rejected
/// - any required claim without Valid evidence => EvidenceRequired
/// - stale counts as missing (needs revalidation)
/// - else Accepted
pub fn evaluate(ir: &Ir, evidences: &[Evidence]) -> DecisionResult {
    let mut results = vec![];
    let mut critical_fail = 0;
    let mut missing = 0;

    for c in &ir.claims {
        let evs: Vec<&Evidence> = evidences.iter().filter(|e| e.claim_id == c.id).collect();
        let best = evs.iter().find(|e| e.state == EvidenceState::Valid);
        if let Some(ev) = best {
            if ev.exit_code != 0 {
                results.push(ClaimResult {
                    claim_id: c.id.clone(),
                    required: c.required,
                    critical: c.critical,
                    state: EvidenceState::Invalid,
                    detail: format!("exit {}", ev.exit_code),
                });
                if c.critical {
                    critical_fail += 1;
                } else if c.required {
                    missing += 1;
                }
            } else {
                results.push(ClaimResult {
                    claim_id: c.id.clone(),
                    required: c.required,
                    critical: c.critical,
                    state: EvidenceState::Valid,
                    detail: format!("evidence {}", ev.id),
                });
            }
        } else {
            let state = evs
                .first()
                .map(|e| e.state.clone())
                .unwrap_or(EvidenceState::Invalid);
            let state = match state {
                EvidenceState::Stale => EvidenceState::Stale,
                _ => EvidenceState::Invalid,
            };
            results.push(ClaimResult {
                claim_id: c.id.clone(),
                required: c.required,
                critical: c.critical,
                state,
                detail: "no valid evidence".into(),
            });
            if c.critical {
                // critical without proof blocks acceptance; count as missing unless explicitly failed
                let failed = evs.iter().any(|e| e.state == EvidenceState::Invalid);
                if failed {
                    critical_fail += 1;
                } else {
                    missing += 1;
                }
            } else if c.required {
                missing += 1;
            }
        }
    }

    let (decision, reason) = if critical_fail > 0 {
        (Decision::Rejected, format!("{critical_fail} critical failure(s)"))
    } else if missing > 0 {
        (
            Decision::EvidenceRequired,
            format!("{missing} required claim(s) without valid evidence"),
        )
    } else {
        (Decision::Accepted, "all required claims verified".into())
    };
    DecisionResult {
        decision,
        claims: results,
        reason,
    }
}

/// Top-level seam: evaluate an IR directly (assure/evaluate per plan §3).
pub fn evaluate_intent(ir: &Ir, evidences: &[Evidence]) -> DecisionResult {
    evaluate(ir, evidences)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ir1() -> Ir {
        Ir {
            uni_version: "0.1".into(),
            intent: uni_ir::IntentIr {
                id: "x".into(),
                domain: "software".into(),
                goal: "g".into(),
            },
            claims: vec![uni_ir::ClaimIr {
                id: "a".into(),
                kind: "claim".into(),
                required: true,
                critical: false,
                ensure: "e".into(),
            }],
            verification: vec![],
            acceptance: uni_ir::AcceptanceIr {
                require_verified: true,
                allow_critical_failures: 0,
            },
        }
    }
    fn ev(state: EvidenceState, code: i32) -> Evidence {
        Evidence {
            id: "a-x".into(),
            claim_id: "a".into(),
            producer: "t".into(),
            command: "c".into(),
            exit_code: code,
            output_hash: "h".into(),
            output_excerpt: "".into(),
            commit_sha: "s".into(),
            workspace_dirty: false,
            state,
            created_at: chrono::Utc::now(),
            duration_ms: 1,
        }
    }
    #[test]
    fn truth_table() {
        assert_eq!(
            evaluate(&ir1(), &[ev(EvidenceState::Valid, 0)]).decision,
            Decision::Accepted
        );
        assert_eq!(
            evaluate(&ir1(), &[]).decision,
            Decision::EvidenceRequired
        );
        assert_eq!(
            evaluate(&ir1(), &[ev(EvidenceState::Stale, 0)]).decision,
            Decision::EvidenceRequired
        );
        let mut ir = ir1();
        ir.claims[0].critical = true;
        assert_eq!(
            evaluate(&ir, &[ev(EvidenceState::Invalid, 1)]).decision,
            Decision::Rejected
        );
    }
}
