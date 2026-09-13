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

Evidence is only trusted while its bindings hold:

1. **Git binding** (`is_stale`): if HEAD moved or dirty-state changed since the
   run, the stored evidence is Stale and `uni verify` re-runs the verifier.
2. **Content binding** (`artifact_hash`): if a verifier declares `files` globs,
   the evidence stores a sha256 over those files' contents. Any change inside the
   same commit invalidates it. This is what stops a previously accepted outcome
   from masking a new bad edit.
3. **Verifier compromise / policy change**: re-verify; stale never counts as proof.

A decision is never silently reused across a binding break: old evidence cannot
mask a new commit (e2e test: `stale_evidence_does_not_mask_new_commit`).

## Audit trail

Every verify appends events (IntentVerified, EvidenceRun, EvidenceReused,
DecisionIssued) to `.uni/events.jsonl` with `uni.*` attributes. See `uni events`.
