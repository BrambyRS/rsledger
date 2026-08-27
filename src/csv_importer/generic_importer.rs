//! Generic configurable CSV importer for bank transaction exports.
//!
//! Provides [`GenericImporter`], a configurable parser that reads a CSV file into
//! [`Transaction`]s and classifies them using an optional TOML rule sheet. All CSV layout
//! details — delimiter, column indices, date format, and number formatting — are supplied
//! at construction time so that any CSV layout can be supported without writing custom
//! parsing code.

use chrono::NaiveDate;
use std::path::PathBuf;

use crate::csv_importer::rules::{read_rule_sheet, RegexRule, RuleAction};
use crate::journal::account::Account;
use crate::journal::commodity_value::CommodityValue;
use crate::journal::transaction::posting::Posting;
use crate::journal::transaction::Transaction;

use super::{EntryImporter, ImportCandidate};

/// An intermediate representation of a single CSV row before classification rules are applied.
///
/// Splitting CSV parsing into two phases — first into `ProtoTransaction`s, then into
/// classified [`ImportCandidate`]s — keeps each step focused and independently testable.
struct ProtoTransaction {
    /// Parsed transaction date.
    date: NaiveDate,
    /// Concatenated description string built from one or more configured CSV columns.
    description: String,
    /// The parsed monetary amount together with its commodity symbol.
    amount: CommodityValue,
}

/// A configurable parser for CSV bank-transaction exports that do not have dedicated
/// parsing logic.
///
/// All CSV layout details (delimiter, column positions, date format, number formatting)
/// are supplied at construction time. Classification is performed by an optional TOML
/// rule sheet; rows with no matching rule are emitted as [`ImportCandidate::Unclassified`],
/// and rows whose matching rule carries action `skip` are dropped entirely.
pub struct GenericImporter {
    /// The bank/asset account that debits and credits appear under.
    account: Account,
    /// Default currency symbol used when no `commodity_column` is configured.
    currency: String,
    /// Loaded classification rules. Empty when no rule sheet was supplied.
    rules: Vec<RegexRule>,
    /// Field delimiter character (e.g. `','` or `';'`).
    delimiter: char,
    /// Whether the first CSV row is a header that should be skipped.
    has_headers: bool,
    /// Zero-based column index of the transaction date.
    date_column: usize,
    /// `strftime`-compatible format string used to parse dates (e.g. `"%Y-%m-%d"`).
    date_format: String,
    /// One or more zero-based column indices whose trimmed values are joined with a space
    /// to form the transaction description. Empty column values are ignored.
    description_columns: Vec<usize>,
    /// Zero-based column index of the transaction amount.
    amount_column: usize,
    /// Optional zero-based column index of the commodity/currency symbol.
    /// Falls back to `currency` when absent.
    commodity_column: Option<usize>,
    /// Optional thousands separator to strip from amount strings
    /// (e.g. `Some('.')` for German-format numbers such as `"1.250,50"`).
    thousands_separator: Option<char>,
    /// Decimal separator used in amount strings
    /// (e.g. `','` for German-format numbers; `'.'` for English-format numbers).
    decimal_separator: char,
}

impl GenericImporter {
    /// Creates a new `GenericImporter`.
    ///
    /// If `rule_sheet` is `Some`, the TOML file at that path is read and compiled into
    /// classification rules; any error is propagated to the caller. Pass `None` to import
    /// without classification (all rows will be [`ImportCandidate::Unclassified`]).
    pub fn new(
        account: Account,
        currency: String,
        rule_sheet: Option<PathBuf>,
        delimiter: char,
        has_headers: bool,
        date_column: usize,
        date_format: String,
        description_columns: Vec<usize>,
        amount_column: usize,
        commodity_column: Option<usize>,
        thousands_separator: Option<char>,
        decimal_separator: char,
    ) -> crate::Result<Self> {
        let rules = match rule_sheet {
            Some(path) => read_rule_sheet(path)?,
            None => Vec::new(),
        };

        Ok(GenericImporter {
            account,
            currency,
            rules,
            delimiter,
            has_headers,
            date_column,
            date_format,
            description_columns,
            amount_column,
            commodity_column,
            thousands_separator,
            decimal_separator,
        })
    }

