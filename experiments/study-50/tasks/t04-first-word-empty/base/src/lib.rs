/// First word of a sentence.
pub fn first_word(s: &str) -> &str {
    // BUG: panics on empty string
    s.split(' ').next().unwrap()
}
