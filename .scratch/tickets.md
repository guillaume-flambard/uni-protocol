# UNI - tickets

Open work only. History lives in git.

`v0.9.3` is released (five assets, marked latest) and verified on the published
binary, not just in CI: the downloaded macOS archive refuses `CLAIM x BANANA`
and a duplicate claim id, and reaches A3 against GitHub's real OIDC issuer. The
action default moved to it, so the closed-vocabulary fixes are no longer
main-only.

State: **v0.9.3**, 148 tests, rustfmt clean, clippy clean at
`-D warnings`, CI green with a `lint` job (fmt, clippy, constitution drift,
every contract) and the action smoke, live A3 against a real issuer,
MIT OR Apache-2.0.

Workflow per `AGENTS.md`: implement (TDD at the seam: `assure_contract` then
`evaluate`) -> code review -> `uni verify` -> `uni explain`.

---

## Closed by the audit pass, for the record

- `CLAIM x BANANA` compiled and `CLAIM x CRITICAL` was silently non-critical.
  Both are now hard parse errors naming the line and the right keyword.
- Duplicate claim ids were accepted and produced a false accept. Now refused
  with both lines named.
- `forbid-N` was numbered on every claim, so inserting an unrelated `CLAIM`
  renumbered it and silently invalidated its binding. Numbered on FORBID alone.
- A `REJECT WHEN` placed right after `GOAL` was swallowed as prose. `GOAL` now
  ends on the whole vocabulary.
- `USING shellcheck` parsed as the inline verifier `shell` running `check`.
  `shell` is now a whole word.
- Assurance was gated on the decision (A1/A0 for anything not Accepted). The
  scale now describes the evidence, as the spec and `docs/decisions.md` always
  said; `A0` means "no evidence was gathered".
- `uni init` now creates `.uni/policies`, which the policy layer reads.
- The Spec Kit constitution copy is generated (`scripts/sync-constitution.sh`)
  with a CI drift check, instead of a second hand-maintained source.
- Twelve documentation claims that the binary contradicted are fixed, and the
  report's stale counts are regenerated.

---

## Open

### 1. Re-release when the next batch lands
`main` is one commit ahead of `v0.9.3` (the action's download retry). The action
file is read by ref, so a user pinning `@v0.9.3` gets the version without the
retry; the next tag carries it. No action needed until then, noted so it is not
forgotten.

### 2. The IR is not validated against its schema
`uni compile` emits the canonical IR but nothing checks it against
`schemas/uni.schema.json`; the spec now says so honestly. Validating at compile
time is cheap now that the shapes match, and it turns the schema from a
description into an assertion. The one real cost is a JSON Schema dependency.
**decision:** add the dependency and validate, or hand-roll the structural
checks that matter (required keys, types, enum membership) and keep the tree
dependency-free.

### 3. `uni run` and the work order
The brief arm showed the handoff must *invoke* the brief, not merely emit it.
**decision:** should `uni run` generate `brief.md` and name it in the executor's
prompt? A behaviour change on a shipped command.

### 4. A non-conforming implementer
The study's one honest gap. t11 on five free implementers with the owner
invariant and the contract hidden: all five Accepted, all integer arithmetic,
all named the required test. The trap is deterministic (the scripted float
delivery is Rejected while its own tests are green), but no model on hand is
careless. Needs a weak or adversarial implementer.

### 5. A second real-repo task
`tasks/t11-fee-conservation/` is the shape to copy. Derived-index coherence is
the natural next one: a denormalized index a plausible fix forgets to update,
checked only by the owner's hidden invariant.

### 6. Test debt
`uni-ir` has 5 tests for 176 lines. The truth table in `uni-decision` deserves a
named test per row. Prefer growing the low crates over the CLI integration
surface.

### 7. Migrate the study and dogfood registries to selector templates
They pin literal test names, so they lint with `pinned-test-selector` warnings.
ADR-002 records why this is deferred on both sides (study comparability,
example-as-documentation). Do it as each file is touched, not as a sweep.

### 8. Low value, do not start before the above
- A3 against a workforce issuer with a human token (the Google example). GitHub
  OIDC already proves the path live and unattended.
- Cloud and org: organizations, dashboards, cost per accepted outcome.
- Vault note `1-Projects/uni.md`; the `PROJECTS.md` line already exists.
- `CODE_OF_CONDUCT.md`. **decision:** add one or not.
- `A1` is now unreferenced by the assurance scale (only the legacy
  decision-only fallback in `assurance_of_json` can produce it). Decide whether
  `assurance_of(decision)` should stay at all, or whether the persisted-decision
  reader can drop it.

---

## Decisions still open from earlier passes

Answered and implemented: `CLAIM ... CRITICAL` is refused, `.uni/policies` is
created by `init`, and assurance is a property of the evidence. Still open:
the schema dependency (2), the `uni run` brief (3), and the code of conduct (8).
