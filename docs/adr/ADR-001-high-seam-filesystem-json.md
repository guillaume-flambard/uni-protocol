# ADR-001 - High seam, filesystem, JSON-only v0.1

- Seam (**as shipped**): `uni_verify::assure_contract(ir, dot_uni, workspace) -> Result<Vec<Evidence>>` and `uni_decision::evaluate(ir, evidences) -> DecisionResult`, with policy applied by `apply_policy` after the truth table. Parser/store/verifiers are internals.
  - Amendment 2026-09-15: this ADR originally named an aspirational `assure(contract, workspace) -> AssuranceResult` / `evaluate(intent) -> Decision` pair that was never implemented. The decision (there is a high seam, and internals stay internal) stands; the names above are what exists, and the seam sits at the compiled IR rather than at the contract path.
- Storage: `.uni/` filesystem content-addressed. No Postgres (deferred to team/cloud phase).
- Wire format: canonical JSON + JSON Schema. No Protobuf until the model stabilizes (v0.5/v1).
- SpecKit importer is candidate-only: `constitution/spec/plan.md → uni import speckit → REVIEW → approved contract`.
- 50-issue study deferred: v0.1 = Can UNI work? v0.2 = catch failures? v0.3 = predict human acceptance? v0.4 = compare agents?
