use crate::cli::utils;
use crate::journalist;
use crate::transaction;

use std::fs;
use std::io::{self, BufRead, Write};

/// Interactively prompts for a date, description, and postings, then appends
/// the resulting transaction to the journal file.
pub fn run_add(
    journal_file: &std::path::PathBuf,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> crate::Result<()> {
    if !journal_file.exists() {
        return Err(crate::error::RsledgerError::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Journal file {} not found.", journal_file.display()),
        )));
    }

    writeln!(
        writer,
        "\nAdding transaction entry to journal: {}",
        journal_file.display()
    )?;
    writeln!(
        writer,
        "Enter postings on the format '<account> <amount> <commodity>'"
    )?;
    writeln!(
        writer,
        "example: 'expenses:food 50.00 SEK') such that all are balanced."
    )?;
    writeln!(writer, "If you leave an amount blank, it will be inferred.")?;
    writeln!(
        writer,
        "Keep adding as many postings as you want, then enter an empty line to finish.\n"
    )?;

    let date: chrono::NaiveDate =
        utils::prompt_for_date("Date (YYYY-MM-DD): ", "%Y-%m-%d", reader, writer)?;
    let description_str: String = utils::prompt_input("Description: ", reader, writer)?;
    let postings: Vec<transaction::posting::Posting> = utils::prompt_for_postings(reader, writer)?;

    let entry = transaction::Transaction::new(date, description_str, postings);

    let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
    journalist::writer::add_transaction_to_file(&mut file, &entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TempJournal(std::path::PathBuf);

    impl TempJournal {
        fn new_empty() -> Self {
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("rsledger_add_test_{}.journal", id));
            std::fs::write(&path, "").unwrap();
            TempJournal(path)
        }
        fn path(&self) -> &std::path::PathBuf {
            &self.0
        }
    }

    impl Drop for TempJournal {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn appends_transaction_to_journal() {
        let tmp = TempJournal::new_empty();
        let input = b"2026-01-15\nGroceries\nexpenses:food 50 SEK\nassets:bank -50 SEK\n\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut output = Vec::new();
        run_add(tmp.path(), &mut reader, &mut output).unwrap();
        let contents = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(contents.contains("Groceries"));
        assert!(contents.contains("expenses:food"));
        assert!(contents.contains("assets:bank"));
    }

    #[test]
    fn returns_error_for_missing_journal() {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("rsledger_add_missing_{}.journal", id));
        let mut reader = Cursor::new(b"");
        let mut output = Vec::new();
        let result = run_add(&path, &mut reader, &mut output);
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_for_unbalanced_transaction() {
        let tmp = TempJournal::new_empty();
        // Two postings both with explicit amounts that don't balance
        let input = b"2026-01-15\nUnbalanced\nexpenses:food 50 SEK\nassets:bank -99 SEK\n\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut output = Vec::new();
        let result = run_add(tmp.path(), &mut reader, &mut output);
        assert!(result.is_err());
    }
}
