<!-- CANONICAL SOURCE: /constitution.md (single source of truth, no spec drift).
     This file is the Spec Kit copy for the constitution-amendment flow; update both together. -->

# UNI Constitution

## Core Principles

### I. Outcome Assurance layer
UNI is the Outcome Assurance layer: Intent -> Claims -> Evidence -> Policy -> Decision. Never a control plane, framework, or trust protocol.

### II. Separation of powers
LLMs propose. Humans/policies authorize. Verifiers prove. UNI decides.

### III. Determinism
The decision engine is deterministic for identical (contract, evidence, policies).

### IV. Trust boundary
Contracts are untrusted declarations. Only `.uni/config.toml [verifiers]` is trusted. No inline `curl|bash` escapes the registry.

### V. Closed vocabulary v0.1
VERSION DOMAIN INTENT GOAL CLAIM REQUIRE ENSURE INVARIANT FORBID VERIFY ACCEPT REJECT ESCALATE. REQUIRE, REJECT WHEN and ESCALATE WHEN are reserved for v0.2 and rejected as hard errors until then; a spec must never promise inert semantics.

### VI. Dogfood rule
Every ticket ends with `uni verify + uni explain`, never with "agent says done".

### VII. Single spec source
Spec Kit is the canonical spec source. Matt skills cut and execute. UNI assures. One source of truth, no spec duplication.

### VIII. Filesystem core
v0.1 ships filesystem-only (.uni/), JSON Schema canonical. No Postgres, no Protobuf, no MCP/A2A in core.

### IX. Evidence Completeness Principle
A claim MUST NOT be accepted from evidence whose identity, scope or observed subject does not fully cover the property being asserted. Partial stdout, partial filesystem, wrong commit, wrong environment, wrong contract, wrong verifier version, expired evidence: all invalidate.

## Governance

Constitution supersedes all other practices. Amendments require documentation and a migration note. Inert syntax is a hard error, never a silent no-op.

**Version**: 0.1.0 | **Ratified**: 2026-09-13 | **Last Amended**: 2026-09-14
