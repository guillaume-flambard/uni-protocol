# UNI Constitution v0.6

1. UNI is the Outcome Assurance layer: Intent → Claims → Evidence → Policy → Decision. Never a control plane, framework, or trust protocol.
2. LLMs propose. Humans/policies authorize. Verifiers prove. UNI decides.
3. The decision engine is deterministic for identical (contract, evidence, policies).
4. Contracts are untrusted declarations. Only `.uni/config.toml [verifiers]` is trusted. No inline `curl|bash` escapes the registry.
5. Closed vocabulary: VERSION DOMAIN INTENT GOAL CLAIM REQUIRE ENSURE INVARIANT FORBID VERIFY ACCEPT REJECT ESCALATE. REQUIRE attaches a resolution requirement to its VERIFY and executes only under an authorized VerifierBinding; REJECT WHEN and ESCALATE WHEN stay reserved hard errors. A spec must never promise inert semantics.
6. Every ticket ends with `uni verify + uni explain`, never with "agent says done".
7. Spec Kit is the canonical spec source. Matt skills cut and execute. UNI assures. One source of truth, no spec duplication.
8. v0.2 ships filesystem-only (.uni/), JSON Schema canonical. No Postgres, no Protobuf, no MCP/A2A in core.
9. Evidence Completeness Principle: a claim MUST NOT be accepted from evidence whose identity, scope or observed subject does not fully cover the property being asserted. Partial stdout, partial filesystem, wrong commit, wrong environment, wrong contract, wrong verifier version, expired evidence: all invalidate.
10. Verification Context: evidence validity = Contract × Subject × Verifier × VerifierConfig × Environment × Policy × Time. Drift on contract, subject, verifier, config, or environment forces re-run; policy drift forces decision recompute; an expired proof (`max_age_hours` on a verifier) is stale and is re-established. Time is the one dimension read from the clock, and it decides availability, not the decision itself. A changed registry never silently reuses old evidence: it names the diff (trust-boundary diff) and requires human review.
11. Assurance scale: A2 trusted verifier; A3-D independent actor with self-declared identity; A3 independent actor with externally verified identity; A4 signed provenance (reserved, no producer yet). Independence and identity assurance are distinct properties: a `--actor` flag is a declaration, never a proof.
12. VerifierBinding: a claim's resolution requirement executes only under a human-authorized binding (claim × verifier × requirement text × selector, hashed). AI may propose bindings; only `uni bind` authorizes. Re-authorization replaces; old proofs stale. A `{{selector}}` template defers the test name to the worker and the authorization to the human: a worker never mints its own evidence.
