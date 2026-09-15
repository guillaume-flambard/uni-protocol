Issue t11: settlement does not collect a fee yet.

`Settlement::settle_with_fee` in `src/settlement.rs` returns `Err("not
implemented")`. Implement it: settle the hold and charge a fee, in basis points
of the held amount, paid into the account named by `FEES_ACCOUNT`.

The fee comes out of what the destination was credited, so the ledger total
does not move. Settling the same hold twice is refused, and must not charge the
fee twice.

Fees are charged in whole cents: 333 basis points on 100 cents is 3 cents.

Add your own tests in `tests/it.rs` and run `cargo test`. Name the test that
proves the fee is exact `fee_is_exact` so it can be found.
