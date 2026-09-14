# Feature 003 - UNI Assurance Model v0.2

### Requirements
- **FR-201**: Evidence records its Verification Context (contract/registry/policy hashes, platform, actor, executor, binding hash).
- **FR-202**: Drift on contract, subject, verifier, config, or environment forces re-run; policy drift forces decision recompute without re-run.
- **FR-203**: A changed registry names its diff (added/removed/changed verifiers), emits RegistryChanged, flags `trust_boundary_changed` for CI, and requires human review; old evidence goes stale, never reused.
- **FR-204**: Assurance derives from the evidence graph: A2 default ceiling; A3-D iff every proof's actor differs from the executor with self-declared identities; A3 iff every actor's identity was externally verified against an issuer pinned in `.uni/config.toml` `[identities]`; A4 refused (no signer).
- **FR-205**: Independence and identity assurance are tracked and displayed as distinct properties; `--actor x` can never mint A3. A JWT in `UNI_IDENTITY_TOKEN`, verified against a pinned issuer (signature via the pinned JWKS; `iss`/`aud`/`exp` checked), is the only way to reach A3; an unverifiable token is a hard error, never a silent downgrade.
- **FR-206**: A verification carrying REQUIRE executes only under a matching authorized VerifierBinding (`uni bind`); mismatch or absence is a hard error; re-authorization stales old proofs.

### Acceptance Scenarios
#### Scenario: registry change is named and flagged
- GIVEN an acknowledged registry, WHEN a verifier is added and committed, THEN verify prints REGISTRY_CHANGED naming it, sets trust_boundary_changed on that run only, and re-runs (exit stays decision-driven).
#### Scenario: actor separation caps honestly
- GIVEN default verify THEN report shows A2/independent NO; GIVEN --actor ci:build-12 THEN A3-D/independent YES/identity SELF-DECLARED; GIVEN --attest THEN hard refusal.
#### Scenario: identity token lifts A3-D to A3
- GIVEN an independent actor and a JWT signed by an issuer pinned in `[identities]` THEN report shows A3/independent YES/identity VERIFIED; GIVEN the same token expired, tampered, or from an undeclared issuer THEN verify fails and mints nothing; GIVEN `--actor` together with the token THEN a hard error.
#### Scenario: binding authorizes resolution
- GIVEN a REQUIRE without binding THEN verify errors naming the exact `uni bind` command and executes nothing; GIVEN a matching bind THEN verify proceeds; GIVEN re-authorization to new text THEN old proof stales and renews.
#### Scenario: stale-but-unreprovable escalates
- GIVEN escalate_on_stale policy plus drifted proof that re-run cannot renew THEN Escalated (not EvidenceRequired).

### Out of scope
Signed provenance (A4), fuzzy test-name resolution, cloud/dashboard. Identity adapters are implemented for `oidc`, `entra` and `spiffe` (all JWT verification against a registry-pinned JWKS); any other scheme stays a declared prefix, never half-wired.
