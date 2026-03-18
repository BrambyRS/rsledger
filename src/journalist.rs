mod input_parser;
mod transaction;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::Args;
use crate::config::Config;

// TODO: Set default config
pub fn new_journal(args: &Args, config: &Config) -> std::io::Result<()> {

    // Use the --path if it has been provided
    let journal_file: PathBuf = match get_journal_file_path(args, config) {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

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
pub fn add_entry(args: &Args, config: &Config) -> std::io::Result<()> {
    // Get Journal path
    let journal_file: PathBuf = match get_journal_file_path(args, config) {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

    if !journal_file.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Journal file {} not found.", journal_file.display())));
    }
    
    let date_str: String = input_parser::prompt_input("Date (YYYY-MM-DD): ")?;
    let description_str: String = input_parser::prompt_input("Description: ")?;
    let account_1_str: String = input_parser::prompt_input("Account 1: ")?;
    let amount_1_str: String = input_parser::prompt_input("Amount: ")?;
    let account_2_str: String = input_parser::prompt_input("Account 2: ")?;
    let amount_2_str: String = input_parser::prompt_input("Amount: ")?;

    let amount_1 = match transaction::commodity_value::CommodityValue::from_str(&amount_1_str) {
        Ok(val) => val,
        Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid amount format for 'Amount 1'.")),
    };

    // If it's empty, we can assume it's the negative of the amount from 'Account 1'.
    let amount_2: transaction::commodity_value::CommodityValue;
    if amount_2_str.len() == 0 {
        amount_2 = -amount_1.clone();
    } else {
        amount_2 = match transaction::commodity_value::CommodityValue::from_str(&amount_2_str) {
            Ok(val) => val,
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid amount format for 'Amount 2'.")),
        };

        // If the currency is the same, validate that the amount is the negative of the amount from 'Account 1'.
        if amount_2.same_commodity(&amount_1) && !amount_2.same_amount(&(-amount_1.clone())) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Amount for 'Account 2' must be the negative of the amount from 'Account 1' when the currency is the same."));
        }
    };

    let entry: transaction::DoubleEntry = transaction::DoubleEntry::new(
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

fn get_journal_file_path(args: &Args, config: &Config) -> std::io::Result<PathBuf> {
    // Use the --path if it has been provided
    if args.journal_path.len() > 0 {
        return Ok(PathBuf::from(&args.journal_path));
    } else {
        // Otherwise, use the default journal from config
        if config.default_journal_folder.len() == 0 || config.default_journal.len() == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No journal path provided and default journal not set in config."));
        }
        return Ok(Path::new(&config.default_journal_folder).join(&config.default_journal));
    }
}
