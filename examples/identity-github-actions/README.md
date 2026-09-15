# A3 against a real issuer: GitHub Actions OIDC (live)

This is the live counterpart of `examples/identity-google/`. GitHub mints a
real, signed OIDC token for the job itself, so the whole path runs unattended:
no human-minted token, no secret stored, nothing added to the repository.

Pinned here: `github.jwks.json`, a snapshot of
`https://token.actions.githubusercontent.com/.well-known/jwks` (4 RS256 keys,
fetched 2026-09-15). Public key material, safe to commit.

## Run it

```bash
gh workflow run identity-live.yml
gh run watch
```

The workflow (`.github/workflows/identity-live.yml`) does exactly four things:

1. asks GitHub for a token with `audience=uni-cli` (needs `id-token: write`),
2. pins nothing new: it verifies against the committed JWKS,
3. runs `UNI_IDENTITY_TOKEN=<token> uni verify identity.uni`,
4. fails unless `uni report --json` says `assurance == "A3"` and
   `identity_assurance == "VERIFIED"`.

## Result

Run `34939375852` (2026-09-15, `workflow_dispatch`, 6s):

```
Identity: oidc:https://token.actions.githubusercontent.com#
  repo:guillaume-flambard@56681566/uni-protocol@1370011909:ref:refs/heads/main (verified)
assurance A3 · identity_assurance VERIFIED · independent_actor true
refused as expected: no [identities] issuer to believe it
```

## What is proven, and what is not

- Proven (live): a real token from a real issuer, verified offline against key
  material pinned beside the registry, lifts an independent proof from A3-D to
  A3. The token is never sent anywhere by `uni`; there is no network call in
  the verification path.
- Not proven here: key rotation is an operator duty. GitHub rotates the signing
  keys, the pin goes stale, and verification **fails closed** (a valid token is
  refused, never accepted on an unchecked key). Re-fetch with:
  ```bash
  curl -s https://token.actions.githubusercontent.com/.well-known/jwks \
    -o examples/identity-github-actions/github.jwks.json
  ```
- The `iss` is pinned exactly (`https://token.actions.githubusercontent.com`);
  a token carrying any other issuer is refused by the trust boundary, not by
  the signature.

## Why this issuer and not Google

`gcloud` is not installed on the development machine, so the Google example
(a documented procedure) needs a human step. GitHub Actions OIDC needs none:
the token is minted by the platform the repository already runs on, which makes
A3 reproducible in CI on every attempt.
