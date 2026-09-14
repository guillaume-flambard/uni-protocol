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

/// Single source of truth for decision -> assurance level (v0.1: derived
/// from the decision only; v0.2 derives it from the evidence graph with
/// actor separation). Every display site must call this, never re-match.
pub fn assurance_of(decision: &Decision) -> u8 {
    match decision {
        Decision::Accepted => 2,
        Decision::Rejected => 1,
        _ => 0,
    }
}

pub fn assurance_label(level: u8) -> &'static str {
    match level {
        0 => "DECLARED",
        1 => "ARTIFACT",
        2 => "VERIFIED",
        3 => "INDEPENDENTLY_VERIFIED",
        4 => "ATTESTED",
        _ => "?",
    }
}

/// assurance_of for decision values read back from JSON (report/explain).
/// Unparseable values map to 0, never to a higher level.
pub fn assurance_of_json(v: &serde_json::Value) -> u8 {
    serde_json::from_value::<Decision>(v.clone())
        .map(|d| assurance_of(&d))
        .unwrap_or(0)
}

/// Independence of the evidence graph (B3): is every proof produced by an
/// actor distinct from the one who launched the work, with no anonymous
/// producers? Independence says nothing about identity strength (see below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Independence {
    Independent,
    SameActor,
    Anonymous,
}

pub fn independence(evidences: &[Evidence]) -> Independence {
    if evidences.is_empty() {
        return Independence::Anonymous;
    }
    for ev in evidences {
        if ev.actor.is_anonymous() || ev.executor.is_anonymous() {
            return Independence::Anonymous;
        }
        if ev.actor.id == ev.executor.id {
            return Independence::SameActor;
        }
    }
    Independence::Independent
}

/// Identity assurance across the evidence set: VERIFIED only if every actor
/// carries an externally verified identity. Anything else is SELF-DECLARED.
/// (v0.2 sets "verified" nowhere: identity adapters are documented stubs.)
pub fn identity_assurance(evidences: &[Evidence]) -> &'static str {
    if evidences.is_empty() {
        return "SELF-DECLARED";
    }
    if evidences.iter().all(|ev| ev.actor.is_verified()) {
        "VERIFIED"
    } else {
        "SELF-DECLARED"
    }
}

/// Full assurance level derived from decision + evidence graph (v0.2).
/// A2: trusted verifier. A3-D: independent actors, self-declared identities
/// (logically independent, identity unproven). A3: independent + externally
/// verified identities. A4 is never returned here: signed provenance has no
/// producer yet (see --attest refusal in the CLI).
pub fn assurance_for(decision: &Decision, evidences: &[Evidence]) -> &'static str {
    if *decision != Decision::Accepted {
        return match assurance_of(decision) {
            1 => "A1",
            _ => "A0",
        };
    }
    match (independence(evidences), identity_assurance(evidences)) {
        (Independence::Independent, "VERIFIED") => "A3",
        (Independence::Independent, _) => "A3-D",
        _ => "A2",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimResult {
    pub claim_id: String,
    pub required: bool,
    pub critical: bool,
    pub state: EvidenceState,
    pub detail: String,
}

/// Policy layer v0.9: deterministic adjustments on top of the evidence truth table.
/// Lives in `.uni/policies/*.toml` (`[policy]` table). UNI never invents policy.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Policy {
    /// REJECT the outcome when any claim evidence is Invalid (default: current behavior)
    #[serde(default = "default_true")]
    pub reject_on_invalid: bool,
    /// ESCALATE instead of EVIDENCE_REQUIRED when evidence went stale
    #[serde(default)]
    pub escalate_on_stale: bool,
    /// ESCALATE when required claims lack evidence
    #[serde(default)]
    pub escalate_on_missing: bool,
    /// Minimum verified claim ratio: below this, REJECT regardless
    #[serde(default)]
    pub min_verified_ratio: f64,
}

fn default_true() -> bool {
    true
}

