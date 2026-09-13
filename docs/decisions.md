# Decisions

The decision function is deterministic: same contract, same evidence, same
policies, same decision. No LLM, no clock, no RNG.

## Stage 1: evidence truth table (`evaluate`)

| Condition | Decision |
|---|---|
| any CRITICAL claim has Invalid evidence | REJECTED |
| any REQUIRED claim lacks Valid evidence (missing, stale, or invalid non-critical) | EVIDENCE_REQUIRED |
| otherwise | ACCEPTED |

## Stage 2: policy layer (`apply_policy`)

Policies never weaken the truth table; they tighten or escalate it.
Sourced from `.uni/policies/*.toml` (`[policy]`, files sorted, later booleans
win, max ratio) or an OPA bundle (`PolicyProvider` trait; when
`.uni/policies/opa.rego` exists and `opa` is on PATH it wins, with graceful
fallback).

| Field | Effect |
|---|---|
| `reject_on_invalid` (default true) | false downgrades non-critical Rejected to ESCALATED |
| `escalate_on_stale` | stale evidence on a non-accepted outcome becomes ESCALATED |
| `escalate_on_missing` | missing evidence becomes ESCALATED instead of EVIDENCE_REQUIRED |
| `min_verified_ratio` | verified/total claims below ratio becomes REJECTED |

ESCALATED means "a human should look", EVIDENCE_REQUIRED means "run more
verifiers"; the CLI distinguishes them on exit and in `uni report`.

## States

`Decision`: `Accepted`, `Rejected`, `EvidenceRequired`, `Escalated`.
CLI exit codes: 0 for Accepted; 1 otherwise (stderr carries the label).
`DRAFT/READY/EXECUTING/...` lifecycle states from the blueprint are a later
scope (needs an execution provider binding).

## Reading a decision

- `uni explain` : per-claim table, evidence detail, reason, next commands.
- `uni report` : byte-stable view for CI/PR summaries (volatile fields excluded).
- `uni events` : append-only journal of how we got here.
