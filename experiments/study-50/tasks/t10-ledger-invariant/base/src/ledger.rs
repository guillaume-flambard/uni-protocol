pub struct Ledger {
    pub balances: Vec<i64>,
}

impl Ledger {
    pub fn new(accounts: usize) -> Self {
        Ledger {
            balances: vec![0; accounts],
        }
    }

    pub fn balance(&self, id: usize) -> i64 {
        self.balances[id]
    }
}

/// Move `amount` from account `from` to account `to`.
///
/// BUG: credits the destination without debiting the source, which creates
/// money out of nothing.
pub fn transfer(ledger: &mut Ledger, from: usize, to: usize, amount: i64) -> Result<(), String> {
    let _ = from;
    ledger.balances[to] += amount;
    Ok(())
}
