# Plan - 003 Assurance Model

- B1: Evidence fields (registry/policy/contract/platform/binding hashes) + EvidenceContext + CacheOutcome Hit/Stale/Miss + stale-unreprovable escalation path in cmd_verify.
- B2: registry snapshot + hash baseline in .uni, registry_diff() pure function, RegistryChanged event, trust_boundary_changed JSON flag, .gitignore runtime files, dirty-bit excludes .uni/.
- B3: Actor{id,source,assurance} + local()/declared() constructors, actor in fingerprint, executor refresh on hit, independence()/identity_assurance()/assurance_for() in uni-decision, --actor/--attest flags, persisted assurance fields in last.json, report/explain display.
- B4: REQUIRE attaches to VERIFY (gap-checked), VerificationIr.requirement + schema, binding store + uni bind/bindings, verify authorization gate, binding_hash invalidation, lint warning.
- B5: constitution v0.2 (rules X-XII) + mirror, docs (evidence/decisions/verification/language), this spec, gate below.

Gate: full suite green x3, 0 warnings, dogfood 5 stacks, report stable, no LLM in verify path (grep), matrix acteur verte, each invalidation dimension covered by at least a unit test (platform e2e impossible single-machine, documented).
