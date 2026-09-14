# UNI DSL v0.1 - closed vocabulary

`VERSION DOMAIN INTENT GOAL CLAIM REQUIRE ENSURE INVARIANT FORBID VERIFY ACCEPT REJECT ESCALATE`

```
VERSION 0.1
DOMAIN software
INTENT booking.cancel

GOAL
  Cancel a confirmed booking safely.

CLAIM booking-state REQUIRED
  ENSURE booking.status == cancelled
INVARIANT ledger-consistency CRITICAL
  ENSURE ledger.balance == expected.balance
FORBID
  direct_write("ledger")

VERIFY booking-state
  USING mini.t.booking
ACCEPT WHEN
  required_claims == VERIFIED
  AND critical_failures == 0
```

- CLAIM: verifiable statement of the outcome. REQUIRED (default) or OPTIONAL.
- INVARIANT: claim that must never break; CRITICAL marks rejection (vs. evidence gap).
- FORBID: prohibition compiled to a critical invariant claim; prove it with an `expect_not` verifier.
- REQUIRE / REJECT WHEN / ESCALATE WHEN: REQUIRE attaches a resolution
  requirement to the preceding VERIFY (`REQUIRE behavior("...")`, blank lines
  allowed, nothing else in between); REJECT WHEN / ESCALATE WHEN stay reserved
  hard errors.
- VERIFY <claim> USING <registry-key> | shell "cmd" - commands run ONLY from
  `.uni/config.toml [verifiers]` (trusted registry). Expect/expect_not/files/timeout fields per verifier.
- ACCEPT WHEN is deterministic; LLMs may draft contracts (human approves candidates).

Decision states: ACCEPTED, REJECTED, EVIDENCE_REQUIRED, ESCALATED. Evidence states: Valid, Invalid, Stale.
