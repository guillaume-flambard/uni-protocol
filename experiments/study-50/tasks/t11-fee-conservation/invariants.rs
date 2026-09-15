//! Owner-provided invariant tests. Frozen: satisfy them, do not edit them.
//!
//! Two rules, both declared in the contract:
//!   - the total never changes after the accounts are opened (conservation);
//!   - the fee is `floor(amount * fee_bps / 10_000)` computed in integers,
//!     with no floating point and no rounding to nearest (exactness).

use mini_app::ledger::Ledger;
use mini_app::settlement::{Settlement, FEES_ACCOUNT};

fn ledger_with(pairs: &[(&str, i64)]) -> Ledger {
    let mut l = Ledger::new();
    for (id, amount) in pairs {
        l.open(id, *amount).unwrap();
    }
    l
}

#[test]
fn total_is_conserved_across_holds_and_fees() {
    let mut l = ledger_with(&[("alice", 1_000_000), ("bob", 0), (FEES_ACCOUNT, 0)]);
    let before = l.total();
    let mut s = Settlement::new();
    for bps in [3333u64, 5000, 1, 9999, 3, 0] {
        let id = s.hold(&mut l, "alice", "bob", 7).unwrap();
        s.settle_with_fee(&mut l, id, bps).unwrap();
    }
    assert_eq!(l.total(), before, "money was created or destroyed");
}

#[test]
fn fee_is_the_exact_integer_floor() {
    // floor(amount * bps / 10_000). The cases are chosen so that a float
    // implementation rounded to nearest gets them wrong:
    //   3 * 3333 = 9999  -> 0 (float 0.9999 rounds to 1)
    //   1 * 5000 = 5000  -> 0 (float 0.5 rounds up)
    //   99 * 50  = 4950  -> 0
    let cases: &[(i64, u64, i64)] = &[
        (100, 3333, 33),
        (3, 3333, 0),
        (1, 5000, 0),
        (99, 50, 0),
        (20000, 1, 2),
        (7, 0, 0),
    ];
    for (amount, bps, expected) in cases {
        let mut l = ledger_with(&[("alice", amount * 2 + 10), ("bob", 0), (FEES_ACCOUNT, 0)]);
        let before = l.total();
        let mut s = Settlement::new();
        let id = s.hold(&mut l, "alice", "bob", *amount).unwrap();
        let charged = s.settle_with_fee(&mut l, id, *bps).unwrap();
        assert_eq!(
            charged, *expected,
            "fee for amount={amount} bps={bps} must be floor, in integers"
        );
        assert_eq!(l.balance(FEES_ACCOUNT), *expected);
        assert_eq!(l.total(), before, "conservation broke at amount={amount}");
    }
}

#[test]
fn a_settled_hold_is_not_settled_twice() {
    let mut l = ledger_with(&[("alice", 1_000), ("bob", 0), (FEES_ACCOUNT, 0)]);
    let before = l.total();
    let mut s = Settlement::new();
    let id = s.hold(&mut l, "alice", "bob", 100).unwrap();
    s.settle_with_fee(&mut l, id, 250).unwrap();
    let after_first = l.balance(FEES_ACCOUNT);
    assert!(
        s.settle_with_fee(&mut l, id, 250).is_err(),
        "settling the same hold twice must be refused"
    );
    assert_eq!(l.balance(FEES_ACCOUNT), after_first, "the fee was taken twice");
    assert_eq!(l.total(), before);
}
