/// Apply a percent price increase, rounded down, at least by 1 if increase > 0.
pub fn bump(price: u64, pct: u64) -> u64 {
    // BUG: pct addition computed on the discounted base
    price.saturating_mul(pct) / 100
}
