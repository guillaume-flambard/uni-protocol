# GitHub integration

## Pull request check

The workflow in `.github/workflows/uni.yml` builds the release binary and runs
`uni verify` on every push/PR, publishing the stable assurance view to the step
summary:

```yaml
- run: ./target/release/uni verify examples/hello/hello.uni
- run: ./target/release/uni report >> "$GITHUB_STEP_SUMMARY"
```

`uni report --json` is byte-stable across runs on unchanged state, so bots and
dashboards can diff it safely. `uni doctor` in CI catches workspace drift
(missing registry, unwritable evidence dir) before contracts run.

## Status semantics

Exit codes map 1:1 to review gates:

| exit | meaning | recommended check |
|---|---|---|
| 0 | ACCEPTED | success |
| 1 + "UNI REJECTED" | disproven or policy-rejected | failure, blocking |
| 1 + "UNI EVIDENCE_REQUIRED" | missing/stale proof | failure, actionable ("Run: uni verify <claim>") |
| 1 + "UNI ESCALATED" | policy wants a human | action required |

## `adapters/github/action.yml`

A composite action template pinned to the embedded binary pattern (no runtime
`npm install`, same stance as spec-guard's action). For public consumption the
action must ship with a release download URL; that packaging is v1 work.

## Evidence lifecycle in CI

Runners are ephemeral, so evidence regenerates per run; git and content bindings
still do their job inside the run, and the `events.jsonl` journal documents
which verifiers executed versus which were cache hits.
