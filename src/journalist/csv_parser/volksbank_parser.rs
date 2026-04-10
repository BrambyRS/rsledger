use crate::journalist::csv_parser;
use crate::journalist::csv_parser::rules::{RegexRule, RuleAction, read_rule_sheet};
use crate::transaction;

use std::path::PathBuf;

pub struct VolksbankParser {
    rules: Vec<RegexRule>,
    account: String,
}

impl csv_parser::CSVImporter for VolksbankParser {
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

        // Volksbank CSVs have one header row (skipped by has_headers(true)).
        // Columns (0-based):
        //   4  = date (DD.MM.YYYY)
        //   6  = description part 1
        //   10 = description part 2
        //   11 = amount ('.' thousands sep, ',' decimal sep)
        //   12 = commodity
        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error reading CSV record: {}", e);
                    continue;
                }
            };

            if record.len() < 13 {
                eprintln!(
                    "Invalid line format (expected at least 13 columns), got {}. Skipping.",
                    record.len()
                );
                continue;
            }

            // Date: DD.MM.YYYY → YYYY-MM-DD
            let date_str_raw = record[4].trim();
            let date_parts: Vec<&str> = date_str_raw.split('.').collect();
            if date_parts.len() != 3 {
                eprintln!("Invalid date format '{}'. Skipping.", date_str_raw);
                continue;
            }
            let date_str = format!("{}-{}-{}", date_parts[2], date_parts[1], date_parts[0]);

            // Description: concatenate col 6 and col 10, separated by a space if both non-empty
            let desc1 = record[6].trim();
            let desc2 = record[10].trim();
            let description_str = match (desc1.is_empty(), desc2.is_empty()) {
                (false, false) => format!("{} {}", desc1, desc2),
                (false, true) => desc1.to_string(),
                (true, false) => desc2.to_string(),
                (true, true) => String::new(),
            };

            // Amount: strip '.' (thousands sep), replace ',' with '.' (decimal sep)
            let amount_raw = record[11].trim().replace('.', "").replace(',', ".");
            let commodity = record[12].trim();
            let amount_str = format!("{} {}", amount_raw, commodity);

            let mut classified = false;
            for rule in &self.rules {
                if rule.pattern.is_match(&description_str) {
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
                                description_str.clone(),
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
                    transaction::Transaction::new(date_str, description_str, postings);
                import_candidates.push(csv_parser::ImportCandidate::Unclassified(transaction));
            }
        }

        return import_candidates;
    }
}

impl VolksbankParser {
    pub fn new(account: String, rule_sheet: PathBuf) -> VolksbankParser {
        let rules: Vec<RegexRule> = match read_rule_sheet(rule_sheet) {
            Ok(rules) => rules,
            Err(_) => {
                eprintln!("Error reading rule sheet. No classification rules will be applied.");
                Vec::new()
            }
        };

        return VolksbankParser { rules, account };
    }
}
