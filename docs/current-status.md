# Current status

Last checked: 2026-09-28. Developer Preview, latest published release v0.9.4.

This page is the short source of truth for the public repository. Dated reports
under `docs/` and `experiments/` remain useful evidence, but they describe the
revision and experiment named in each file.

## Shipped

- One Rust CLI, `uni`, with a filesystem-backed `.uni/` workspace.
- Contract compilation, linting, verification, explanation, stable reports,
  work orders, execution handoff, bindings, bundles, events, and health checks.
- A trusted verifier registry with shell commands and a command-free file-hash
  adapter.
- Evidence bound to contract, revision, watched content, verifier
  configuration, platform, authorization, policy, and optional expiry.
- Deterministic decisions for identical contract, evidence, and policy inputs.
- Offline JWT verification against issuer keys pinned by path in the trusted
  registry.
- Source builds on Ubuntu, macOS, and Windows. Release assets cover five target
  triples.

The current `main` CI runs formatting, Clippy with warnings denied, the Rust
test suite, every shipped contract, examples across Rust, Python, Node, shell,
and file hashes, plus the published action on Linux, macOS, and Windows.

## Measured

- The stale-evidence benchmark contains 100 manufactured drift scenarios. Its
  report separates detectable scenarios from cases no declared subject could
  observe.
- The corrected real-agent study has n=6, with no false accepts and one false
  reject. It did not support the claim that UNI beats an honest agent's
  self-report. An older self-report column was invalid because the harness
  asserted DONE on the agent's behalf; those rows cannot be repaired.
- Two scripted plausible deliveries keep their visible behavior green while
  violating a hidden owner invariant. UNI rejects both. These fixtures prove
  the verification mechanism, not a failure rate for coding models.
- The same two tasks have a scripted post-verification drift arm. Evidence from
  the earlier revision becomes stale, the delivered revision is re-verified,
  and both incorrect deliveries are rejected.
- The scheduled identity workflow obtains a real GitHub OIDC token and requires
  A3. It also proves that the same token is refused when its issuer is absent
  from the registry.

Source reports:

- [Stale evidence benchmark](../experiments/stale-bench/RESULTS-2026-09-14.md)
- [Reviewed agent study](../experiments/study-50/RESULTS-2026-09-14.md)
- [Scripted adversarial arm](../experiments/study-50/RESULTS-2026-09-19-adversarial-arm.md)
- [Post-verification drift arm](../experiments/study-50/RESULTS-2026-09-28-drift-arm.md)
- [Live identity workflow](../.github/workflows/identity-live.yml)

## Known limits

- This is a developer preview, not a stable 1.0 protocol.
- The shell verifier is the only command executor. File hashes are the only
  command-free built-in adapter.
- The core is local and filesystem-only. It has no hosted service, organization
  accounts, or multi-user control plane.
- A contract declares claims and verifier names. Human review still decides the
  contract and trusted registry.
- A verifier can prove only what it observes. Untracked content outside declared
  `files` globs does not invalidate evidence by itself.
- The committed adversarial and drift arms are scripted. They show the checker
  behaving correctly on those fixtures, not how often a real model fails.
- The v0.9.4 Windows release asset is a ZIP payload with a `.tar.gz` name. The
  action on `main` detects that legacy asset. The immutable v0.9.4 action tag
  still needs a successor release before Windows users can pin the corrected
  action by version.

## Release and repository boundaries

The release page, repository rules, vulnerability-reporting setting, dependency
alerts, code scanning, topics, and social preview are GitHub state. They are not
changed by an in-repository documentation commit. `SECURITY.md` describes the
reporting route that is actually available today.

The historical [September 2026 report](REPORT-2026-09-14.md) is retained for
traceability. It must not be used as the current project status.