    /// Opens `csv_path` and parses every data row into a [`ProtoTransaction`].
    ///
    /// Returns an error on the first row that cannot be parsed, so the caller
    /// receives a precise description of which field failed and why.
    fn read_rows(&self, csv_path: &PathBuf) -> crate::Result<Vec<ProtoTransaction>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(self.has_headers)
            .delimiter(self.delimiter as u8)
            .from_path(csv_path)
            .map_err(|e| {
                crate::error::RsledgerError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        // Determine the minimum number of columns required so underflowing rows can be
        // detected early with a descriptive error rather than an index-out-of-bounds panic.
        let min_columns = [self.date_column, self.amount_column]
            .into_iter()
            .chain(self.description_columns.iter().copied())
            .chain(self.commodity_column)
            .max()
            .unwrap_or(0)
            + 1;

        let mut rows: Vec<ProtoTransaction> = Vec::new();

        for result in reader.records() {
            let record = result.map_err(|e| {
                crate::error::RsledgerError::ParseError(
                    csv_path.display().to_string(),
                    format!("Failed to read CSV record: {}", e),
                )
            })?;

            if record.len() < min_columns {
                return Err(crate::error::RsledgerError::ParseError(
                    csv_path.display().to_string(),
                    format!(
                        "Row has {} column(s) but the column configuration requires at least {}.",
                        record.len(),
                        min_columns
                    ),
                ));
            }

            // --- Date ---
            let date_str = record[self.date_column].trim();
            let date =
                NaiveDate::parse_from_str(date_str, &self.date_format).map_err(|_| {
                    crate::error::RsledgerError::ParseError(
                        csv_path.display().to_string(),
                        format!(
                            "Could not parse date '{}' with format '{}'.",
                            date_str, self.date_format
                        ),
                    )
                })?;

            // --- Description ---
            // Join one or more columns, skipping any that are empty after trimming.
            let description = self
                .description_columns
                .iter()
                .map(|&col| record[col].trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join(" ");

            // --- Amount ---
            // Normalise the raw string by stripping the thousands separator (if any)
            // and replacing the decimal separator with '.' before parsing.
            let mut amount_raw = record[self.amount_column].trim().to_string();
            if let Some(ts) = self.thousands_separator {
                amount_raw = amount_raw.replace(ts, "");
            }
            if self.decimal_separator != '.' {
                amount_raw = amount_raw.replace(self.decimal_separator, ".");
            }
            let commodity = match self.commodity_column {
                Some(col) => record[col].trim().to_string(),
                None => self.currency.clone(),
            };
            let amount_str = format!("{} {}", amount_raw, commodity);
            let amount = CommodityValue::from_str(&amount_str).map_err(|_| {
                crate::error::RsledgerError::ParseError(
                    csv_path.display().to_string(),
                    format!("Could not parse amount '{}'.", amount_str),
                )
            })?;

            rows.push(ProtoTransaction {
                date,
                description,
                amount,
            });
        }

        Ok(rows)
    }

    /// Applies classification rules to a single [`ProtoTransaction`] and returns the
    /// appropriate [`ImportCandidate`], or `None` when the matching rule is `skip`.
    ///
    /// Rules are evaluated in declaration order; the first match wins. Rows that match
    /// no rule are returned as [`ImportCandidate::Unclassified`] with a single posting
    /// (no counterpart account).
    fn classify_row(&self, proto: ProtoTransaction) -> Option<ImportCandidate<Transaction>> {
        for rule in &self.rules {
            if rule.pattern.is_match(&proto.description) {
                match &rule.action {
                    RuleAction::AssignAccount(against_account) => {
                        // Build a balanced two-posting transaction: the configured account
                        // receives the explicit amount; the counterpart auto-balances.
                        let main_posting = Posting::new(self.account.clone(), Some(proto.amount));
                        let counterpart_posting = Posting::new(against_account.clone(), None);
                        let transaction = Transaction::new(
                            proto.date,
                            proto.description,
                            vec![main_posting, counterpart_posting],
                        );
                        return Some(ImportCandidate::Classified(transaction));
                    }
                    RuleAction::Skip => return None,
                }
            }
        }

        // No rule matched — emit as unclassified with only the main posting.
        let posting = Posting::new(self.account.clone(), Some(proto.amount));
        let transaction = Transaction::new(proto.date, proto.description, vec![posting]);
        Some(ImportCandidate::Unclassified(transaction))
    }
}

impl EntryImporter<Transaction> for GenericImporter {
    fn import_csv(&self, csv_path: PathBuf) -> crate::Result<Vec<ImportCandidate<Transaction>>> {
        // Phase 1: parse every CSV row into an intermediate ProtoTransaction.
        // Phase 2: apply classification rules, dropping any rows matched by a `skip` rule.
        let rows = self.read_rows(&csv_path)?;
        let candidates = rows
            .into_iter()
            .filter_map(|proto| self.classify_row(proto))
            .collect();
        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("csvs")
            .join(filename)
    }

    fn rule_sheet_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("rule_sheets")
            .join(filename)
    }

    fn seb_importer(rule_sheet: &str) -> GenericImporter {
        GenericImporter::new(
            Account::from_str("assets:bank:seb-lönekonto").unwrap(),
            "SEK".to_string(),
            Some(rule_sheet_path(rule_sheet)),
            ';',
            true,
            0,
            "%Y-%m-%d".to_string(),
            vec![3],
            4,
            None,
            None,
            '.',
        )
        .unwrap()
    }

    fn volksbank_importer(rule_sheet: &str) -> GenericImporter {
        GenericImporter::new(
            Account::from_str("assets:bank:volksbank").unwrap(),
            "EUR".to_string(),
            Some(rule_sheet_path(rule_sheet)),
            ';',
            true,
            4,
            "%d.%m.%Y".to_string(),
            vec![6, 10],
            11,
            Some(12),
            Some('.'),
            ',',
        )
        .unwrap()
    }

    // -------------------------------------------------------------------------
    // SEB import tests
    // -------------------------------------------------------------------------

    #[test]
    fn seb_classified_csv_imports_all_transactions() {
        let importer = seb_importer("valid_rules.toml");
        let candidates = importer.import_csv(csv_path("seb_classified.csv")).unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(
            matches!(&candidates[0], ImportCandidate::Classified(_)),
            "GROCERY STORE ICA should be classified"
        );
        assert!(
            matches!(&candidates[1], ImportCandidate::Classified(_)),
            "SPOTIFY should be classified"
        );
    }

    #[test]
    fn seb_classified_csv_parses_dates_correctly() {
        let importer = seb_importer("valid_rules.toml");
        let candidates = importer.import_csv(csv_path("seb_classified.csv")).unwrap();

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(*t.date(), NaiveDate::from_ymd_opt(2026, 3, 21).unwrap());
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(*t.date(), NaiveDate::from_ymd_opt(2026, 3, 20).unwrap());
        }
    }

