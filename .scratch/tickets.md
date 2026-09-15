# UNI — tickets

Current: **v0.9.1 + 2026-09-15 work (committed, pushed, unreleased)** · 134
tests, 0 warnings · public repo `github.com/guillaume-flambard/uni-protocol` ·
CI green on ubuntu/macOS/Windows + POSIX examples + a smoke job that runs the
published action · release `v0.9.1` with five binaries. Since `v0.9.1`:
ADR-002 + lint warning, the live A3 workflow, the Google example, the
second-model study arm. Those want a `v0.9.2` tag.

## Done — 2026-09-15 (committed, pushed as `ac6701d..272fdc5`)

- **Base re-verified.** `cargo test` 134/134 green with
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (plain `git`/`cargo`
  fail: Xcode license not accepted, `sudo xcodebuild -license` still pending).
  Release binary rebuilt (`uni 0.9.1`); `verify` + `explain` on
  `examples/hello` green.
- **A3 against a real issuer: CLOSED, live.** GitHub Actions mints a real OIDC
  token; `examples/identity-github-actions/` pins GitHub's real JWKS (4 RS256
  keys, snapshot 2026-09-15); `.github/workflows/identity-live.yml` requires
  `assurance == A3` and proves the refusal too (same token, issuer undeclared,
  refused). Run `34939375852` reached A3. No secret, no human step, no network
  call in the verify path. Sibling `examples/identity-google/` keeps the
  workforce-identity shape (`gcloud`, one human step, `gcloud` absent here).
- **Second-model arm: UNBLOCKED and run.** `bai/*` refused, `opencode/*`
  billed or no-op; the free OpenRouter tier
  (`openrouter/cohere/north-mini-code:free`, $0, real tool use, write-probed
  before use) carried it. n=10 no-brief: FAR 0%, FRR 30%, all 3 false rejects
  are test-name coupling. Brief arm t03/t07/t08: a `brief.md` merely present is
  ignored 3/3, named in the prompt it is followed 3/3 (substance reviewed).
- **Test-name tension DECIDED (ADR-002).** Pinned test names are a contract
  smell; `{{selector}}` templates + `uni bind --selector` + `uni brief` as the
  handoff are the blessed pattern. Enforced as a `uni lint` warning
  (`pinned-test-selector`, whole-token cargo/node/unittest shapes,
  warning-only so the decision engine is untouched): `pinned_test_selector()` +
  unit test in `uni-verify`, lint wiring in `cmd/contract.rs`, golden test in
  `cli_golden.rs`. Explicitly NOT done: auto-emitting `brief.md` in `uni run`
  (behaviour change, needs review).
- **Hygiene.** `.gitignore` now covers `.uni` runtime state at any depth (the
  example's evidence/decisions had been tracked by a root-anchored pattern);
  report, README, `docs/index.md` and `docs/verification.md` updated to 134
  tests and to the live A3 result.

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

## Done — identity adapters: A3 is reachable (v0.9.0, corrected in v0.9.1)

A proof's actor identity is verified, not merely named.

- **The adapter.** `UNI_IDENTITY_TOKEN=<jwt> uni verify` verifies a JWT offline
  against an issuer pinned in `.uni/config.toml` `[identities]` (`source` is
  oidc/entra/spiffe, `jwks_file`, optional `audiences`/`algorithms`). Signature
  via the pinned JWKS, `iss`/`aud`/`exp` checked, `kid` selects the key.
  Success mints an actor with assurance `verified`, which the decision engine
  already turns into A3 when the actor is independent. `--actor` stays
  self-declared (A3-D), a bad token is a hard error (never a silent downgrade),
  and `--actor` together with a token is refused.
- **Trust root unchanged.** The JWKS lives beside the registry, not behind a
  network call: the registry stays the only root of trust, and a token whose
  `iss` is not declared there is refused.
- **The trust boundary names an issuer edit too.** `registry_diff` now diffs
  `[identities]` as well as `[verifiers]`, entries prefixed `identity:`, so an
  issuer-only registry change reports what moved instead of an empty diff.
- **Proven end to end.** `crates/uni-cli/tests/cli_identity.rs` (A3, A3-D, A2,
  mutual exclusion, expired/untrusted refused, issuer change named) plus the
  adapter unit tests (wrong issuer, tampering, audience, spiffe subject,
  half-wired config).
- **Constitution v0.7.** Rules 4, 10 and 11 amended with a migration note:
  `[identities]` joins the trusted registry, a token's `exp` sits in the same
  availability class as evidence expiry (neither enters the decision), and A3
  states how it is reached. No DSL keyword, evidence field, or decision path
  changed.

## Open

Ordered by value. Nothing here is started unless marked.

1. **Real-repo study tasks** — the toy family is exhausted (FAR 0% everywhere,
   FRR fully explained by test naming). Needs tasks that are multi-file, carry
   a declared invariant, and where plausible-but-wrong is the norm; same
   harness, both models (the free OpenRouter implementer still works).
2. **`uni run` and the work order** — the brief arm showed the handoff must
   *invoke* the brief, not merely emit it. Decide (behaviour change on a
   shipped command, needs review) whether `uni run` generates `brief.md` and
   names it in the executor's prompt.
3. **Migrate to selector templates** — study and dogfood registries still pin
   literal test names, so they now lint with `pinned-test-selector` warnings.
   Mechanical, do it as each file is touched.
4. **Release `v0.9.2`** — ADR-002 + lint warning, live A3 workflow, Google
   example, second-model results. Tag + five assets; bump the published
   action's default from `v0.9.1` if the binary changed behaviour (it did:
   lint warns).
5. **A3 workforce example, live** — the Google example still needs a
   human-minted token (`gcloud` is not installed). Low value now that GitHub
   OIDC proves the path live.
6. **Cloud / org** — organizations, dashboards, `cost per accepted outcome`.
   Deliberately after the single-user story is convincing.
7. **Vault note** — `1-Projects/uni.md` does not exist; `PROJECTS.md` line is
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
  cannot extend it, and an expired proof is stale (the only clock read inside
  the evidence context, constitution rule 10). The journal rotates past 1 MiB keeping the
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
  All tags pushed; releases `v0.3.0` through `v0.9.1` with five assets each,
  `v0.9.1` marked latest. `v0.3.0`/`v0.4.0` tag CI stays red on purpose: those
  versions predate the fixes, which is the honest record.

## Test debt

- `crates/uni-cli/tests/` is the bulk of the suite (golden, security, binding,
  bundle). The three low crates gained unit tests in v0.1; keep that direction
  rather than growing the CLI integration surface.
- Study fixtures are committed intentionally; the harness refuses untracked
  fixture drift.
