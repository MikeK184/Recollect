//! One definition of "acceptable free text" for names, reasons and descriptions.

/// At most `limit` characters, no control characters (a line break and tab are
/// allowed when `newlines`), and not blank when `required`. Callers keep their
/// own user-facing message.
pub(crate) fn valid(value: &str, limit: usize, required: bool, newlines: bool) -> bool {
    !(required && value.trim().is_empty())
        && value.chars().count() <= limit
        && !value
            .chars()
            .any(|c| c.is_control() && !(newlines && matches!(c, '\n' | '\t')))
}

#[cfg(test)]
mod tests {
    use super::valid;

    #[test]
    fn limits_required_and_newlines_are_independent() {
        assert!(valid("a", 1, true, false));
        assert!(!valid("ab", 1, true, false));
        assert!(!valid("  ", 10, true, false));
        assert!(valid("  ", 10, false, false));
        assert!(valid("one\ntwo\tthree", 20, true, true));
        assert!(!valid("one\ntwo", 20, true, false));
        assert!(!valid("bell\u{7}", 20, true, true));
        assert!(valid("日本語", 3, true, false));
    }
}
