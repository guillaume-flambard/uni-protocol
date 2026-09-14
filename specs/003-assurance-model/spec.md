# Feature 003 - UNI Assurance Model v0.2

### Requirements
- **FR-201**: Evidence records its Verification Context (contract/registry/policy hashes, platform, actor, executor, binding hash).
- **FR-202**: Drift on contract, subject, verifier, config, or environment forces re-run; policy drift forces decision recompute without re-run.
- **FR-203**: A changed registry names its diff (added/removed/changed verifiers), emits RegistryChanged, flags `trust_boundary_changed` for CI, and requires human review; old evidence goes stale, never reused.
- **FR-204**: Assurance derives from the evidence graph: A2 default ceiling; A3-D iff every proof's actor differs from the executor with self-declared identities; A3 iff all identities externally verified; A4 refused (no signer).
- **FR-205**: Independence and identity assurance are tracked and displayed as distinct properties; `--actor x` can never mint A3.
- **FR-206**: A verification carrying REQUIRE executes only under a matching authorized VerifierBinding (`uni bind`); mismatch or absence is a hard error; re-authorization stales old proofs.

### Acceptance Scenarios
#### Scenario: registry change is named and flagged
- GIVEN an acknowledged registry, WHEN a verifier is added and committed, THEN verify prints REGISTRY_CHANGED naming it, sets trust_boundary_changed on that run only, and re-runs (exit stays decision-driven).
#### Scenario: actor separation caps honestly
- GIVEN default verify THEN report shows A2/independent NO; GIVEN --actor ci:build-12 THEN A3-D/independent YES/identity SELF-DECLARED; GIVEN --attest THEN hard refusal.
#### Scenario: binding authorizes resolution
- GIVEN a REQUIRE without binding THEN verify errors naming the exact `uni bind` command and executes nothing; GIVEN a matching bind THEN verify proceeds; GIVEN re-authorization to new text THEN old proof stales and renews.
#### Scenario: stale-but-unreprovable escalates
- GIVEN escalate_on_stale policy plus drifted proof that re-run cannot renew THEN Escalated (not EvidenceRequired).

### Out of scope
Identity adapters (spiffe/entra/oidc verify paths), signed provenance, fuzzy test-name resolution, cloud/dashboard. All documented as stubs, never half-wired.
