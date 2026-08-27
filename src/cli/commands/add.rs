use crate::cli::utils;
use crate::journal;

use std::io::{self, BufRead, Write};

/// Interactively prompts for a date, description, and postings, then appends
/// the resulting transaction to the journal file.
pub fn run_add(
    mut journal_file: journal::JournalFile,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> crate::Result<()> {
    if !journal_file.path().exists() {
        return Err(crate::error::RsledgerError::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Journal file {} not found.", journal_file.path().display()),
        )));
    }

    match writeln!(
        writer,
        "\nAdding transaction entry to journal: {}",
        journal_file.path().display()
    ) {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }
    match writeln!(
        writer,
        "Enter postings on the format '<account> <amount> <commodity>'"
    ) {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }
    match writeln!(
        writer,
        "example: 'expenses:food 50.00 SEK') such that all are balanced."
    ) {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }
    match writeln!(writer, "If you leave an amount blank, it will be inferred.") {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }
    match writeln!(
        writer,
        "Keep adding as many postings as you want, then enter an empty line to finish.\n"
    ) {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }

    let date: chrono::NaiveDate =
        match utils::prompt_for_date("Date (YYYY-MM-DD): ", "%Y-%m-%d", reader, writer) {
            Ok(d) => d,
            Err(e) => return Err(e),
        };
    let description_str: String = match utils::prompt_input("Description: ", reader, writer) {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    let postings: Vec<journal::transaction::posting::Posting> =
        match utils::prompt_for_postings(reader, writer) {
            Ok(p) => p,
            Err(e) => return Err(e),
        };

    let entry = journal::transaction::Transaction::new(date, description_str, postings);

    if !entry.validate() {
        return Err(crate::error::RsledgerError::ValidationError(
            "Transaction".to_string(),
            "Transaction is not balanced.".to_string(),
        ));
    }

    return journal_file.add_entry(&entry);
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
        fn journal_file(&self) -> journal::JournalFile {
            journal::JournalFile::new(self.0.clone())
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
        run_add(tmp.journal_file(), &mut reader, &mut output).unwrap();
        let contents = std::fs::read_to_string(&tmp.0).unwrap();
        assert!(contents.contains("Groceries"));
        assert!(contents.contains("expenses:food"));
        assert!(contents.contains("assets:bank"));
    }

    #[test]
    fn returns_error_for_missing_journal() {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("rsledger_add_missing_{}.journal", id));
        let jf = journal::JournalFile::new(path);
        let mut reader = Cursor::new(b"");
        let mut output = Vec::new();
        let result = run_add(jf, &mut reader, &mut output);
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_for_unbalanced_transaction() {
        let tmp = TempJournal::new_empty();
        // Two postings both with explicit amounts that don't balance
        let input = b"2026-01-15\nUnbalanced\nexpenses:food 50 SEK\nassets:bank -99 SEK\n\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut output = Vec::new();
        let result = run_add(tmp.journal_file(), &mut reader, &mut output);
        assert!(result.is_err());
    }
}
