use crate::cli::utils;
use crate::commodity_value;
use crate::journalist;
use crate::price;

use std::fs;
use std::io::{self, BufRead, Write};

/// Interactively prompts for a date, commodity, and value, then appends a
/// price directive to the journal file.
pub fn run_price(
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
        "\nAdding price entry to journal: {}",
        journal_file.display()
    )?;

    let date: chrono::NaiveDate =
        utils::prompt_for_date("Date (YYYY-MM-DD): ", "%Y-%m-%d", reader, writer)?;
    let commodity = commodity_value::commodity::Commodity {
        name: utils::prompt_input("Commodity: ", reader, writer)?,
    };
    let value: commodity_value::CommodityValue =
        utils::prompt_for_value("Value: ", reader, writer)?;

    let entry = price::PriceDirective {
        date,
        commodity,
        value,
    };

    let mut file = fs::OpenOptions::new().append(true).open(journal_file)?;
    journalist::writer::add_price_to_file(&mut file, &entry)
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
    fn appends_price_directive_to_journal() {
        let tmp = TempJournal::new_empty();
        let input = b"2026-01-15\nAAPL\n150.00 USD\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut output = Vec::new();
        run_price(tmp.path(), &mut reader, &mut output).unwrap();
        let contents = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(contents.contains("AAPL"));
        assert!(contents.contains("150"));
        assert!(contents.contains("USD"));
    }

    #[test]
    fn returns_error_for_missing_journal() {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("rsledger_price_missing_{}.journal", id));
        let mut reader = Cursor::new(b"");
        let mut output = Vec::new();
        let result = run_price(&path, &mut reader, &mut output);
        assert!(result.is_err());
    }
}
