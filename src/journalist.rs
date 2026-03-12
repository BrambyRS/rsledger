mod input_parser;
mod double_entry;

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::Args;

// TODO: Set default config
pub fn new_journal(args: &Args) -> std::io::Result<()> {
    let journal_file = Path::new(&args.journal_path);

    // Create the directory if it doesn't exist
    if let Some(parent) = journal_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Create an empty journal file
    fs::File::create(journal_file)?;

    return Ok(());
}

/*
Add entry to journal file
*/
// TODO: Input validation, error handling, multi currency support, multi entry support, etc.
pub fn add_entry(args: &Args) -> std::io::Result<()> {
    // Get Journal path
    let journal_file = Path::new(&args.journal_path);
    if !journal_file.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Journal file {} not found.", journal_file.display())));
    }
    
    let date_str: String = input_parser::prompt_input("Date (YYYY-MM-DD): ")?;
    let description_str: String = input_parser::prompt_input("Description: ")?;
    let account_from_str: String = input_parser::prompt_input("From Account: ")?;
    let amount_from_str: String = input_parser::prompt_input("Amount: ")?;
    let account_to_str: String = input_parser::prompt_input("To Account: ")?;
    let amount_to_str: String = input_parser::prompt_input("Amount: ")?;

    let amount_from = match double_entry::TransactionAmount::from_str(&amount_from_str) {
        Some(val) => val,
        None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid amount format for 'From Account'.")),
    };

    // If it's empty, we can assume it's the negative of the amount from the 'From Account'.
    let amount_to: double_entry::TransactionAmount;
    if amount_to_str.len() == 0 {
        amount_to = -amount_from.clone();
    } else {
        amount_to = match double_entry::TransactionAmount::from_str(&amount_to_str) {
            Some(val) => val,
            None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid amount format for 'To Account'.")),
        };

        // If the currency is the same, validate that the amount is the negative of the amount from the 'From Account'.
        if amount_to.same_currency(&amount_from) && !amount_to.same_amount(&(-amount_from.clone())) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Amount for 'To Account' must be the negative of the amount from 'From Account' if the currency is the same."));
        }
    };

    let entry: double_entry::DoubleEntry = double_entry::DoubleEntry::new(
        date_str,
        description_str,
        account_from_str,
        amount_from,
        account_to_str,
        amount_to);

    // Append entry to journal file
    let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
    write!(file, "{entry}")?;

    Ok(())
}