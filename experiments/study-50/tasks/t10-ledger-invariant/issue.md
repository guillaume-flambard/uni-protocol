Issue t10: money transfers are broken.

`transfer` currently credits the destination account without debiting the
source, so it creates money. Make transfers correct in src/ledger.rs: money
must actually move from `from` to `to`.

The crate ships an owner-provided invariant test file, tests/invariants.rs.
It is frozen: do not edit it, satisfy it. Add your own tests in tests/it.rs
and run `cargo test`.
