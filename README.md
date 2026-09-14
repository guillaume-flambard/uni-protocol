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
| | `uni lint <contract>` | preflight without execution (coverage, registry refs, duplicates) |
| Evidence | `uni verify <contract>` | runs trusted-registry verifiers; incremental, git-+content-bound evidence; STALE on commit or watched-file change |
| | `uni explain` | claims table + per-claim evidence detail (PRD §16 UX) |
| Policy | `.uni/policies/*.toml` | reject_on_invalid, escalate_on_stale/missing, min_verified_ratio (deterministic) |
| Decision | - | truth table → policy → ACCEPTED/REJECTED/EVIDENCE_REQUIRED/ESCALATED |
| Audit | `uni report` | byte-stable CI/PR view (`GitHub step summary` ready) |
| | `uni events` | append-only journal with `uni.*` attributes (OTel-ready) |
| Health | `uni doctor` | workspace healthcheck (git, registry, policies, writability) |
| Packs | `uni pack list` / `uni pack template <pack> <name>` | Domain Packs: reusable claim templates (`packs/software`) |
| Execute | `uni run <contract> -- <command>` | runs your executor (any agent, any tool), then verifies; the exit code is the decision's |
| Work order | `uni brief <contract>` | deterministic agent handoff: claims + exact evidence required (markdown/json, byte-stable) |
| Authorization | `uni bind` / `uni bindings` | VerifierBindings for REQUIRE-carrying verifications (human act, journaled) |
| Transport | `uni bundle export` / `uni bundle verify` | audit surface as JSONL with per-record sha256; verify is offline and read-only |
| Study | `experiments/study-50/` | harness + metrics (agent self-report vs UNI vs human) |

Stack independence verified in CI: `examples/` covers Rust, Python (unittest),
Node (`node --test`), and a `file-hash` artifact digest. CI matrix:
ubuntu, macos, windows for the core; POSIX-shell examples gated to Linux.
Release workflow builds five cross-compiled targets; the composite action in
`adapters/github/` downloads the matching release binary (no build).

## DSL (closed vocabulary v0.1)

`VERSION DOMAIN INTENT GOAL CLAIM REQUIRE ENSURE INVARIANT FORBID VERIFY ACCEPT REJECT ESCALATE`

Example - `examples/booking/booking.uni`:

```
VERSION 0.1
DOMAIN software
INTENT booking.cancel

CLAIM booking-state REQUIRED
  ENSURE booking.status == cancelled
INVARIANT ledger-consistency CRITICAL
  ENSURE ledger.balance == expected.balance

VERIFY booking-state
  USING mini.t.booking
ACCEPT WHEN
  required_claims == VERIFIED
  AND critical_failures == 0
```

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

Start at [docs/index.md](docs/index.md): why UNI, quickstart, language,
claims, evidence, verification, decisions, GitHub, writing verifiers,
specification.

## Dogfood & tests

```bash
cargo test                  # 25 tests incl. propperty-based decision determinism
./target/release/uni verify examples/hello/hello.uni && echo OK
```

Study results so far: UNI 100% agreement on 5-task sample, agent self-report 80%,
0 false accepts (release criterion), 1/1 true block.

## Constitution

`constitution.md` - UNI is the outcome assurance layer. LLMs propose, humans/policies authorize, verifiers prove, UNI decides. Spec source: `specs/001-uni-software-v01/spec.md`.