/// Deterministic policy application on top of the engine result.
pub fn apply_policy(
    mut result: DecisionResult,
    policy: &Policy,
) -> DecisionResult {
    let total = result.claims.len();
    let verified = result.claims.iter().filter(|c| c.state == EvidenceState::Valid).count();

    let ratio: f64 = verified as f64 / total.max(1) as f64;
    if total > 0 && policy.min_verified_ratio > 0.0 && ratio < policy.min_verified_ratio {
        result.decision = Decision::Rejected;
        result.reason = format!(
            "policy min_verified_ratio: {verified}/{total} below {:.0}%",
            policy.min_verified_ratio * 100.0
        );
        return result;
    }

    let has_stale = result.claims.iter().any(|c| c.state == EvidenceState::Stale);
    let has_invalid = result.claims.iter().any(|c| c.state == EvidenceState::Invalid);

    if has_stale
        && policy.escalate_on_stale
        && result.decision != Decision::Accepted
    {
        result.decision = Decision::Escalated;
        result.reason = format!("policy escalate_on_stale: {}", result.reason);
        return result;
    }
    let base = format!("{:?}", result.decision);
    if has_invalid && !policy.reject_on_invalid && base == "Rejected" {
        result.decision = Decision::Escalated;
        result.reason = format!("policy reject_on_invalid=false: {}", result.reason);
        return result;
    }
    if has_invalid
        && policy.escalate_on_missing
        && base == "EvidenceRequired"
    {
        result.decision = Decision::Escalated;
        result.reason = format!("policy escalate_on_missing: {}", result.reason);
    }
    result
}

/// Load the merged policy from `.uni/policies/*.toml` ([policy] tables; files sorted by name,
/// Policy provider seam (v0.12). Core never contains a policy engine: only
/// deterministic providers. Deterministic contract: same (inputs, provider
/// state) → same outcome, and providers must not depend on wall-clock or RNG.
pub trait PolicyProvider {
    fn resolve(&self) -> Policy;
}

/// Local TOML stack (`[policy]` tables in `.uni/policies/*.toml`).
pub struct TomlPolicy<'a> {
    pub dir: &'a std::path::Path,
}

impl PolicyProvider for TomlPolicy<'_> {
    fn resolve(&self) -> Policy {
        load_policies(self.dir)
    }
}

/// Files sorted by name; later boolean values win, ratio takes the max.
pub fn load_policies(dir: &std::path::Path) -> Policy {
    let mut policy = Policy { reject_on_invalid: true, ..Default::default() };
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut files: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "toml").unwrap_or(false))
            .collect();
        files.sort();
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else {
                continue;
            };
            let Ok(val) = text.parse::<toml::Value>() else {
                continue;
            };
            let Some(p) = val.get("policy").and_then(|p| p.as_table()) else {
                continue;
            };
            let get_bool = |k: &str| p.get(k).and_then(|v| v.as_bool());
            let get_f64 = |k: &str| p.get(k).and_then(|v| v.as_float());
            if let Some(b) = get_bool("reject_on_invalid") {
                policy.reject_on_invalid = b;
            }
            if let Some(b) = get_bool("escalate_on_stale") {
                policy.escalate_on_stale = policy.escalate_on_stale || b;
            }
            if let Some(b) = get_bool("escalate_on_missing") {
                policy.escalate_on_missing = policy.escalate_on_missing || b;
            }
            if let Some(f) = get_f64("min_verified_ratio") {
                policy.min_verified_ratio = policy.min_verified_ratio.max(f);
            }
        }
    }
    policy
}

/// OPA outbound adapter (wired, engine stays external). Expects a rego
/// bundle exposing `data.uni.rules` boolean fields mirroring the Policy
/// shape. Falls back to defaults when `opa` is absent or the bundle fails
/// to evaluate; never blocks UNI.
pub struct OpaPolicy {
    pub bundle: std::path::PathBuf,
}

fn escalate(b: Option<bool>) -> bool {
    b.unwrap_or(false)
}

