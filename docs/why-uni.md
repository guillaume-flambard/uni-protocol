# Why UNI

Coding agents made "done" cheap and unreliable. The current loop is:

```
prompt -> agent -> diff -> "done"
```

Someone still has to answer, by hand: did it do what was asked, did it break an
invariant, were the right tests run, is the evidence still valid after the next
commit? As agent volume grows, this human review collapses.

UNI is the outcome assurance layer for autonomous work:

```
INTENT -> CLAIMS -> EVIDENCE -> POLICY -> DECISION
```

It is not an agent framework, not an orchestrator, not a control plane, not a
trust/attestation protocol. It answers one question about any piece of agent
work: **is this outcome acceptable, and can we prove it?**

The product thesis: "UNI ACCEPTED" eventually conveys more confidence than
"Agent completed."

## How it decides

- Contracts (`.uni` DSL) declare what must be true. They are untrusted input.
- A trusted registry (`.uni/config.toml [verifiers]`) maps named verifiers to commands.
- Verifiers run and produce evidence: exit codes, output expectations, content hashes, git binding.
- A deterministic engine turns (contract, evidence, policy) into ACCEPTED / REJECTED / EVIDENCE_REQUIRED / ESCALATED.
- LLMs may draft contracts (candidates require human approval). They never decide.

## Where it sits

| Layer | UNI's stance |
|---|---|
| Spec Kit | produces the intent; `uni import-speckit` compiles candidates |
| Codex / Claude / any agent | executors; UNI only checks outcomes |
| MCP / A2A | transport and tools; UNI defines what must be true |
| Temporal / Restate | execution engines; UNI does not orchestrate them, it checks what they delivered |
| Observability | answers "what happened"; UNI answers "is it sufficient evidence?" |
| OPA | the one policy engine wired in, through the `PolicyProvider` trait (`.uni/policies/opa.rego`) |

Spec source: `specs/001-uni-software-v01/spec.md`. Invariants: `constitution.md`.
