use crate::cli::args::ParserOptions;
use crate::csv_importer::avanza_transactions::AvanzaImporter;
use crate::csv_importer::generic_importer::GenericImporter;
use crate::csv_importer::{import_entries, EntryImporter};
use crate::journal;
use crate::journal::account::Account;

use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Runs the CSV import pipeline for the given parser option.
///
/// 1. Constructs the appropriate importer for the named bank format.
/// 2. Parses the CSV into a list of [`ImportCandidate`]s.
/// 3. Deduplicates against the existing journal and appends new
///    transactions, prompting the user for any unclassified entries.
pub fn run_import(
    mut journal_file: journal::JournalFile,
    csv_file: &PathBuf,
    parser_opt: ParserOptions,
    rule_sheet: &str,
    accept_partial_matches: bool,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> crate::Result<()> {
    // An empty rule-sheet argument means "no classification rules".
    let rule_sheet_path: Option<PathBuf> = if rule_sheet.is_empty() {
        None
    } else {
        Some(PathBuf::from(rule_sheet))
    };

    match parser_opt {
        // Avanza has a dedicated parser that handles all transaction types internally.
        ParserOptions::Avanza => {
            let importer = AvanzaImporter::new();
            let candidates = importer.import_csv(csv_file.clone())?;
            import_entries(candidates, &mut journal_file, accept_partial_matches, reader, writer)
        }

        // HSBC current account (debit) — UK DD/MM/YYYY comma-delimited format.
        ParserOptions::HSBCDebit => {
            let importer = GenericImporter::new(
                Account::from_str("assets:bank:hsbc")?,
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
            )?;
            let candidates = importer.import_csv(csv_file.clone())?;
            import_entries(candidates, &mut journal_file, accept_partial_matches, reader, writer)
        }

        // HSBC credit card — same file format as the debit account.
        ParserOptions::HSBCCredit => {
            let importer = GenericImporter::new(
                Account::from_str("liabilities:credit-card:hsbc")?,
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
            )?;
            let candidates = importer.import_csv(csv_file.clone())?;
            import_entries(candidates, &mut journal_file, accept_partial_matches, reader, writer)
        }

        // SEB lönekonto (checking) — Swedish semicolon-delimited format.
        ParserOptions::SebDebit => {
            let importer = GenericImporter::new(
                Account::from_str("assets:bank:seb-l\u{f6}nekonto")?,
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
            )?;
            let candidates = importer.import_csv(csv_file.clone())?;
            import_entries(candidates, &mut journal_file, accept_partial_matches, reader, writer)
        }

        // SEB sparkonto (savings) — identical file format to lönekonto.
        ParserOptions::SebSavings => {
            let importer = GenericImporter::new(
                Account::from_str("assets:bank:seb-sparkonto")?,
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
            )?;
            let candidates = importer.import_csv(csv_file.clone())?;
            import_entries(candidates, &mut journal_file, accept_partial_matches, reader, writer)
        }

        // Volksbank — German semicolon-delimited format with comma decimals.
        // The currency is read from the CSV (col 12) so `currency` is a fallback only.
        ParserOptions::Volksbank => {
            let importer = GenericImporter::new(
                Account::from_str("assets:bank:volksbank")?,
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
            )?;
            let candidates = importer.import_csv(csv_file.clone())?;
            import_entries(candidates, &mut journal_file, accept_partial_matches, reader, writer)
        }
    }
}
