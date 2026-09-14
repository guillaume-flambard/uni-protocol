# UNI Specification v0.1 (dev)

Status: Developer Preview. The wire format is JSON + JSON Schema; Protobuf is
deferred until the data model stabilizes (v0.5+). Governance stays
maintainer-driven until UEPs make sense.

## Grammar (closed vocabulary)

```
VERSION  <semver>                  (v0.1 = "0.1")
DOMAIN   <id>
INTENT   <id>
GOAL     <free text lines>
CLAIM      <id> [REQUIRED|OPTIONAL]
INVARIANT  <id> [CRITICAL]
ENSURE     <free text>
FORBID     <expr> (same line) or bare FORBID + indented expression
VERIFY     <claim-id> USING <registry-key> | shell "<cmd>"
ACCEPT WHEN  required_claims == VERIFIED [AND critical_failures == 0]

Reserved for v0.2 (hard parse errors until then): REQUIRE, REJECT WHEN,
ESCALATE WHEN. A spec must never promise inert semantics (constitution rule 5).
```

Multi-line form: `VERIFY <id>` followed by an indented `USING ...`. `#` comments.
Unknown directives are hard parse errors with `line N:` diagnostics.

## Canonical IR

`schemas/uni.schema.json` (draft 2020-12) is the normative artifact, generated
shape from `crates/uni-ir`:

```json
{
  "uni_version": "0.1",
  "intent": { "id": "...", "domain": "software", "goal": "..." },
  "claims": [{ "id", "kind": "claim|invariant", "required", "critical", "ensure" }],
  "verification": [{ "claim_id", "verifier_ref", "inline_shell" }],
  "acceptance": { "require_verified": true },
}
```

Evidence and DecisionResult shapes: see `docs/evidence.md` and the Rust types
(`uni-evidence`, `uni-decision`); their serde output is the format.

## Behavior contract

1. `compile` validates (claim ids unique, VERIFY references known claims).
2. `lint` never executes; errors = claim without VERIFY; warnings = unknown
   registry key, duplicate verification.
3. `verify` runs registry-resolved commands, writes evidence, applies the truth
   table then the policy, persists `.uni/decisions/last.json`, appends
   `.uni/events.jsonl`.
4. Evidence bindings: `commit_sha + workspace_dirty` always; `artifact_hash`
   when the verifier declares `files`.
5. `report` output is stable: only intent, decision, reason, per-claim
   `{claim_id, state}`.
6. Determinism: identical (contract, evidence, policies) yields identical decision.

## Compatibility rules

- v0.1 vocabulary is frozen: additions are spec changes (UEP later), unknown
  directives must stay hard errors.
- Registry fields (`expect`, `expect_not`, `files`, `timeout`) are additive;
  absent means the v0.1 default behavior.
- Repository split `uni-protocol/spec` vs `uni-protocol/uni` (runtime) stays on
  the v1 checklist; `schemas/` is the local source of truth until then.
