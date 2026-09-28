# Drift after an accepted revision, on both hidden-invariant tasks

Date: 2026-09-28. Arm: `careless`, a scripted delivery. $0 spent.

## What this asks

`RESULTS-2026-09-19-adversarial-arm.md` closed with the case it had not run:

> **One revision, one patch.** The outcome is deterministic and shows the invariant
> separates floor from round, and index from balances. It does not exercise drift
> after verification; that case belongs to the `careless` arm.

The `careless` arm existed and ran once, on t09, a one-function task with no
hidden invariant. t11 and t12 are the two tasks where a drift is worth
measuring: the owner's invariant is withheld until delivery, so a delivery can
be wrong in a way the worker cannot see or test for. Both now carry a
`regression.patch` and both have a committed run.

## The arm

`agent_careless` applies `fix.patch`, runs the worker's own `cargo test` (green),
verifies with UNI, then edits the code again with no re-run and no re-verify,
and reports DONE. The contract and the registry are visible to the arm, so the
agent can verify: that is the premise of the case, and without it there is no
earlier revision to go stale.

The two late edits are single hunk changes on top of the correct fix, each with
the reasoning that makes it look like a tidy-up rather than a mistake:

- **t11** drops `ledger.debit(&hold.to, fee)` from `settle_with_fee`, on the
  reasoning that the hold already moved the amount so the fee is only a slice
  of what the destination was credited. The fee arithmetic is untouched: still
  `amount * bps / 10_000` in integers, still returned to the caller. The fee is
  minted instead of taken.
- **t12** replaces the `self.apply(entry)` call in the batch loop with direct
  writes to `self.accounts`, on the reasoning that the projection already holds
  the outcome so re-checking each entry is redundant. Balances are exactly
  right, because the projection was right. The per-owner index stops agreeing
  with the accounts it is derived from.

Neither patch touches `crates/`, and neither touches `tests/invariants.rs`,
which is `expect_sha256` bound. Both hashes still match the registry:
`3a5b846f...` for t11, `4a57b1e0...` for t12.

## Reproduce

```bash
cd experiments/study-50
UNI_AGENT_MODEL=careless python3 run.py --agent careless \
  --only t11-fee-conservation --out results-careless.csv --evidence evidence
UNI_AGENT_MODEL=careless python3 run.py --agent careless \
  --only t12-derived-index --append --out results-careless.csv --evidence evidence
```

`UNI_AGENT_MODEL` is mandatory for a scripted arm: without it the harness writes
`diffs/<task>.patch`, the name the real-agent run used, and overwrites a
committed real delivery. With it, the diff and the bundle take the
`__careless` suffix.

There is no `--hide-contract` here, unlike the `plausible` arm. Measured, not
assumed: with `--hide-contract` the arm's own `uni verify` fails, because the
contract is what it verifies against, so there is no earlier evidence and the
journal records zero `EvidenceStale`. The run still ends Rejected, but it is a
fresh cache-miss run, not the revision binding this note is about. The guard
below now refuses that combination outright.

## Result row

`results-careless.csv`:

| issue | model | agent_done | self-report | UNI | claims | baseline | contract visible | diff lines |
|---|---|---|---|---|---|---|---|---|
| t11-fee-conservation | careless | 1 | DONE | **Rejected** | 2/4 | Rejected | 1 | 15 |
| t12-derived-index | careless | 1 | DONE | **Rejected** | 2/4 | Rejected | 1 | 19 |

Same shape as the `plausible` rows in `results-adversarial.csv`: the behavioural
claim is Valid on both, the hidden owner invariant (`conservation` on t11,
`index-coherent` on t12) is Invalid and CRITICAL on both, `tests-green` is
Invalid and CRITICAL on both because the owner's tests are part of the run, and
`invariants-frozen` is Valid on both, so the owner's own tests are present and
untouched.

## What was caught

```
t11                                 t12
fee-is-exact        Valid          apply-atomic      Valid
conservation        Invalid  CRIT   index-coherent    Invalid  CRIT
invariants-frozen   Valid          invariants-frozen Valid
tests-green         Invalid  CRIT   tests-green       Invalid  CRIT

Decision: Rejected, 2 critical failure(s), both tasks
```

Read from `verify.claims` in the bundles, not from this note's memory.

