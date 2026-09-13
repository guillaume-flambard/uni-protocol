/// Sum of integers 1..=n (inclusive).
pub fn sum_to(n: i64) -> i64 {
    // BUG: off by one - excludes n
    (1..n).sum()
}
