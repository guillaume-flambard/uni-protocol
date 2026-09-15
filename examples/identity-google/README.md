# A3 against a real issuer: Google (worked example)

Status: mechanism ships (v0.9.0); this example pins **real Google key
material** and documents the live-token procedure. Positive verification
against a live Google-signed token still needs one human step (mint the
token), because only you can authenticate as you.

For the fully unattended version, see the sibling example
`../identity-github-actions/`: GitHub mints a real OIDC token for the CI job
itself, so A3 is reached with no human step at all. This Google example keeps
its value as the shape a workforce identity takes (a user authenticating as
themselves), which is what `gcloud auth print-identity-token` produces.

## What is pinned here

- `google.jwks.json`: snapshot of `https://www.googleapis.com/oauth2/v3/certs`
  fetched 2026-09-15 (2 RS256 keys). Public key material, safe to commit.
- `google.uni`: trivial contract; the identity check happens before any
  verifier runs, so the contract content does not matter for the A3 step.

## Live procedure (one human step)

1. Re-fetch the keys (Google rotates them; never verify against a stale pin
   without knowing it):
   ```bash
   curl -s https://www.googleapis.com/oauth2/v3/certs -o google.jwks.json
   ```
2. Declare the issuer in the workspace registry (`.uni/config.toml`):
   ```toml
   [verifiers]
   "pass" = "true"

   [identities."https://accounts.google.com"]
   source = "oidc"
   jwks_file = "examples/identity-google/google.jwks.json"
   audiences = ["<your-oauth-client-id>"]
   ```
   The `iss` must match the token exactly (`https://accounts.google.com`;
   some flows emit `accounts.google.com` without scheme: pin the one your
   tokens carry, and refuse the other).
3. Mint a token (authenticate as yourself, e.g.):
   ```bash
   gcloud auth print-identity-token --audiences=<your-oauth-client-id> > /tmp/me.jwt
   ```
4. Verify:
   ```bash
   UNI_IDENTITY_TOKEN="$(cat /tmp/me.jwt)" uni verify examples/identity-google/google.uni
   uni report --json   # expect assurance A3, identity_assurance VERIFIED
   ```

## What is already proven without a live token

- The pinned file parses as a JWKS with >= 2 RS256 keys (test
  `real_google_jwks_pins_verifiable_keys`).
- A token signed by UNI's throwaway test key is **refused** against the
  Google issuer entry (`rejected by every trusted issuer`), even with the
  right `iss`: the trust boundary holds against real key material, not just
  the test fixture (test `test_key_is_refused_by_real_google_keys`).
- Key rotation is an operator duty: re-fetch before a live run; a stale pin
  fails closed (valid tokens refused), never open.

## Entra / SPIFFE

Same shape, different `iss`: Entra v2 is
`https://login.microsoftonline.com/<tenant>/v2.0` (pin the exact string your
tokens carry); SPIFFE pins the bare trust domain and requires a `spiffe://`
subject. Verification code path is identical; only the registry entry differs.
