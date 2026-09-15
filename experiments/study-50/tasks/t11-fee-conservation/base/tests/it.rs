use mini_app::ledger::Ledger;
use mini_app::settlement::Settlement;

fn accounts() -> Ledger {
    let mut l = Ledger::new();
    l.open("alice", 100).unwrap();
    l.open("bob", 0).unwrap();
    l
}

#[test]
fn crate_loads() {
    assert_eq!(mini_app::version(), "0.1");
}

#[test]
fn hold_moves_the_amount_immediately() {
    let mut l = accounts();
    let mut s = Settlement::new();
    s.hold(&mut l, "alice", "bob", 30).unwrap();
    assert_eq!(l.balance("alice"), 70);
    assert_eq!(l.balance("bob"), 30);
}

#[test]
fn hold_is_refused_without_funds() {
    let mut l = accounts();
    let mut s = Settlement::new();
    assert!(s.hold(&mut l, "alice", "bob", 300).is_err());
    assert_eq!(l.balance("alice"), 100);
}
