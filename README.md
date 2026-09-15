# UNI - Outcome Assurance Protocol

**Never trust "done" from an agent. Verify the outcome.**

```
INTENT → CLAIMS → EVIDENCE → POLICY → DECISION
```

Rust workspace, single CLI binary `uni`. Deterministic. Filesystem-only. No cloud required.

## Capability map (v0.9)

| Area | Command | Behavior |
|---|---|---|
| Contract | `uni init` | creates `.uni/{config.toml,contracts,evidence,decisions,policies} + uni/intents` |
| | `uni compile | inspect` | `.uni` DSL → canonical JSON IR (schema `schemas/uni.schema.json`) |
| | `uni import-speckit <dir>` | Spec Kit spec.md/plan.md → **candidate** DSL for human review |
| | `uni lint <contract>` | preflight without execution (coverage, registry refs, duplicates, selector bindings, pinned test names) |
| Evidence | `uni verify <contract>` | runs trusted-registry verifiers; incremental, git-+content-bound evidence; STALE on commit or watched-file change |
| | `uni explain` | claims table + per-claim evidence detail (PRD §16 UX) |
| Policy | `.uni/policies/*.toml` | reject_on_invalid, escalate_on_stale/missing, min_verified_ratio (deterministic) |
| Decision | - | truth table → policy → ACCEPTED/REJECTED/EVIDENCE_REQUIRED/ESCALATED |
| Audit | `uni report` | byte-stable CI/PR view (`GitHub step summary` ready) |
| | `uni events` | append-only journal with `uni.*` attributes; `--otlp` emits OTLP/JSON, `--all` reads archives |
| Health | `uni doctor` | workspace healthcheck (git, registry, policies, writability) |
| Packs | `uni pack list` / `uni pack template <pack> <name>` | Domain Packs: reusable claim templates (`packs/software`) |
| Execute | `uni run <contract> -- <command>` | runs your executor (any agent, any tool), then verifies; the exit code is the decision's |
| Work order | `uni brief <contract>` | deterministic agent handoff: claims + exact evidence required (markdown/json, byte-stable) |
| Authorization | `uni bind` / `uni bindings` | VerifierBindings for REQUIRE-carrying verifications (human act, journaled) |
| Identity | `UNI_IDENTITY_TOKEN` + `[identities]` | verifies a signed JWT against a registry-pinned issuer (JWKS offline), lifting an independent proof from A3-D to A3 |
| Transport | `uni bundle export` / `uni bundle verify` | audit surface as JSONL with per-record sha256; verify is offline and read-only |
| Study | `experiments/study-50/` | harness + metrics (agent self-report vs UNI vs human) |

Stack independence verified in CI: `examples/` covers Rust, Python (unittest),
Node (`node --test`), and a `file-hash` artifact digest. CI matrix:
ubuntu, macos, windows for the core; POSIX-shell examples gated to Linux.
Release workflow builds five cross-compiled targets; the composite action in
`adapters/github/` downloads the matching release binary (no build).

## DSL (closed vocabulary v0.1)

`VERSION DOMAIN INTENT GOAL CLAIM REQUIRE ENSURE INVARIANT FORBID VERIFY ACCEPT REJECT ESCALATE`

Verbatim from `examples/booking/booking.uni`:

```
VERSION 0.1
DOMAIN software
INTENT booking.cancel
GOAL
  Cancel a confirmed booking safely.
CLAIM booking-state REQUIRED
  ENSURE booking.status == cancelled
CLAIM inventory REQUIRED
  ENSURE inventory.available == inventory.before + booking.seats
INVARIANT ledger-consistency CRITICAL
  ENSURE ledger.balance == expected.balance
VERIFY booking-state
  USING test.true
VERIFY inventory
  USING test.true
VERIFY ledger-consistency
  USING test.true
ACCEPT WHEN
  required_claims == VERIFIED
  AND critical_failures == 0
```

Every claim carries its own `VERIFY`, the invariant included: `uni lint` fails
otherwise. The verifier names resolve through the trusted registry, never
through the contract.

## Trusted registry (contracts are untrusted)

```toml
[verifiers]
"mini.tests" = "cargo test"

[verifiers."forbid.dirty-write"]
run = "! grep -Rn 'EVIL_WRITE' examples/forbid/sources.txt"
expect_not = "EVIL_WRITE"
files = ["examples/forbid/sources.txt"]
timeout = 30
```

`expect` = output must contain; `expect_not` = output must not contain; `files` = content-bound
evidence invalidation by SHA-256 over watched files.

## Documentation

The complete report, including what is proven and what is not:
[docs/REPORT-2026-09-14.md](docs/REPORT-2026-09-14.md).

Start at [docs/index.md](docs/index.md): why UNI, quickstart, language,
claims, evidence, verification, decisions, GitHub, writing verifiers,
specification.

## Dogfood & tests

```bash
cargo test                  # 142 tests incl. property-based decision determinism
./target/release/uni verify examples/hello/hello.uni && echo OK
```

Evidence so far, in four parts:

- **The check, in the pull request**: a drifted proof lands as an inline
  annotation on the file that moved. [docs/flagship-check.md](docs/flagship-check.md)
- **Stale Evidence Benchmark**, 100 manufactured drift scenarios with no model
  involved: on the 70 where the artifact stopped satisfying the claims, an
  exit-code CI stayed green 50 times, a cached CI 70 times, UNI 0 times. UNI
  detection 86 percent (100 percent in every detectable category), false-stale 0
  percent. [experiments/stale-bench/RESULTS-2026-09-14.md](experiments/stale-bench/RESULTS-2026-09-14.md)
- **Agent study**, 15 reviewed runs plus a five-model sweep: 0 false accepts; the
  false rejects were all test naming, and `uni brief` plus selector bindings
  remove the class. The models never misreported. H1/H2 (UNI beats an honest
  agent's self-report) are **not** supported, and the write-up says so.
  [experiments/study-50/RESULTS-2026-09-14.md](experiments/study-50/RESULTS-2026-09-14.md)
- **The task where it counts** (`t11`), a multi-file delivery whose own test
  suite is green and whose author reports DONE, on an owner invariant the author
  never saw: **Rejected**. The plausible implementation is wrong at exactly one
  boundary, and the owner's invariant is the only place it shows.
  [experiments/study-50/RESULTS-2026-09-15-t11-real-task.md](experiments/study-50/RESULTS-2026-09-15-t11-real-task.md)

## Contributing

Issues and pull requests are welcome. Start with
[CONTRIBUTING.md](CONTRIBUTING.md): it covers the one workflow worth knowing
(contract first, then `uni verify`), how to write a verifier adapter, and the
house rule that every claim is proven by a verifier rather than asserted in
prose. If you want to work on something and are not sure where, open an issue
with what you tried and what you expected.

Two things make this project easy to contribute to on purpose: a single binary
with no cloud dependency (`git clone` to a green `uni verify` in under a
minute), and a test suite that runs the real CLI on real files, so a change that
works locally is very likely to work in CI.

## Constitution

`constitution.md` - UNI is the outcome assurance layer. LLMs propose, humans/policies authorize, verifiers prove, UNI decides. Spec source: `specs/001-uni-software-v01/spec.md`.
