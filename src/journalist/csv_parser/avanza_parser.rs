//! This implements a custom CSV parser for Avanza's transaction exports.
//! All transactions can be perfectly classified, but require custom logic
//! above what is possible with the regex-based rule system.

use crate::journalist::csv_parser;
use crate::transaction;

use std::fs::File;
use std::io::Lines;
use std::io::{BufRead, BufReader};
use std::iter::Peekable;
use std::path::PathBuf;

pub struct AvanzaParser;

/// The import rule for the Avanza CSVs is very basic
///
/// The date format is YYYY-MM-DD in the CSV which is already correct
/// The description is just the action + the commodity
/// Everything comes in and out of assets:bank:avanza
/// The profit/loss comes from equity:capital-gains
/// Dividends come from income:dividends
/// And the fees go into expenses:bank:avanza
///
/// Deposits or withdrawals are always assumed to come from assets:bank:seb-lönekonto
impl csv_parser::CSVImporter for AvanzaParser {
    fn import_csv(&self, csv_path: PathBuf) -> Vec<csv_parser::ImportCandidate> {
        let file: File = match File::open(&csv_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error opening file {}: {}", csv_path.display(), e);
                return Vec::new();
            }
        };

        let mut lines: Peekable<Lines<BufReader<File>>> = BufReader::new(file).lines().peekable();

        let mut import_candidates: Vec<csv_parser::ImportCandidate> = Vec::new();

        // Avanza CSV has a header which we will skip
        lines.next();
        for line in lines {
            let this_line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Error reading line in file {}: {}", csv_path.display(), e);
                    continue;
                }
            };

            // The CSV colums are semi-colon seperated and the column order is
            // Datum;Konto;Typ av transaktion;Värdepapper/beskrivning;Antal;Kurs;Belopp;Transaktionsvaluta;Courtage;Valutakurs;Instrumentvaluta;ISIN;Resultat
            let columns: Vec<&str> = this_line.split(';').collect();
            if columns.len() < 13 {
                eprintln!(
                    "Invalid line format in file {}: {}. Expected at least 13 columns.",
                    csv_path.display(),
                    this_line
                );
                continue;
            }

            let date_str = columns[0].to_string();
            let date = match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    eprintln!(
                        "Invalid date format in line '{}'. Expected format YYYY-MM-DD.",
                        this_line
                    );
                    continue;
                }
            };
            let action = columns[2].to_string();
            let name = columns[3].to_string();
            let amount_commodity = columns[4].to_string().replace(',', ".");
            let amount_cash = columns[6].to_string().replace(',', ".");
            let currency = columns[7].to_string();
            let fee_amount = columns[8].to_string().replace(',', ".");
            let profit = columns[12].to_string().replace(',', ".");

            // The fee amount will sometimes be empty, so we will treat that as 0.00
            let fee_amount = if fee_amount.is_empty() {
                "0.00".to_string()
            } else {
                fee_amount
            };

            // Handle different transaction types
            if action == "Insättning" || action == "Uttag" {
                let amount_str: String = format!("{} {}", amount_cash, currency);

                let postings: Vec<transaction::posting::Posting> = vec![
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(&amount_str)
                                .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "expenses:bank:internal-transfers".to_string(),
                        None,
                    ),
                ];
                import_candidates.push(csv_parser::ImportCandidate::Classified(
                    transaction::Transaction::new(date, action + " " + &name, postings),
                ));
            } else if action == "Köp" {
                let commodity_amount_str: String = format!("{} {}", amount_commodity, name);
                let cash_amount_str: String = format!("{} {}", amount_cash, currency);
                let fee_amount_str: String = format!("{} SEK", fee_amount);

                let postings: Vec<transaction::posting::Posting> = vec![
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(
                                &commodity_amount_str,
                            )
                            .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(
                                &cash_amount_str,
                            )
                            .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "expenses:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(&fee_amount_str)
                                .unwrap(),
                        ),
                    ),
                ];

                import_candidates.push(csv_parser::ImportCandidate::Classified(
                    transaction::Transaction::new(date, action + " " + &name, postings),
                ));
            } else if action == "Sälj" {
                let commodity_amount_str: String = format!("{} {}", amount_commodity, name);
                let cash_amount_str: String = format!("{} {}", amount_cash, currency);
                let fee_amount_str: String = format!("{} SEK", fee_amount);
                let profit_str: String = format!("{} SEK", profit);

                let profit_commodity_value =
                    match transaction::commodity_value::CommodityValue::from_str(&profit_str) {
                        Ok(val) => -&val,
                        Err(_) => {
                            eprintln!(
                                "Invalid profit format in line '{}'. Skipping this line.",
                                this_line
                            );
                            continue;
                        }
                    };

                let postings: Vec<transaction::posting::Posting> = vec![
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(
                                &commodity_amount_str,
                            )
                            .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(
                                &cash_amount_str,
                            )
                            .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "expenses:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(&fee_amount_str)
                                .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "equity:capital-gains".to_string(),
                        Some(profit_commodity_value),
                    ),
                ];

                import_candidates.push(csv_parser::ImportCandidate::Classified(
                    transaction::Transaction::new(date, action + " " + &name, postings),
                ));
            } else if action == "Utdelning" {
                let cash_amount_str: String = format!("{} {}", amount_cash, currency);

                let postings: Vec<transaction::posting::Posting> = vec![
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(
                                &cash_amount_str,
                            )
                            .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new("income:dividends".to_string(), None),
                ];

                import_candidates.push(csv_parser::ImportCandidate::Classified(
                    transaction::Transaction::new(date, action + " " + &name, postings),
                ));
            } else if action == "Utländsk källskatt" {
                let tax_amount_str: String = format!("{} {}", amount_cash, currency);

                let postings: Vec<transaction::posting::Posting> = vec![
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(&tax_amount_str)
                                .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "expenses:taxes:withholding".to_string(),
                        None,
                    ),
                ];

                import_candidates.push(csv_parser::ImportCandidate::Classified(
                    transaction::Transaction::new(date, action + " " + &name, postings),
                ));
            } else if action == "Utlåningsränta" {
                let interest_amount_str: String = format!("{} {}", amount_cash, currency);

                let postings: Vec<transaction::posting::Posting> = vec![
                    transaction::posting::Posting::new(
                        "assets:bank:avanza".to_string(),
                        Some(
                            transaction::commodity_value::CommodityValue::from_str(
                                &interest_amount_str,
                            )
                            .unwrap(),
                        ),
                    ),
                    transaction::posting::Posting::new(
                        "expenses:bank:avanza:interest".to_string(),
                        None,
                    ),
                ];

                import_candidates.push(csv_parser::ImportCandidate::Classified(
                    transaction::Transaction::new(date, action + " " + &name, postings),
                ));
            } else {
                eprintln!(
                    "Unknown transaction type '{}' in file {}. Skipping this line.",
                    action,
                    csv_path.display()
                );
            }
        }

        return import_candidates;
    }
}

impl AvanzaParser {
    pub fn new() -> Self {
        AvanzaParser
    }
}
