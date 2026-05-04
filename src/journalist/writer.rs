pub mod prices_importer;
pub mod transaction_importer;

use std::fs;
use std::io::Write;

use crate::price;
use crate::transaction;

/// NEW_JOURNAL
/// Creates an empty journal file at `journal_file`.
/// Intermediate directories are created automatically if they do not exist.
pub fn new_journal(journal_file: &std::path::PathBuf) -> crate::Result<()> {
    if let Some(parent) = journal_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::File::create(journal_file)?;
    Ok(())
}

/// ADD_TRANSACTION_TO_FILE
/// Validates that the transaction is balanced and appends it to the open file.
/// Returns an error without modifying the file if validation fails.
pub fn add_transaction_to_file(
    f: &mut fs::File,
    transaction: &transaction::Transaction,
) -> crate::Result<()> {
    if !transaction.validate() {
        return Err(crate::error::RsledgerError::ValidationError(
            "Invalid Transaction".to_string(),
            "Transaction is not balanced.".to_string(),
        ));
    }
    write!(f, "\n{transaction}\n")?;
    Ok(())
}

/// ADD_PRICE_TO_FILE
/// Appends a price directive to the open file.
pub fn add_price_to_file(f: &mut fs::File, price: &price::PriceDirective) -> crate::Result<()> {
    write!(f, "{price}\n")?;
    Ok(())
}
