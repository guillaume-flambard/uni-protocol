# The execute half (`uni run`)

```bash
uni run conv.uni -- 'opencode run --dir . "implement issue.md"'
uni run conv.uni --timeout-ms 1800000 -- 'codex exec "implement issue.md"'
```

`uni run` executes the command you give it in the workspace, reports the
executor's output and exit code, then runs the normal verification and exits on
the decision.

## Two rules

1. **The command comes from the command line, never from the contract.** UNI
   integrates no agent and no runtime: you name the executor, so this adds no
   trust surface. The contract still only references verifier names.
2. **The executor's exit code is data, not truth.** An agent that exits 7 after
   doing sound work still gets `ACCEPTED`; an agent that exits 0 after breaking
   the work still gets `REJECTED` or `EVIDENCE_REQUIRED`. Only evidence decides,
   and the process exit code is the decision's, not the executor's.

Each run is journaled as an `ExecutionRun` (command, exit code, duration) next
to the evidence it produced, so `uni events` shows what was executed and what it
proved.

Use `uni brief` to produce the work order the executor should follow, and
`--timeout-ms` to bound a runaway agent (default 900000, i.e. 15 minutes; the
executor is killed and the verification still runs).