The point is which revision each half of that table was produced at. The
journal for both runs:

```
journal: 3x IntentVerified, 4x EvidenceStale, 12x EvidenceRun, 3x DecisionIssued
tail:    EvidenceStale x4 -> EvidenceRun x4 -> DecisionIssued
```

and the stale block records why, for the critical claims only:

```
t11  conservation  commit_changed | artifact moved from commit c1300f93 to 62df57cc
t11  tests-green   commit_changed | artifact moved from commit c1300f93 to 62df57cc
t12  index-coherent commit_changed | artifact moved from commit 332ed9f8 to 0f214012
t12  tests-green    commit_changed | artifact moved from commit 332ed9f8 to 0f214012
```

So the rejection came from the revision binding and not from re-reading the
worker: the earlier proof was marked stale because the artifact moved, the
verifiers were re-run against what was delivered, and only then was the decision
issued. The shas differ on every run, since the harness builds a fresh temporary
repository each time; the invariant is the relationship, not the values.

What each task shows, on its own:

- **t11** separates exactness from conservation. The late edit keeps the fee
  exact, and `fee_is_the_exact_integer_floor` still returns and records the
  right fee for every case. What breaks is that the fee is minted rather than
  taken: the owner's conservation test reports `1000011` against `1000010`,
  exactly the eleven cents the six settlements collected.
- **t12** separates balances from the index derived from them. All three
  atomicity owner invariants still pass, because the projection genuinely
  decides correctly what to refuse. One test fails, and it is the coherence one:
  `after batch 0: owner 'alice' index is out of step with the accounts`,
  `600` against `590`.

The worker's own suite is green and its own claim is true at the delivered
revision in both cases. That is the difference from the cohere run recorded in
`RESULTS-2026-09-15-t12-derived-index.md:35-53`, where the implementation was
right, the worker's own test was wrong, and the rejection came from
`tests-green` against a self-contradicting delivery. Here `fee-is-exact` and
`apply-atomic` are Valid, and the only thing failing is a check the worker
never saw.

Evidence bundles: `evidence/t11-fee-conservation__careless.json` and
`evidence/t12-derived-index__careless.json`. Each holds the whole `uni verify`
JSON (claims with state, criticality and detail, the reason, the
trust-boundary flag) plus a `trace` with the journal event names in order, the
delivered commit and the stale block. Diffs of the delivered revision:
`diffs/t11-fee-conservation__careless.patch` (15 lines),
`diffs/t12-derived-index__careless.patch` (19 lines).

## The defect

The more important half. `agent_careless` guarded the drift with

```python
reg = os.path.abspath(os.path.join(task, "regression.patch"))
if os.path.exists(reg):
    ...
return True, "DONE"
```

No `else`. A task with no regression patch therefore fell straight through to
`return True, "DONE"`: self-report DONE, `agent_done` 1, and a result row for a
run in which no drift was ever applied. Reproduced with the pre-guard harness
on t11 with no patch, captured in
`evidence/agent-careless-noop-preguard.txt`:

```
t11-fee-conservation,careless,1,DONE,Accepted,,4,4,Rejected,0,1,,,0
```

`diff_lines=0` is the whole story: nothing was delivered, and the row says
Accepted 4/4. In a study whose release criterion is a false-accept rate, an
arm that manufactures an `Accepted` is worse than one that is biased, because
it adds a true-looking acceptance to the acceptance column.

`agent_careless` now refuses instead of reporting. Three refusals, all of them
loud, and all of them leaving no result row behind:

| refused because | how it was produced | message ends |
|---|---|---|
| no `regression.patch` | `t01-upper-clamp`, which has none | `no regression.patch. The careless arm is 'verified at one revision, edited at a later one', so without a patch there is no later revision and no drift to detect.` |
| the patch does not apply | t11 with a deliberately broken `regression.patch` in a copied task tree | `regression.patch does not apply to the fixed revision (git apply exit 1)` |
| nothing valid to go stale | `--hide-contract` on t11 | `no claim carries valid evidence at the pre-drift revision (no decision (uni verify exit 1))` |