impl PolicyProvider for OpaPolicy {
    fn resolve(&self) -> Policy {
        let output = std::process::Command::new("opa")
            .args(["eval", "data.uni.rules", "-d", self.bundle.display().to_string().as_str(), "-f", "values"])
            .output();
        let Ok(out) = output else {
            return Policy { reject_on_invalid: true, ..Default::default() };
        };
        if !out.status.success() {
            return Policy { reject_on_invalid: true, ..Default::default() };
        }
        let Ok(text) = String::from_utf8(out.stdout) else {
            return Policy { reject_on_invalid: true, ..Default::default() };
        };
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        let first = v.as_array().and_then(|a| a.first()).and_then(|x| x.as_object()).cloned().unwrap_or_default();
        Policy {
            reject_on_invalid: first.get("reject_on_invalid").and_then(|x| x.as_bool()).unwrap_or(true),
            escalate_on_stale: escalate(first.get("escalate_on_stale").and_then(|x| x.as_bool())),
            escalate_on_missing: escalate(first.get("escalate_on_missing").and_then(|x| x.as_bool())),
            min_verified_ratio: first.get("min_verified_ratio").and_then(|x| x.as_f64()).unwrap_or(0.0),
        }
    }
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
            artifact_hash: String::new(),
            fingerprint: String::new(),
            expires_at: None,
            registry_hash: String::new(),
            policy_hash: String::new(),
            contract_hash: String::new(),
            platform: String::new(),
            binding_hash: String::new(),
            actor: uni_evidence::Actor::local(),
            executor: uni_evidence::Actor::local(),
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
            artifact_hash: String::new(),
            fingerprint: String::new(),
            expires_at: None,
            registry_hash: String::new(),
            policy_hash: String::new(),
            contract_hash: String::new(),
            platform: String::new(),
            binding_hash: String::new(),
            actor: uni_evidence::Actor::local(),
            executor: uni_evidence::Actor::local(),
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
            acceptance: uni_ir::AcceptanceIr { require_verified: true },
        };
        let mk = |st: EvidenceState, code: i32| Evidence {
            id: "a-e".into(), claim_id: "a".into(), producer: "t".into(), command: "c".into(),
            exit_code: code, output_hash: "h".into(), output_excerpt: "".into(),
            commit_sha: "s".into(), workspace_dirty: false, state: st,
            created_at: chrono::Utc::now(), duration_ms: 1, artifact_hash: String::new(), fingerprint: String::new(),
            expires_at: None,
            registry_hash: String::new(),
            policy_hash: String::new(),
            contract_hash: String::new(),
            platform: String::new(),
            binding_hash: String::new(),
            actor: uni_evidence::Actor::local(),
            executor: uni_evidence::Actor::local(),
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

#[cfg(test)]
mod policy_tests {
    use super::*;
    use uni_evidence::EvidenceState;
    use uni_ir::{AcceptanceIr, ClaimIr, IntentIr, Ir};

    fn ir_claims(n: usize) -> Ir {
        Ir {
            uni_version: "0.1".into(),
            intent: IntentIr { id: "p".into(), domain: "s".into(), goal: "g".into() },
            claims: (0..n)
                .map(|i| ClaimIr {
                    id: format!("c{i}"),
                    kind: "claim".into(),
                    required: true,
                    critical: false,
                    ensure: "e".into(),
                })
                .collect(),
            verification: vec![],
            acceptance: AcceptanceIr { require_verified: true },
        }
    }
    fn ev(claim: &str, state: EvidenceState, code: i32) -> Evidence {
        Evidence {
            id: format!("{claim}-e"),
            claim_id: claim.into(),
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
            artifact_hash: String::new(),
            fingerprint: String::new(),
            expires_at: None,
            registry_hash: String::new(),
            policy_hash: String::new(),
            contract_hash: String::new(),
            platform: String::new(),
            binding_hash: String::new(),
            actor: uni_evidence::Actor::local(),
            executor: uni_evidence::Actor::local(),
        }
    }

    #[test]
    fn default_policy_matches_legacy_behavior() {
        let ir = ir_claims(1);
        let base = evaluate(&ir, &[]);
        let out = apply_policy(base, &Policy { reject_on_invalid: true, ..Default::default() });
        assert_eq!(out.decision, Decision::EvidenceRequired);
    }

    #[test]
    fn escalate_on_stale_upgrades_required() {
        let ir = ir_claims(1);
        let base = evaluate(&ir, &[ev("c0", EvidenceState::Stale, 0)]);
        let out = apply_policy(base, &Policy { escalate_on_stale: true, ..Default::default() });
        assert_eq!(out.decision, Decision::Escalated);
        assert!(out.reason.contains("escalate_on_stale"));
    }

    #[test]
    fn min_verified_ratio_rejects() {
        let ir = ir_claims(4);
        let evs: Vec<Evidence> = (0..1).map(|i| ev(&format!("c{i}"), EvidenceState::Valid, 0)).collect();
        let base = evaluate(&ir, &evs);
        let out = apply_policy(base, &Policy { min_verified_ratio: 0.75, ..Default::default() });
        assert_eq!(out.decision, Decision::Rejected);
        assert!(out.reason.contains("min_verified_ratio"));
    }

    #[test]
    fn policies_load_from_toml_stack() {
        let dir = std::env::temp_dir().join(format!("uni-pol-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("01-base.toml"),
"[policy]\nescalate_on_stale = true\nmin_verified_ratio = 0.5\n").unwrap();
        std::fs::write(dir.join("02-override.toml"),
"[policy]\nreject_on_invalid = false\n").unwrap();
        let p = load_policies(&dir);
        assert!(!p.reject_on_invalid);
        assert!(p.escalate_on_stale);
        assert!((p.min_verified_ratio - 0.5).abs() < f64::EPSILON);
    }
}

#[cfg(test)]
mod policy_property {
    use super::*;
    use proptest::prelude::*;
    use uni_ir::{ClaimIr, IntentIr, Ir};

    fn ir_n(n: usize) -> Ir {
        Ir {
            uni_version: "0.1".into(),
            intent: IntentIr { id: "x".into(), domain: "s".into(), goal: "g".into() },
            claims: (0..n)
                .map(|i| ClaimIr {
                    id: format!("c{i}"),
                    kind: "claim".into(),
                    required: true,
                    critical: false,
                    ensure: "e".into(),
                })
                .collect(),
            verification: vec![],
            acceptance: uni_ir::AcceptanceIr { require_verified: true },
        }
    }
    fn ev(claim: &str, state: EvidenceState, code: i32) -> Evidence {
        Evidence {
            id: format!("{claim}-e"),
            claim_id: claim.into(),
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
            artifact_hash: String::new(),
            fingerprint: String::new(),
            expires_at: None,
            registry_hash: String::new(),
            policy_hash: String::new(),
            contract_hash: String::new(),
            platform: String::new(),
            binding_hash: String::new(),
            actor: uni_evidence::Actor::local(),
            executor: uni_evidence::Actor::local(),
        }
    }

    proptest! {
        #[test]
        fn policy_determinism(n in 1usize..5,
                             s0 in prop::sample::select(vec![0usize,1usize,2usize,3usize]),
                             s1 in prop::sample::select(vec![0usize,1usize,2usize,3usize]),
                             ratio in 0.0f64..=1.0) {
            let ir = ir_n(n);
            let state_of = |sel: usize| match sel {
                1 => (EvidenceState::Invalid, 1),
                2 => (EvidenceState::Stale, 0),
                _ => (EvidenceState::Valid, 0),
            };
            let evs: Vec<Evidence> = (0..n)
                .map(|i: usize| {
                    let sel = if i == 0 { s0 } else if i == 1 { s1 } else { 0 };
                    let (state, code) = state_of(sel);
                    ev(&format!("c{i}"), state, code)
                })
                .collect();
            let policy = Policy {
                reject_on_invalid: true,
                escalate_on_stale: n % 2 == 0,
                escalate_on_missing: false,
                min_verified_ratio: ratio,
            };
            let d1 = apply_policy(evaluate(&ir, &evs), &policy);
            let d2 = apply_policy(evaluate(&ir, &evs), &policy);
            assert_eq!((d1.decision, d1.reason, d1.claims), (d2.decision, d2.reason, d2.claims));
        }
    }
}

#[cfg(test)]
mod assurance_tests {
    use super::*;

    #[test]
    fn assurance_mapping_is_total() {
        assert_eq!(assurance_of(&Decision::Accepted), 2);
        assert_eq!(assurance_of(&Decision::Rejected), 1);
        assert_eq!(assurance_of(&Decision::EvidenceRequired), 0);
        assert_eq!(assurance_of(&Decision::Escalated), 0);
        assert_eq!(assurance_label(2), "VERIFIED");
        assert_eq!(assurance_label(99), "?");
    }

    #[test]
    fn assurance_of_json_never_upgrades_garbage() {
        assert_eq!(assurance_of_json(&serde_json::json!("Accepted")), 2);
        assert_eq!(assurance_of_json(&serde_json::json!("nonsense")), 0);
        assert_eq!(assurance_of_json(&serde_json::json!(null)), 0);
    }
}

#[cfg(test)]
mod independence_tests {
    use super::*;
    use uni_evidence::{Actor, Evidence, EvidenceState};

    fn ev(actor_id: &str, executor_id: &str, assurance: &str) -> Evidence {
        Evidence {
            id: "e".into(),
            claim_id: "c".into(),
            producer: "t".into(),
            command: "c".into(),
            exit_code: 0,
            output_hash: "h".into(),
            output_excerpt: "".into(),
            commit_sha: "s".into(),
            workspace_dirty: false,
            state: EvidenceState::Valid,
            created_at: chrono::Utc::now(),
            duration_ms: 1,
            artifact_hash: String::new(),
            fingerprint: "f".into(),
            registry_hash: String::new(),
            policy_hash: String::new(),
            contract_hash: String::new(),
            platform: String::new(),
            binding_hash: String::new(),
            expires_at: None,
            actor: Actor { id: actor_id.into(), source: "cli".into(), assurance: assurance.into() },
            executor: Actor { id: executor_id.into(), source: "local".into(), assurance: "self-declared".into() },
        }
    }

    #[test]
    fn empty_or_anonymous_is_never_independent() {
        assert_eq!(independence(&[]), Independence::Anonymous);
        assert_eq!(
            independence(&[ev("", "local:alice", "self-declared")]),
            Independence::Anonymous
        );
    }

    #[test]
    fn same_actor_is_not_independent() {
        assert_eq!(
            independence(&[ev("local:alice", "local:alice", "self-declared")]),
            Independence::SameActor
        );
    }

    #[test]
    fn distinct_actors_are_independent() {
        assert_eq!(
            independence(&[ev("ci:build-12", "local:alice", "self-declared")]),
            Independence::Independent
        );
    }

    #[test]
    fn assurance_splits_independence_from_identity() {
        let declared =
            vec![ev("ci:build-12", "local:alice", "self-declared")];
        assert_eq!(assurance_for(&Decision::Accepted, &declared), "A3-D");
        assert_eq!(identity_assurance(&declared), "SELF-DECLARED");
        let verified = vec![ev("spiffe://acme/v", "local:alice", "verified")];
        assert_eq!(assurance_for(&Decision::Accepted, &verified), "A3");
        assert_eq!(identity_assurance(&verified), "VERIFIED");
        let same = vec![ev("local:alice", "local:alice", "self-declared")];
        assert_eq!(assurance_for(&Decision::Accepted, &same), "A2");
        // Non-accepted decisions never upgrade, whatever the actors.
        assert_eq!(assurance_for(&Decision::Rejected, &verified), "A1");
        assert_eq!(assurance_for(&Decision::EvidenceRequired, &verified), "A0");
    }

    #[test]
    fn actor_parsing_never_confuses_prefix_with_proof() {
        let a = Actor::declared("ci:build-12");
        assert_eq!(a.source, "cli");
        assert_eq!(a.assurance, "self-declared");
        let s = Actor::declared("spiffe://acme/verifier/b12");
        assert_eq!(s.source, "spiffe");
        assert_eq!(s.assurance, "self-declared");
        assert!(Actor { id: "".into(), source: "cli".into(), assurance: "self-declared".into() }.is_anonymous());
        assert!(!Actor::local().is_anonymous());
    }
}
