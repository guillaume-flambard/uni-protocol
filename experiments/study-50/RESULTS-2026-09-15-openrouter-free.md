# Second-model arm 2026-09-15 (OpenRouter free tier, unblocks the ticket)

Model: `openrouter/cohere/north-mini-code:free` (free, $0 spent), headless via
`opencode run --pure --auto --dir <work>`, one attempt per task, no-brief arm
(contract + registry visible, issue-only prompt). Harness unchanged
(`run.py --agent opencode`, `UNI_AGENT_MODEL`).

Why this model: `bai/*` refused (credit/deposit), `opencode/*` billed or
headless no-ops (checked 2026-09-14 and 2026-09-15). `OPENROUTER_API_KEY` was
already in the environment; a write-probe (`hello.txt`) confirmed real tool
use before spending study time.

## Results (n=10, all reviewed, diffs in `diffs/`)

| task | agent | UNI decision | human | verdict |
|---|---|---|---|---|
| t01-upper-clamp | DONE | ACCEPTED 4/4 | accept | True Accept |
| t02-upper-clamp-partial | DONE | ACCEPTED 4/4 | accept | True Accept |
| t03-int-sum-offbyone | DONE | EVIDENCE_REQUIRED 1/3 | accept | **False Reject** (named `sum_to_three`, contract pins `sum_three`) |
| t04-first-word-empty | DONE | ACCEPTED 3/3 | accept | True Accept (used the exact pinned name) |
| t05-async-order | DONE | ACCEPTED 3/3 | accept | True Accept |
| t06-greeting | DONE | ACCEPTED 2/2 | accept | True Accept |
| t07-layered-cancel | DONE | EVIDENCE_REQUIRED 2/4 | accept | **False Reject** (`cancel_existing_booking` vs pinned `cancel_ok`; layering + behaviour correct) |
| t08-port-contract | DONE | EVIDENCE_REQUIRED 2/4 | accept | **False Reject** (`parse_port_valid_range` vs pinned `parse_valid`; correct impl, no unwrap) |
| t09-api-freeze | DONE | ACCEPTED 3/3 | accept | True Accept |
| t10-ledger-invariant | DONE | ACCEPTED 4/4 | accept | True Accept |

```
                 human accepts   human rejects
UNI accepts         7 (TA)          0 (FA)
UNI rejects         3 (FR)          0 (TR)

FALSE ACCEPTANCE RATE   0%
FALSE REJECTION RATE   30%
precision             100%   recall 70%   evidence coverage 82% (28/34)
AGENT BASELINE (trust every DONE): 10/10 claimed, FA=0, FAR=0%
```

Artifacts: `results-openrouter-free.csv` (human_review filled), `diffs/*.patch`.

## Reading

1. **The arm is unblocked, and it confirms ADR-002.** All 3 false rejections
   are evidence-name coupling again (a second, unrelated model, same failure
   mode). The lint warning (`pinned-test-selector`) now names this at
   contract-authoring time.
2. **FAR 0% holds on a second model.** No false accept in 10 runs; the model
   never lied (10/10 DONE all human-accepted).
3. **Variance note.** t04 failed on the first model (wrong test name) and
   passes here (exact name): naming failures are a roll of the dice, which is
   why the fix belongs in the contract/handoff, not in hoping.
4. **Caveat.** `diffs/*.patch` were overwritten by these runs (harness
   behaviour); the 2026-09-14 rows keep their CSVs and narrative, but their
   patches are superseded. n=10, one free model, toy-to-small tasks: same
   limits as before.

## Brief arm (t03/t07/t08): the handoff must name the work order

Two phases, same 3 tasks, both with `brief.md` generated into the workspace:

| phase | prompt mentions brief.md? | result |
|---|---|---|
| silent brief | no (`run.py` as committed) | 3x EVIDENCE_REQUIRED, same naming rejects: the model wrote `sum_to_three`, `cancel_existing_booking`, `parse_port_valid_port` and never opened the file |
| named brief | yes (harness fix, `UNI_BRIEF=1` appends "Read brief.md first and follow it exactly") | 3x ACCEPTED (3/3, 4/4, 4/4), exact pinned names, substance reviewed (correct values, no unwrap, layering held) |

Brief arm: FAR 0%, FRR 0%, evidence 100% (11/11). Artifacts:
`results-openrouter-brief.csv` (reviewed; phase-2 rows).

Reading: `uni brief` as a file is decoration; as an instruction it is the
fix. The handoff theory holds a second time, with the mechanism made
explicit. Product consequence: whatever invokes the worker (`uni run`, CI
handoff docs, the study harness) must point at the brief, not merely emit
it. This sharpens ADR-002's open item (auto-emit in `uni run`): emission
without invocation changes nothing.

Caveat: `diffs/` for t03/t07/t08 now hold the brief-arm patches (harness
overwrites); the no-brief decisions stand in `results-openrouter-free.csv`.

## Next

- Real-repo tasks per the 2026-09-14 plan; more easy tasks prove nothing.
