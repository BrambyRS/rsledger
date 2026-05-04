use crate::commodity_value::CommodityValue;
use crate::journalist::Journal;
use crate::price;
use crate::transaction;

use std::io::BufRead;
use std::io::Lines;
use std::iter::Peekable;

enum DirectiveType {
    Transaction,
    Price,
    None,
}

impl DirectiveType {
    /// SCAN LINE
    /// Scans a line to see what type of directive it is.
    ///
    /// Takes a line of input and determines what type of directive it is.
    /// Currently only distinguishes between transaction directives and price directives.
    /// Transaction directives are identified as starting with a date in the format YYYY-MM-DD.
    /// Price directives are identified as starting with the word "P".
    fn scan_line(line: &str) -> DirectiveType {
        let first_token: &str = line.split_whitespace().next().unwrap_or("");
        if is_date(first_token) {
            return DirectiveType::Transaction;
        } else if first_token == "P" {
            return DirectiveType::Price;
        }

        // Return None otherwise
        return DirectiveType::None;
    }
}

/// IS_DATE
/// Checks if a string is in the format YYYY-MM-DD.
///
/// This is a basic check to identify the date format YYYY-MM-DD.
/// It does not check that the date itself is valid, that is left to
/// the function that parses the actual data.
///
/// is_date("2023-01-01") -> true
/// is_date("2023-1-1") -> false
fn is_date(s: &str) -> bool {
    // Check if the string is in the format YYYY-MM-DD
    if s.len() != 10 {
        return false;
    }

    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }

    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    return true;
}

/// PARSE_JOURNAL
/// Parses a journal file and returns a vector of transactions.
pub fn parse_journal<R: BufRead>(journal_lines: &mut Peekable<Lines<R>>) -> crate::Result<Journal> {
    let mut transactions: Vec<transaction::Transaction> = Vec::new();
    let mut prices: Vec<price::PriceDirective> = Vec::new();

    // Iterate over lines in the file, looking for transactions
    loop {
        // Peek at the next line without consuming it
        let raw_line = match journal_lines.peek() {
            Some(Ok(line)) => line.clone(),
            Some(Err(_)) => {
                journal_lines.next();
                continue;
            }
            None => break, // End of file
        };

        // Remove trailing whitespace and comments (anything after a ';' character)
        let stripped_line = raw_line
            .split(';')
            .next()
            .unwrap_or("")
            .trim_end()
            .to_string();

        // Check that the line isn't empty after stripping; consume and skip if so
        if stripped_line.is_empty() {
            journal_lines.next();
            continue;
        }

        match DirectiveType::scan_line(&stripped_line) {
            DirectiveType::Transaction => {
                // Get the transaction starting at this line
                let transaction: transaction::Transaction = match parse_transaction(journal_lines) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!(
                            "Error parsing transaction starting at line '{}': {}",
                            stripped_line, e
                        );
                        continue;
                    }
                };
                transactions.push(transaction);

                // parse_transaction will consume the lines corresponding to the transaction
                // So there is no need to manually advance the iterator at the end
            }
            DirectiveType::Price => {
                // Get the price directive starting at this line
                let price_directive: price::PriceDirective =
                    match price::PriceDirective::from_str(&stripped_line) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!(
                                "Error parsing price directive at line '{}': {}",
                                stripped_line, e
                            );
                            journal_lines.next();
                            continue;
                        }
                    };
                prices.push(price_directive);
                journal_lines.next(); // Consume the line after parsing the price directive
            }
            DirectiveType::None => {
                // Move to next line
                journal_lines.next();
            }
        }
    }

    return Ok(Journal {
        transactions,
        prices,
    });
}

