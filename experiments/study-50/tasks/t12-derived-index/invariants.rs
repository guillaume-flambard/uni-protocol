//! Owner-provided invariant tests. Frozen: satisfy them, do not edit them.
//!
//! Two rules, both declared in the contract:
//!   - `owner_total` agrees with the accounts it is derived from, after any
//!     sequence of mutations (index coherence);
//!   - a batch that is refused changes nothing at all, accounts and index
//!     alike (atomicity).

use std::collections::BTreeMap;

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
    l.open("bob-checking", "bob", 40).unwrap();
    l.open("carol-checking", "carol", 0).unwrap();
    l
}

/// Recomputed from the accounts, never from the index.
fn recomputed_by_owner(l: &Ledger) -> BTreeMap<String, i64> {
    let mut out: BTreeMap<String, i64> = BTreeMap::new();
    for (_, owner, balance) in l.snapshot() {
        *out.entry(owner).or_insert(0) += balance;
    }
    out
}

fn assert_index_matches(l: &Ledger, context: &str) {
    let expected = recomputed_by_owner(l);
    for (owner, want) in &expected {
        assert_eq!(
            l.owner_total(owner),
            *want,
            "{context}: owner '{owner}' index is out of step with the accounts"
        );
    }
    // And the other direction: an owner the index knows about but the accounts
    // do not, which is what a naive decrement leaves behind.
    for (owner, got) in [("alice", l.owner_total("alice")), ("bob", l.owner_total("bob")), ("carol", l.owner_total("carol"))] {
        assert_eq!(
            got,
            expected.get(owner).copied().unwrap_or(0),
            "{context}: owner '{owner}' index is out of step with the accounts"
        );
    }
}

#[test]
fn the_index_keeps_up_with_a_sequence_of_batches() {
    let mut l = ledger();
    let batches: Vec<Vec<Entry>> = vec![
        vec![entry("alice-checking", "bob-checking", 10)],
        vec![
            entry("alice-savings", "alice-checking", 100),
            entry("bob-checking", "carol-checking", 50),
        ],
        vec![entry("alice-checking", "carol-checking", 7)],
        vec![],
    ];
    let before = l.total();
    for (i, batch) in batches.iter().enumerate() {
        let applied = l.apply_atomic(batch).unwrap();
        assert_eq!(applied, batch.len(), "batch {i} should apply in full");
        assert_index_matches(&l, &format!("after batch {i}"));
    }
    // Moving money never changes how much money there is.
    assert_eq!(l.total(), before);
}

#[test]
fn a_refused_batch_changes_nothing() {
    let mut l = ledger();
    let accounts_before: Vec<(String, i64)> = l
        .snapshot()
        .into_iter()
        .map(|(a, _, b)| (a, b))
        .collect();
    let owners_before = recomputed_by_owner(&l);

    // The third entry is impossible: bob never has 4000.
    let batch = vec![
        entry("alice-checking", "bob-checking", 10),
        entry("alice-savings", "carol-checking", 5),
        entry("bob-checking", "carol-checking", 4000),
    ];
    assert!(
        l.apply_atomic(&batch).is_err(),
        "an impossible entry must fail the whole batch"
    );

    let accounts_after: Vec<(String, i64)> = l
        .snapshot()
        .into_iter()
        .map(|(a, _, b)| (a, b))
        .collect();
    assert_eq!(
        accounts_after, accounts_before,
        "a refused batch must not move money"
    );
    assert_eq!(
        recomputed_by_owner(&l),
        owners_before,
        "a refused batch must not change what anyone holds"
    );
    assert_index_matches(&l, "after a refused batch");
}

/// Atomicity has to be decided against what the batch would leave behind, not
/// against the balances it starts from: each entry here is affordable on its
/// own, and together they are not.
#[test]
fn a_batch_that_overspends_only_in_total_is_refused() {
    let mut l = ledger();
    let accounts_before: Vec<(String, i64)> = l
        .snapshot()
        .into_iter()
        .map(|(a, _, b)| (a, b))
        .collect();

    assert_eq!(l.balance("alice-checking"), 500);
    let batch = vec![
        entry("alice-checking", "bob-checking", 400),
        entry("alice-checking", "carol-checking", 200),
    ];
    assert!(
        l.apply_atomic(&batch).is_err(),
        "400 and then 200 out of 500 must not both apply"
    );

    let accounts_after: Vec<(String, i64)> = l
        .snapshot()
        .into_iter()
        .map(|(a, _, b)| (a, b))
        .collect();
    assert_eq!(
        accounts_after, accounts_before,
        "the affordable first entry must not have been applied on its own"
    );
    assert_index_matches(&l, "after a collectively-overspending batch");
}

#[test]
fn an_unknown_account_fails_the_whole_batch() {
    let mut l = ledger();
    let before = l.total();
    let batch = vec![
        entry("alice-checking", "bob-checking", 1),
        entry("alice-checking", "nobody-here", 1),
    ];
    assert!(l.apply_atomic(&batch).is_err());
    assert_eq!(l.total(), before);
    assert_index_matches(&l, "after a batch naming an unknown account");
}
