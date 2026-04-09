use crate::transaction;
use crate::transaction::commodity_value::CommodityValue;

use std::io::BufRead;
use std::io::Lines;
use std::iter::Peekable;

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

/// Parses a journal file and returns a vector of transactions.
pub fn parse_journal<R: BufRead>(
    journal_lines: &mut Peekable<Lines<R>>,
) -> Result<Vec<transaction::Transaction>, Box<dyn std::error::Error>> {
    let mut transactions: Vec<transaction::Transaction> = Vec::new();

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

        // If it starts with a date, it's a transaction header line.
        // Don't consume — let parse_transaction read the header itself.
        let first_token: &str = stripped_line.split_whitespace().next().unwrap_or("");
        if is_date(first_token) {
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
        } else {
            // Non-transaction line — consume and ignore
            journal_lines.next();
        }
    }

    return Ok(transactions);
}

fn parse_transaction<I: Iterator<Item = std::io::Result<String>>>(
    journal_lines: &mut I,
) -> Result<transaction::Transaction, Box<dyn std::error::Error>> {
    // Read first line to get the date and description
    let first_line = match journal_lines.next() {
        Some(Ok(line)) => line,
        Some(Err(e)) => return Err(Box::new(e)),
        None => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected end of file while reading transaction header.",
            )));
        }
    };
    let date_str = first_line[..10].to_string();
    let description = first_line[11..].trim().to_string();
    // Expect lines with leading whitespace to be postings
    // Stop either when the next line is empty,
    // when the next line starts with a non-whitespace character,
    // or when we reach the end of the file
    let mut postings: Vec<transaction::posting::Posting> = Vec::new();
    loop {
        let line = match journal_lines.next() {
            Some(Ok(l)) => l,
            Some(Err(e)) => return Err(Box::new(e)),
            None => break, // End of file
        };

        // Remove any comment
        let line: String = line.split(';').next().unwrap_or("").to_string();

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
        transaction::Transaction::new(date_str.to_string(), description, postings);

    Ok(transaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test_parse_single_transaction() {
        let file = File::open("test/single_transaction.journal").unwrap();
        let mut lines = BufReader::new(file).lines().peekable();
        let transactions = parse_journal(&mut lines).unwrap();

        assert_eq!(transactions.len(), 1);
        assert_eq!(
            format!("{}", transactions[0]),
            "2025-04-03 Test transaction\n\tassets:bank  -435 GBP\n\texpenses:travel:flights\n\n"
        );
    }

    #[test]
    fn test_parse_basic_transactions() {
        let file = File::open("test/basic_transactions.journal").unwrap();
        let mut lines = BufReader::new(file).lines().peekable();
        let transactions = parse_journal(&mut lines).unwrap();

        assert_eq!(transactions.len(), 15);

        // Opening balance: auto-balance posting (None amount) on the last line.
        assert_eq!(
            format!("{}", transactions[0]),
            "2026-01-01 Opening balance\n\
             \tassets:bank:checking  50000 SEK\n\
             \tassets:bank:savings  20000 SEK\n\
             \tassets:cash  2000 SEK\n\
             \tliabilities:credit-card  -5000 SEK\n\
             \tequity:opening-balance\n\n"
        );

        // Spotify subscription: second posting is auto-balance (no amount).
        assert_eq!(
            format!("{}", transactions[6]),
            "2026-02-01 Spotify AB | Monthly subscription\n\
             \texpenses:entertainment  119 SEK\n\
             \tassets:bank:checking\n\n"
        );
    }
}
