---
name: Bug report
about: Something the binary does that it should not, or does not that it should
labels: bug
---

## What happened

<!-- One or two sentences. The observable behaviour, not the diagnosis. -->

## What you expected

<!-- What the contract, the docs, or the constitution says should happen. -->

## Smallest reproduction

<!--
A contract plus the command you ran beats a description. If a registry entry is
involved, include it: most bugs here live at the contract-and-registry seam.
-->

```
# paste the .uni contract (trim to the smallest one that still fails)

```

```bash
# the exact command, and the output you got
```

## Version and platform

- `uni --version`:
- OS:
- installed from: <!-- release binary, cargo build, the GitHub action -->

## Anything else

<!--
If you already know which part of the trust boundary is involved (contract
resolution, registry, evidence freshness, identity, the decision table), say so:
it shortens the path to a fix.
-->
