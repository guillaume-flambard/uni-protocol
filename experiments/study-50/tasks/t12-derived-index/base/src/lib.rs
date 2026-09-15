//! A ledger with a derived index: accounts hold the money, and `by_owner` is a
//! running total per owner, kept in step by every mutation.

pub mod ledger;

pub use ledger::{Entry, Ledger};

pub fn version() -> &'static str {
    "0.1"
}
