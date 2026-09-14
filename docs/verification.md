# Verification

`uni verify <contract>` executes verifiers for every claim and asks the decision
engine. It is incremental: valid, still-bound evidence is reused; only missing,
invalid, or stale evidence re-runs.

## The trust boundary

- The contract (`.uni`) is **untrusted input**: it may only reference verifier names.
- `.uni/config.toml [verifiers]` is the **trusted registry**: it holds the actual commands.
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
```

Gotchas verified against real runners: match the reporter the runtime actually
prints (Node's spec reporter emits `ℹ fail 0`, not `# fail 0`), and prefer file
globs over directory args for test runners.

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

## Authorized resolution (VerifierBinding, v0.2)

A verification may carry a resolution requirement:

```
VERIFY clamp-upper
  USING test-suite
  REQUIRE behavior("clamps values above upper bound")
```

Such a verification executes ONLY under a matching authorized binding
(`.uni/bindings/<claim>.json` with identical claim, verifier, and requirement
text). Authorize explicitly — this is the human act AI proposals cannot replace:

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
