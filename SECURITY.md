# Security policy

UNI decides whether work is acceptable, so a flaw here is a flaw in whatever
trusts it. Reports are welcome and taken seriously.

## Reporting a vulnerability

Do not disclose vulnerability details in a public issue.

GitHub private vulnerability reporting is the intended channel, but it is not
enabled for this repository yet. Until the **Report a vulnerability** button is
available under the Security tab, open a minimal issue titled `Private security
report requested`. Include no technical detail. The maintainer will arrange a
private channel and close the public coordination issue.

A useful report contains: what an attacker can do that they should not be able
to, the smallest reproduction (a contract, a registry, a command sequence), the
version or commit, and whether the flaw needs a malicious contract, a malicious
registry, a malicious token, or only a hostile repository layout.

## What we consider in scope

The trust boundary is the product, so these are the cases that matter most:

- **Contract-to-command escape.** A `.uni` contract must never be able to
  introduce a command. Only `.uni/config.toml` `[verifiers]` holds commands, and
  an inline `shell "..."` is accepted only while the registry is empty, or when
  an identical command already exists in the registry.
- **Identity forgery.** A token must not be believed unless its issuer is
  declared in `[identities]` and its signature verifies against key material
  pinned by path. A refusal must stay a hard error, never a silent downgrade to
  a self-declared actor, and no key material may be fetched at verification
  time.
- **Evidence that lies.** A proof that appears valid while its identity, scope,
  or observed subject does not cover the claim: wrong revision, wrong contract,
  wrong verifier version, truncated output, expired evidence, or a cache entry
  reused across a different context or actor.
- **Decision bypass.** Any input that makes the engine accept work the declared
  policy rejects, or that makes a critical failure stop being critical.
- **Parser escapes.** Input that reaches an unexpected path through the DSL: a
  reserved keyword that does not fail, a directive swallowed as prose, or a
  registry key mistaken for the inline verifier.

Reports are especially valuable when they come with a failing case that the test
suite does not already cover. The parser, the registry resolver, the evidence
context, and the identity adapter are the highest-value places to look.

## Out of scope

- Vulnerabilities in dependencies, unless UNI's use of them turns a bug into a
  trust-boundary failure. Report those upstream and tell us if it affects us.
- Anything requiring an attacker who already controls `.uni/config.toml`. That
  file is the trust root by design: whoever edits it authorizes commands.
- A malicious verifier, which is the same point: if it is in the registry, it
  was authorized by a human.
- Denial of service through an intentionally expensive contract on your own
  machine. Timeouts are per-verifier and documented.

## What to expect

This is a solo-maintained project, so honesty beats a service-level promise:
acknowledgement when the report is read, an assessment of severity and whether
it is in scope, and a fix or a public explanation of why it is not one. Fixes
land with a regression test, and the advisory names the version that contains
it. Please give us a reasonable window before publishing, and tell us if a
deadline matters to you.
