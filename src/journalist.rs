mod csv_parser;
mod journal_parser;

use std::fs;
use std::io::{self, Write};

use crate::transaction;

/// Prints `prompt` to stdout, flushes the buffer, reads a line from stdin,
/// and returns the trimmed result.
fn prompt_input(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Creates a new journal file at the path resolved from `args` and `config`.
/// Intermediate directories are created automatically if they do not exist.
/// If the flag --open is provided, an opening transaction with the current date
/// is also added to the journal.
pub fn new_journal(journal_file: &std::path::PathBuf, create_opening: bool) -> std::io::Result<()> {
    // Create the directory if it doesn't exist
    if let Some(parent) = journal_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Create an empty journal file
    fs::File::create(journal_file)?;

    if create_opening {
        // If --open flag is provided, add an opening transaction with the current date

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        println!("\nCreating opening transaction at {today} with description 'Opening balance'.");
        println!(
            "Enter the opening balance postings for each account on the format '<account> <amount> <commodity>'"
        );
        println!(
            "example: 'assets:bank 1000.00 SEK' (for a positive balance) or 'assets:bank -1000.00 SEK' (for a negative balance)."
        );
        println!(
            "Keep adding as many postings as you want, and then enter an empty line to finish the transaction.\n"
        );
        println!(
            "The transaction will be balanced automatically against equity:opening-balance.\n"
        );
        let mut postings: Vec<transaction::Posting> = Vec::new();

        loop {
            let posting_input: String = prompt_input("Posting: ")?;
            if posting_input.len() == 0 {
                break;
            }
            let parts: Vec<&str> = posting_input.split_whitespace().collect();
            if parts.len() == 3 {
                let account_str: String = parts[0].to_string();
                let amount_str: String = parts[1..].join(" ");
                let amount = match transaction::commodity_value::CommodityValue::from_str(
                    &amount_str,
                ) {
                    Ok(val) => Some(val),
                    Err(_) => {
                        println!(
                            "Invalid amount format. Please enter a valid commodity amount (e.g. '1000.00 SEK')."
                        );
                        continue;
                    }
                };
                postings.push(transaction::Posting::new(account_str, amount));
            } else {
                println!(
                    "Invalid posting format. Please enter in the format '<account> <amount> <commodity>' (e.g. 'assets:bank 1000.00 SEK')."
                );
                continue;
            }
        }

        postings.push(transaction::Posting::new(
            "equity:opening-balance".to_string(),
            None,
        ));

        let opening_transaction =
            transaction::Transaction::new(today, "Opening balance".to_string(), postings);

        // Append opening transaction to journal file
        let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
        write!(file, "{opening_transaction}")?;
    }

    return Ok(());
}

/// Interactively prompts the user for a date, description, and one or more postings,
/// then appends the resulting [`transaction::Transaction`] to the journal file.
///
/// Postings can be entered as:
/// - `<account>` — amount will be inferred (auto-balancing posting)
/// - `<account> <amount> <commodity>` — e.g. `expenses:food 50.00 SEK`
///
/// An empty line terminates posting input.
pub fn add_entry(journal_file: &std::path::PathBuf) -> std::io::Result<()> {
    if !journal_file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Journal file {} not found.", journal_file.display()),
        ));
    }

    println!("\nAdding entry to journal: {}", journal_file.display());
    println!("Enter postings on the format '<account> <amount> <commodity>'");
    println!("example: 'expenses:food 50.00 SEK') such that all are balanced.");
    println!("If you leave an amount blank, it will be inferred.");
    println!(
        "Keep adding as many postings as you want, and then enter an empty line to finish the transaction.\n"
    );
    let date_str: String = prompt_input("Date (YYYY-MM-DD): ")?;
    let description_str: String = prompt_input("Description: ")?;
    let mut postings: Vec<transaction::Posting> = Vec::new();

    loop {
        let posting_input: String = prompt_input("Posting: ")?;
        if posting_input.len() == 0 {
            break;
        }
        let parts: Vec<&str> = posting_input.split_whitespace().collect();
        if parts.len() == 1 {
            let account_str: String = parts[0].to_string();
            let amount: Option<transaction::commodity_value::CommodityValue> = None;

            postings.push(transaction::Posting::new(account_str, amount));
        } else if parts.len() == 3 {
            let account_str: String = parts[0].to_string();
            let amount_str: String = parts[1..].join(" ");
            let amount = match transaction::commodity_value::CommodityValue::from_str(&amount_str) {
                Ok(val) => Some(val),
                Err(_) => {
                    println!(
                        "Invalid amount format. Please enter a valid commodity amount (e.g. '50.00 SEK')."
                    );
                    continue;
                }
            };
            postings.push(transaction::Posting::new(account_str, amount));
        } else {
            println!(
                "Invalid posting format. Please enter in the format '<account> <amount> <commodity>' (e.g. 'expenses:food 50.00 SEK') or '<account>' (e.g. 'assets:bank' for an auto-balancing posting)."
            );
            continue;
        }
    }

    let entry: transaction::Transaction =
        transaction::Transaction::new(date_str, description_str, postings);

    // Validate the transaction before writing to the journal
    if !entry.validate() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Transaction is not balanced. Please ensure that the amounts sum to zero.",
        ));
    }

    // Append entry to journal file
    let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
    write!(file, "{entry}")?;

    Ok(())
}
