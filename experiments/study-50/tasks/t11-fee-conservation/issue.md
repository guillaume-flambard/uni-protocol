Issue t11: settlement does not collect a fee yet.

`Settlement::settle_with_fee` in `src/settlement.rs` returns `Err("not
implemented")`. Implement it: settle the hold and take a fee, in basis points of
the held amount, into the `FEES_ACCOUNT` ("fees"), which is a normal account you
must open like any other in your tests.

The rules are in the doc comment and in the frozen invariant file:

- the fee is `floor(amount * fee_bps / 10_000)`, computed in integer
  arithmetic. No floating point, and no rounding to nearest: for
  `amount = 3, fee_bps = 3333` the fee is `0`, not `1`.
- the fee comes out of what the destination was credited, so the ledger total
  does not move.
- settling the same hold twice is refused, and must not take the fee twice.

`tests/invariants.rs` is owner-provided and frozen: do not edit it, satisfy it.
Add your own tests in `tests/it.rs` and run `cargo test`.

Name the test that proves the fee is exact `fee_is_exact` so it can be found.
