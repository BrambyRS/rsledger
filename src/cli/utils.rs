use crate::journal;

/// Prints `prompt` to stdout, flushes the buffer, reads a line from stdin,
/// and returns the trimmed result.
pub fn prompt_input(
    prompt: &str,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> crate::Result<String> {
    match write!(writer, "{prompt}") {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }
    match writer.flush() {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }

    let mut input = String::new();
    match reader.read_line(&mut input) {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }
    return Ok(input.trim().to_string());
}

/// Prompts the user to enter a date in a format specified in the argument and returns a chrono::NaiveDate
pub fn prompt_for_date(
    prompt: &str,
    format: &str,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> crate::Result<chrono::NaiveDate> {
    loop {
        let date_input = match prompt_input(prompt, reader, writer) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        match chrono::NaiveDate::parse_from_str(&date_input, format) {
            Ok(date) => return Ok(date),
            Err(_) => {
                match writeln!(
                    writer,
                    "Invalid date format. Please enter a date in the format YYYY-MM-DD (e.g. 2024-03-15)."
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
                }
                continue;
            }
        }
    }
}

/// Prompts the user for a commodity value
pub fn prompt_for_value(
    prompt: &str,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> crate::Result<journal::commodity_value::CommodityValue> {
    loop {
        let value_input = match prompt_input(prompt, reader, writer) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        match journal::commodity_value::CommodityValue::from_str(&value_input) {
            Ok(value) => return Ok(value),
            Err(_) => {
                match writeln!(
                    writer,
                    "Invalid commodity value format. Please enter a valid commodity value (e.g. '500.00 SEK')."
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
                }
                continue;
            }
        };
    }
}

/// Prompts the user to enter an account name, and returns it as a validated Account.
pub fn prompt_for_account(
    prompt: &str,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> crate::Result<journal::account::Account> {
    loop {
        let account_input = match prompt_input(prompt, reader, writer) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        if account_input.is_empty() {
            match writeln!(
                writer,
                "Account name cannot be empty. Please enter a valid account name (e.g. 'assets:bank')."
            ) {
                Ok(_) => {}
                Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
            }
            continue;
        }
        match journal::account::Account::from_str(&account_input) {
            Ok(account) => return Ok(account),
            Err(_) => {
                match writeln!(
                    writer,
                    "Invalid account '{}'. Root must be one of: assets, liabilities, equity, income, expenses (e.g. 'assets:bank').",
                    account_input
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
                }
                continue;
            }
        }
    }
}

/// Prompts the user to enter one or more postings, and returns them as a vector of [`journal::transaction::posting::Posting`].
///
/// Postings can be entered as:
/// - `<account>` — amount will be inferred (auto-balancing posting)
/// - `<account> <amount> <commodity>` — e.g. `expenses:food 50.00 SEK`
/// An empty line terminates posting input.
pub fn prompt_for_postings(
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> crate::Result<Vec<journal::transaction::posting::Posting>> {
    let mut postings: Vec<journal::transaction::posting::Posting> = Vec::new();

    loop {
        let posting_input: String =
            match prompt_input("Posting (ex. 'expenses:food 500 SEK'): ", reader, writer) {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
        if posting_input.is_empty() {
            break;
        }
        let parts: Vec<&str> = posting_input.split_whitespace().collect();
        if parts.len() == 1 {
            let account = match journal::account::Account::from_str(parts[0]) {
                Ok(a) => a,
                Err(_) => {
                    match writeln!(
                        writer,
                        "Invalid account name '{}'. Root must be one of: assets, liabilities, equity, income, expenses (e.g. 'assets:bank').",
                        parts[0]
                    ) {
                        Ok(_) => {}
                        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
                    }
                    continue;
                }
            };
            postings.push(journal::transaction::posting::Posting::new(account, None));
        } else if parts.len() == 3 {
            let account = match journal::account::Account::from_str(parts[0]) {
                Ok(a) => a,
                Err(_) => {
                    match writeln!(
                        writer,
                        "Invalid account name '{}'. Root must be one of: assets, liabilities, equity, income, expenses (e.g. 'assets:bank').",
                        parts[0]
                    ) {
                        Ok(_) => {}
                        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
                    }
                    continue;
                }
            };
            let amount_str: String = parts[1..].join(" ");
            let amount: Option<journal::commodity_value::CommodityValue> =
                match journal::commodity_value::CommodityValue::from_str(&amount_str) {
                    Ok(val) => Some(val),
                    Err(_) => {
                        match writeln!(
                            writer,
                            "Invalid amount format. Please enter a valid commodity amount (e.g. '500.00 SEK')."
                        ) {
                            Ok(_) => {}
                            Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
                        }
                        continue;
                    }
                };
            postings.push(journal::transaction::posting::Posting::new(account, amount));
        } else {
            match writeln!(
                writer,
                "Invalid posting format. Please enter in the format '<account>' or '<account> <amount> <commodity>' (e.g. 'assets:bank 500.00 SEK')."
            ) {
                Ok(_) => {}
                Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
            }
            continue;
        }
    }

    return Ok(postings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // -------------------------------------------------------------------------
    // prompt_input tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_prompt_input_returns_line() {
        let mut input = Cursor::new(b"hello\n");
        let mut output = Vec::new();
        let result = prompt_input("Enter: ", &mut input, &mut output).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_prompt_input_writes_prompt() {
        let mut input = Cursor::new(b"hello\n");
        let mut output = Vec::new();
        prompt_input("Enter: ", &mut input, &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "Enter: ");
    }

    #[test]
    fn test_prompt_input_trims_surrounding_whitespace() {
        let mut input = Cursor::new(b"  spaces  \n");
        let mut output = Vec::new();
        let result = prompt_input("Enter: ", &mut input, &mut output).unwrap();
        assert_eq!(result, "spaces");
    }

    #[test]
    fn test_prompt_input_empty_line() {
        let mut input = Cursor::new(b"\n");
        let mut output = Vec::new();
        let result = prompt_input("Enter: ", &mut input, &mut output).unwrap();
        assert_eq!(result, "");
    }

    // -------------------------------------------------------------------------
    // prompt_for_account tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_prompt_for_account_valid_input() {
        let mut input = Cursor::new(b"assets:bank\n");
        let mut output = Vec::new();
        let result = prompt_for_account("Account: ", &mut input, &mut output).unwrap();
        assert_eq!(result.to_string(), "assets:bank");
    }

    #[test]
    fn test_prompt_for_account_reprompts_on_empty() {
        let mut input = Cursor::new(b"\nassets:bank\n");
        let mut output = Vec::new();
        let result = prompt_for_account("Account: ", &mut input, &mut output).unwrap();
        assert_eq!(result.to_string(), "assets:bank");
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Account name cannot be empty"),
            "expected reprompt message in output"
        );
    }

    // -------------------------------------------------------------------------
    // prompt_for_postings tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_prompt_for_postings_empty_returns_empty_vec() {
        let mut input = Cursor::new(b"\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert!(postings.is_empty());
    }

    #[test]
    fn test_prompt_for_postings_single_valid_posting() {
        let mut input = Cursor::new(b"expenses:food 500 SEK\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 1);
        assert_eq!(postings[0].account().to_string(), "expenses:food");
        assert_eq!(postings[0].amount().unwrap().to_string(), "500 SEK");
    }

    #[test]
    fn test_prompt_for_postings_multiple_postings() {
        let mut input = Cursor::new(b"expenses:food 500 SEK\nassets:bank -500 SEK\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 2);
        assert_eq!(postings[0].account().to_string(), "expenses:food");
        assert_eq!(postings[1].account().to_string(), "assets:bank");
    }

    #[test]
    fn test_prompt_for_postings_decimal_amount() {
        let mut input = Cursor::new(b"expenses:food 123.45 GBP\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 1);
        assert_eq!(postings[0].amount().unwrap().to_string(), "123.45 GBP");
    }

    #[test]
    fn test_prompt_for_postings_invalid_format_reprompts() {
        // 4-token line is invalid, then a valid posting, then empty line to finish
        let mut input = Cursor::new(b"too many tokens here\nexpenses:food 500 SEK\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 1);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Invalid posting format"),
            "expected invalid format message in output"
        );
    }

    #[test]
    fn test_prompt_for_postings_invalid_amount_reprompts() {
        let mut input = Cursor::new(b"expenses:food notanumber SEK\nexpenses:food 500 SEK\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 1);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Invalid amount format"),
            "expected invalid amount message in output"
        );
    }

    #[test]
    fn test_prompt_for_postings_account_only_has_none_amount() {
        let mut input = Cursor::new(b"assets:bank\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 1);
        assert_eq!(postings[0].account().to_string(), "assets:bank");
        assert!(postings[0].amount().is_none());
    }

    #[test]
    fn test_prompt_for_postings_mixed_none_and_valued() {
        let mut input = Cursor::new(b"expenses:food 500 SEK\nassets:bank\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 2);
        assert!(postings[0].amount().is_some());
        assert!(postings[1].amount().is_none());
    }
}
