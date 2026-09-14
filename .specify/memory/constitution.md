<!-- CANONICAL SOURCE: /constitution.md (single source of truth, no spec drift).
     This file is the Spec Kit copy for the constitution-amendment flow; update both together. -->

# UNI Constitution v0.7

## Core Principles

### I. Outcome Assurance layer
UNI is the Outcome Assurance layer: Intent -> Claims -> Evidence -> Policy -> Decision. Never a control plane, framework, or trust protocol.

### II. Separation of powers
LLMs propose. Humans/policies authorize. Verifiers prove. UNI decides.

### III. Determinism
The decision engine is deterministic for identical (contract, evidence, policies).

### IV. Trust boundary
Contracts are untrusted declarations. Only `.uni/config.toml` is trusted: `[verifiers]` for what may run, `[identities]` for whose token may be believed. No inline `curl|bash` escapes the registry, and no identity is trusted from a URL fetched at runtime: key material is pinned by path beside the registry.

### V. Closed vocabulary
VERSION DOMAIN INTENT GOAL CLAIM REQUIRE ENSURE INVARIANT FORBID VERIFY ACCEPT REJECT ESCALATE. REQUIRE attaches a resolution requirement to its VERIFY and executes only under an authorized VerifierBinding; REJECT WHEN and ESCALATE WHEN stay reserved hard errors. A spec must never promise inert semantics.

### VI. Dogfood rule
Every ticket ends with `uni verify + uni explain`, never with "agent says done".

### VII. Single spec source
Spec Kit is the canonical spec source. Matt skills cut and execute. UNI assures. One source of truth, no spec duplication.

### VIII. Filesystem core
v0.1 ships filesystem-only (.uni/), JSON Schema canonical. No Postgres, no Protobuf, no MCP/A2A in core.

### IX. Evidence Completeness Principle
A claim MUST NOT be accepted from evidence whose identity, scope or observed subject does not fully cover the property being asserted. Partial stdout, partial filesystem, wrong commit, wrong environment, wrong contract, wrong verifier version, expired evidence: all invalidate.

### X. Verification Context
Evidence validity = Contract x Subject x Verifier x VerifierConfig x Environment x Policy x Time. Drift on contract, subject, verifier, config, or environment forces re-run; policy drift forces decision recompute; an expired proof is stale and re-established. Time is the only evidence-validity dimension read from the clock and decides availability, not the decision; a token's `exp` is that same availability check applied to an identity. A changed registry names its diff (over `[verifiers]` and `[identities]`) and requires human review.

### XI. Assurance scale
A2 trusted verifier; A3-D independent actor with self-declared identity; A3 independent actor with externally verified identity, reached only by a JWT verified against an issuer pinned in `[identities]`; A4 signed provenance (reserved). Independence and identity assurance are distinct: a flag is a declaration, never a proof.

### XII. VerifierBinding
A resolution requirement executes only under a human-authorized binding (claim x verifier x requirement text x selector, hashed). AI may propose bindings; only `uni bind` authorizes. Re-authorization replaces; old proofs stale. A template selector defers the test name to the worker and the authorization to the human.

## Governance

Constitution supersedes all other practices. Amendments require documentation and a migration note. Inert syntax is a hard error, never a silent no-op.

### Amendment v0.7 (2026-09-14): identity adapters (release v0.9.0)

IV now names `[identities]` part of the trusted registry and forbids trusting a
key fetched at runtime; X scopes the clock to evidence validity and places a
token's `exp` in the same availability class; XI states that A3 is reached by a
JWT verified against a pinned issuer.

Migration: none required. No DSL keyword, evidence field, or decision path
changed. A verify that presents no token behaves byte-identically to v0.8, and a
scheme prefix on `--actor` stays a self-declared A3-D declaration.

**Version**: 0.7.0 | **Ratified**: 2026-09-13 | **Last Amended**: 2026-09-14
