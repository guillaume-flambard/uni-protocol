# Writing verifiers

v0.1 ships one executor (shell) plus the trusted registry. Two extension paths:

## 1. Register a command (no code, recommended default)

Any tool with a meaningful exit code or output already works:

```toml
[verifiers."npm.test-auth"]
run = "npm test -- --run auth"
expect = "Tests  12 passed"
files = ["src/auth/**"]
timeout = 300

[verifiers."no-secrets"]
run = "! grep -RIn 'AKIA[0-9A-Z]\\{16\\}' ."
expect_not = "AKIA"
```

Design the expectation on the tool's real stdout (capture a run first; the
reporter format is part of the contract, see the Node `ℹ fail 0` gotcha in
`verification.md`).

## 2. Write a code adapter (when exit codes are not enough)

The Verifier seam (planned v0.2+ as a trait in `uni-verify`, mirroring the
Execution/Verification interfaces in the blueprint):

```rust
pub trait Verifier {
    fn supports(&self, claim: &Claim) -> bool;
    fn verify(&self, input: &VerificationInput) -> Result<Evidence>;
}
```

An adapter must:

1. Produce `Evidence` with every binding it relied on filled
   (`commit_sha`, `workspace_dirty`, `artifact_hash` for watched files).
2. Map "the tool said pass" to `state`, never to a custom truthiness.
3. Be reproducible: no network, no clock, no RNG (the determinism rule).
4. Register in `.uni/config.toml` only, so contracts stay declarative.

Candidates for first-class adapters (ranked by v0.1 demand): git diff range,
Semgrep/CodeQL findings, Playwright report parser, Lighthouse budget, coverage
ratio with claim-specific test selection.

## Test the adapter at its seam

Given a fixture workspace + a fixed contract, assert the produced
`Evidence.state` and the final `Decision`. External behavior only, never
internals (same rule the core follows: test `assure()`, not the parser).
