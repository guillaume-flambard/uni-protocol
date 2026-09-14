# Study-50 - agent self-report vs UNI vs human review (PRD §20)

Driver + analyzer for the outcome-assurance experiment.

## Layout
```
tasks/<id>/
  issue.md          deliverable statement
  contract.uni      UNI contract (claims + per-test verifiers)
  base/             starting (buggy) state of the mini repo
  fix.patch         ground-truth fix (fixture agent mode)
```

## Run
```bash
python3 run.py --agent fixture            # ground-truth fixture replay
python3 run.py --agent opencode           # real headless implementer (--dir pinned)
python3 run.py --agent codex              # real Codex implementer (wire-in: agent_codex; codex CLI absent on this machine)
python3 run.py --agent claude             # real Claude implementer (blocked 2026-09-13: org disabled Claude Code subscription; wire-in armed)
python3 collect.py results.csv            # agreement metrics
```
Then fill the `human_review` column (1 accept / 0 reject, blind review) and re-run collect.
Use `--append` when running one task at a time, and `--keep` to preserve the
workspace for diff review. Results so far: [RESULTS-2026-09-13.md](RESULTS-2026-09-13.md).

## Results

- [RESULTS-2026-09-14.md](RESULTS-2026-09-14.md) - corrected real-agent study
  (n=6, FAR 0%, FRR 20%, H1/H2 not supported on this sample).
- [RESULTS-2026-09-13.md](RESULTS-2026-09-13.md) - INVALIDATED harness-bug run.

## Smoke result (2 tasks, 2026-09-13)
- t01 correct fix → ACCEPTED (4/4 claims verified).
- t02 partial fix (upper bound clamped to wrong value) → EvidenceRequired, UNI refuses silently-accepted work.
- The smoke itself caught a real model hole: 4 claims sharing one `cargo test` verifier all passed while the fix was broken. Registry now supports per-test verifiers (`run` + `expect = "test result: ok. 1 passed"`).

## Release criterion
`false accepts (UNI ACCEPTED but human rejected)` must be 0.
