use crate::commodity_value;
use crate::transaction;

/// Prints `prompt` to stdout, flushes the buffer, reads a line from stdin,
/// and returns the trimmed result.
pub fn prompt_input(
    prompt: &str,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> std::io::Result<String> {
    write!(writer, "{prompt}")?;
    writer.flush()?;

    let mut input = String::new();
    reader.read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn prompt_for_date(
    prompt: &str,
    format: &str,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> std::io::Result<chrono::NaiveDate> {
    loop {
        let date_input = prompt_input(prompt, reader, writer)?;
        match chrono::NaiveDate::parse_from_str(&date_input, format) {
            Ok(date) => return Ok(date),
            Err(_) => {
                writeln!(
                    writer,
                    "Invalid date format. Please enter a date in the format YYYY-MM-DD (e.g. 2024-03-15)."
                )?;
                continue;
            }
        }
    }
}

/// Prompts the user to enter an account name, and returns it as a string.
pub fn prompt_for_account(
    prompt: &str,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> std::io::Result<String> {
    // Loop until the user enters a non-empty account name
    loop {
        let account_input = prompt_input(prompt, reader, writer)?;
        if account_input.is_empty() {
            writeln!(
                writer,
                "Account name cannot be empty. Please enter a valid account name (e.g. 'assets:bank')."
            )?;
            continue;
        }
        return Ok(account_input);
    }
}

/// Prompts the user to enter one or more postings, and returns them as a vector of [`transaction::posting::Posting`].
///
/// Postings can be entered as:
/// - `<account>` — amount will be inferred (auto-balancing posting)
/// - `<account> <amount> <commodity>` — e.g. `expenses:food 50.00 SEK`
/// An empty line terminates posting input.
pub fn prompt_for_postings(
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> std::io::Result<Vec<transaction::posting::Posting>> {
    let mut postings: Vec<transaction::posting::Posting> = Vec::new();

    loop {
        let posting_input: String =
            prompt_input("Posting (ex. 'expenses:food 500 SEK'): ", reader, writer)?;
        if posting_input.is_empty() {
            break;
        }
        let parts: Vec<&str> = posting_input.split_whitespace().collect();
        if parts.len() == 1 {
            let account_str: String = parts[0].to_string();
            postings.push(transaction::posting::Posting::new(account_str, None));
        } else if parts.len() == 3 {
            let account_str: String = parts[0].to_string();
            let amount_str: String = parts[1..].join(" ");
            let amount: Option<commodity_value::CommodityValue> =
                match commodity_value::CommodityValue::from_str(&amount_str) {
                    Ok(val) => Some(val),
                    Err(_) => {
                        writeln!(
                            writer,
                            "Invalid amount format. Please enter a valid commodity amount (e.g. '500.00 SEK')."
                        )?;
                        continue;
                    }
                };
            postings.push(transaction::posting::Posting::new(account_str, amount));
        } else {
            writeln!(
                writer,
                "Invalid posting format. Please enter in the format '<account>' or '<account> <amount> <commodity>' (e.g. 'assets:bank 500.00 SEK')."
            )?;
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
        assert_eq!(result, "assets:bank");
    }

    #[test]
    fn test_prompt_for_account_reprompts_on_empty() {
        let mut input = Cursor::new(b"\nassets:bank\n");
        let mut output = Vec::new();
        let result = prompt_for_account("Account: ", &mut input, &mut output).unwrap();
        assert_eq!(result, "assets:bank");
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
        assert_eq!(postings[0].get_account(), "expenses:food");
        assert_eq!(postings[0].get_amount().unwrap().to_string(), "500 SEK");
    }

    #[test]
    fn test_prompt_for_postings_multiple_postings() {
        let mut input = Cursor::new(b"expenses:food 500 SEK\nassets:bank -500 SEK\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 2);
        assert_eq!(postings[0].get_account(), "expenses:food");
        assert_eq!(postings[1].get_account(), "assets:bank");
    }

    #[test]
    fn test_prompt_for_postings_decimal_amount() {
        let mut input = Cursor::new(b"expenses:food 123.45 GBP\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 1);
        assert_eq!(postings[0].get_amount().unwrap().to_string(), "123.45 GBP");
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
        assert_eq!(postings[0].get_account(), "assets:bank");
        assert!(postings[0].get_amount().is_none());
    }

    #[test]
    fn test_prompt_for_postings_mixed_none_and_valued() {
        let mut input = Cursor::new(b"expenses:food 500 SEK\nassets:bank\n\n");
        let mut output = Vec::new();
        let postings = prompt_for_postings(&mut input, &mut output).unwrap();
        assert_eq!(postings.len(), 2);
        assert!(postings[0].get_amount().is_some());
        assert!(postings[1].get_amount().is_none());
    }
}
