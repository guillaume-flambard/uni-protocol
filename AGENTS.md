# UNI - agent rules

- Constitution: `constitution.md`. Spec source: `specs/001-uni-software-v01/spec.md` (canonical, no duplicate spec).
- Tickets: `.scratch/tickets.md`. Workflow: implement (TDD at the seam: `assure_contract` then `evaluate`) → code-review → `uni verify` → `uni explain` → merge.
- Security: contracts untrusted; trusted registry is `.uni/config.toml` only (`[verifiers]` for what may run, `[identities]` for whose token may be believed). Key material is pinned by path, never fetched.
- v0.1: filesystem `.uni/`, JSON Schema canonical. No Postgres/Protobuf/MCP/A2A in core.
- Registry selectors: a verifier entry that names one test pins that test to the contract. When you touch a `.uni/config.toml` entry that pins a test name, migrate it to a `{{selector}}` template and authorize the name with `uni bind` (never a sweep); the study tasks migrate only when their family is rewritten. See `docs/adr/ADR-002-pinned-test-names-are-a-contract-smell.md`.

## Agent skills

### Issue tracker

Local markdown under `.scratch/` (solo phase; GitHub later). See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.
