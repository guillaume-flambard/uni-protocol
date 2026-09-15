# ADR-002 - Pinned test names are a contract smell

Date: 2026-09-15. Decides ticket "Contracts still pin test names".

## Data

The 15-run study (`experiments/study-50/RESULTS-2026-09-14.md`): FAR 0%,
FRR 23% (3/15). All 3 false rejections share one cause: the registry command
pins a test name (`cargo test <name> -- --exact`) the worker was never told,
so correct work is refused. `uni brief` removed the class where applied
(t04: EVIDENCE_REQUIRED to ACCEPTED); `uni bind --selector` makes the
authorization readable. But a contract that demands a name still demands one.

## Decision

1. A literal test name inside a verifier `run` is a **contract smell**, not a
   pattern. The blessed pattern is a `{{selector}}` template: the worker names
   the test, a human authorizes it (`uni bind --selector`), the selector joins
   the binding hash and the evidence fingerprint.
2. `uni brief` is the **default handoff**: the work order resolves the
   registry to test-name indirection mechanically instead of hoping the agent
   follows `VERIFY → USING → config.toml`.
3. Enforcement is a `uni lint` **warning** (`pinned-test-selector`), never an
   error: the decision engine stays deterministic and untouched. Existing
   contracts keep verifying; new ones get nudged.
4. A remaining false rejection is triaged as a **contract defect first**
   (rename to a template or brief the worker), a model defect second.

## Consequences

- `pinned_test_selector()` in `uni-verify` detects the smell for cargo, node
  `--test-name-pattern`, and `unittest` dotted paths; suites, `--test`
  targets, `discover`, templates, and non-shell verifiers are excluded.
- Study and dogfood registries still pin names (they measure the old shape);
  they lint with warnings, which is the honest record, not a failure.
- Not decided: auto-emitting `brief.md` inside `uni run` (behaviour change to
  a shipped command; needs a human review, not a solo patch).
- Cloud/org work stays after this: no point scaling a handoff that still
  rejects correct work by default.