/// PARSE_TRANSACTION
fn parse_transaction<I: Iterator<Item = std::io::Result<String>>>(
    journal_lines: &mut I,
) -> crate::Result<transaction::Transaction> {
    // Read first line to get the date and description
    let first_line = match journal_lines.next() {
        Some(Ok(line)) => line,
        Some(Err(e)) => {
            return Err(crate::error::RsledgerError::ParseError(
                "Journal Parse".to_string(),
                format!("Error reading line: {}", e),
            ));
        }
        None => {
            return Err(crate::error::RsledgerError::ParseError(
                "Journal Parse".to_string(),
                "Unexpected end of file while reading transaction header.".to_string(),
            ));
        }
    };
    // Trim any trailing comments and whitespace
    let first_line = first_line
        .split(';')
        .next()
        .expect("Unexpected comment in transaction header line.")
        .trim_end()
        .to_string();
    let date = match chrono::NaiveDate::parse_from_str(&first_line[..10], "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            return Err(crate::error::RsledgerError::ParseError(
                "Journal Parse".to_string(),
                format!("Invalid date format in transaction header: {e}"),
            ));
        }
    };
    let description = first_line[11..].trim().to_string();
    // Expect lines with leading whitespace to be postings
    // Stop either when the next line is empty,
    // when the next line starts with a non-whitespace character,
    // or when we reach the end of the file
    let mut postings: Vec<transaction::posting::Posting> = Vec::new();
    loop {
        let line = match journal_lines.next() {
            Some(Ok(l)) => l,
            Some(Err(e)) => {
                return Err(crate::error::RsledgerError::ParseError(
                    "Journal Parse".to_string(),
                    format!("Error reading line: {e}"),
                ));
            }
            None => break, // End of file
        };

        // Remove any comment
        let line: String = line
            .split(';')
            .next()
            .expect("Unexpected comment in posting line.")
            .trim_end()
            .to_string();

        // Skip if the line is empty or does not start with whitespace
        if line.trim().len() == 0 {
            break;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }

        // Parse the posting
        // Split into account and amount parts
        let mut parts = line.trim().split_whitespace();
        let account_str = parts.next().unwrap_or("");
        let amount_str = parts.collect::<Vec<&str>>().join(" ");
        let amount = if amount_str.is_empty() {
            None
        } else {
            match CommodityValue::from_str(&amount_str) {
                Ok(val) => Some(val),
                Err(_) => {
                    eprintln!(
                        "Invalid amount format in posting '{}'. Skipping this posting.",
                        line.trim()
                    );
                    continue;
                }
            }
        };
        let posting = transaction::posting::Posting::new(account_str.to_string(), amount);

        postings.push(posting);
    }

    // Create the transaction
    let transaction: transaction::Transaction =
        transaction::Transaction::new(date, description, postings);

    Ok(transaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    fn test_journal(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("journals")
            .join(name)
    }

    #[test]
    fn test_parse_basic_transactions() {
        let file = File::open(test_journal("basic_transactions.journal")).unwrap();
        let mut lines = BufReader::new(file).lines().peekable();
        let journal = parse_journal(&mut lines).unwrap();
        let transactions = &journal.transactions;

        assert_eq!(transactions.len(), 15);

        // Opening balance: auto-balance posting (None amount) on the last line.
        assert_eq!(
            format!("{}", transactions[0]),
            "2026-01-01 Opening balance\n\
             \tassets:bank:checking  50000 SEK\n\
             \tassets:bank:savings  20000 SEK\n\
             \tassets:cash  2000 SEK\n\
             \tliabilities:credit-card  -5000 SEK\n\
             \tequity:opening-balance"
        );

        // Spotify subscription: second posting is auto-balance (no amount).
        assert_eq!(
            format!("{}", transactions[6]),
            "2026-02-01 Spotify AB | Monthly subscription\n\
             \texpenses:entertainment  119 SEK\n\
             \tassets:bank:checking"
        );
    }

    #[test]
    fn test_parse_prices_only() {
        let file = File::open(test_journal("prices_only.journal")).unwrap();
        let mut lines = BufReader::new(file).lines().peekable();
        let journal = parse_journal(&mut lines).unwrap();

        assert_eq!(journal.transactions.len(), 0);
        assert_eq!(journal.prices.len(), 3);

        // Simple unquoted commodity on both sides; trailing zero in 10.50 is stripped.
        assert_eq!(
            format!("{}", journal.prices[0]),
            "P 2026-01-01 USD 10.5 SEK"
        );
        // Quoted COMMODITY_1 (contains a space).
        assert_eq!(
            format!("{}", journal.prices[1]),
            "P 2026-01-15 \"Gold Bar\" 1234.56 SEK"
        );
        // Quoted COMMODITY_2 (contains a space).
        assert_eq!(
            format!("{}", journal.prices[2]),
            "P 2026-01-15 USD 8.75 \"Silver Coin\""
        );
    }

    #[test]
    fn test_parse_transactions_and_prices() {
        let file = File::open(test_journal("transactions_and_prices.journal")).unwrap();
        let mut lines = BufReader::new(file).lines().peekable();
        let journal = parse_journal(&mut lines).unwrap();

        assert_eq!(journal.transactions.len(), 2);
        assert_eq!(journal.prices.len(), 2);

        // Price before any transaction.
        assert_eq!(
            format!("{}", journal.prices[0]),
            "P 2026-01-01 USD 10.5 SEK"
        );
        // Price between two transactions, with a quoted commodity.
        assert_eq!(
            format!("{}", journal.prices[1]),
            "P 2026-01-15 \"Gold Bar\" 1234.56 SEK"
        );

        // First transaction (appears after the first price directive).
        assert_eq!(
            format!("{}", journal.transactions[0]),
            "2026-01-25 * Salary\n\
             \tassets:bank:checking  35000 SEK\n\
             \tincome:salary  -35000 SEK"
        );
        // Second transaction (appears after the second price directive).
        assert_eq!(
            format!("{}", journal.transactions[1]),
            "2026-02-01 Subscription\n\
             \texpenses:entertainment  119 SEK\n\
             \tassets:bank:checking  -119 SEK"
        );
    }
}
