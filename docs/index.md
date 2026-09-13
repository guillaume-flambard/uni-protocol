# UNI docs (Developer Preview v0.1)

| Doc | Contents |
|---|---|
| [why-uni.md](why-uni.md) | the category: Outcome Assurance, where UNI sits, what it is not |
| [quickstart.md](quickstart.md) | 60-second loop: init, lint, verify, report |
| [language.md](language.md) | the closed `.uni` vocabulary v0.1 |
| [claims.md](claims.md) | CLAIM / INVARIANT / FORBID / REQUIRE semantics |
| [evidence.md](evidence.md) | evidence shape, states, git + content invalidation |
| [verification.md](verification.md) | trusted registry, expect matchers, the trust boundary |
| [decisions.md](decisions.md) | truth table, policy layer, exit-code semantics |
| [github-integration.md](github-integration.md) | PR check, step summary, stable report |
| [writing-verifiers.md](writing-verifiers.md) | registering commands vs writing adapters |
| [specification.md](specification.md) | grammar, canonical IR, behavior contract |

Everything here describes the shipped binary (`cargo build --release`,
`./target/release/uni`). When in doubt, run it: what executes beats what a doc
describes. Constitution: `../constitution.md`. Spec source:
`../specs/001-uni-software-v01/spec.md`.
