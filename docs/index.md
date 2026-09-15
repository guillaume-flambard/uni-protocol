# UNI docs (Developer Preview v0.9)

| Doc | Contents |
|---|---|
| [why-uni.md](why-uni.md) | the category: Outcome Assurance, where UNI sits, what it is not |
| [quickstart.md](quickstart.md) | 60-second loop: init, lint, verify, report |
| [language.md](language.md) | the closed `.uni` vocabulary v0.1 |
| [claims.md](claims.md) | CLAIM / INVARIANT / FORBID / REQUIRE semantics |
| [evidence.md](evidence.md) | evidence shape, states, git + content invalidation |
| [verification.md](verification.md) | trusted registry, expect matchers, the trust boundary |
| [decisions.md](decisions.md) | truth table, policy layer, exit-code semantics, the assurance scale and how A3 is earned |
| [github-integration.md](github-integration.md) | PR check, step summary, stable report |
| [writing-verifiers.md](writing-verifiers.md) | registering commands vs writing adapters |
| [brief.md](brief.md) | the work order handed to an implementing agent |
| [run.md](run.md) | the execute half: run your command, then verify |
| [specification.md](specification.md) | grammar, canonical IR, behavior contract |
| [REPORT-2026-09-14.md](REPORT-2026-09-14.md) | **complete report**: thesis, architecture, capabilities, study, what is proven |
| [DEEP-ANALYSIS-2026-09-14.md](DEEP-ANALYSIS-2026-09-14.md) | earlier audit at v0.18, superseded; kept as the record of how two real bugs were found |

Everything here describes the shipped binary (`cargo build --release`,
`./target/release/uni`). When in doubt, run it: what executes beats what a doc
describes. Constitution: `../constitution.md`. Spec source:
`../specs/001-uni-software-v01/spec.md`.

## Stale evidence

- [The check, in the pull request](flagship-check.md)
- [Why a proof stopped applying](decisions.md#why-a-proof-stopped-applying)
- [Stale Evidence Benchmark, 2026-09-14](../experiments/stale-bench/RESULTS-2026-09-14.md)

## Verified identity (A3)

- [The assurance scale, and how a token earns A3](decisions.md)
- [End-to-end proof, run against the built binary](../crates/uni-cli/tests/cli_identity.rs)
- [Registry keys for `[identities]`](verification.md)
- [**Live, against a real issuer**: GitHub Actions OIDC, run unattended](../examples/identity-github-actions/README.md)
  (`.github/workflows/identity-live.yml`; run `34939375852` reached A3)
- [Worked example: Google, needs a human-minted token](../examples/identity-google/README.md)
