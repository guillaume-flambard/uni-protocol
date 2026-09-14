use mini_app::{transfer, Ledger};

#[test]
fn transfer_moves_money() {
    let mut ledger = Ledger::new(2);
    ledger.balances[0] = 100;
    transfer(&mut ledger, 0, 1, 30).expect("transfer should succeed");
    assert_eq!(ledger.balance(0), 70);
    assert_eq!(ledger.balance(1), 30);
}
