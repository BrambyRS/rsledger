use crate::cli::args::ParserOptions;
use crate::journalist::writer::transaction_importer;

use std::io::{BufRead, Write};

/// Instantiates the appropriate parser from `parser_opt` and imports transactions
/// from `csv_file` into `journal_file`, deduplicating against existing entries.
pub fn run_import(
    journal_file: &std::path::PathBuf,
    csv_file: &std::path::PathBuf,
    parser_opt: ParserOptions,
    rule_sheet: &str,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> crate::Result<()> {
    let rule_sheet_path = std::path::PathBuf::from(rule_sheet);

    let parser: Box<dyn transaction_importer::TransactionImporter> = match parser_opt {
        ParserOptions::Avanza => Box::new(
            transaction_importer::avanza_importer::AvanzaParser::new(),
        ),
        ParserOptions::HSBCDebit => Box::new(
            transaction_importer::default_importer::DefaultParser::new(
                "assets:bank:hsbc".to_string(),
                "GBP".to_string(),
                rule_sheet_path,
                ',',
                false,
                0,
                "%d/%m/%Y".to_string(),
                vec![1],
                2,
                None,
                Some(','),
                '.',
            ),
        ),
        ParserOptions::HSBCCredit => Box::new(
            transaction_importer::default_importer::DefaultParser::new(
                "liabilities:credit:hsbc-credit-card".to_string(),
                "GBP".to_string(),
                rule_sheet_path,
                ',',
                false,
                0,
                "%d/%m/%Y".to_string(),
                vec![1],
                2,
                None,
                Some(','),
                '.',
            ),
        ),
        ParserOptions::SebDebit => Box::new(
            transaction_importer::default_importer::DefaultParser::new(
                "assets:bank:seb-lönekonto".to_string(),
                "SEK".to_string(),
                rule_sheet_path,
                ';',
                true,
                0,
                "%Y-%m-%d".to_string(),
                vec![3],
                4,
                None,
                None,
                '.',
            ),
        ),
        ParserOptions::SebSavings => Box::new(
            transaction_importer::default_importer::DefaultParser::new(
                "assets:bank:seb-sparkonto".to_string(),
                "SEK".to_string(),
                rule_sheet_path,
                ';',
                true,
                0,
                "%Y-%m-%d".to_string(),
                vec![3],
                4,
                None,
                None,
                '.',
            ),
        ),
        ParserOptions::Volksbank => Box::new(
            transaction_importer::default_importer::DefaultParser::new(
                "assets:bank:volksbank".to_string(),
                "EUR".to_string(),
                rule_sheet_path,
                ';',
                true,
                4,
                "%d.%m.%Y".to_string(),
                vec![6, 10],
                11,
                Some(12),
                Some('.'),
                ',',
            ),
        ),
    };

    transaction_importer::import_transactions(&*parser, csv_file, journal_file, reader, writer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn csv_path(filename: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("csvs")
            .join(filename)
    }

    fn rule_sheet_path(filename: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("rule_sheets")
            .join(filename)
    }

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TempJournal(std::path::PathBuf);

    impl TempJournal {
        fn new_empty() -> Self {
            let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let path =
                std::env::temp_dir().join(format!("rsledger_import_test_{}.journal", id));
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
    fn imports_classified_seb_csv_no_prompts() {
        let tmp = TempJournal::new_empty();
        let mut reader = Cursor::new(b"");
        let mut output = Vec::new();
        run_import(
            tmp.path(),
            &csv_path("seb_classified.csv"),
            ParserOptions::SebDebit,
            rule_sheet_path("valid_rules.toml").to_str().unwrap(),
            &mut reader,
            &mut output,
        )
        .unwrap();
        let contents = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(!contents.is_empty(), "expected transactions to be imported");
    }

    #[test]
    fn imports_classified_hsbc_csv_no_prompts() {
        let tmp = TempJournal::new_empty();
        let mut reader = Cursor::new(b"");
        let mut output = Vec::new();
        run_import(
            tmp.path(),
            &csv_path("hsbc_classified.csv"),
            ParserOptions::HSBCDebit,
            rule_sheet_path("valid_rules.toml").to_str().unwrap(),
            &mut reader,
            &mut output,
        )
        .unwrap();
        let contents = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(!contents.is_empty(), "expected transactions to be imported");
    }
}
