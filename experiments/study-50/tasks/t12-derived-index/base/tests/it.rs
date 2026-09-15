use mini_app::ledger::{Entry, Ledger};

fn entry(from: &str, to: &str, amount: i64) -> Entry {
    Entry {
        from: from.to_string(),
        to: to.to_string(),
        amount,
    }
}

fn ledger() -> Ledger {
    let mut l = Ledger::new();
    l.open("alice-checking", "alice", 500).unwrap();
    l.open("alice-savings", "alice", 100).unwrap();
    l.open("bob-checking", "bob", 0).unwrap();
    l
}

#[test]
fn crate_loads() {
    assert_eq!(mini_app::version(), "0.1");
}

#[test]
fn a_transfer_moves_money_between_accounts() {
    let mut l = ledger();
    l.apply(&entry("alice-checking", "bob-checking", 30)).unwrap();
    assert_eq!(l.balance("alice-checking"), 470);
    assert_eq!(l.balance("bob-checking"), 30);
}

#[test]
fn an_impossible_transfer_is_refused() {
    let mut l = ledger();
    assert!(l.apply(&entry("alice-checking", "bob-checking", 900)).is_err());
    assert_eq!(l.balance("alice-checking"), 500);
    assert_eq!(l.balance("bob-checking"), 0);
}
