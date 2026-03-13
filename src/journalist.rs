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
    let account_1_str: String = input_parser::prompt_input("Account 1: ")?;
    let amount_1_str: String = input_parser::prompt_input("Amount: ")?;
    let account_2_str: String = input_parser::prompt_input("Account 2: ")?;
    let amount_2_str: String = input_parser::prompt_input("Amount: ")?;

    let amount_1 = match double_entry::TransactionAmount::from_str(&amount_1_str) {
        Some(val) => val,
        None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid amount format for 'Amount 1'.")),
    };

    // If it's empty, we can assume it's the negative of the amount from 'Account 1'.
    let amount_2: double_entry::TransactionAmount;
    if amount_2_str.len() == 0 {
        amount_2 = -amount_1.clone();
    } else {
        amount_2 = match double_entry::TransactionAmount::from_str(&amount_2_str) {
            Some(val) => val,
            None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid amount format for 'Amount 2'.")),
        };

        // If the currency is the same, validate that the amount is the negative of the amount from 'Account 1'.
        if amount_2.same_currency(&amount_1) && !amount_2.same_amount(&(-amount_1.clone())) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Amount for 'Account 2' must be the negative of the amount from 'Account 1' when the currency is the same."));
        }
    };

    let entry: double_entry::DoubleEntry = double_entry::DoubleEntry::new(
        date_str,
        description_str,
        account_1_str,
        amount_1,
        account_2_str,
        amount_2);

    // Append entry to journal file
    let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
    write!(file, "{entry}")?;

    Ok(())
}
