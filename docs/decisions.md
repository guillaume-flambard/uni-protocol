# Decisions

The decision function is deterministic: same contract, same evidence, same
policies, same decision. No LLM, no clock, no RNG.

Two validity checks read the clock by design, and neither enters the decision
function: evidence expiry (`max_age_hours`), which decides whether a proof is
still available to the decision, and token expiry (`exp`), which decides whether
a presented identity is still valid. An expired proof is stale, the verifier
re-runs, and the decision is taken on the fresh evidence; an expired token is
refused. The decision for a given (contract, evidence, policy) stays
deterministic.

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
| `reject_on_invalid` (default true) | false downgrades a **critical** rejection to ESCALATED |
| `escalate_on_stale` | stale evidence on a non-accepted outcome becomes ESCALATED |
| `escalate_on_missing` | missing evidence becomes ESCALATED instead of EVIDENCE_REQUIRED |
| `min_verified_ratio` | verified/total claims below ratio becomes REJECTED |

ESCALATED means "a human should look", EVIDENCE_REQUIRED means "run more
verifiers". All three non-accepted decisions exit 1; the stderr label
(`UNI REJECTED`, `UNI EVIDENCE_REQUIRED`, `UNI ESCALATED`) is what separates
them, and `uni report` carries the decision itself.

A non-critical failure never becomes REJECTED in the first place: the truth
table maps it to EVIDENCE_REQUIRED. Only a critical failure rejects, which is
why `reject_on_invalid` downgrades a critical rejection and nothing else.

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
| A0 | No evidence at all: nothing was proven, so there is no evidence level to report. |
| A2 | Trusted verifier observed a complete subject. Default ceiling. |
| A3-D | Independent actor (`executor != verifier`), identity SELF-DECLARED. Logically independent, identity unproven. |
| A3 | Independent actor with an EXTERNALLY VERIFIED identity. |
| A4 | Reserved: signed provenance has no producer yet (`--attest` refuses explicitly). |

The scale describes the **evidence**, not the verdict. A rejected claim whose
proof is independent and externally verified is A3-grade evidence that the claim
is false, and it reports A3. The decision is not an input to the scale, which is
why `A0` means "no evidence was gathered" rather than "the run did not accept".

Rules: `uni verify` alone caps at A2 (local actor). `uni verify --actor ci:build-12`
enables A3-D and `uni report`/`explain` display `independent actor: YES,
identity: SELF-DECLARED`. `--actor bob` is a declaration, never a proof: passing
someone else's name cannot mint A3.

A3 needs a verified identity, and the one source of one is a signed token:

```
UNI_IDENTITY_TOKEN=<jwt> uni verify c.uni
```

The token is verified offline against an issuer pinned in the registry, never
against a URL fetched at runtime:

```toml
[identities."https://issuer.example"]
source = "oidc"                 # oidc | entra | spiffe
jwks_file = ".uni/identity/issuer.jwks.json"
audiences = ["uni-cli"]         # optional; when set, aud must intersect
algorithms = ["RS256"]          # optional; default RS256, ES256
```

The registry is the only root of trust, so the key material lives beside it, not
behind a network call: a token whose `iss` the registry does not declare is
refused. The id comes from the token, never the flag: `oidc:<issuer>#<sub>`,
`entra:<issuer>#<sub>`, or the SPIFFE id for `spiffe`. A token that fails
verification (bad signature, wrong issuer, wrong audience, expired) is a hard
error, not a silent downgrade to A3-D, and `--actor` and `UNI_IDENTITY_TOKEN`
are mutually exclusive. A `spiffe://`/`entra://`/`oidc://` prefix on `--actor`
with no token is recorded as an `IdentityUnverified` declaration and stays
self-declared.


## Why a proof stopped applying

Staleness is data, not a boolean. Each dimension that drifted is recorded as a
reason, and `uni explain` narrates it:

```
$ uni explain

CLAIM ledger.integrity
  status       Invalid
  command      grep -q 'A: u32 = 1' src/ledger.rs
  watched      src/ledger.rs
  why          artifact moved from commit 0f7ef140 to 14da6ff5 (commit_changed)
  why          watched subject moved (changed: src/ledger.rs) (subject_changed)
  action       uni verify ledger.integrity
```

Dimensions: `commit_changed`, `uncommitted_changes`, `subject_changed` (with the
files named), `contract_changed`, `verifier_config_changed`,
`platform_changed`, `authorization_changed`, `expired`.

Two properties worth knowing. First, the drift is **remembered**: the reasons
live in `.uni/decisions/stale.json` until the claim is proved again, so a later
run whose re-verification already overwrote the proof file can still explain
what moved. Second, staleness **forces re-verification, it does not fail a
claim**: if the verifier passes again on the delivered revision, the outcome is
`Accepted` and the old proof is simply replaced. Only a verifier that fails
leaves the claim unproven.

The same reasons travel in the journal (`uni.stale.reasons`,
`uni.stale.detail`) and in `uni verify --json`, so a CI annotation or a bot can
render them without parsing prose.

## Reading a decision

- `uni explain` : per-claim table, evidence detail, reason, next commands.
- `uni report` : byte-stable view for CI/PR summaries (volatile fields excluded).
- `uni events` : append-only journal of how we got here.
