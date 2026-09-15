# UNI - tickets

Open work only. The history lives in git and in the release notes.

State: **v0.9.2**, 142 tests, rustfmt clean, clippy clean at `-D warnings`, CI
green on ubuntu/macOS/Windows plus a `lint` job and the action smoke, live A3
against a real issuer, MIT OR Apache-2.0.

Every ticket below came out of a code, test, doc and collaboration audit on
2026-09-15. Tickets marked **decision** need Guillaume's call before anyone
implements them; the rest are ready to pick up. Workflow per `AGENTS.md`:
implement (TDD at the seam) -> code review -> `uni verify` -> `uni explain`.

---

## P0 - the closed vocabulary has two holes

### 1. A modifier token on CLAIM is never validated
`CLAIM x CRITICAL` parses, discards `CRITICAL`, and compiles to
`critical: false`. A failing verifier then yields `EVIDENCE_REQUIRED` where the
author wrote CRITICAL expecting `REJECTED`. Worse, `CLAIM x BANANA` compiles
clean, exit 0.

Verified:
```
CLAIM x CRITICAL + failing verifier  -> Decision: EvidenceRequired
CLAIM x BANANA                       -> compiles, critical: false
```
That contradicts constitution rule 5 and `docs/specification.md` ("unknown
directives are hard parse errors"). The parser validates the modifier position
on `INVARIANT` but not on `CLAIM`.

Where: `crates/uni-parser/src/lib.rs`, the `CLAIM` branch.
Done when: `CLAIM x BANANA` fails with a located error, and there is a test for
it in `crates/uni-parser/tests/fixtures_tests.rs`.
**decision:** does `CLAIM x CRITICAL` become a hard error ("use INVARIANT") or
does it set `critical: true`? Error is the smaller change and keeps two words
for one idea; honouring it makes `CLAIM`/`INVARIANT` interchangeable.

### 2. Duplicate claim ids are accepted
`docs/specification.md` says compile validates that claim ids are unique.
Nothing does. Two `CLAIM a` blocks compile, `lint` reports **clean**, `verify`
returns **Accepted** and lists `a PASS` twice from a single `VERIFY` - so a
second obligation with no verifier of its own is silently satisfied by the
first one's evidence. That is a false accept, the one outcome the whole project
exists to prevent.

Where: `crates/uni-ir` (`compile`) is the natural home; `uni lint` could also
carry it as an error.
Done when: a duplicate id fails with a located error, plus a test, plus a
regression that the two-claims-one-VERIFY case cannot be Accepted.

---

## P1 - docs that promise what the binary does not do

Each of these is small and checkable. Group them into one or two commits.

### 3. `CLAIM`/`INVARIANT`/`FORBID` need their own VERIFY, and the sample does not say so
`docs/language.md` presents a contract as complete; running `uni lint` on it
gives `ERROR missing-verify` twice and exit 1. A `FORBID` compiles to a claim
`forbid-N` that also needs its own `VERIFY`. `docs/claims.md` says it in prose
and the sample contradicts it.
Done when: the sample in `docs/language.md` lints clean, and the `FORBID` row in
`docs/claims.md` shows the `VERIFY forbid-N`.

### 4. The "high seam" function names do not exist
`assure(contract, workspace) -> AssuranceResult` and
`evaluate(intent) -> Decision` are cited as the public API in six places. The
real entry points are
`assure_contract(ir, dot_uni, workspace) -> Result<Vec<Evidence>>`
(`crates/uni-verify/src/lib.rs:638`), `evaluate(ir, evidences)` and
`evaluate_intent` (`crates/uni-decision/src/lib.rs:318,400`), with policy applied
separately by `apply_policy`.

`AssuranceResult` appears nowhere. Files that carry the wrong name: `CONTEXT.md`,
`AGENTS.md`, `specs/001-uni-software-v01/plan.md`, `docs/adr/ADR-001`, `docs/REPORT-2026-09-14.md`,
`docs/DEEP-ANALYSIS-2026-09-14.md`, `docs/writing-verifiers.md`.
Done when: either the docs name the real functions, or the code grows the
promised seam. Prefer naming reality: a rename would churn every caller for
philosophy.

### 5. Two extension points that do not exist
`docs/why-uni.md:41` claims execution engines are "replaceable behind
`EXECUTION_PROVIDER`", and no such symbol exists. `docs/why-uni.md:43`,
`docs/REPORT:56` and `CONTRIBUTING.md` list Cedar as a wired policy engine;
there is no Cedar adapter, only `OpaPolicy`.
What is real: the `PolicyProvider` trait
(`crates/uni-decision/src/lib.rs:183`) with `TomlPolicy` and `OpaPolicy`.
Done when: the row says what exists, and the "replaceable behind X" claim is
removed or implemented.

### 6. `uni init` does not create `.uni/policies`
`README.md` lists `policies` among the created directories; `init` creates
`artifacts` instead. The policy layer reads `.uni/policies/*.toml`
(`crates/uni-decision/src/lib.rs:199`), so a fresh workspace has nowhere to put
a policy without a `mkdir`.
**decision:** make `init` create `.uni/policies` (small, and the README becomes
true), or fix the README and document the `mkdir`. Prefer creating it.

### 7. `docs/decisions.md` gets two mechanics wrong
`reject_on_invalid = false` downgrades a **critical** rejection to ESCALATED,
not a non-critical one; a non-critical failure is `EVIDENCE_REQUIRED` from the
start and never becomes REJECTED. And the CLI does not distinguish the
non-accepted decisions on exit code: `Rejected`, `EvidenceRequired` and
`Escalated` all exit 1, and only the stderr label separates them.
Done when: both sentences match the binary, and the exit-code table points at
the stderr label.

### 8. Assurance is gated on the decision, not only on the evidence graph
`docs/decisions.md`, `docs/REPORT:191` and `specs/003-assurance-model` FR-204
say the scale is derived from the evidence graph. `assurance_for`
(`crates/uni-decision/src/lib.rs:88`) returns `A0`/`A1` for any non-`Accepted`
decision before it looks at the graph: a rejected run with an independent actor
reports `assurance A1`, `independent_actor: true`.
**decision:** is assurance a property of the evidence (A3 evidence for a
rejected claim is still A3 evidence) or of the outcome? If evidence, move the
gate; if outcome, fix FR-204 and the three docs. Either answer is defensible,
and they are not the same product.

### 9. `specs/001` promises three things that do not exist
- "`uni compile` emits canonical JSON IR (JSON Schema validated)": nothing in
  `crates/` references `schemas/uni.schema.json`. The shape matches, the
  validation does not happen.
- "Commit/dirty change -> STALE -> claim UNVERIFIED -> NEEDS_REVALIDATION": the
  only states are `Valid|Invalid|Stale` and `Accepted|Rejected|EvidenceRequired|Escalated`.
- "Action `uni-protocol/verify@v1` running the embedded binary": the shipped
  action is `adapters/github/action.yml`, it downloads the release archive and
  builds nothing, and `docs/github-integration.md` already says so.
Done when: the spec says what exists. Separately worth a ticket of its own:
actually validating the IR against the schema in `compile`, which is cheap now
that the schema matches.

### 10. The report carries stale and invented content
`docs/REPORT-2026-09-14.md` is the headline document, so its numbers are read.
- test count 134 (line 19) and "Total 130" (line 307): actual 142.
- per-crate test counts (lines 301-306) and LOC (line 18, says 6 497): actual
  7 148 source lines.
- "10 contrats" in `examples/` and "16 documents et un ADR": 12 contracts,
  2 ADRs.
- `docs/REPORT:80-86` describes seven decision states plus "internal states
  DRAFT READY EXECUTING VERIFYING". The enum has four variants and none of
  those four exist.
- line 148 says `init` takes no `--json`; it does.
Done when: numbers regenerated and the state list matches the enum.

### 11. Small doc defects, one line each
- `docs/flagship-check.md` quotes "2 errors and 2 warnings"; the drift demo
  yields one `::error` and one `::warning`. Structural claims are correct.
- `CONTEXT.md:5` omits `ESCALATED` from the decision list.
- `README.md:44-61` labels a sample contract as `examples/booking/booking.uni`;
  the real file has three claims and three different verifiers.
- `docs/evidence.md` lists `BindingAuthorized` among the events "every verify
  appends"; that one comes from `uni bind`.

---

## P2 - structure and hygiene

### 12. Two copies of the constitution, already drifted
`.specify/memory/constitution.md` is tracked and carries a header saying "update
both together". They no longer match in structure, and the project's own rule 7
is "one source of truth, no spec duplication". This is the maintenance hazard
the repo warns others about.
Done when: one source (the root `constitution.md`), and the Spec Kit copy is
either generated with a check in CI or replaced by a pointer.

### 13. Two generated artifacts are tracked in the repo
`.uni/contracts/candidate-sk-fixtures.brief.md` and (before the audit removed it)
`uni/intents/tests-pass.uni` are outputs of a command, not sources. The project's
own principle elsewhere is that a mirror is never a source. Decide whether a
package or repo may ship generated `.uni/` artifacts at all; if yes, say which
and why in `.gitignore` comments, if no, untrack them and regenerate.
Done when: `.gitignore` states the rule and the tree matches it.

### 14. Finish the test-debt direction
The low crates gained unit tests; `uni-ir` still has 2 tests for 131 lines, and
`uni-decision` (17 tests, 1021 lines) is where a wrong truth-table row is most
expensive. Prefer growing those rather than the CLI integration surface, and
prefer a test per decision-table row.
Done when: the truth table has a named test per row and `uni-ir` covers the
compile error paths.

---

## P3 - open work carried over (unchanged priority)

### 15. A non-conforming implementer
The study's one honest gap. t11 was run on five free implementers with the owner
invariant and the contract hidden: all five Accepted, all integer arithmetic,
all named the required test. The trap is deterministic (the scripted float
delivery is Rejected while its own tests are green), but no model on hand is
careless. Needs a weak or adversarial implementer, or a task whose plausible
path is wrong by default.

### 16. A second real-repo task
`tasks/t11-fee-conservation/` is the shape to copy. Derived-index coherence is
the natural next one: a denormalized index that a plausible fix forgets to
update, checked only by the owner's hidden invariant.

### 17. `uni run` and the work order
The brief arm showed the handoff must *invoke* the brief, not merely emit it.
**decision:** should `uni run` generate `brief.md` and name it in the executor's
prompt? A behaviour change on a shipped command, so it needs review.

### 18. Migrate the study and dogfood registries to selector templates
They pin literal test names, so they lint with `pinned-test-selector` warnings.
ADR-002 records why this is deferred on both sides (study comparability,
example-as-documentation). Do it as each file is touched, not as a sweep.

### 19. Low value, do not start before the above
- A3 against a workforce issuer with a human token (the Google example). GitHub
  OIDC already proves the path live and unattended.
- Cloud and org: organizations, dashboards, cost per accepted outcome.
- Vault note `1-Projects/uni.md`; the `PROJECTS.md` line already exists.
- `CODE_OF_CONDUCT.md`. **decision:** add one or not.

---

## Notes on what was deliberately left alone

- `.specify/memory/constitution.md` and the two old results files use em dashes
  as section separators. They are records, not published prose, and the tickets
  above decide their fate rather than a blanket rewrite.
- `examples/identity-google` is not in CI on purpose: it needs a human-minted
  token. `examples/negative` fails on purpose. `examples/playwright` needs npx.
- The three `pinned-test-selector` warnings on the repo's own contracts are the
  intended shape of the warning, not a defect.
