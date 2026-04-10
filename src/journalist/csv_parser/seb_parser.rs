use crate::journalist::csv_parser;
use crate::journalist::csv_parser::rules::{RegexRule, RuleAction, read_rule_sheet};
use crate::transaction;

use std::path::PathBuf;

pub struct SebParser {
    rules: Vec<RegexRule>,
    account: String,
}

impl csv_parser::CSVImporter for SebParser {
    fn import_csv(&self, csv_path: PathBuf) -> Vec<csv_parser::ImportCandidate> {
        let mut reader = match csv::ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b';')
            .from_path(&csv_path)
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error opening CSV file {}: {}", csv_path.display(), e);
                return Vec::new();
            }
        };

        let mut import_candidates: Vec<csv_parser::ImportCandidate> = Vec::new();

        // SEB CSVs have one header row (skipped by has_headers(true)).
        // Columns (0-based): 0=date (YYYY-MM-DD), 3=description, 4=amount
        // Decimal character is '.'. Commodity is always SEK.
        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error reading CSV record: {}", e);
                    continue;
                }
            };

            if record.len() < 5 {
                eprintln!(
                    "Invalid line format (expected at least 5 columns), got {}. Skipping.",
                    record.len()
                );
                continue;
            }

            let date_str = record[0].trim().to_string();
            let description_str = record[3].trim();
            let amount_str_raw = record[4].trim();

            let amount_str = format!("{} SEK", amount_str_raw);

            let mut classified = false;
            for rule in &self.rules {
                if rule.pattern.is_match(description_str) {
                    match &rule.action {
                        RuleAction::AssignAccount(against_account) => {
                            let mut postings: Vec<transaction::posting::Posting> = Vec::new();
                            postings.push(transaction::posting::Posting::new(
                                self.account.clone(),
                                Some(
                                    match transaction::commodity_value::CommodityValue::from_str(
                                        &amount_str,
                                    ) {
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
                            ));
                            postings.push(transaction::posting::Posting::new(
                                against_account.clone(),
                                None,
                            ));
                            let transaction = transaction::Transaction::new(
                                date_str.clone(),
                                description_str.to_string(),
                                postings,
                            );
                            import_candidates
                                .push(csv_parser::ImportCandidate::Classified(transaction));
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
                let postings = vec![transaction::posting::Posting::new(
                    self.account.clone(),
                    Some(
                        match transaction::commodity_value::CommodityValue::from_str(&amount_str) {
                            Ok(value) => value,
                            Err(e) => {
                                eprintln!("Error parsing amount '{}': {}", amount_str, e);
                                continue;
                            }
                        },
                    ),
                )];
                let transaction =
                    transaction::Transaction::new(date_str, description_str.to_string(), postings);
                import_candidates.push(csv_parser::ImportCandidate::Unclassified(transaction));
            }
        }

        return import_candidates;
    }
}

impl SebParser {
    pub fn new(account: String, rule_sheet: PathBuf) -> SebParser {
        let rules: Vec<RegexRule> = match read_rule_sheet(rule_sheet) {
            Ok(rules) => rules,
            Err(_) => {
                eprintln!("Error reading rule sheet. No classification rules will be applied.");
                Vec::new()
            }
        };

        return SebParser { rules, account };
    }
}
