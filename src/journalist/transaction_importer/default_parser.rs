//! This implements a default CSV parser for bank transaction exports
//! that do not have custom parsing logic implemented.
//! It supports classification of transactions based on the regex-based rule system

use crate::commodity_value;
use crate::journalist::transaction_importer;
use crate::journalist::transaction_importer::rules::{RegexRule, RuleAction, read_rule_sheet};
use crate::transaction;

use std::path::PathBuf;

pub struct DefaultParser {
    account: String,
    currency: String,
    rules: Vec<RegexRule>,
    delimiter: char,
    has_headers: bool,
    date_column: usize,
    date_format: String,
    description_column: Vec<usize>, // Can be several columns to concatenate
    amount_column: usize,
    commodity_column: Option<usize>,
    thousands_separator: Option<char>,
    decimal_separator: char,
}

impl DefaultParser {
    pub fn new(
        account: String,
        currency: String,
        rule_sheet: PathBuf,
        delimiter: char,
        has_headers: bool,
        date_column: usize,
        date_format: String,
        description_column: Vec<usize>,
        amount_column: usize,
        commodity_column: Option<usize>,
        thousands_separator: Option<char>,
        decimal_separator: char,
    ) -> DefaultParser {
        let rules: Vec<RegexRule> = match read_rule_sheet(rule_sheet) {
            Ok(rules) => rules,
            Err(_) => {
                eprintln!("Error reading rule sheet. No classification rules will be applied.");
                Vec::new()
            }
        };

        DefaultParser {
            account,
            currency,
            rules,
            delimiter,
            has_headers,
            date_column,
            date_format,
            description_column,
            amount_column,
            commodity_column,
            thousands_separator,
            decimal_separator,
        }
    }
}

