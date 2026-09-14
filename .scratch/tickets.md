# UNI — tickets

Current: **v0.6.0** (latest release, `uni 0.5.2`) · 97 tests, 0 warnings ·
public repo `github.com/guillaume-flambard/uni-protocol` · CI green on
ubuntu/macOS/Windows + POSIX examples + a smoke job that runs the published
action.

## Done — evidence lifecycle completed (v0.6.0)

The two gaps left in the Verification Context are closed:

- **Time is real.** A verifier may declare `max_age_hours`; the expiry is stamped
  on the evidence so it travels with the proof and a registry change cannot
  extend it. An expired proof is stale, the verifier re-runs, a fresh window is
  stamped. This is the one clock-reading predicate, and it decides availability,
  not the decision (documented in `docs/decisions.md`, constitution rule 10).
- **The journal rotates.** Past 1 MiB the current journal is archived as
  `events.<timestamp>.jsonl`, keeping the three newest; `uni events --all` reads
  the history and `uni doctor` reports size and cap. The retention predicate
  excludes the live journal: a test caught a version that would have pruned it.

104 tests. Remaining gap in this area: none known.

## Done — CI hosting decision (2026-09-14)

Briefly self-hosted, then reverted: the repository is public, so GitHub-hosted
runners are free and unlimited, and the only gain was ~40 s per push. That is
not worth executing every dependency's build scripts on the production host.
lab-infra PR #68 added the runner and PR #69 removed it (runner unregistered,
service and user deleted). Hosted is the choice for public repos; self-hosting
stays right for the private ones, where minutes are billed and images must be
built beside the local registry.

## Open

Ordered by value. Nothing here is started unless marked.

1. **Second-model study arm** — BLOCKED externally. Every `bai/*` model is
   refused (credit/deposit), and the `opencode/*` "free" models are headless
   no-ops (they print the session header and exit). The harness is ready:
   `UNI_AGENT_MODEL=<model> python3 run.py --agent opencode --only <task>`.
   Needs: one working implementer, then the same 15-task protocol.
2. **Contract-level test binding without a registry hop** — PARTIAL. Selector
   templates (`{{selector}}`) plus `uni bind --selector` close the false
   rejection the study measured, but the binding still lives per claim and is
   authorized by hand. Open question: is a per-claim binding the right
   granularity for a repo with hundreds of claims, or does it need a
   suite-level binding?
3. **A3 real** — the assurance scale reaches A3-D (independent actor,
   self-declared identity). A3 needs identity adapters (spiffe/entra/oidc);
   they are stubs today, so A3 is unreachable outside unit tests. A4 stays
   refused by design (`--attest` names the missing signer).
4. **Execution** — no `uni run`, no execution provider. The blueprint's
   Codex/Claude/Temporal adapters are unbuilt; UNI only verifies what is
   already on disk.
5. **Observability export** — events carry `uni.*` attributes but nothing
   exports them. An OTel exporter is a small, self-contained slice.
6. **Evidence lifecycle gaps** — event journal has no rotation; evidence has no
   expiry (the Time dimension is recorded, not enforced).
7. **`crates/uni-cli/src/main.rs`** — all commands plus helpers in one file
   (~1000 lines). Split before the cloud phase.
8. **Cloud / org** — organizations, dashboards, `cost per accepted outcome`.
   Deliberately after the single-user story is convincing.
9. **Vault note** — `1-Projects/uni.md` does not exist; `PROJECTS.md` line is
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
- **Review + deploy.** Two-axis review applied (spec honesty, remediation
  branching, finding ids as data, JSON token leak, unobservable subject).
  All tags pushed; releases `v0.3.0` through `v0.5.2` with five assets each,
  `v0.5.2` marked latest. `v0.3.0`/`v0.4.0` tag CI stays red on purpose: those
  versions predate the fixes, which is the honest record.

## Test debt

- `crates/uni-cli/tests/` is the bulk of the suite (golden, security, binding,
  bundle). The three low crates gained unit tests in v0.1; keep that direction
  rather than growing the CLI integration surface.
- Study fixtures are committed intentionally; the harness refuses untracked
  fixture drift.
