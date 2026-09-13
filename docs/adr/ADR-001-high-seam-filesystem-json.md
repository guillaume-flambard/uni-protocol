# ADR-001 - High seam, filesystem, JSON-only v0.1

- Seam: `assure(contract, workspace) -> AssuranceResult` / `evaluate(intent) -> Decision`. Parser/store/verifiers are internals.
- Storage: `.uni/` filesystem content-addressed. No Postgres (deferred to team/cloud phase).
- Wire format: canonical JSON + JSON Schema. No Protobuf until the model stabilizes (v0.5/v1).
- SpecKit importer is candidate-only: `constitution/spec/plan.md → uni import speckit → REVIEW → approved contract`.
- 50-issue study deferred: v0.1 = Can UNI work? v0.2 = catch failures? v0.3 = predict human acceptance? v0.4 = compare agents?
