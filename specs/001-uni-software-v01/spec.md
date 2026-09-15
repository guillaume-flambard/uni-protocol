# SPEC-001 - UNI Software v0.1 (canonical)

Single source of truth. Matt `to-tickets` slices this; no second spec.

## Capability A - Contract (FR-001..004)
- `uni init` creates `.uni/{config.toml,contracts/,evidence/,decisions/,artifacts/}` + `uni/intents/`.
- Parser `.uni` with precise line diagnostics; closed vocabulary.
- `uni compile` emits canonical JSON IR. `schemas/uni.schema.json` is the
  normative definition of that shape, and `crates/uni-ir/tests/schema.rs`
  asserts both directions between them: every emitted field is declared, and
  every declared/required field is emitted.

## Capability B - Evidence (FR-005..013)
- `uni verify` runs trusted-registry verifiers: shell, generic test runners (npm/pnpm/bun/cargo/pytest), Playwright.
- Evidence binds commit SHA + dirty state + SHA-256; states VALID/INVALID/STALE.
- Commit or watched-file change → evidence STALE → the claim is no longer
  covered → `EVIDENCE_REQUIRED` (or `REJECTED` if it is critical). The evidence
  states are `Valid | Invalid | Stale`; there is no `UNVERIFIED` or
  `NEEDS_REVALIDATION` state.

## Capability C - Decision (FR-014..015)
- Deterministic truth table: critical Invalid → REJECTED; required missing/stale → EVIDENCE_REQUIRED; else ACCEPTED.
- `uni explain` human + `--json` machine output.

## Capability D - Developer UX (FR-016 + errors)
- `--json` on all commands; error messages propose the next `uni verify <claim>` command.
- No cloud dependency; `git clone + uni verify` in 60s.

## Capability E - GitHub (FR-017..018)
- Action `adapters/github/action.yml`, downloaded from a release (it builds
  nothing), reporting the `UNI Assurance` view. See `docs/github-integration.md`.

## Out of scope v0.1
Orchestration, registry, MCP gateway, A2A server, firewall, IdP, crypto reputation, blockchain, control plane, Postgres, Protobuf.

## Security
Contracts untrusted; registry trusted; no secrets in evidence.
