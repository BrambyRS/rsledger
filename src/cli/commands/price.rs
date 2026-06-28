use crate::cli::utils;
use crate::journal;

use std::io::{self, BufRead, Write};

/// Interactively prompts for a date, commodity, and value, then appends a
/// price directive to the journal file.
pub fn run_price(
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
        "\nAdding price entry to journal: {}",
        journal_file.path().display()
    ) {
        Ok(_) => {}
        Err(e) => return Err(crate::error::RsledgerError::IoError(e)),
    }

    let date: chrono::NaiveDate =
        match utils::prompt_for_date("Date (YYYY-MM-DD): ", "%Y-%m-%d", reader, writer) {
            Ok(d) => d,
            Err(e) => return Err(e),
        };
    let commodity_name = match utils::prompt_input("Commodity: ", reader, writer) {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    let commodity = journal::commodity_value::commodity::Commodity {
        name: commodity_name,
    };
    let value: journal::commodity_value::CommodityValue =
        match utils::prompt_for_value("Value: ", reader, writer) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

    let entry = journal::price::PriceDirective {
        date,
        commodity,
        value,
    };

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
            let path = std::env::temp_dir().join(format!("rsledger_price_test_{}.journal", id));
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
    fn appends_price_directive_to_journal() {
        let tmp = TempJournal::new_empty();
        let input = b"2026-01-15\nAAPL\n150.00 USD\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut output = Vec::new();
        run_price(tmp.journal_file(), &mut reader, &mut output).unwrap();
        let contents = std::fs::read_to_string(&tmp.0).unwrap();
        assert!(contents.contains("AAPL"));
        assert!(contents.contains("150"));
        assert!(contents.contains("USD"));
    }

    #[test]
    fn returns_error_for_missing_journal() {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("rsledger_price_missing_{}.journal", id));
        let jf = journal::JournalFile::new(path);
        let mut reader = Cursor::new(b"");
        let mut output = Vec::new();
        let result = run_price(jf, &mut reader, &mut output);
        assert!(result.is_err());
    }
}
