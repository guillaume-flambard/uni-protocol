/// Format a price given in cents, without any currency symbol.
///
/// This signature is part of the published API and must not change.
pub fn format_price(cents: u64) -> String {
    cents.to_string()
}
