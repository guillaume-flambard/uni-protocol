# Quickstart (60 seconds)

```bash
cargo build --release
./target/release/uni init
# write or import a contract:
./target/release/uni import-speckit path/to/speckit-feature/   # candidate for review
# or hand-write one (see examples/hello/hello.uni)
./target/release/uni brief uni/intents/feature.uni --out brief.md  # hand to the agent
./target/release/uni lint   examples/hello/hello.uni   # preflight, no execution
./target/release/uni verify examples/hello/hello.uni   # trusted verifiers
./target/release/uni report                            # stable PR/CI view
./target/release/uni explain                           # per-claim evidence detail
```

Exit codes: 0 = ACCEPTED, 1 = REJECTED / EVIDENCE_REQUIRED / ESCALATED.

Stack-independent: the same three claims work with `cargo test`,
`python3 -m unittest`, or `node --test` - the registry maps verifier
names to trusted commands (see `examples/` and `README.md`).
