# UNI Constitution v0.1

1. UNI is the Outcome Assurance layer: Intent → Claims → Evidence → Policy → Decision. Never a control plane, framework, or trust protocol.
2. LLMs propose. Humans/policies authorize. Verifiers prove. UNI decides.
3. The decision engine is deterministic for identical (contract, evidence, policies).
4. Contracts are untrusted declarations. Only `.uni/config.toml [verifiers]` is trusted. No inline `curl|bash` escapes the registry.
5. Closed vocabulary v0.1: VERSION DOMAIN INTENT GOAL CLAIM REQUIRE ENSURE INVARIANT FORBID VERIFY ACCEPT REJECT ESCALATE. REQUIRE, REJECT WHEN and ESCALATE WHEN are reserved for v0.2 and rejected as hard errors until then; a spec must never promise inert semantics.
6. Every ticket ends with `uni verify + uni explain`, never with "agent says done".
7. Spec Kit is the canonical spec source. Matt skills cut and execute. UNI assures. One source of truth, no spec duplication.
8. v0.1 ships filesystem-only (.uni/), JSON Schema canonical. No Postgres, no Protobuf, no MCP/A2A in core.
9. Evidence Completeness Principle: a claim MUST NOT be accepted from evidence whose identity, scope or observed subject does not fully cover the property being asserted. Partial stdout, partial filesystem, wrong commit, wrong environment, wrong contract, wrong verifier version, expired evidence: all invalidate.
