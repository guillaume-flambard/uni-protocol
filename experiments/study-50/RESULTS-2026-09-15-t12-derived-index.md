# t12 and two harness defects, one of which invalidates a published column

Date: 2026-09-15. Free OpenRouter implementers, $0 spent. Read the correction
section first if you have read any earlier results file.

## t12: the task

The second multi-file task, same shape as t11: the owner's invariant is written
in only at verification time, the arm hides the contract and the registry, and
the behavioural claim goes through an authorized selector rather than a pinned
test name.

`Ledger::apply_atomic` must apply a batch of entries all-or-nothing. Two
independent, plausible failure modes are built into it, and each is caught by a
different named property:

| plausible delivery | what it gets wrong | which owner invariant catches it |
|---|---|---|
| the batch path mutates the accounts itself | the derived per-owner index is left behind | `the_index_keeps_up_with_a_sequence_of_batches` |
| the batch path checks each entry against the starting balances | a batch can spend the same money twice | `a_batch_that_overspends_only_in_total_is_refused` |

Both were built and verified as real deliveries before the task was committed:
each passes its own `apply_atomic_works`, each fails exactly one owner
invariant, and the ground-truth patch applies to a fresh base and passes 4 + 4.

## Arms

| arm | its own `cargo test` | UNI | note |
|---|---|---|---|
| fixture (ground truth) | green | Accepted 4/4 | |
| scripted plausible (index forgotten) | **green** | **Rejected 2/4** | the demonstration |
| real agent, cohere | **red** | **Rejected 2/4** | see below; the owner's four invariants all passed |
| real agent, ling | green | Accepted 4/4 | implementation correct, invariants pass |

## The first real-model non-acceptance in the whole study, and it is not the trap

The cohere run on t12 is the first time a real model was not accepted. It is
worth stating precisely what happened, because it is not the trap firing:

- Its `apply_atomic` **is correct**: all four owner invariants pass, index
  coherence and cumulative atomicity included.
- Its **own test** is wrong. It asserts that a batch of 100 out of
  `alice-checking` (500) and 200 out of `alice-savings` (100) succeeds, then
  asserts `alice-savings == 0`. The implementation correctly refuses that batch.
  So the delivery ships a red suite, and it contradicts the implementation.
- UNI rejects it on `tests-green`, which the contract declares CRITICAL, and on
  the selector claim whose test is the one failing.

That is a defensible rejection: nobody merges a red suite, and the delivery's
own claim about itself is false. But it is a *self-contradiction* finding, not a
detection of a plausible-but-wrong implementation. The honest summary: on real
multi-file work, the first model failure came from the worker not running (or
not believing) its own tests, not from the trap the task was built around.

## The correction: the worker's self-report was invented by the harness

Found by running t12, and it affects published numbers.

`agent_opencode` returned a hardcoded `"DONE"`. So for every run of the
real-agent arm, the `agent_self_report` column was **asserted by the harness,
not read from the worker**. The only two producers of `FAILED` in `run.py` are
scripted patches that fail to apply, which means the single `FAILED` row in
`results-all.csv` and `results-opencode.csv` could not have come from this code.
It was entered by hand.

What this breaks:

- `RESULTS-2026-09-14.md` states the method as "Agent self-report parsed from its
  own output (`agent_self_report`), never assumed". For the real-agent arm that
  sentence is false.
- The `AGENT BASELINE (trust every DONE)` metric is computed from that column. A
  harness that always records DONE **maximises** the baseline's claimed
  successes, so the bias runs against the baseline, which is the same thing as
  running in UNI's favour. That is the wrong direction for a study whose whole
  point is being hard on itself.

Fixed: the report is now read from the worker's last `DONE`/`FAILED` line, and
anything else is `UNPARSED`, which is a result rather than a guess. Applied to
the three honest re-runs immediately:

| task | model | self-report | UNI |
|---|---|---|---|
| t12 | cohere | UNPARSED | Rejected |
| t12 | ling | UNPARSED | Accepted |
| t11 | cohere | DONE | EvidenceRequired |

Two of the three models never said DONE: they wrote a prose summary instead of
the final token the prompt asks for. The old harness would have recorded both as
claimed successes. `results-honest-selfreport.csv` holds these rows.

What is *not* done: the older rows cannot be repaired. Their self-report column
is not harness output, and the models are non-deterministic, so re-running would
produce different samples rather than recover the originals. They are kept with
this note, the way the invalidated 2026-09-13 run was kept, and every arm from
here on records the column honestly.

## Also fixed: a hung implementer blocked the harness

No implementer had a timeout. A cohere run on t12 hung and blocked the harness
for 40 minutes before it was noticed; the `sh()` helper takes an optional
timeout now and `UNI_AGENT_TIMEOUT` bounds the opencode arm, with a timeout
recorded as a non-zero exit. A second defect in that fix, where a timeout
carries bytes even in text mode and concatenating a string onto them raised,
was found by exercising the timeout path directly and is fixed.

## Limits

n=1 per (task, model). The free tier is a weak implementer with a high variance:
the same model accepted t11 in one run and returned 3/4 in another. Nothing here
measures a capable model, and nothing here measures the trap against a model
that reads its own test output.
