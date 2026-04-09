pub mod avanza_parser;

use crate::journalist::journal_parser;
use crate::journalist::prompt_input;
use crate::transaction;

use std::io::{BufRead, Lines, Write};
use std::iter::Peekable;

/// A struct that combines the transaction with its functional and partial hashes for easy comparison
struct HashedTransaction {
    functional_hash: u64,
    partial_hash: u64,
    transaction: transaction::Transaction,
}

fn read_and_hash_journal(journal_path: std::path::PathBuf) -> Option<Vec<HashedTransaction>> {
    let file = match std::fs::File::open(&journal_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening journal file: {}", e);
            return None;
        }
    };

    let mut lines: Peekable<Lines<std::io::BufReader<std::fs::File>>> =
        std::io::BufReader::new(file).lines().peekable();

    let transactions: Vec<transaction::Transaction> =
        match journal_parser::parse_journal(&mut lines) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error parsing journal: {}", e);
                return None;
            }
        };

    let hashed_transactions: Vec<HashedTransaction> = transactions
        .into_iter()
        .map(|t| {
            let functional_hash = t.functional_hash();
            let partial_hash = t.partial_hash();
            HashedTransaction {
                functional_hash,
                partial_hash,
                transaction: t,
            }
        })
        .collect();

    return Some(hashed_transactions);
}

/// Enum representing a candidate transaction from the CSV
/// It can either be classifiable (i.e all the postings could be automatically resolved)
/// or unclassifiable (i.e. postings need manual review)
pub enum ImportCandidate {
    /// A fully classified transaction to add as it is
    Classified(transaction::Transaction),
    /// An unclassifiable transaction for the user to classify manually (should only have the first posting defined)
    Unclassified(transaction::Transaction),
}

/// Trait for csv importers.
///
/// Each CSV importer can define arbitrarily complex logic to parse a CSV
/// and classify the transactions it contains. It must then return a list of
/// `ImportCandidate` objects.
pub trait CSVImporter {
    fn import_csv(&self, csv_path: std::path::PathBuf) -> Vec<ImportCandidate>;
}

/// Handles the ImportCandidate objects and deduplicates against existing transactions in the journal.
///
/// For classified candidates, it skips those which already exist in the journal by checking
/// the functional hash. For unclassified candidates, it checks against the partial hash
/// and prompts the user to either confirm the match or to classify the transaction manually,
/// and then have it added to the journal as a new transaction.
fn deduplicate_transactions(
    existing_transactions: Vec<HashedTransaction>,
    candidates: Vec<ImportCandidate>,
) -> Vec<transaction::Transaction> {
    let mut new_transactions: Vec<transaction::Transaction> = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        match candidate {
            ImportCandidate::Classified(c) => {
                let candidate_hash: u64 = c.functional_hash();
                if existing_transactions
                    .iter()
                    .any(|t| t.functional_hash == candidate_hash)
                {
                    // Skip this transaction as it already exists in the journal
                    continue;
                } else {
                    new_transactions.push(c);
                }
            }
            ImportCandidate::Unclassified(u) => {
                // Compute the equivalent of the partial hash for this unclassified transaction
                let candidate_partial_hash: u64 = u.partial_hash();
                let mut skip: bool = false;

                for existing in &existing_transactions {
                    if existing.partial_hash == candidate_partial_hash {
                        // Ask the user if they want to classify this transaction as the existing one
                        println!("Found a potential match for the unclassified transaction:");
                        println!("{}", u);
                        println!("With as the existing transaction:");
                        println!("{}", existing.transaction);

                        let user_input: String = prompt_input(
                            "Do you want to classify this transaction as the existing one? (y/n) ",
                        )
                        .unwrap();
                        if user_input.to_lowercase() == "y" {
                            // User confirmed the match, so we skip adding this transaction
                            skip = true;
                            break;
                        }
                    }
                }

                // If we get here and skip is false, it means there were no approved matches
                if !skip {
                    println!("{u}");
                    let user_classification: String = prompt_input("Please enter the account to balance this transaction against (e.g. 'expenses:food') or leave empty to skip: ")
                    .unwrap();
                    if user_classification.is_empty() {
                        continue;
                    }
                    let second_posting =
                        transaction::posting::Posting::new(user_classification, None);
                    let classified_transaction = transaction::Transaction::new(
                        u.get_date().to_string(),
                        u.get_description().to_string(),
                        vec![u.get_postings()[0].clone(), second_posting],
                    );
                    new_transactions.push(classified_transaction);
                }
            }
        }
    }

    return new_transactions;
}

