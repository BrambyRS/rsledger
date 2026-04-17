pub mod csv_parser;
mod journal_parser;

use std::fs;
use std::io::{self, Write};

use crate::cli_utils;
use crate::commodity_value;
use crate::transaction;

/// NEW_JOURNAL
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

        let today: chrono::NaiveDate = chrono::Local::now().date_naive();

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
        let mut postings: Vec<transaction::posting::Posting> = Vec::new();

        loop {
            let posting_input: String = cli_utils::prompt_input(
                "Posting: ",
                &mut std::io::stdin().lock(),
                &mut std::io::stdout(),
            )?;
            if posting_input.len() == 0 {
                break;
            }
            let parts: Vec<&str> = posting_input.split_whitespace().collect();
            if parts.len() == 3 {
                let account_str: String = parts[0].to_string();
                let amount_str: String = parts[1..].join(" ");
                let amount = match commodity_value::CommodityValue::from_str(&amount_str) {
                    Ok(val) => Some(val),
                    Err(_) => {
                        println!(
                            "Invalid amount format. Please enter a valid commodity amount (e.g. '1000.00 SEK')."
                        );
                        continue;
                    }
                };
                postings.push(transaction::posting::Posting::new(account_str, amount));
            } else {
                println!(
                    "Invalid posting format. Please enter in the format '<account> <amount> <commodity>' (e.g. 'assets:bank 1000.00 SEK')."
                );
                continue;
            }
        }

        postings.push(transaction::posting::Posting::new(
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

/// ADD_ENTRY
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
    let date: chrono::NaiveDate = cli_utils::prompt_for_date(
        "Date (YYYY-MM-DD): ",
        "%Y-%m-%d",
        &mut std::io::stdin().lock(),
        &mut std::io::stdout(),
    )?;
    let description_str: String = cli_utils::prompt_input(
        "Description: ",
        &mut std::io::stdin().lock(),
        &mut std::io::stdout(),
    )?;
    let postings: Vec<transaction::posting::Posting> =
        cli_utils::prompt_for_postings(&mut std::io::stdin().lock(), &mut std::io::stdout())?;

    let entry: transaction::Transaction =
        transaction::Transaction::new(date, description_str, postings);

    // Append entry to journal file
    let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
    add_transaction_to_file(&mut file, &entry)?;

    Ok(())
}

/// ADD_TRANSACTION_TO_FILE
/// Appends a transaction to the journal file
///
/// Validates that the transaction is balanced before writing.
/// If the transaction fails validation, an error is returned and the journal file is not modified.
fn add_transaction_to_file(
    f: &mut fs::File,
    transaction: &transaction::Transaction,
) -> std::io::Result<()> {
    // Validate the transaction before writing to the journal
    if !transaction.validate() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Transaction is not balanced. Please ensure that the amounts sum to zero.",
        ));
    }

    write!(f, "\n{transaction}\n")?;
    Ok(())
}
