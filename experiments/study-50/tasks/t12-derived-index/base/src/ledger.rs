//! Accounts, owners, and the per-owner total derived from them.

use std::collections::BTreeMap;

/// One movement of money between two accounts.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub from: String,
    pub to: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Ledger {
    accounts: BTreeMap<String, i64>,
    owner_of: BTreeMap<String, String>,
    /// Derived from `accounts` and `owner_of`, and kept in step by every
    /// mutation that touches an account.
    by_owner: BTreeMap<String, i64>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, account: &str, owner: &str, amount: i64) -> Result<(), String> {
        if amount < 0 {
            return Err("opening balance cannot be negative".to_string());
        }
        if self.accounts.contains_key(account) {
            return Err(format!("account '{account}' already exists"));
        }
        self.accounts.insert(account.to_string(), amount);
        self.owner_of.insert(account.to_string(), owner.to_string());
        *self.by_owner.entry(owner.to_string()).or_insert(0) += amount;
        Ok(())
    }

    pub fn balance(&self, account: &str) -> i64 {
        self.accounts.get(account).copied().unwrap_or(0)
    }

    /// What an owner holds, read from the index. `snapshot` is the other way to
    /// get it, by recomputing from the accounts themselves.
    pub fn owner_total(&self, owner: &str) -> i64 {
        self.by_owner.get(owner).copied().unwrap_or(0)
    }

    /// `(account, owner, balance)` for every account.
    pub fn snapshot(&self) -> Vec<(String, String, i64)> {
        self.accounts
            .iter()
            .map(|(account, balance)| {
                let owner = self.owner_of.get(account).cloned().unwrap_or_default();
                (account.clone(), owner, *balance)
            })
            .collect()
    }

    pub fn total(&self) -> i64 {
        self.accounts.values().sum()
    }

    /// Everything `apply` refuses an entry for, without mutating anything.
    fn check(&self, entry: &Entry) -> Result<(), String> {
        if entry.amount <= 0 {
            return Err("amount must be positive".to_string());
        }
        if !self.accounts.contains_key(&entry.from) || !self.accounts.contains_key(&entry.to) {
            return Err("unknown account".to_string());
        }
        if self.balance(&entry.from) < entry.amount {
            return Err(format!("insufficient balance in '{}'", entry.from));
        }
        Ok(())
    }

    /// Apply one entry. Both the account balances and the per-owner index move
    /// here, together: there is no other place that writes an account.
    pub fn apply(&mut self, entry: &Entry) -> Result<(), String> {
        self.check(entry)?;
        let from_owner = self.owner_of.get(&entry.from).cloned().unwrap_or_default();
        let to_owner = self.owner_of.get(&entry.to).cloned().unwrap_or_default();
        *self.accounts.get_mut(&entry.from).unwrap() -= entry.amount;
        *self.accounts.get_mut(&entry.to).unwrap() += entry.amount;
        if from_owner != to_owner {
            *self.by_owner.entry(from_owner).or_insert(0) -= entry.amount;
            *self.by_owner.entry(to_owner).or_insert(0) += entry.amount;
        }
        Ok(())
    }

    /// Apply every entry, all or nothing: if any entry is refused, the ledger
    /// is left exactly as it was, accounts and index alike. Returns how many
    /// entries were applied.
    pub fn apply_atomic(&mut self, _entries: &[Entry]) -> Result<usize, String> {
        Err("apply_atomic is not implemented".to_string())
    }
}