/// Main function to handle the CSV import process
///
/// 1. Reads and hashes existing transactions in the journal
/// 2. Uses the provided CSVImporter to parse the CSV and get a list of ImportCandidates
/// 3. Deduplicates the candidates against existing transactions and prompts the user for manual classification when needed
/// 4. Appends the new transactions to the journal file
pub fn import_transactions_from_csv(
    csv_importer: &dyn CSVImporter,
    csv_path: &std::path::PathBuf,
    journal_path: &std::path::PathBuf,
) -> std::io::Result<()> {
    let existing_transactions: Vec<HashedTransaction> =
        match read_and_hash_journal(journal_path.clone()) {
            Some(t) => t,
            None => {
                eprintln!(
                    "Error reading and hashing existing transactions from {}. Aborting import.",
                    journal_path.display()
                );
                return Ok(());
            }
        };

    let candidates: Vec<ImportCandidate> = csv_importer.import_csv(csv_path.clone());

    let new_transactions: Vec<transaction::Transaction> =
        deduplicate_transactions(existing_transactions, candidates);

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&journal_path)?;

    for transaction in new_transactions {
        write!(file, "{}", transaction)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal_path(filename: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join(filename)
    }

    #[test]
    fn read_and_hash_single_transaction_returns_one_entry() {
        let result = read_and_hash_journal(journal_path("single_transaction.journal"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn read_and_hash_basic_transactions_returns_all_entries() {
        let result = read_and_hash_journal(journal_path("basic_transactions.journal"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 15);
    }

    #[test]
    fn read_and_hash_is_deterministic() {
        let first = read_and_hash_journal(journal_path("basic_transactions.journal")).unwrap();
        let second = read_and_hash_journal(journal_path("basic_transactions.journal")).unwrap();

        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.functional_hash, b.functional_hash);
            assert_eq!(a.partial_hash, b.partial_hash);
        }
    }

    #[test]
    fn functional_and_partial_hashes_differ_for_single_transaction() {
        let result = read_and_hash_journal(journal_path("single_transaction.journal")).unwrap();
        let h = &result[0];
        // The functional hash excludes the description; the partial hash includes it.
        // They should differ for any real transaction.
        assert_ne!(h.functional_hash, h.partial_hash);
    }

    #[test]
    fn basic_transactions_functional_hashes_are_unique() {
        let result = read_and_hash_journal(journal_path("basic_transactions.journal")).unwrap();
        let mut seen = std::collections::HashSet::new();
        for h in &result {
            assert!(
                seen.insert(h.functional_hash),
                "duplicate functional_hash found: {}",
                h.functional_hash
            );
        }
    }

    // -------------------------------------------------------------------------
    // Spot-check tests: explicitly constructed transactions vs parsed hashes
    // -------------------------------------------------------------------------

    #[test]
    fn spot_check_single_transaction_hashes() {
        use crate::transaction::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;

        let result = read_and_hash_journal(journal_path("single_transaction.journal")).unwrap();

        let expected = transaction::Transaction::new(
            "2025-04-03".to_string(),
            "Test transaction".to_string(),
            vec![
                Posting::new(
                    "assets:bank".to_string(),
                    Some(CommodityValue::from_str("-435 GBP").unwrap()),
                ),
                Posting::new("expenses:travel:flights".to_string(), None),
            ],
        );

        assert_eq!(result[0].functional_hash, expected.functional_hash());
        assert_eq!(result[0].partial_hash, expected.partial_hash());
    }

    #[test]
    fn spot_check_basic_transactions_salary_january() {
        use crate::transaction::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;

        let result = read_and_hash_journal(journal_path("basic_transactions.journal")).unwrap();

        // Transaction index 1: "2026-01-25 * Salary January"
        let expected = transaction::Transaction::new(
            "2026-01-25".to_string(),
            "* Salary January".to_string(),
            vec![
                Posting::new(
                    "assets:bank:checking".to_string(),
                    Some(CommodityValue::from_str("35000.00 SEK").unwrap()),
                ),
                Posting::new(
                    "income:salary".to_string(),
                    Some(CommodityValue::from_str("-35000.00 SEK").unwrap()),
                ),
            ],
        );

        assert_eq!(result[1].functional_hash, expected.functional_hash());
        assert_eq!(result[1].partial_hash, expected.partial_hash());
    }

    #[test]
    fn spot_check_basic_transactions_spotify_autobalance() {
        use crate::transaction::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;

        let result = read_and_hash_journal(journal_path("basic_transactions.journal")).unwrap();

        // Transaction index 6: "2026-02-01 Spotify AB | Monthly subscription" (auto-balance posting)
        let expected = transaction::Transaction::new(
            "2026-02-01".to_string(),
            "Spotify AB | Monthly subscription".to_string(),
            vec![
                Posting::new(
                    "expenses:entertainment".to_string(),
                    Some(CommodityValue::from_str("119.00 SEK").unwrap()),
                ),
                Posting::new("assets:bank:checking".to_string(), None),
            ],
        );

        assert_eq!(result[6].functional_hash, expected.functional_hash());
        assert_eq!(result[6].partial_hash, expected.partial_hash());
    }
}
