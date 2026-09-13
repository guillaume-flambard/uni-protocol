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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimResult {
    pub claim_id: String,
    pub required: bool,
    pub critical: bool,
    pub state: EvidenceState,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    fn ir_n(n_required: usize, n_critical: usize) -> Ir {
        let mut claims = vec![];
        for i in 0..n_required {
            claims.push(uni_ir::ClaimIr {
                id: format!("r{i}"),
                kind: "claim".into(),
                required: true,
                critical: false,
                ensure: "e".into(),
            });
        }
        for i in 0..n_critical {
            claims.push(uni_ir::ClaimIr {
                id: format!("c{i}"),
                kind: "invariant".into(),
                required: true,
                critical: true,
                ensure: "e".into(),
            });
        }
        Ir {
            uni_version: "0.1".into(),
            intent: uni_ir::IntentIr {
                id: "x".into(),
                domain: "software".into(),
                goal: "g".into(),
            },
            claims,
            verification: vec![],
            acceptance: uni_ir::AcceptanceIr {
                require_verified: true,
                allow_critical_failures: 0,
            },
        }
    }

    fn ev_of(state: EvidenceState, code: i32) -> impl Fn(String) -> Evidence {
        move |claim_id: String| Evidence {
            id: format!("{claim_id}-ev"),
            claim_id,
            producer: "t".into(),
            command: "c".into(),
            exit_code: code,
            output_hash: "h".into(),
            output_excerpt: "".into(),
            commit_sha: "s".into(),
            workspace_dirty: false,
            state: state.clone(),
            created_at: chrono::Utc::now(),
            duration_ms: 1,
        }
    }

    proptest! {
        #[test]
        fn determinism(n in 0usize..4, m in 0usize..3) {
            let ir = ir_n(n, m);
            let mk = ev_of(EvidenceState::Valid, 0);
            let evs: Vec<Evidence> = ir.claims.iter().map(|c| mk(c.id.clone())).collect();
            let d1 = evaluate(&ir, &evs);
            let d2 = evaluate(&ir, &evs);
            assert_eq!((d1.decision, d1.claims), (d2.decision, d2.claims));
        }

        #[test]
        fn all_valid_required_is_accepted(n in 0usize..5) {
            let ir = ir_n(n, 0);
            let mk = ev_of(EvidenceState::Valid, 0);
            let evs: Vec<Evidence> = ir.claims.iter().map(|c| mk(c.id.clone())).collect();
            assert_eq!(evaluate(&ir, &evs).decision, Decision::Accepted);
        }

        #[test]
        fn any_invalid_critical_rejected(n in 0usize..4, m in 1usize..3) {
            let ir = ir_n(n, m);
            let mut evs: Vec<Evidence> = vec![];
            let mk_ok = ev_of(EvidenceState::Valid, 0);
            let mk_bad = ev_of(EvidenceState::Invalid, 1);
            for (i, c) in ir.claims.iter().enumerate() {
                // first critical claim fails
                if c.critical && evs.iter().all(|e| e.state != EvidenceState::Invalid) && i >= n {
                    evs.push(mk_bad(c.id.clone()));
                } else {
                    evs.push(mk_ok(c.id.clone()));
                }
            }
            prop_assert_eq!(evaluate(&ir, &evs).decision, Decision::Rejected);
        }
    }
}

#[cfg(test)]
mod decision_matrix {
    use super::*;

    // Exhaustive truth table: {valid, missing, stale, invalid} evidence per claim type.
    #[test]
    fn required_claim_matrix() {
        let ir = Ir {
            uni_version: "0.1".into(),
            intent: uni_ir::IntentIr { id: "x".into(), domain: "s".into(), goal: "g".into() },
            claims: vec![uni_ir::ClaimIr {
                id: "a".into(), kind: "claim".into(), required: true, critical: false, ensure: "e".into(),
            }],
            verification: vec![],
            acceptance: uni_ir::AcceptanceIr { require_verified: true, allow_critical_failures: 0 },
        };
        let mk = |st: EvidenceState, code: i32| Evidence {
            id: "a-e".into(), claim_id: "a".into(), producer: "t".into(), command: "c".into(),
            exit_code: code, output_hash: "h".into(), output_excerpt: "".into(),
            commit_sha: "s".into(), workspace_dirty: false, state: st,
            created_at: chrono::Utc::now(), duration_ms: 1,
        };
        // valid + exit 0 → Accepted
        assert_eq!(evaluate(&ir, &[mk(EvidenceState::Valid, 0)]).decision, Decision::Accepted);
        // valid but exit != 0 → EvidenceRequired (no critical)
        assert_eq!(evaluate(&ir, &[mk(EvidenceState::Valid, 1)]).decision, Decision::EvidenceRequired);
        // missing
        assert_eq!(evaluate(&ir, &[]).decision, Decision::EvidenceRequired);
        // stale → needs revalidation
        assert_eq!(evaluate(&ir, &[mk(EvidenceState::Stale, 0)]).decision, Decision::EvidenceRequired);
        // invalid
        assert_eq!(evaluate(&ir, &[mk(EvidenceState::Invalid, 1)]).decision, Decision::EvidenceRequired);

        // critical: missing → EvidenceRequired (cannot silently accept), invalid → REJECTED
        let mut irc = ir.clone();
        irc.claims[0].critical = true;
        assert_eq!(evaluate(&irc, &[mk(EvidenceState::Valid, 0)]).decision, Decision::Accepted);
        assert_eq!(evaluate(&irc, &[mk(EvidenceState::Invalid, 1)]).decision, Decision::Rejected);
        // partial: 2 required, one valid one missing
        let mut ir2 = ir.clone();
        ir2.claims.push(uni_ir::ClaimIr {
            id: "b".into(), kind: "claim".into(), required: true, critical: false, ensure: "e".into(),
        });
        assert_eq!(evaluate(&ir2, &[mk(EvidenceState::Valid, 0)]).decision, Decision::EvidenceRequired);
    }
}
