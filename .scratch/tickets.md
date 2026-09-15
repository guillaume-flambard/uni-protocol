# UNI - tickets

Open work only. History lives in git.

State: **main**, 158 tests, rustfmt clean, clippy clean at `-D warnings`, CI
green with a `lint` job (fmt, clippy, constitution drift, every contract) and
the action smoke, live A3 against a real issuer, MIT OR Apache-2.0.

Workflow per `AGENTS.md`: implement (TDD at the seam: `assure_contract` then
`evaluate`) -> code review -> `uni verify` -> `uni explain`.

---

## Closed since v0.9.3, for the record

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

## Open

### 1. A non-conforming implementer
The study's one honest gap. t11 on five free implementers with the owner
invariant and the contract hidden: all five Accepted, all integer arithmetic,
all named the required test. The trap is deterministic (the scripted float
delivery is Rejected while its own tests are green), but no model on hand is
careless. Needs a weak or adversarial implementer.

### 2. A second real-repo task
`tasks/t11-fee-conservation/` is the shape to copy. Derived-index coherence is
the natural next one: a denormalized index a plausible fix forgets to update,
checked only by the owner's hidden invariant.

### 3. Migrate the study and dogfood registries to selector templates
They pin literal test names, so they lint with `pinned-test-selector` warnings.
ADR-002 records why this is deferred on both sides (study comparability,
example-as-documentation). Do it as each file is touched, not as a sweep.

### 4. Low value, do not start before the above
- A3 against a workforce issuer with a human token (the Google example). GitHub
  OIDC already proves the path live and unattended.
- Cloud and org: organizations, dashboards, cost per accepted outcome.
- Vault note `1-Projects/uni.md`; the `PROJECTS.md` line already exists.
- `CODE_OF_CONDUCT.md`. **decision:** add one or not.
- `A1` is unreferenced by the assurance scale (only the legacy decision-only
  fallback in `assurance_of_json` can produce it). Decide whether
  `assurance_of(decision)` stays at all.
- The Google JWKS snapshot ages; re-fetch it before any live use of that example.

---

