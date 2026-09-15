# Plan - SPEC-001 UNI Software v0.1

Language: Rust 1.98 workspace (`crates/*`), single CLI binary `uni`.

## Architecture (ADR-001)
- High seam: `assure_contract(ir, dot_uni, workspace) -> Result<Vec<Evidence>>` in `uni-verify`; `evaluate(ir, evidences) -> DecisionResult` in `uni-decision`, with `apply_policy` applied after the truth table. Both take the compiled IR, not a contract path.
- Storage: filesystem `.uni/{config.toml,contracts,evidence,decisions,artifacts}`, content-addressed SHA-256.
- Wire format: JSON IR + JSON Schema (`schemas/uni.schema.json`). No Protobuf until v0.5.
- Trusted registry: `.uni/config.toml [verifiers]` (run + optional expect/timeout). Contracts untrusted.
- Decision: deterministic truth table, no LLM in the loop.

## Verification stack
- shell verifier (exit code + output expectation matcher).
- Per-test verifier refs (e.g. `cargo test <name> -- --exact` + `expect "test result: ok. 1 passed"`).
- Test runners: npm/pnpm/bun/cargo/pytest via generic command adapter.

## Integrations (not replacements)
- Spec Kit importer: `uni import-speckit` → candidate DSL → human approve.
- CI: GitHub workflow runs verifiers; PR check "UNI Assurance".
- Deferred: OPA, OTel, MCP, A2A, Temporal (adapters only, never core).

## Verification stages
uni init → uni compile → uni verify (incremental, git-bound evidence) → uni explain.

## Risks
- Same verifier validating several claims → per-test registry (fixed v0.4).
- Evidence staleness across commits → commit-bound evidence (fixed v0.2).