A smaller defect sat two lines above, in the same family but not one of the
counted three. The arm called `uni verify contract.uni` without `--json`, never
got JSON, and fell back to the exit code: 0 became `ACCEPTED`, anything else
became `code:1`. Measured on t11, `uni verify` exits 0 on Accepted and 1 on
Rejected, so the exit code did separate those two. It does not separate Rejected
from EvidenceRequired, both of which were measured at 1, and the line that
states the arm's premise is not the place to print `code:1`: on t11 the
pre-drift `Rejected` did exactly that, which is why the pre-guard reproduction
above reads `uni verify at this revision -> code:1` on a run where `uni verify`
did run and did issue a decision. It now reads the decision out of `--json`
output, like every other decision in the harness.

That makes three instances of the same class in this harness, after the
hardcoded self-report and the missing timeout. All three are false positives in
the same direction: the harness asserting a measurement it did not take.

## Harness change

`run.py` only. Three things:

1. `agent_careless` refuses when there is no regression patch, when the patch
   does not apply, and when the pre-drift revision carries no valid evidence.
   Only the first refusal existed, and it was the silent one. The second closes
   a silent no-op: an inapplicable patch used to print to stderr and carry on to
   `return True, "DONE"`. The third closes a silent degradation found while
   measuring the first: `--hide-contract` removes the contract the arm verifies
   against, so the run continued with no earlier proof at all and still produced
   a row.
2. The premise is asserted as valid evidence per claim, not as an `Accepted`
   decision. On a task that withholds `invariants.rs` until delivery the
   contract cannot be Accepted at the agent's revision by construction, and
   requiring that would refuse the arm for the wrong reason. Staleness is
   decided per claim in `uni-verify`, so the guard counts claims in state
   `Valid` before the late edit. Both tasks carry two.
3. The evidence bundle gained a `trace` key: the journal's event names in
   order, the delivered commit, and the stale block. A bundle otherwise says
   what was decided but not why an earlier proof was not reused, which is the
   whole question on a drifted delivery. The committed `plausible` bundles
   predate this and do not have it; nothing in them is invalidated, they just
   answer a narrower question.

## Limits of the sample

- **The implementer is scripted, not observed.** Same as `plausible`. This arm
  proves the mechanism fires on a delivery that was verified and then drifted.
  It does not show that a model would emit one, and no model was asked to.
- **n=1 per task.** No false-accept or false-reject rate is measured. FAR and
  FRR stay where the sweep documents put them. This adds a detection artifact,
  not a rate, and it says nothing about H1 or H2.
- **The drift is deterministic and hand-placed.** The regression patch is a
  fixture written by the same person who wrote the invariant, so it is known to
  break the contract. That is what makes it a drift case and not a surprise;
  it is also why it proves nothing about how hard real drifts are to find.
- **t12's drift lands on the failure mode `wrong.patch` already stages.** The
  batch path forgetting the index is the same wrong answer reached by a
  different route and on a different revision. That is the point of the arm,
  not a discovery: the trap re-opens on a correct delivery, and revision
  binding is what re-opens it. It is not a second independent failure mode.
- **The guarantee is bound to the revision, so an uncommitted late edit is
  invisible when no verifier watches the edited file.** Measured: apply the t11
  regression without committing it and re-verify at the same commit, and the
  evidence is reused and the decision still reads Accepted, because
  `mini.settlement.invariants` watches `tests/invariants.rs`, not
  `src/settlement.rs`. Nothing here contradicts constitution rule 10; it is the
  rule's actual boundary, and it is worth stating rather than leaving implied.
- **The pre-drift decision is Rejected on t11 and t12, by construction.** The
  owner's invariant is withheld for the whole agent phase, so the contract
  cannot be Accepted until delivery. The earlier proof that goes stale is
  therefore per claim (2 of 4), not a whole-contract acceptance. Anyone wanting
  the contract-level version has t09, which has no hidden invariant and reaches
  Accepted 3/3 before its late edit.

## Measured, not copied

`cargo test` on this commit: **158 passed, 22 suites, 0 failed, 0 ignored**.

`README.md` claimed 142 in its `cargo test` line, and `.scratch/tickets.md`
claimed 158, at the same version. Measured is 158, so `README.md` was the stale
one and is corrected in this commit. Per `CONTRIBUTING.md:45-46`, a number in a
document is a fact only if something re-derives it, and nothing did.
