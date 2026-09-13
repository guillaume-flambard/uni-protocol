pub fn add(a: i64, b: i64) -> i64 {
    a + b
}

pub fn clamp(x: i64, lo: i64, hi: i64) -> i64 {
    // BUG: upper bound never enforced
    if x < lo {
        lo
    } else {
        x
    }
}
