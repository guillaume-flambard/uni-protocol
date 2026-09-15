# UNI - context

Outcome Assurance Protocol: Intent → Claims → Evidence → Policy → Decision.
Rust workspace (`crates/*`), CLI binary `uni`. High seam: `uni_verify::assure_contract(ir, dot_uni, workspace) -> Result<Vec<Evidence>>` then `uni_decision::evaluate(ir, evidences) -> DecisionResult`, with `apply_policy` after the truth table. Parser, store and verifiers are internals.
Glossary: intent, claim, invariant, evidence (VALID/INVALID/STALE), verifier (trusted registry), decision (ACCEPTED/REJECTED/EVIDENCE_REQUIRED/ESCALATED), assurance levels A0-A4 (the scale describes the evidence, not the verdict).
