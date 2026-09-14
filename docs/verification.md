# Verification

`uni verify <contract>` executes verifiers for every claim and asks the decision
engine. It is incremental: valid, still-bound evidence is reused; only missing,
invalid, or stale evidence re-runs.

## The trust boundary

- The contract (`.uni`) is **untrusted input**: it may only reference verifier names.
- `.uni/config.toml [verifiers]` is the **trusted registry**: it holds the actual commands.
- `.uni/config.toml [identities]` is the **trusted issuer list**: it says whose
  token may prove an actor's identity. Key material is pinned by path, never
  fetched, so the registry stays the only root of trust.
- An inline `shell "cmd"` in a contract is accepted only while the registry is
  empty (bootstrap) or when an identical command already exists in the registry.
  Arbitrary commands from contracts are refused, always.

## Registry formats

Simple (exit code only):

```toml
[verifiers]
"project.check" = "cargo check"
```

Full (all fields optional except `run`):

```toml
[verifiers."mini.t.clamp.upper"]
run = "cargo test clamp_upper_works -- --exact"
expect = "test result: ok. 1 passed"   # substring required in output
expect_not = "EVIL_WRITE"              # substring forbidden in output
files = ["src/**"]                     # content-bound invalidation globs
timeout = 120                          # seconds
max_age_hours = 24                     # time-bound: the proof decays
```

Gotchas verified against real runners: match the reporter the runtime actually
prints (Node's spec reporter emits `ℹ fail 0`, not `# fail 0`), and prefer file
globs over directory args for test runners.

## Trusted issuers (v0.9): how an actor stops being self-declared

An identity is the one thing a flag cannot grant. `--actor ci:build-12` names an
actor, it never proves one, so the ceiling stays A3-D. A3 requires a signature.

Present a JWT in the environment and `uni verify` checks it before anything runs:

```bash
UNI_IDENTITY_TOKEN="$(cat token.jwt)" uni verify delivery.uni
```

Declare which issuers may be believed, and where their keys live:

```toml
[identities."https://accounts.example"]
source = "oidc"                              # oidc | entra | spiffe
jwks_file = ".uni/identity/accounts.jwks.json"
audiences = ["uni-cli"]                      # optional; when set, aud must intersect
algorithms = ["RS256"]                       # optional; default RS256, ES256
```

What is enforced, and what that means in practice:

- The signature is checked against the pinned `jwks_file`, selected by the
  token's `kid`. With no `kid`, only a single-key set is usable: guessing among
  several keys would put key resolution back in the trust path.
- `iss`, `exp` and a non-empty `sub` are always required. `aud` is required only
  when the entry declares `audiences`, and then the token must carry one of
  them; an entry with no `audiences` does not check audience, and says so.
  `exp` is one of the two clock reads in UNI (the other is evidence expiry), and
  like it, it gates availability, never the decision. A leeway of 60s tolerates
  skew.
- An issuer absent from `[identities]` is refused. Nothing is fetched from a
  `jwks_uri` or an OIDC discovery document: a URL is not a root of trust.
- The verified id comes from the token: `oidc:<iss>#<sub>`, `entra:<iss>#<sub>`,
  or the SPIFFE id itself for `source = "spiffe"`, where `sub` must be a
  `spiffe://` URI.
- A token that fails is a hard error, never a quiet fall back to A3-D, and
  `--actor` together with a token is refused rather than silently ignored.
- `entra` and `spiffe` change nothing about verification, only about what you
  declare and what the subject must look like. For Entra, the key is the issuer
  exactly as its tokens carry it (`https://login.microsoftonline.com/<tenant>/v2.0`
  for v2). For SPIFFE, the key is the bare trust domain, because that is what a
  JWT-SVID puts in `iss`, and the subject must be a `spiffe://` URI.

Because the actor id enters the evidence fingerprint, a proof made as a verified
actor is never reused by a run that only declares one. Edit `[identities]` and
the trust-boundary diff says so: those entries are named with an `identity:`
prefix.

## Built-in verifiers v0.1

The shell verifier is the only executor today; every other tool (cargo, npm,
pnpm, bun, pytest, unittest, node --test, playwright) is wired through the
registry as a command. Writing a first-class verifier adapter: see
`writing-verifiers.md`.

## Contract side

```
VERIFY <claim-id>
  USING <registry-key>
```

or one-line `VERIFY <claim-id> USING shell "cmd"`. `uni lint <contract>` checks
coverage and unknown registry keys without executing anything.

Hand the worker the requirements explicitly with `uni brief` (see
[brief.md](brief.md)): the study showed that leaving the registry-to-test-name
hop implicit makes correct work fail verification.

## Selector templates (v0.5): the worker names the test, the human authorizes it

The study measured the dominant defect: a contract pins a test name, the worker
picks a different one, and correct work is rejected. A registry command may
therefore defer the selector:

```toml
[verifiers."suite"]
run = "cargo test {{selector}} -- --exact"
expect = "test result: ok. 1 passed"
```

```
VERIFY clamp-upper
  USING suite
  REQUIRE behavior("clamps values above the upper bound")
```

Then the flow is: the worker writes the test and picks a clear name; a human
authorizes that exact name; only then does it count as evidence.

```bash
uni bind --claim clamp-upper --verifier suite \
  --requirement 'behavior("clamps values above the upper bound")' \
  --selector clamp_upper_works
```

Authorize one claim, or a whole reviewed file in one act:

```bash
uni bind --claim clamp-upper --verifier suite --selector clamp_upper_works
uni bind --from reviewed.toml     # every entry, one command, one review
uni bindings                      # what is currently authorized
```

`reviewed.toml` lists `[bindings.<claim>]` entries with `verifier`, optional
`requirement`, and optional `selector`; the act stamps each one with who and
when, so fifty claims are one reviewable diff and one command instead of fifty
files. Bindings live in `.uni/bindings.toml`; legacy per-claim
`.uni/bindings/<claim>.json` files still load, and a claim mentioned in the file
wins.

Rules: a template without an authorized selector is a hard error naming the
`uni bind` command, and `uni lint` warns before anything runs. The selector is
part of the authorization: re-binding to a different selector invalidates the
earlier evidence. Two selectors on the same key are different proofs and never
share a cache entry. Nothing here lets a worker mint its own evidence: an
authorized selector is still run by a trusted verifier.

## Authorized resolution (VerifierBinding, v0.2)

A verification may carry a resolution requirement:

```
VERIFY clamp-upper
  USING test-suite
  REQUIRE behavior("clamps values above upper bound")
```

Such a verification executes ONLY under a matching authorized binding
(`.uni/bindings/<claim>.json` with identical claim, verifier, and requirement
text). Authorize explicitly: this is the human act AI proposals cannot replace.

```bash
uni bind --claim clamp-upper --verifier test-suite --requirement 'behavior("clamps values above upper bound")'
uni bindings   # list; every bind is journaled as BindingAuthorized
```

Rules: no binding (or a binding for different text/verifier) = hard error naming
the exact `uni bind` command, never a silent run. Re-authorization replaces the
binding; evidence gathered under the old `binding_hash` goes stale and renews.
`uni lint` warns on requirements without matching bindings. Claim -> requirement
-> binding -> evidence: AI proposes, trusted configuration authorizes, the
verifier proves.
