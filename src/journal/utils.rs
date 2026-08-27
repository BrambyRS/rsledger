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

    // A transaction header requires exactly 10 leading characters (YYYY-MM-DD).
    // Return early for lines that are too short to avoid an index-out-of-bounds panic.
    if first_ten.len() < 10 {
        return DirectiveType::None;
    }

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

/// Extracts the path from an include directive.
///
/// # Examples
/// ```
/// let line = "include other.journal";
/// let path = match parse_include(line) {
///     Ok(p) => p,
///     Err(e) => panic!("Failed to parse include directive: {}", e),
/// };
/// ```
pub fn parse_include(line: &str) -> crate::Result<std::path::PathBuf> {
    let stripped_line = trim_comments(line);

    // Skip the first 7 characters ("include") and then trim any leading/trainling whitespace
    let path_str: &str = stripped_line[7..].trim();

    // The path may or may not be enclosed in quotes. If it is, remove the quotes.
    if path_str.starts_with('"') && path_str.ends_with('"') {
        return Ok(std::path::PathBuf::from(&path_str[1..path_str.len() - 1]));
    } else {
        return Ok(std::path::PathBuf::from(path_str));
    };
}

pub fn parse_transaction_header(line: &str) -> crate::Result<(chrono::NaiveDate, String)> {
    let stripped_line = trim_comments(line);

    // The first 10 characters should be the date in YYYY-MM-DD format
    let date_str = &stripped_line[..10];
    // Assume the rest is the description, trim any leading/trailing whitespace
    let description = stripped_line[10..].trim().to_string();

    // Parse the date string into a NaiveDate
    match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(date) => Ok((date, description)),
        Err(e) => Err(crate::error::RsledgerError::ParseError(
            line.to_string(),
            format!("Failed to parse transaction date: {}", e),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------
    // Tests for the `trim_comments`
    // ---------------------------------------------------

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

    // ---------------------------------------------------
    // Tests for the `identify_directive_type`
    // ---------------------------------------------------

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

    // ---------------------------------------------------
    // Tests for the `include` parser
    // ---------------------------------------------------

    #[test]
    fn test_parse_include() {
        let line = "include other.journal";
        let result = parse_include(line).unwrap();
        assert_eq!(result, std::path::PathBuf::from("other.journal"));
    }

    #[test]
    fn test_include_with_comments() {
        let line = "include other.journal ; this is a comment";
        let result = parse_include(line).unwrap();
        assert_eq!(result, std::path::PathBuf::from("other.journal"));
    }

    #[test]
    fn test_include_with_whitespace() {
        let line = "include    other.journal   ";
        let result = parse_include(line).unwrap();
        assert_eq!(result, std::path::PathBuf::from("other.journal"));
    }

    #[test]
    fn test_include_with_whitespace_in_path() {
        let line = "include    other journal.journal   ";
        let result = parse_include(line).unwrap();
        assert_eq!(result, std::path::PathBuf::from("other journal.journal"));
    }

    #[test]
    fn test_include_with_quotes() {
        let line = "include \"other.journal\"";
        let result = parse_include(line).unwrap();
        assert_eq!(result, std::path::PathBuf::from("other.journal"));
    }

    // ---------------------------------------------------
    // Tests for the `parse_transaction_header`
    // ---------------------------------------------------

    #[test]
    fn test_parse_transaction_header() {
        let line = "2026-01-01 * \"Transaction\"";
        let result = parse_transaction_header(line).unwrap();
        let expected_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_eq!(result, (expected_date, "* \"Transaction\"".to_string()));
    }

    #[test]
    fn test_parse_transaction_header_with_comments() {
        let line = "2026-01-01 * \"Transaction\" ; this is a comment";
        let result = parse_transaction_header(line).unwrap();
        let expected_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_eq!(result, (expected_date, "* \"Transaction\"".to_string()));
    }

    #[test]
    fn test_parse_transaction_header_invalid_date() {
        let line = "2026-13-01 * \"Transaction\"";
        let result = parse_transaction_header(line);
        assert!(result.is_err());
    }
}
