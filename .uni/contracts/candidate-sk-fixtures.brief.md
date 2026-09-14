# Work order: candidate-sk-fixtures

Goal: Imported from Spec Kit (candidate — review required).

Acceptance is decided from EVIDENCE, not from a report of completion.
A claim is accepted only when its verifier produces green evidence that is
still valid for the current commit; your own "done" message is never evidence.

## Claims to satisfy

### requirements (claim, required)
Must hold: Requirements

Evidence required (verifier `project.tests`):
- command: `cargo test`

### fr-001 (claim, required)
Must hold: Users can log in with valid credentials

Evidence required (verifier `project.tests`):
- command: `cargo test`

### fr-002 (claim, required)
Must hold: Expired tokens are rejected

Evidence required (verifier `project.tests`):
- command: `cargo test`

### scenario-04 (claim, required)
Must hold: fresh tokens accepted

Evidence required (verifier `project.tests`):
- command: `cargo test`

### requirement (claim, required)
Must hold: Requirement contracts compile fast

Evidence required (verifier `project.tests`):
- command: `cargo test`

## Rules

- Commands run only from `.uni/config.toml [verifiers]`; do not invent commands.
- Do not edit the contract, the registry, or previously passing tests to make evidence green.
- Report what you actually did; UNI decides acceptance, not you.

