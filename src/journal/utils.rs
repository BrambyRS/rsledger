//! This module contains private support functions and types for use
//! within the `journal` module. Any code needed by other modules should not be placed here.

/// The types of directives that can be stored in a journal.
/// Limited support at the moment, needs to be expanded.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum DirectiveType {
    /// For include directives.
    Include,
    /// For transactions.
    Transaction,
    /// For price directives.
    Price,
    /// For unrecognised directives.
    None,
}

/// Identifies the type of directive based on the content of a line.
pub fn identify_directive_type(line: &str) -> DirectiveType {
    // Trim comments
    let trimmed_line = trim_comments(line);

    // Check for include directive (line starts with `include`)
    if trimmed_line.starts_with("include") {
        return DirectiveType::Include;
    };

    // Check for price directive (line starts with P)
    if trimmed_line.starts_with('P') {
        return DirectiveType::Price;
    };

    // Check for transaction header (line starts with a date in YYYY-MM-DD)
    let first_ten: Vec<char> = trimmed_line.chars().take(10).collect();
    let mut is_transaction: bool = true;

    for i in 0..10 {
        // Check for dashes
        if i == 4 || i == 7 {
            if first_ten[i] != '-' {
                is_transaction = false;
                break;
            }
        } else if !first_ten[i].is_digit(10) {
            is_transaction = false;
            break;
        };
    }

    if is_transaction {
        return DirectiveType::Transaction;
    };

    // If none of the above, return None
    return DirectiveType::None;
}

/// Removes comments from a line and trims any leading and trailing whitespace.
/// Comments start with a semicolon `;` and continues to the end of the line.
pub fn trim_comments(line: &str) -> &str {
    return match line.find(';') {
        Some(index) => &line[..index].trim(),
        None => line.trim(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_comments() {
        assert_eq!(trim_comments(""), "");
        assert_eq!(
            trim_comments("This is a line; with a comment"),
            "This is a line"
        );
        assert_eq!(
            trim_comments("This is a line without a comment"),
            "This is a line without a comment"
        );
        assert_eq!(trim_comments("; This is a comment only"), "");
        assert_eq!(
            trim_comments("   This line has leading and trailing whitespace   ; and a comment"),
            "This line has leading and trailing whitespace"
        );
        assert_eq!(
            trim_comments("  surrounding whitespace no commment  "),
            "surrounding whitespace no commment"
        );
        assert_eq!(
            trim_comments("  surrounding whitespace with comment  ; and a comment"),
            "surrounding whitespace with comment"
        );
    }

    #[test]
    fn test_identify_directive_type() {
        assert_eq!(
            identify_directive_type("include \"other.journal\""),
            DirectiveType::Include
        );
        assert_eq!(
            identify_directive_type("P 2026-01-01 EUR 11.00 SEK"),
            DirectiveType::Price
        );
        assert_eq!(
            identify_directive_type("2026-01-01 * \"Transaction\""),
            DirectiveType::Transaction
        );
        assert_eq!(
            identify_directive_type("Unrecognized directive"),
            DirectiveType::None
        );
    }
}
