//! Named accounts in integer cents, and the total that must never change.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Ledger {
    accounts: BTreeMap<String, i64>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open an account. This is the only place new money enters the system, so
    /// every later operation must leave `total()` exactly where it was.
    pub fn open(&mut self, id: &str, amount: i64) -> Result<(), String> {
        if amount < 0 {
            return Err("opening balance cannot be negative".to_string());
        }
        if self.accounts.contains_key(id) {
            return Err(format!("account '{id}' already exists"));
        }
        self.accounts.insert(id.to_string(), amount);
        Ok(())
    }

    pub fn exists(&self, id: &str) -> bool {
        self.accounts.contains_key(id)
    }

    pub fn balance(&self, id: &str) -> i64 {
        self.accounts.get(id).copied().unwrap_or(0)
    }

    /// Sum of every account. Conservation means this number never changes
    /// after the accounts are opened.
    pub fn total(&self) -> i64 {
        self.accounts.values().sum()
    }

    pub(crate) fn credit(&mut self, id: &str, amount: i64) {
        *self.accounts.entry(id.to_string()).or_insert(0) += amount;
    }

    pub(crate) fn debit(&mut self, id: &str, amount: i64) {
        *self.accounts.entry(id.to_string()).or_insert(0) -= amount;
    }
}
