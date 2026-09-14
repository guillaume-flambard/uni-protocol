# Evidence

Evidence is a machine-checkable record that a verifier ran and what it produced.
It lives in `.uni/evidence/<claim-id>.json` (canonical JSON, see `Evidence` in
`crates/uni-evidence`).

## Shape

| Field | Meaning |
|---|---|
| id | claim + hash of the verifier command |
| claim_id | the claim this proves |
| producer | which verifier produced it (`shell-verifier` v0.1) |
| command | the trusted command that ran |
| exit_code | process exit status |
| output_hash / output_excerpt | sha256 of combined stdout+stderr, plus first 2000 chars |
| commit_sha / workspace_dirty | git binding at run time |
| artifact_hash | sha256 over watched files (see `files` in verification.md) |
| state | `Valid` / `Invalid` / `Stale` |
| created_at / duration_ms | wall clock and runtime (volatile, excluded from `uni report`) |

## States

- **Valid**: command exited 0, `expect` matched (if any), `expect_not` absent (if any).
- **Invalid**: non-zero exit or expectation mismatch.
- **Stale**: was valid once, no longer bound to current state.

## Invalidation (the reason this exists)

Evidence is only trusted while its bindings hold. Validity is a product of
seven dimensions (Verification Context, v0.2):

| Dimension | Recorded as | Drift means |
|---|---|---|
| Contract | `contract_hash` (sha256 of the `.uni` source) | re-run |
| Subject | `artifact_hash` (sha256 over watched `files`) | re-run |
| Verifier | `fingerprint` (ref + run + expects + files) | re-run (different verification) |
| Verifier configuration | `registry_hash` (sha256 of `config.toml`) | re-run |
| Environment | `commit_sha` + dirty flag, `platform` (os-arch) | re-run |
| Policy | `policy_hash` (recorded for audit) | decision recompute, no re-run |
| Time | `created_at` (recorded; expiry policies are v0.3+) | nothing yet |

1. **Git binding** (`is_stale`): if HEAD moved or dirty-state changed since the
   run, the stored evidence is Stale and `uni verify` re-runs the verifier.
   Dirtiness excludes `.uni/` runtime files (a verify never invalidates its
   own fresh evidence through the files it writes; concurrent verifies stay
   consistent). Trust-relevant `.uni` changes are covered by registry/policy
   hashes instead.
2. **Content binding** (`artifact_hash`): if a verifier declares `files` globs,
   the evidence stores a sha256 over those files' contents. Any change inside the
   same commit invalidates it. This is what stops a previously accepted outcome
   from masking a new bad edit.
3. **Context binding** (`contract_hash`, `registry_hash`, `platform`): editing
   the contract, touching any registry entry, or moving across platforms
   invalidates. Legacy v0.1.0 files (empty hashes) still load.
4. **Policy binding** (`policy_hash`): recorded for audit; a policy change
   recomputes the decision from stored evidence without re-running verifiers.
5. **Stale-but-unreprovable**: a claim whose previous proof drifted AND whose
   re-run cannot renew it escalates under `escalate_on_stale` (human look),
   instead of merely reporting missing evidence.

A decision is never silently reused across a binding break: old evidence cannot
mask a new commit (e2e test: `stale_evidence_does_not_mask_new_commit`).

## Audit trail

Every verify appends events (IntentVerified, EvidenceRun, EvidenceReused,
EvidenceStale, RegistryChanged, BindingAuthorized, IdentityUnverified,
DecisionIssued) to `.uni/events.jsonl` with `uni.*` attributes. See `uni events`.

## Bundles (v0.3)

```bash
uni bundle export contract.uni --out bundle.jsonl
uni bundle verify bundle.jsonl    # offline, read-only
```

A bundle is JSONL: a header (version, intent, registry_hash, contract_hash,
tool) then one record per artifact (contract, registry, policy, evidence,
binding, decision, events). Every record carries `sha256` over its canonical
body, so transport tampering is detectable offline. Cross-checks reject
bundles whose pieces do not belong together: evidence gathered under another
registry or contract, a decision citing claims with no bundled evidence, a
REQUIRE without its binding, evidence for claims absent from the contract.

Importing a bundle never injects proofs into a live cache: verification only
reports. Transport is not authority.
