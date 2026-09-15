## What this changes

<!-- One paragraph. What is true after this that was not true before. -->

## How it is proven

<!--
This project's whole point: a claim is worth what its evidence is worth. Paste
the command and its result, not a description of the result.
-->

```bash
cargo test
cargo clippy --all-targets -- -D warnings
./target/release/uni verify <the contract this touches>   # if a contract changed
```

## What would make this wrong

<!--
The useful review question. If you cannot think of an answer, that is itself
worth saying: it tells the reviewer where to look hardest.
-->

## Checklist

- [ ] `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` pass
- [ ] `cargo test` passes
- [ ] A test covers the new behaviour, at the level the behaviour lives
- [ ] No document in `docs/` describes behaviour the binary does not have
- [ ] Nothing in the decision path reads the network, the clock, or randomness
- [ ] If a trust-boundary file changed (`[verifiers]` or `[identities]`), the change is named in the description
- [ ] Contributions are licensed under MIT OR Apache-2.0 (see CONTRIBUTING.md)
