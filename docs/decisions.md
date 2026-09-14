# Decisions

The decision function is deterministic: same contract, same evidence, same
policies, same decision. No LLM, no clock, no RNG.

One predicate reads the clock by design: evidence expiry (`max_age_hours`). It
does not make the decision non-deterministic; it decides whether a proof is
still available to the decision. An expired proof is stale, the verifier re-runs,
and the decision is taken on the fresh evidence.

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

## Assurance scale (v0.2: derived from the evidence graph)

Independence (who proved vs who launched) and identity assurance (how strongly
the prover's identity is proven) are DISTINCT properties:

| Level | Meaning |
|---|---|
| A2 | Trusted verifier observed a complete subject. Default ceiling. |
| A3-D | Independent actor (`executor != verifier`), identity SELF-DECLARED. Logically independent, identity unproven. |
| A3 | Independent actor with EXTERNALLY VERIFIED identity (adapters are stubs in v0.2, so unreachable yet except in unit tests). |
| A4 | Reserved: signed provenance has no producer yet (`--attest` refuses explicitly). |

Rules: `uni verify` alone caps at A2 (local actor). `uni verify --actor ci:build-12`
enables A3-D and `uni report`/`explain` display `independent actor: YES,
identity: SELF-DECLARED`. A `spiffe://` (or entra/oidc) prefix is recorded but
stays self-declared with an `IdentityUnverified` journal event. `--actor bob`
is a declaration, never a proof: passing someone else's name cannot mint A3.

## Reading a decision

- `uni explain` : per-claim table, evidence detail, reason, next commands.
- `uni report` : byte-stable view for CI/PR summaries (volatile fields excluded).
- `uni events` : append-only journal of how we got here.