impl transaction_importer::TransactionImporter for DefaultParser {
    fn import_csv(&self, csv_path: PathBuf) -> Vec<transaction_importer::ImportCandidate> {
        let mut reader = match csv::ReaderBuilder::new()
            .has_headers(self.has_headers)
            .delimiter(self.delimiter as u8)
            .from_path(&csv_path)
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error opening CSV file {}: {}", csv_path.display(), e);
                return Vec::new();
            }
        };

        // Determine the minimum number of columns needed from the configured indices
        let min_columns = [self.date_column, self.amount_column]
            .into_iter()
            .chain(self.description_column.iter().copied())
            .chain(self.commodity_column)
            .max()
            .unwrap_or(0)
            + 1;

        let mut import_candidates: Vec<transaction_importer::ImportCandidate> = Vec::new();

        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error reading CSV record: {}", e);
                    continue;
                }
            };

            if record.len() < min_columns {
                eprintln!(
                    "Invalid line format (expected at least {} columns), got {}. Skipping.",
                    min_columns,
                    record.len()
                );
                continue;
            }

            // --- Date ---
            let date_str_raw = record[self.date_column].trim();
            let date = if self.date_format == "%Y-%m-%d" {
                // Already in the target format, no parsing needed
                match chrono::NaiveDate::parse_from_str(date_str_raw, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => {
                        eprintln!("Invalid date format '{}'. Skipping.", date_str_raw);
                        continue;
                    }
                }
            } else {
                match chrono::NaiveDate::parse_from_str(date_str_raw, &self.date_format) {
                    Ok(d) => d,

                    Err(_) => {
                        eprintln!("Invalid date format '{}'. Skipping.", date_str_raw);
                        continue;
                    }
                }
            };

            // --- Description ---
            let description_str: String = self
                .description_column
                .iter()
                .map(|&col| record[col].trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join(" ");

            // --- Amount ---
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

            // --- Classification ---
            let mut classified = false;
            for rule in &self.rules {
                if rule.pattern.is_match(&description_str) {
                    match &rule.action {
                        RuleAction::AssignAccount(against_account) => {
                            let first_posting = transaction::posting::Posting::new(
                                self.account.clone(),
                                Some(
                                    match commodity_value::CommodityValue::from_str(&amount_str) {
                                        Ok(value) => value,
                                        Err(e) => {
                                            eprintln!(
                                                "Error parsing amount '{}': {}",
                                                amount_str, e
                                            );
                                            continue;
                                        }
                                    },
                                ),
                            );
                            let second_posting =
                                transaction::posting::Posting::new(against_account.clone(), None);
                            let transaction = transaction::Transaction::new(
                                date,
                                description_str.clone(),
                                vec![first_posting, second_posting],
                            );
                            import_candidates.push(
                                transaction_importer::ImportCandidate::Classified(transaction),
                            );
                            classified = true;
                            break;
                        }
                        RuleAction::Skip => {
                            classified = true;
                            break;
                        }
                    }
                }
            }

            if !classified {
                let posting = transaction::posting::Posting::new(
                    self.account.clone(),
                    Some(
                        match commodity_value::CommodityValue::from_str(&amount_str) {
                            Ok(value) => value,
                            Err(e) => {
                                eprintln!("Error parsing amount '{}': {}", amount_str, e);
                                continue;
                            }
                        },
                    ),
                );
                let transaction =
                    transaction::Transaction::new(date, description_str, vec![posting]);
                import_candidates.push(transaction_importer::ImportCandidate::Unclassified(
                    transaction,
                ));
            }
        }

        import_candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journalist::transaction_importer::{ImportCandidate, TransactionImporter};

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

    fn seb_parser(rule_sheet: &str) -> DefaultParser {
        DefaultParser::new(
            "assets:bank:seb-lönekonto".to_string(),
            "SEK".to_string(),
            rule_sheet_path(rule_sheet),
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
    }

    fn volksbank_parser(rule_sheet: &str) -> DefaultParser {
        DefaultParser::new(
            "assets:bank:volksbank".to_string(),
            "EUR".to_string(),
            rule_sheet_path(rule_sheet),
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
    }

    // -------------------------------------------------------------------------
    // SEB import tests
    // -------------------------------------------------------------------------

    #[test]
    fn seb_classified_csv_imports_all_transactions() {
        let parser = seb_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("seb_classified.csv"));

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
        let parser = seb_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("seb_classified.csv"));

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(
                *t.get_date(),
                chrono::NaiveDate::from_ymd_opt(2026, 3, 21).unwrap()
            );
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(
                *t.get_date(),
                chrono::NaiveDate::from_ymd_opt(2026, 3, 20).unwrap()
            );
        }
    }

    #[test]
    fn seb_classified_csv_parses_amounts_as_sek() {
        let parser = seb_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("seb_classified.csv"));

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(
                t.get_postings()[0].get_amount().unwrap().to_string(),
                "-250 SEK"
            );
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(
                t.get_postings()[0].get_amount().unwrap().to_string(),
                "-129 SEK"
            );
        }
    }

    #[test]
    fn seb_mixed_csv_produces_classified_and_unclassified() {
        let parser = seb_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("seb_mixed.csv"));

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Classified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    #[test]
    fn seb_unclassified_has_single_posting() {
        let parser = seb_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("seb_mixed.csv"));

        if let ImportCandidate::Unclassified(t) = &candidates[1] {
            assert_eq!(t.get_postings().len(), 1);
            assert_eq!(
                t.get_postings()[0].get_account(),
                "assets:bank:seb-lönekonto"
            );
            assert_eq!(
                t.get_postings()[0].get_amount().unwrap().to_string(),
                "-75.5 SEK"
            );
        } else {
            panic!("expected unclassified transaction");
        }
    }

    #[test]
    fn seb_empty_rules_leaves_all_unclassified() {
        let parser = seb_parser("empty_rules.toml");
        let candidates = parser.import_csv(csv_path("seb_classified.csv"));

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Unclassified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    // -------------------------------------------------------------------------
    // Volksbank import tests
    // -------------------------------------------------------------------------

    #[test]
    fn volksbank_classified_csv_imports_all_transactions() {
        let parser = volksbank_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_classified.csv"));

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
        let parser = volksbank_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_classified.csv"));

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(
                *t.get_date(),
                chrono::NaiveDate::from_ymd_opt(2026, 3, 21).unwrap()
            );
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(
                *t.get_date(),
                chrono::NaiveDate::from_ymd_opt(2026, 3, 20).unwrap()
            );
        }
    }

    #[test]
    fn volksbank_classified_csv_concatenates_description_columns() {
        let parser = volksbank_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_classified.csv"));

        // Col 6 = "GROCERY STORE REWE", col 10 = "Einkauf Filiale 42"
        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(t.get_description(), "GROCERY STORE REWE Einkauf Filiale 42");
        }
    }

    #[test]
    fn volksbank_classified_csv_parses_comma_decimal_amounts() {
        let parser = volksbank_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_classified.csv"));

        if let ImportCandidate::Classified(t) = &candidates[0] {
            assert_eq!(
                t.get_postings()[0].get_amount().unwrap().to_string(),
                "-25 EUR"
            );
        }
        if let ImportCandidate::Classified(t) = &candidates[1] {
            assert_eq!(
                t.get_postings()[0].get_amount().unwrap().to_string(),
                "-9.99 EUR"
            );
        }
    }

    #[test]
    fn volksbank_classified_csv_reads_commodity_from_column() {
        let parser = volksbank_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_classified.csv"));

        // Commodity comes from col 12 ("EUR"), not from the currency field
        if let ImportCandidate::Classified(t) = &candidates[0] {
            let amount_str = t.get_postings()[0].get_amount().unwrap().to_string();
            assert!(amount_str.ends_with("EUR"));
        }
    }

    #[test]
    fn volksbank_mixed_csv_produces_classified_and_unclassified() {
        let parser = volksbank_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_mixed.csv"));

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Classified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }

    #[test]
    fn volksbank_mixed_csv_handles_thousands_separator() {
        let parser = volksbank_parser("valid_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_mixed.csv"));

        // The unclassified row has amount "-1.250,50" → strip '.' thousands sep, replace ',' → "-1250.50"
        if let ImportCandidate::Unclassified(t) = &candidates[1] {
            assert_eq!(
                t.get_postings()[0].get_amount().unwrap().to_string(),
                "-1250.5 EUR"
            );
        } else {
            panic!("expected unclassified transaction");
        }
    }

    #[test]
    fn volksbank_empty_rules_leaves_all_unclassified() {
        let parser = volksbank_parser("empty_rules.toml");
        let candidates = parser.import_csv(csv_path("volksbank_classified.csv"));

        assert_eq!(candidates.len(), 2);
        assert!(matches!(&candidates[0], ImportCandidate::Unclassified(_)));
        assert!(matches!(&candidates[1], ImportCandidate::Unclassified(_)));
    }
}
