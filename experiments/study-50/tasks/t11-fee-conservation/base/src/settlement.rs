//! Holds and their settlement. A hold moves the money immediately; settling
//! only decides that the move stands, and optionally takes a fee.

use crate::ledger::Ledger;

/// The account every fee is paid into. It is opened like any other.
pub const FEES_ACCOUNT: &str = "fees";

#[derive(Debug, Clone, PartialEq)]
pub struct Hold {
    pub id: u64,
    pub from: String,
    pub to: String,
    pub amount: i64,
    pub settled: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Settlement {
    pub holds: Vec<Hold>,
    next_id: u64,
}

impl Settlement {
    pub fn new() -> Self {
        Self::default()
    }

    /// Place a hold: the amount leaves `from` and is credited to `to` straight
    /// away, and the record waits to be settled.
    pub fn hold(
        &mut self,
        ledger: &mut Ledger,
        from: &str,
        to: &str,
        amount: i64,
    ) -> Result<u64, String> {
        if !ledger.exists(from) || !ledger.exists(to) {
            return Err("unknown account".to_string());
        }
        if amount <= 0 {
            return Err("amount must be positive".to_string());
        }
        if ledger.balance(from) < amount {
            return Err(format!("insufficient balance in '{from}'"));
        }
        ledger.debit(from, amount);
        ledger.credit(to, amount);
        let id = self.next_id;
        self.next_id += 1;
        self.holds.push(Hold {
            id,
            from: from.to_string(),
            to: to.to_string(),
            amount,
            settled: false,
        });
        Ok(id)
    }

    /// Settle a hold with no fee.
    pub fn settle(&mut self, id: u64) -> Result<(), String> {
        let hold = self
            .holds
            .iter_mut()
            .find(|h| h.id == id)
            .ok_or_else(|| format!("unknown hold {id}"))?;
        if hold.settled {
            return Err(format!("hold {id} is already settled"));
        }
        hold.settled = true;
        Ok(())
    }

    /// Settle a hold and collect a fee, in basis points of the held amount,
    /// into `FEES_ACCOUNT`. The fee is taken out of what `to` was credited.
    ///
    /// The fee is `floor(amount * fee_bps / 10_000)`, exactly, in integer
    /// arithmetic: no floating point, no rounding to nearest.
    ///
    /// Returns the fee that was collected.
    pub fn settle_with_fee(
        &mut self,
        _ledger: &mut Ledger,
        _id: u64,
        _fee_bps: u64,
    ) -> Result<i64, String> {
        Err("settle_with_fee is not implemented".to_string())
    }
}
