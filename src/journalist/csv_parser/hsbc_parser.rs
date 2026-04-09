use crate::journalist::csv_parser;
use crate::transaction;
use serde::Deserialize;
use toml;

use std::path::PathBuf;

enum RuleAction {
    AssignAccount(String),
    Skip,
}

struct RegexRule {
    pattern: regex::Regex,
    action: RuleAction,
}

#[derive(Deserialize)]
struct RegexRuleFromFile {
    pattern: String,
    action: String,
    account: Option<String>,
}

#[derive(Deserialize)]
struct RuleSheetFile {
    rules: Vec<RegexRuleFromFile>,
}

pub struct HSBCParser {
    rules: Vec<RegexRule>,
    account: String,
}

impl csv_parser::CSVImporter for HSBCParser {
    fn import_csv(&self, csv_path: PathBuf) -> Vec<csv_parser::ImportCandidate> {
        let mut reader = match csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(&csv_path)
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error opening CSV file {}: {}", csv_path.display(), e);
                return Vec::new();
            }
        };

        let mut import_candidates: Vec<csv_parser::ImportCandidate> = Vec::new();

        // HSBC CSVs do not have a header, so we can start parsing immediately
        // There are only three columns: Date, Description, Amount
        // Separator is ","
        // Date is on the format DD/MM/YYYY
        // Description is just free text
        // Amount is either \d+.\d{2} or \"\d+,\d+.\d{2}\" (with thousands separator)
        let date_regex = regex::Regex::new(r"^\d{2}/\d{2}/\d{4}$").unwrap();

        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error reading CSV record: {}", e);
                    continue;
                }
            };

            if record.len() != 3 {
                eprintln!(
                    "Invalid line format (expected 3 columns), got {}. Skipping.",
                    record.len()
                );
                continue;
            }

            let date_str_raw = record[0].trim();
            let description_str = record[1].trim();
            let amount_str_raw = record[2].trim();

            let date_str = if !date_regex.is_match(date_str_raw) {
                eprintln!("Invalid date format '{}'. Skipping.", date_str_raw);
                continue;
            } else {
                // Convert to YYYY-MM-DD format
                let date_parts: Vec<&str> = date_str_raw.split('/').collect();
                format!("{}-{}-{}", date_parts[2], date_parts[1], date_parts[0])
            };

            // The csv crate strips surrounding quotes, so we only need to remove
            // the thousands separator before appending the commodity.
            let amount_str = format!("{} GBP", amount_str_raw.replace(",", ""));

            // We will try to match the description against the rules to classify the transactions
            // that can be classified
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

impl HSBCParser {
    pub fn new(account: String, rule_sheet: PathBuf) -> HSBCParser {
        let rules: Vec<RegexRule> = match read_rule_sheet(rule_sheet) {
            Ok(rules) => rules,
            Err(_) => {
                eprintln!("Error reading rule sheet. No classification rules will be applied.");
                Vec::new()
            }
        };

        return HSBCParser { rules, account };
    }
}

fn read_rule_sheet(path: PathBuf) -> Result<Vec<RegexRule>, Box<dyn std::error::Error>> {
    let rule_sheet_str = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading rule sheet {}: {}", path.display(), e);
            return Err(e.into());
        }
    };

    let rules_from_file: Vec<RegexRuleFromFile> =
        match toml::from_str::<RuleSheetFile>(&rule_sheet_str) {
            Ok(sheet) => sheet.rules,
            Err(e) => {
                eprintln!(
                    "Error parsing rule sheet {}: {}. Make sure it is a valid TOML file.",
                    path.display(),
                    e
                );
                return Err(e.into());
            }
        };

    let mut rules: Vec<RegexRule> = Vec::with_capacity(rules_from_file.len());
    for rule_from_file in rules_from_file {
        let action = if rule_from_file.action.to_lowercase() == "skip" {
            RuleAction::Skip
        } else {
            match rule_from_file.account {
                Some(account) => RuleAction::AssignAccount(account),
                None => {
                    eprintln!(
                        "Rule with pattern '{}' has action 'assign_account' but no account. Skipping.",
                        rule_from_file.pattern
                    );
                    continue;
                }
            }
        };
        match regex::Regex::new(&rule_from_file.pattern) {
            Ok(pattern) => rules.push(RegexRule { pattern, action }),
            Err(e) => eprintln!(
                "Error compiling regex pattern '{}': {}. Skipping this rule.",
                rule_from_file.pattern, e
            ),
        }
    }

    return Ok(rules);
}
