Issue t12: batches are not atomic yet.

`Ledger::apply_atomic` in `src/ledger.rs` returns `Err("not implemented")`.
Implement it: apply every entry in the slice, or none of them, and return how
many were applied.

The rules:

- All or nothing. If any entry cannot be applied, the ledger must be left
  exactly as it was. Returns the number of entries applied on success.
- Decide against what the whole batch would leave behind, not against the
  balances it starts from. A batch whose entries are each affordable on their
  own but which together spend more than an account holds must be refused,
  and the affordable first entry must not be applied on its own.
- `Ledger::apply` is the single-entry path and already encodes what a legal
  entry is; an entry it refuses is one the batch must refuse too.

Add your own tests in `tests/it.rs` and run `cargo test`. Name the test that
proves a valid batch applies in full and an impossible one is refused
`apply_atomic_works` so it can be found.
