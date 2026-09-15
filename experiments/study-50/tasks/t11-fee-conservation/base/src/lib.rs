//! A small settlement crate: accounts, holds, and the fee taken at settlement.
//!
//! The one rule everything else answers to: money only moves between accounts.
//! `Ledger::total` is fixed the moment the accounts are opened.

pub mod ledger;
pub mod settlement;

pub use ledger::Ledger;
pub use settlement::{Settlement, FEES_ACCOUNT};

pub fn version() -> &'static str {
    "0.1"
}
