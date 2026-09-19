# UNI - tickets

Open work only. History lives in git.

State: **v0.9.4**, 158 tests, rustfmt clean, clippy clean at `-D warnings`, CI
green with a `lint` job (fmt, clippy, constitution drift, every contract) and
the action smoke, live A3 against a real issuer, MIT OR Apache-2.0.

Workflow per `AGENTS.md`: implement (TDD at the seam: `assure_contract` then
`evaluate`) -> code review -> `uni verify` -> `uni explain`.

---

## Closed since v0.9.3, for the record

- **Vault.** `1-Projects/uni.md` existed but described v0.8.0 with 114 tests and
  called the identity adapters stubs. It now describes v0.9.4, the live A3, ADR-002,
  the hidden-owner-invariant design, and the self-report correction. Three vault
  side-effects: `Projects-MOC.md` described `[[uni]]` as an unrelated old project,
  which is fixed; the log-grading note in `3-Garden/tech/` gained Lie 4 (the grader
  that invents the verdict), which is where that lesson belongs; and a new note,
  `Evaluating a Checker with a Hidden Specification`, holds the task-design lesson.
  Vault audit: 0 schema issues, 0 broken wikilinks, 0 duplicates.

- **t12, the second real-repo task**, with two independent plausible failure
  modes, each caught by a named owner property: a batch path that forgets the
  derived index, and one that checks entries against the starting balances so it
  can spend the same money twice. Both built and verified as real deliveries.
- **A harness defect that invalidated a published column.** The worker's
  self-report was hardcoded to DONE for the real-agent arm, so `agent_self_report`
  and the "trust every DONE" baseline were asserted rather than observed, biased
  in UNI's favour. Now parsed, with `UNPARSED` as an honest outcome; the affected
  results files carry a correction note. First real-model non-acceptance in the
  study: a correct implementation shipping a self-contradicting test.
- **Two more harness defects**: no implementer timeout (a hung model blocked the
  harness for 40 minutes), and a bytes/str bug in the timeout path.

- **The schema is now asserted, not described.** `crates/uni-ir/tests/schema.rs`
  checks both directions between `schemas/uni.schema.json` and the emitted IR:
  no field without a schema entry, no required entry the compiler omits. No
  dependency; the checked subset is what this repository actually uses. Verified
  that it fails on real drift by removing `requirement` from the schema.
- **`uni run --brief`** writes the work order to `.uni/brief.md` and exports
  `UNI_BRIEF` with its absolute path to the executor. Opt-in, so the default
  behaviour and the command line are untouched. This is the answer to ADR-002's
  open question: the handoff has to be named, and `uni run` names it rather than
  guessing at anyone's prompt.
- **Truth-table rows have names.** Seven tests in `decision_rows` cover the rows
  the matrix reached without naming: OPTIONAL with no evidence and with
  disproving evidence, a critical claim that is stale or has no evidence at all
  (revalidation, not rejection), and the `expect_not` case where the command
  exits 0 while disproving a critical claim.
- **The action retries its asset download**, which is why the tag commits for
  v0.9.2 and v0.9.3 both went red on the smoke job and needed a manual re-run.

- **The suite is portable again.** The CLI tests resolved `CARGO_BIN_EXE_uni`
  and `CARGO_MANIFEST_DIR` at compile time, so a moved checkout or target
  directory left them pointing at the old location and failed for a reason
  unrelated to the code under test. They now locate the sibling binary and the
  workspace root from the running test process, through a shared `support`
  module, and the shared fixture includes are relative to their source file.
  No decision or verification behaviour changed.

- **A non-conforming implementer is rejected.** The scripted `plausible` arm runs
  t11 and t12 with the contract hidden, the worker's own tests green and a `DONE`
  self-report, and UNI rejects both at 2/4 claims: the hidden owner invariant and
  `tests-green`. Committed run, both patches and verification bundles:
  `experiments/study-50/RESULTS-2026-09-19-adversarial-arm.md`. That note states
  the limits plainly: the implementer is scripted, not observed, and n=1 per task.
  The harness gained `--evidence`, which persists the `uni verify` JSON per run.
  Recorded caveat: a scripted arm must set `UNI_AGENT_MODEL`, or its run
  overwrites the un-slugged real-agent diff.

## Open

### U3. Migrate selectors only when a registry changes

**Outcome:** each touched study or dogfood registry uses a `{{selector}}`
template and reviewed `uni bind --selector` authorization instead of a literal
test name.

**Scope:** no bulk migration. Preserve historical study comparability and
example readability, as required by ADR-002.

**Done when:** a touched registry has no new `pinned-test-selector` warning and
its example or study run still verifies.

## Parked until U2 produces a real need

- A3 with a human Google identity token. GitHub OIDC already proves unattended
  A3.
- Cloud organisations, dashboards and cost per accepted outcome.
- Decide whether to add `CODE_OF_CONDUCT.md`.
- Decide whether the legacy A1 fallback in `assurance_of_json` should exist.
- Refresh the Google JWKS snapshot immediately before a live Google example.

## Closed

- **t12 / derived-index coherence.** This was previously duplicated as “a
  second real-repo task”. It is already delivered and recorded above: its two
  hidden owner invariants catch both a forgotten index update and double spend
  across a batch. No new task is required.

---
