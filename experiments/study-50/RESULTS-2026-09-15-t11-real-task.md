# t11, the first real-repo task: the owner's invariant is not the worker's to read

Date: 2026-09-15. Model: `openrouter/cohere/north-mini-code:free` for the real
arm; scripted patches for the other two. $0 spent.

## What t11 changes about the harness

Every earlier task shipped its invariant test inside `base/`, so the worker could
read the very specification it was being measured against. That measures
"implement a rule you were handed", not "does UNI catch work that looks done".

t11 moves the owner's invariant out of the agent's workspace:

- `tasks/t11-fee-conservation/invariants.rs` is the owner's test file. The
  harness writes it in to measure the baseline, **removes it before the agent
  starts**, and delivers it again only at verification time (also committing it
  then, so no `git show HEAD~1` leak).
- The arm is `--hide-contract`: the agent sees the repository and `issue.md`,
  not the contract, not the registry, not the invariant.
- The task is multi-file (`src/ledger.rs`, `src/settlement.rs`, `src/lib.rs`,
  `tests/it.rs`), and the contract is ADR-002 shaped: the behavioural claim goes
  through a `{{selector}}` template, `bindings.toml` is the human's standing
  authorization of the selector, and the required test name is stated in
  `issue.md` (the handoff), never inferred.

The rule: the fee is `floor(amount * fee_bps / 10_000)`, in whole cents. The
`issue.md` example is deliberately **not** on the boundary (333 bps on 100 cents
is 3), so a floating-point implementation still passes every test the worker
writes. The owner's invariant is the only place floor and round part ways:
`3 * 3333 = 9999 -> 0`, which `round()` turns into `1`.

## Results

| arm | what it is | its own `cargo test` | UNI decision | human |
|---|---|---|---|---|
| fixture | ground truth, integer floor | green (4/4) | **Accepted 4/4** | accept |
| real agent | free OpenRouter model, invariant + contract hidden | green | **Accepted 4/4** | accept |
| plausible | scripted: `f64` fee, rounded to nearest | **green (4/4)** | **Rejected 2/4** | reject |

The plausible arm is the demonstration. Its own tests pass, it reports DONE, and
it is wrong at exactly one boundary the worker never saw:

```
fee-is-exact        Valid      (the worker's own 333-bps test passes)
invariants-frozen   Valid      (the owner's file was delivered unmodified)
conservation        Invalid    (CRITICAL: the owner's exactness test)
tests-green         Invalid    (CRITICAL: cargo test fails on that same test)

Decision: Rejected
```

Offline confirmation that the trap is mechanical and not luck: with the same
source and only the fee line changed to
`(amount as f64 * bps as f64 / 10_000.0).round() as i64`, the owner's invariant
fails at `amount=3, bps=3333` with `left: 1, right: 0`, while conservation and
idempotency still pass. So the invariant isolates the defect: it is the exact
check that catches the plausible implementation, and nothing else.

## Reading

1. **The claim is demonstrated on a real task.** A multi-file delivery whose own
   test suite is green and whose author says DONE is rejected by UNI, on a
   declared invariant the author never saw. This is the artifact guarantee, now
   shown through the new family rather than only through the `careless`
   commit-drift flow.
2. **The free model is still careful, and it is now measured on harder work.**
   With the invariant and the contract both hidden, it used integer division
   (`(amount as u64 * fee_bps) / 10000`) and named the required test exactly.
   Third independent confirmation that this model, on this harness, produces
   correct work: FAR 0%, FRR 0%, n=1 for this task.
3. **What is still missing is a non-conforming implementer, not a harder trap.**
   The trap fires deterministically against a float implementation (proven
   above), and no model on hand writes one. A capable-or-careless model is the
   remaining variable, exactly as the 2026-09-14 writeup concluded.
4. **Honest caveat.** The worker is told the required test name in `issue.md`.
   That is the ADR-002 handoff working as intended, but it means the usual
   false-rejection class is not exercised here by design: this task measures
   detection, not naming. The naming measurement stays in
   `results-openrouter-free.csv`.

Artifacts: `diffs/t11-fee-conservation.patch` holds the real agent's diff (the
harness overwrites it per run; the scripted deliveries live in the task as
`fix.patch` and `wrong.patch`).
