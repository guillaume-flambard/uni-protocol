# Feature 002 - expect_not matcher + FORBID enforcement (v0.7)

### Requirements
- **FR-101**: `[verifiers."x"]` accepts an optional `expect_not` field.
- **FR-102**: Evidence produced when output contains `expect_not` is INVALID, even with exit 0.

### Acceptance Scenarios
#### Scenario: forbidden write pattern detected
- GIVEN registry run = "grep -Rn 'todo' src" with expect_not = "todo"
- WHEN source contains 'todo' → claim INVALID, decision EvidenceRequired/Rejected per criticality.
#### Scenario: clean tree passes
- WHEN no match → exit 1 from grep? No: run "! grep …" (exit 0 when clean) + expect_not double-guard.

### Out of scope
Forbid DSL semantics beyond claim+verifier mapping (no AST-leveltei promises until v1).
