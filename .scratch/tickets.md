# UNI — tickets

Current: **v0.8.1** · 114 tests, 0 warnings · public repo
`github.com/guillaume-flambard/uni-protocol` · CI green on ubuntu/macOS/Windows +
POSIX examples + a smoke job that runs the published action · release `v0.8.1`
with five binaries.

## Done — CI hosting decision (2026-09-14)

Briefly self-hosted, then reverted: the repository is public, so GitHub-hosted
runners are free and unlimited, and the only gain was ~40 s per push. That is
not worth executing every dependency's build scripts on the production host.
lab-infra PR #68 added the runner and PR #69 removed it (runner unregistered,
service and user deleted). Hosted is the choice for public repos; self-hosting
stays right for the private ones, where minutes are billed and images must be
built beside the local registry.

## Done — the check earns its keep (v0.8.0 / v0.8.1)

The proof is now something a reviewer sees in the pull request, and its failure
is measured.

- **Stale reasons as data.** A stale verdict carries the dimension
  (`uncommitted_changes`, `subject_changed`, `commit_changed`,
  `contract_changed`, `verifier_config_changed`, `authorization_changed`,
  `expired`, ...) and the named files that moved. The drift is recorded durably,
  and `uni explain` narrates what changed instead of a generic "stale".
- **GitHub annotations.** `uni explain --annotations` emits one `::error`
  (CRITICAL claim) or `::warning` per moved file, with `file=` set, so the
  drifted claim lands inline on the diff. A reason that names no path gets no
  duplicate note on the workflow file.
- **The published action reports.** `adapters/github/action.yml` verifies, and on
  failure annotates the drift and appends the plain-word narration to the job
  summary. Default bumped to `v0.8.1`.
- **The Stale Evidence Benchmark.** `experiments/stale-bench/` builds 100
  manufactured drift scenarios and asks three oracles. 70 lose the proof; UNI
  detects 100% of those it can (86% overall; the missing item is a weakened
  trusted registry, which it names in 100% and leaves to a human). A plain
  exit-code CI missed 71% of the 70, a result cache missed all 70. Full table in
  `experiments/stale-bench/RESULTS-2026-09-14.md`.
- **Flagship page.** `docs/flagship-check.md`, with screenshots of the red check
  and the step order (prove, drift, annotate, fail).

## Open

Ordered by value. Nothing here is started unless marked.

1. **Second-model study arm** — BLOCKED externally. Every `bai/*` model is
   refused (credit/deposit), and the `opencode/*` "free" models are headless
   no-ops (they print the session header and exit). The harness is ready:
   `UNI_AGENT_MODEL=<model> python3 run.py --agent opencode --only <task>`.
   Needs: one working implementer, then the same 15-task protocol.
2. **A3 real** — the assurance scale reaches A3-D (independent actor,
   self-declared identity). A3 needs identity adapters (spiffe/entra/oidc);
   they are stubs today, so A3 is unreachable outside unit tests. A4 stays
   refused by design (`--attest` names the missing signer).
3. **Cloud / org** — organizations, dashboards, `cost per accepted outcome`.
   Deliberately after the single-user story is convincing.
4. **Vault note** — `1-Projects/uni.md` does not exist; `PROJECTS.md` line is
   present. Low value until the project has a broader audience.

## Done

Condensed by milestone; the detailed history is in git.

- **v0.1 — core frozen.** DSL (closed vocabulary, hard errors for reserved
  syntax), canonical IR + JSON Schema, trusted-registry verifiers, git- and
  content-bound evidence, deterministic decision engine, `explain`/`report`/
  `events`/`lint`/`doctor`, Software Pack, 5 stack examples. Evidence
  Completeness Principle in the constitution.
- **v0.2 — assurance model.** Verification Context (registry/policy/contract/
  platform hashes), Hit/Stale/Miss cache outcomes, registry trust-boundary diff
  with `trust_boundary_changed` for CI, actor/executor separation with the
  A2 / A3-D / A3 / A4 scale, VerifierBinding (`uni bind`, `REQUIRE`).
- **v0.3 — portability and distribution.** Native verifier timeout, `cmd /C`
  vs `sh -c`, public `Verifier` trait with a `file-hash` adapter, evidence
  bundles (`uni bundle export|verify`, read-only verification), release
  workflow for five targets, published action that downloads the release
  binary.
- **v0.4 — the study, corrected.** The first real-agent run (n=7) was
  invalidated: `opencode run` resolved a stale project directory and fixtures
  were silently pre-fixed. The harness now pins `--dir`, copies the task
  statement, refuses a base that already satisfies the contract, restores
  fixtures, and keeps the agent's diff. Corrected 15-run results:
  **FAR 0%, FRR 23%**, the model claimed DONE 13/13 with zero human rejections,
  and the only baseline false accept is the scripted "careless" run
  (verified at revision A, delivered revision B). Four designed traps
  (architecture invariant, vague issue, frozen API, conservation semantics) did
  not fire: the model read the contract and behaved.
- **v0.5 — work order and selector templates.** `uni brief` (deterministic
  claims + exact evidence, including the test selector, byte-stable, markdown
  and JSON) and its emission beside imported Spec Kit candidates; selector
  templates with `uni bind --selector` (worker names the test, human authorizes
  it, selector included in the binding hash); tracked-only dirtiness (a
  verifier that compiles no longer stales its own evidence); Windows path
  separator fix; a declared watch that observes nothing is Invalid.
- **v0.6 — evidence lifecycle completed.** Time is real: a verifier may declare
  `max_age_hours`, the expiry is stamped on the proof so a registry change
  cannot extend it, and an expired proof is stale (the one clock-reading
  predicate, constitution rule 10). The journal rotates past 1 MiB keeping the
  three newest archives, `uni events --all` reads the history, `uni doctor`
  reports size and cap.
- **v0.7 — execution, export, one bindings file.** `uni run <contract> --
  <command>` executes your executor then verifies (its exit code is data, the
  decision drives the exit). `uni events --otlp` emits an OTLP/JSON document
  with deterministic trace/span ids. `.uni/bindings.toml` holds every binding in
  one reviewed file, `uni bind --from <file>` authorizes the whole review in one
  act (legacy per-claim files still load). `main.rs` split into `src/cmd/`
  (1404 -> 127 lines).
- **v0.8 — the check earns its keep.** Stale reasons as data (dimension + named
  files, durable drift record, `uni explain` narration), GitHub annotations per
  moved file, the published action annotates and writes the job summary, and the
  Stale Evidence Benchmark (100 scenarios; UNI detects 100% of what it can, a
  plain exit-code CI misses 71% of the lost proofs, a cache misses all 70).
- **Review + deploy.** Two-axis review applied (spec honesty, remediation
  branching, finding ids as data, JSON token leak, unobservable subject).
  All tags pushed; releases `v0.3.0` through `v0.8.1` with five assets each,
  `v0.8.1` marked latest. `v0.3.0`/`v0.4.0` tag CI stays red on purpose: those
  versions predate the fixes, which is the honest record.

## Test debt

- `crates/uni-cli/tests/` is the bulk of the suite (golden, security, binding,
  bundle). The three low crates gained unit tests in v0.1; keep that direction
  rather than growing the CLI integration surface.
- Study fixtures are committed intentionally; the harness refuses untracked
  fixture drift.
