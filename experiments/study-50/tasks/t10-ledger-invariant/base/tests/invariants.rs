//! Owner-provided invariant tests. This file is frozen: it is hashed by the
//! contract and must not be edited, only satisfied.
use mini_app::{transfer, Ledger};

fn total(ledger: &Ledger) -> i64 {
    (0..ledger.balances.len()).map(|i| ledger.balance(i)).sum()
}

#[test]
fn conservation_and_refusals() {
    let mut ledger = Ledger::new(3);
    ledger.balances[0] = 500;
    ledger.balances[1] = 200;
    let before = total(&ledger);

    assert!(transfer(&mut ledger, 0, 1, 100).is_ok());
    assert!(transfer(&mut ledger, 1, 2, 50).is_ok());

    // Overdraft must be refused, and refuse without moving anything.
    let snapshot = ledger.balances.clone();
    assert!(transfer(&mut ledger, 2, 0, 9_999).is_err());
    assert_eq!(ledger.balances, snapshot, "a refused transfer must not move money");

    // Out-of-range accounts must be refused, not panic.
    assert!(transfer(&mut ledger, 0, 99, 10).is_err());

    assert_eq!(total(&ledger), before, "transfers must conserve the total");
    assert_eq!(ledger.balance(0), 400);
    assert_eq!(ledger.balance(1), 250);
    assert_eq!(ledger.balance(2), 50);
}

#[test]
fn self_transfer_is_a_noop() {
    let mut ledger = Ledger::new(1);
    ledger.balances[0] = 100;
    assert!(transfer(&mut ledger, 0, 0, 40).is_ok());
    assert_eq!(ledger.balance(0), 100);
    assert_eq!(total(&ledger), 100);
}
