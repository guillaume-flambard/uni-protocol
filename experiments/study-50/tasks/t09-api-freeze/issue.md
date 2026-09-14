Issue t09: prices must be renderable with a currency symbol.

Today `format_price` prints the raw cent count. We need prices to render with a
symbol and decimals, e.g. 150 cents must render as `$1.50` when the symbol is
`$`.

Expose the new capability from the crate and add a test named exactly
`format_symbol` in tests/it.rs that asserts the `$1.50` case. Keep the existing
test passing and run `cargo test`.
