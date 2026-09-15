# UNI Specification (dev, v0.9)

Status: Developer Preview (v0.9). The wire format is JSON + JSON Schema;
Protobuf is deferred until the data model stabilizes. Governance stays
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
REQUIRE    <expr>            (attached to the preceding VERIFY; optional)
ACCEPT WHEN  required_claims == VERIFIED [AND critical_failures == 0]

REQUIRE is shipped (v0.2 for the plain form, v0.5 with `--selector` templates):
it authorizes the resolution of the claim's verification. Still reserved and
rejected as hard parse errors: REJECT WHEN, ESCALATE WHEN. A spec must never
promise inert semantics (constitution rule 5).
```

Multi-line form: `VERIFY <id>` followed by an indented `USING ...`. `#` comments.
Unknown directives are hard parse errors with `line N:` diagnostics, and every
diagnostic names its line.

`GOAL` is free-form prose, and it ends at the first line that begins with a
directive from the closed vocabulary. The terminator is the whole vocabulary,
not a hand-picked subset: a reserved `REJECT WHEN` placed right after `GOAL`
must still be the hard error the vocabulary promises, not a line of goal text.
`shell` is likewise a whole word, so a registry key that merely starts with
those letters (`shellcheck`) stays a registry reference.

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

## Selector templates

A registry entry may contain `{{selector}}`; the concrete name comes from an
authorized `VerifierBinding`, never from the contract or the worker alone.

## Behavior contract

1. `compile` validates (claim ids unique, VERIFY references known claims) and
   refuses a duplicate claim id with both lines named. Its output is checked
   against `schemas/uni.schema.json` by `crates/uni-ir/tests/schema.rs`, in both
   directions, so the schema cannot silently stop describing the compiler.
2. `lint` never executes; errors = claim without VERIFY (plus malformed contract);
   warnings = unknown registry key, duplicate verification, a `{{selector}}`
   template with no authorized selector, a `REQUIRE` with no matching binding,
   and a verifier whose command pins a literal test name (ADR-002). Warnings
   never change a decision: the truth table is untouched by them.
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
