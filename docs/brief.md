# The work order (`uni brief`)

UNI decides acceptance from evidence. An implementing agent that never learns
which evidence is required will guess, and a correct fix can be rejected for
something as arbitrary as a test's name. That is not hypothetical: it is the
single disagreement in the corrected study (`RESULTS-2026-09-14.md`, task t04).

`uni brief` closes that gap from the contract side, deterministically:

```bash
uni brief uni/intents/feature.uni                 # markdown, byte-stable
uni brief uni/intents/feature.uni --out brief.md  # write for the agent's workspace
uni brief --json uni/intents/feature.uni          # machine-readable
```

It emits, per claim: the obligation (`ensure`), required/critical status, the
resolved registry command, the `expect`/`expect_not` strings, watched files,
and - when the command selects a test - the exact test name that must exist
(`cargo test clamp_upper_works -- --exact` -> create `clamp_upper_works`).
Unresolvable verifiers are reported instead of silently omitted, and the same
`uni lint` problems surface here.

For selector-template verifiers (`cargo test {{selector}}`), the brief says so
in plain language: the worker writes the test and picks a name, and a human
authorizes that name with `uni bind --selector`. The raw token never reaches
the work order.

## What it is not

This is guidance handed to a worker, never authority:

- nothing executes, nothing is verified by producing a brief;
- the registry stays the only source of commands;
- the agent's own "done" remains non-evidence;
- the output carries no timestamp, so it can be committed and diffed like any
  other spec artifact, and a registry change is visible against it.

The trust chain is untouched: AI proposes work, the contract states what must
be true, the verifier proves it, UNI decides. `uni brief` only makes sure the
first two steps see the same requirements.
