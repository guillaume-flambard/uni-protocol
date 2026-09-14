# Claims

A claim is a verifiable statement about the outcome, written in the `.uni` DSL
(see `language.md`). UNI compiles claims into the canonical IR and refuses
acceptance until every required claim is backed by valid evidence.

## Kinds

- `CLAIM <id> REQUIRED | OPTIONAL` - a positive statement of the outcome.
  `ENSURE <text>` gives the human-readable obligation (v0.1: `ensure` is
  documentation; verification lives in `VERIFY`).
- `INVARIANT <id> CRITICAL` - a statement that must never break. A critical
  claim with Invalid evidence yields REJECTED, not merely EVIDENCE_REQUIRED.
- `FORBID <expr>` - compiled to a critical invariant claim
  (`forbid-N`, ensure `FORBID <expr>`). Prove it with an `expect_not` verifier
  (see `verification.md` and `examples/forbid/`).
- `REQUIRE <expr>` - reserved for v0.2 (VerifierBinding); hard parse error in v0.1.

## Example

```
CLAIM booking-state REQUIRED
  ENSURE booking.status == cancelled
INVARIANT ledger-consistency CRITICAL
  ENSURE ledger.balance == expected.balance
```

## Rules

- Every claim must have at least one `VERIFY` (else `uni lint` fails preflight).
- Claim ids are unique per contract; unknown ids in `VERIFY` are parse errors.
- `OPTIONAL` claims can lack evidence without blocking acceptance.
- Criticality comes from the claim, not from the verifier.

## Evidence states a claim can end in

Valid (proven), Invalid (disproven or expectation miss), Stale (bound commit or
watched content changed; counts as needing re-verification). See `evidence.md`.
