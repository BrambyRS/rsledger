mod input_parser;

use std::fs;
use std::io;
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

// TODO: Input validation, error handling, multi currency support, multi entry support, etc.
pub fn add_entry(args: &Args) -> std::io::Result<()> {
    // Get Journal path
    let journal_file = Path::new(&args.journal_path);
    if !journal_file.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Journal file {} not found.", journal_file.display())));
    }
    
    let date = input_parser::prompt_input("Date (YYYY-MM-DD): ")?;
    let description = input_parser::prompt_input("Description: ")?;
    let from_account = input_parser::prompt_input("From Account: ")?;
    let amount_from = input_parser::prompt_input("Amount: ")?;
    let to_account = input_parser::prompt_input("To Account: ")?;
    let amount_to = input_parser::prompt_input("Amount: ")?;

    // Append entry to journal file
    let entry: String = format!("{date} {description}\n\t{from_account} {amount_from}\n\t{to_account} {amount_to}\n\n");
    fs::OpenOptions::new().append(true).open(journal_file)?;
    fs::write(journal_file, entry)?;

    Ok(())
}