    #[test]
    fn seb_classified_csv_parses_amounts_as_sek() {
        let importer = seb_importer("valid_rules.toml");
        let candidates = importer.import_csv(csv_path("seb_classified.csv")).unwrap();

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "-250 SEK");
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "-129 SEK");
        }
    }

    #[test]
    fn seb_mixed_csv_produces_classified_and_unclassified() {
        let importer = seb_importer("valid_rules.toml");
        let candidates = importer.import_csv(csv_path("seb_mixed.csv")).unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Classified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    #[test]
    fn seb_unclassified_has_single_posting() {
        let importer = seb_importer("valid_rules.toml");
        let candidates = importer.import_csv(csv_path("seb_mixed.csv")).unwrap();

        if let ImportCandidate::Unclassified(t) = &candidates[1] {
            assert_eq!(t.postings().len(), 1);
            assert_eq!(
                t.postings()[0].account().to_string(),
                "assets:bank:seb-lönekonto"
            );
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "-75.5 SEK");
        } else {
            panic!("expected unclassified transaction");
        }
    }

    #[test]
    fn seb_empty_rules_leaves_all_unclassified() {
        let importer = seb_importer("empty_rules.toml");
        let candidates = importer.import_csv(csv_path("seb_classified.csv")).unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Unclassified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    // -------------------------------------------------------------------------
    // Volksbank import tests
    // -------------------------------------------------------------------------

    #[test]
    fn volksbank_classified_csv_imports_all_transactions() {
        let importer = volksbank_importer("valid_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_classified.csv"))
            .unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(
            matches!(&candidates[0], ImportCandidate::Classified(_)),
            "GROCERY STORE REWE should be classified"
        );
        assert!(
            matches!(&candidates[1], ImportCandidate::Classified(_)),
            "NETFLIX INTERNATIONAL should be classified"
        );
    }

    #[test]
    fn volksbank_classified_csv_converts_dates_from_ddmmyyyy() {
        let importer = volksbank_importer("valid_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_classified.csv"))
            .unwrap();

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(*t.date(), NaiveDate::from_ymd_opt(2026, 3, 21).unwrap());
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(*t.date(), NaiveDate::from_ymd_opt(2026, 3, 20).unwrap());
        }
    }

    #[test]
    fn volksbank_classified_csv_concatenates_description_columns() {
        let importer = volksbank_importer("valid_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_classified.csv"))
            .unwrap();

        // Col 6 = "GROCERY STORE REWE", col 10 = "Einkauf Filiale 42"
        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(t.description(), "GROCERY STORE REWE Einkauf Filiale 42");
        }
    }

    #[test]
    fn volksbank_classified_csv_parses_comma_decimal_amounts() {
        let importer = volksbank_importer("valid_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_classified.csv"))
            .unwrap();

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "-25 EUR");
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "-9.99 EUR");
        }
    }

    #[test]
    fn volksbank_classified_csv_reads_commodity_from_column() {
        let importer = volksbank_importer("valid_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_classified.csv"))
            .unwrap();

        // Commodity comes from col 12 ("EUR"), not from the currency field
        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert!(t.postings()[0].amount().unwrap().to_string().ends_with("EUR"));
        }
    }

    #[test]
    fn volksbank_mixed_csv_produces_classified_and_unclassified() {
        let importer = volksbank_importer("valid_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_mixed.csv"))
            .unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Classified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    #[test]
    fn volksbank_mixed_csv_handles_thousands_separator() {
        let importer = volksbank_importer("valid_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_mixed.csv"))
            .unwrap();

        // The unclassified row has amount "-1.250,50" → strip '.' thousands sep,
        // replace ',' decimal sep → "-1250.50 EUR"
        if let ImportCandidate::Unclassified(t) = &candidates[1] {
            assert_eq!(
                t.postings()[0].amount().unwrap().to_string(),
                "-1250.5 EUR"
            );
        } else {
            panic!("expected unclassified transaction");
        }
    }

    #[test]
    fn volksbank_empty_rules_leaves_all_unclassified() {
        let importer = volksbank_importer("empty_rules.toml");
        let candidates = importer
            .import_csv(csv_path("volksbank_classified.csv"))
            .unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Unclassified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    #[test]
    fn no_rule_sheet_leaves_all_unclassified() {
        let importer = GenericImporter::new(
            Account::from_str("assets:bank:seb-lönekonto").unwrap(),
            "SEK".to_string(),
            None,
            ';',
            true,
            0,
            "%Y-%m-%d".to_string(),
            vec![3],
            4,
            None,
            None,
            '.',
        )
        .unwrap();
        let candidates = importer.import_csv(csv_path("seb_classified.csv")).unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Unclassified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    #[test]
    fn invalid_rule_sheet_path_returns_error() {
        let result = GenericImporter::new(
            Account::from_str("assets:bank:seb-lönekonto").unwrap(),
            "SEK".to_string(),
            Some(PathBuf::from("non_existent_rule_sheet.toml")),
            ';',
            true,
            0,
            "%Y-%m-%d".to_string(),
            vec![3],
            4,
            None,
            None,
            '.',
        );
        assert!(result.is_err());
    }
}
