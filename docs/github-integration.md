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

## Supported platforms (v0.3)

`cargo test` and the pure-Rust examples (`hello`, `multi`, `booking`) run on
ubuntu, macos, and windows in CI. The shell verifier is platform-gated
(`sh -c` on POSIX, `cmd /C` on Windows) and timeouts are enforced natively, so
a hanging verifier is killed and recorded Invalid on every platform.

POSIX-only examples (`forbid` uses `grep`, `python`/`node` examples use POSIX
paths and reporter formats) run on Linux only, by design: platform portability
is claimed for the core, not for POSIX shell recipes.

## Release workflow

`.github/workflows/release.yml` builds five targets on tag push:
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, each as
`uni-<target>.tar.gz` attached to the GitHub release.

## `adapters/github/action.yml`

A composite action that DOWNLOADS the release binary for the runner platform
(no build, no npm). Use it as a subdirectory action, or copy the file to an
action repo root:

```yaml
- uses: your-org/uni/adapters/github@v0.5.2
  with:
    contract: uni/intents/feature.uni
    version: v0.5.2
    repository: your-org/uni
```

Inputs: `contract` (required), `version` (default `v0.5.2`), `repository`
(default `guillaume-flambard/uni-protocol`, replace with your fork), `report-to-summary`
(default true). Unknown runner OS fails loudly instead of silently skipping.

## Verified in CI (2026-09-14)

All of the above ran for real on the published repository:

- `uni` workflow: matrix ubuntu / macos / windows (build, portable unit tests,
  POSIX integration tests outside Windows) plus a POSIX-examples job, all green.
- `release` workflow: five targets built and attached on tag push
  (`uni-<target>.tar.gz`), for `v0.5.0` and `v0.5.2`.
- the composite action itself: an `action-smoke` job downloads the released
  binary and verifies a runtime-free contract (`examples/artifact`).

Pushing the tag and `main` in the same breath races the smoke job against the
release assets; push the tag first, or re-run the job.

Published releases: `v0.3.0`, `v0.4.0`, `v0.5.0`, `v0.5.1`, `v0.5.2` (latest), five
assets each. `v0.1.0` predates the release workflow, so it has no release, and
its CI run is the only historical one that is green by construction. The CI
runs for `v0.3.0` and `v0.4.0` fail: those tags predate the isolation and
portability fixes below, which is the honest record of versions that were never
CI-clean.

## Evidence lifecycle in CI

Runners are ephemeral, so evidence regenerates per run; git and content bindings
still do their job inside the run, and the `events.jsonl` journal documents
which verifiers executed versus which were cache hits.
