use crate::cli::utils;
use crate::journalist;
use crate::transaction;

use std::fs;
use std::io::{BufRead, Write};

/// Creates a new journal file. If `create_opening` is true, interactively
/// prompts the user for opening balance postings and appends an opening
/// transaction to the file.
pub fn run_new(
    journal_file: &std::path::PathBuf,
    create_opening: bool,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> crate::Result<()> {
    // Create the directory if it doesn't exist
    if let Some(parent) = journal_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    journalist::writer::new_journal(journal_file)?;

    if create_opening {
        let today: chrono::NaiveDate = chrono::Local::now().date_naive();

        writeln!(
            writer,
            "\nCreating opening transaction at {today} with description 'Opening balance'."
        )?;
        writeln!(
            writer,
            "Enter the opening balance postings on the format '<account> <amount> <commodity>'"
        )?;
        writeln!(writer, "example: 'assets:bank 1000.00 SEK'")?;
        writeln!(
            writer,
            "Keep adding as many postings as you want, then enter an empty line to finish.\n"
        )?;
        writeln!(
            writer,
            "The transaction will be balanced automatically against equity:opening-balance.\n"
        )?;

        let mut postings = utils::prompt_for_postings(reader, writer)?;

        // Only postings with explicit amounts make sense for opening balance
        postings.push(transaction::posting::Posting::new(
            "equity:opening-balance".to_string(),
            None,
        ));

        let opening_transaction =
            transaction::Transaction::new(today, "Opening balance".to_string(), postings);

        let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
        journalist::writer::add_transaction_to_file(&mut file, &opening_transaction)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TempJournal(std::path::PathBuf);

    impl TempJournal {
        fn new() -> Self {
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("rsledger_new_test_{}.journal", id));
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
    fn creates_empty_journal_file() {
        let tmp = TempJournal::new();
        let mut input = Cursor::new(b"");
        let mut output = Vec::new();
        run_new(tmp.path(), false, &mut input, &mut output).unwrap();
        assert!(tmp.path().exists());
        assert_eq!(std::fs::read_to_string(tmp.path()).unwrap(), "");
    }

    #[test]
    fn creates_intermediate_directories() {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir()
            .join(format!("rsledger_new_dir_test_{}", id))
            .join("sub")
            .join("journal.journal");
        let mut input = Cursor::new(b"");
        let mut output = Vec::new();
        run_new(&path, false, &mut input, &mut output).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn creates_journal_with_opening_transaction() {
        let tmp = TempJournal::new();
        // One posting with an amount, then empty line to finish
        let mut input = Cursor::new(b"assets:bank 1000 SEK\n\n");
        let mut output = Vec::new();
        run_new(tmp.path(), true, &mut input, &mut output).unwrap();
        let contents = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(contents.contains("Opening balance"));
        assert!(contents.contains("assets:bank"));
        assert!(contents.contains("equity:opening-balance"));
    }

    #[test]
    fn opening_transaction_prompt_written_to_writer() {
        let tmp = TempJournal::new();
        let mut input = Cursor::new(b"\n");
        let mut output = Vec::new();
        run_new(tmp.path(), true, &mut input, &mut output).unwrap();
        let out = String::from_utf8(output).unwrap();
        assert!(out.contains("Opening balance"));
    }
}
