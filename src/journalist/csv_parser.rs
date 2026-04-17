pub mod avanza_parser;
pub mod default_parser;
pub mod rules;

use crate::cli_utils;
use crate::journalist;
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
        match journalist::journal_parser::parse_journal(&mut lines) {
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
    reader: &mut impl BufRead,
    writer: &mut impl Write,
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
                        println!("{}\n", u);
                        println!("With as the existing transaction:");
                        println!("{}\n", existing.transaction);

                        let user_input: String = cli_utils::prompt_input(
                            "Do you want to classify this transaction as the existing one? (y/n) ",
                            reader,
                            writer,
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
                    println!(
                        "This transaction could not be automatically classified:\n{}\n",
                        u
                    );
                    let user_classification: String = cli_utils::prompt_for_account("Please enter the account to balance this transaction against (e.g. 'expenses:food') or leave empty to skip: ", reader, writer)
                    .unwrap();
                    if user_classification.is_empty() {
                        continue;
                    }
                    let second_posting =
                        transaction::posting::Posting::new(user_classification, None);
                    let classified_transaction = transaction::Transaction::new(
                        *u.get_date(),
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
    reader: &mut impl BufRead,
    writer: &mut impl Write,
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
        deduplicate_transactions(existing_transactions, candidates, reader, writer);

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&journal_path)?;

    for transaction in new_transactions {
        journalist::add_transaction_to_file(&mut file, &transaction)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal_path(filename: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("journals")
            .join(filename)
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
    fn spot_check_basic_transactions_salary_january() {
        use crate::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;

        let result = read_and_hash_journal(journal_path("basic_transactions.journal")).unwrap();

        // Transaction index 1: "2026-01-25 * Salary January"
        let expected = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 25).unwrap(),
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
        use crate::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;

        let result = read_and_hash_journal(journal_path("basic_transactions.journal")).unwrap();

        // Transaction index 6: "2026-02-01 Spotify AB | Monthly subscription" (auto-balance posting)
        let expected = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
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

    // -------------------------------------------------------------------------
    // Test helpers for CSV import tests
    // -------------------------------------------------------------------------

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

    static TEMP_JOURNAL_COUNTER: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    struct TempJournal(std::path::PathBuf);

    impl TempJournal {
        fn new_empty() -> Self {
            let id = TEMP_JOURNAL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("rsledger_test_{}.journal", id));
            std::fs::write(&path, "").unwrap();
            TempJournal(path)
        }

        fn path(&self) -> &std::path::PathBuf {
            &self.0
        }

        fn transaction_count(&self) -> usize {
            read_and_hash_journal(self.0.clone())
                .map(|v| v.len())
                .unwrap_or(0)
        }
    }

    impl Drop for TempJournal {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    // -------------------------------------------------------------------------
    // deduplicate_transactions: classified dedup ignores description
    // -------------------------------------------------------------------------

    /// Functional hash covers date and postings only, not description.
    /// A classified candidate with a different description but identical date/postings
    /// must be treated as a duplicate and not added.
    #[test]
    fn classified_dedup_ignores_description() {
        use crate::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;
        use std::io::Cursor;

        let existing_tx = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE (journal description)".to_string(),
            vec![
                Posting::new(
                    "assets:bank:hsbc".to_string(),
                    Some(CommodityValue::from_str("-25 GBP").unwrap()),
                ),
                Posting::new("expenses:food:groceries".to_string(), None),
            ],
        );
        let existing = vec![HashedTransaction {
            functional_hash: existing_tx.functional_hash(),
            partial_hash: existing_tx.partial_hash(),
            transaction: existing_tx,
        }];

        let candidate_tx = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE BRACKLEY (different CSV description)".to_string(),
            vec![
                Posting::new(
                    "assets:bank:hsbc".to_string(),
                    Some(CommodityValue::from_str("-25 GBP").unwrap()),
                ),
                Posting::new("expenses:food:groceries".to_string(), None),
            ],
        );

        let result = deduplicate_transactions(
            existing,
            vec![ImportCandidate::Classified(candidate_tx)],
            &mut Cursor::new(b""),
            &mut Vec::new(),
        );

        assert_eq!(
            result.len(),
            0,
            "classified transaction should be deduplicated even when descriptions differ"
        );
    }

    // -------------------------------------------------------------------------
    // deduplicate_transactions: unclassified partial match ignores description
    // -------------------------------------------------------------------------

    /// Partial hash covers date and first posting only.
    /// An unclassified candidate whose description differs from the existing journal entry
    /// must still be offered as a partial match, and skipped when the user confirms.
    #[test]
    fn unclassified_partial_match_ignores_description() {
        use crate::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;
        use std::io::Cursor;

        // Existing fully-classified journal entry.
        let existing_tx = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
            "SOME UNKNOWN SHOP original".to_string(),
            vec![
                Posting::new(
                    "assets:bank:hsbc".to_string(),
                    Some(CommodityValue::from_str("-15.50 GBP").unwrap()),
                ),
                Posting::new("expenses:misc".to_string(), None),
            ],
        );
        let existing = vec![HashedTransaction {
            functional_hash: existing_tx.functional_hash(),
            partial_hash: existing_tx.partial_hash(),
            transaction: existing_tx,
        }];

        // Unclassified candidate: same date + first posting, different description.
        let candidate_tx = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
            "SOME UNKNOWN SHOP re-import different description".to_string(),
            vec![Posting::new(
                "assets:bank:hsbc".to_string(),
                Some(CommodityValue::from_str("-15.50 GBP").unwrap()),
            )],
        );

        // User confirms the partial match → transaction should be skipped.
        let result = deduplicate_transactions(
            existing,
            vec![ImportCandidate::Unclassified(candidate_tx)],
            &mut Cursor::new(b"y\n"),
            &mut Vec::new(),
        );

        assert_eq!(
            result.len(),
            0,
            "unclassified transaction should be skipped when user confirms partial match, even if descriptions differ"
        );
    }

    // -------------------------------------------------------------------------
    // deduplicate_transactions: redundant trailing zeros are normalised
    // -------------------------------------------------------------------------

    /// FixedDecimal strips trailing zeros on parse, so "-25.00 GBP" and "-25 GBP"
    /// produce identical hashes and must be treated as duplicates.
    #[test]
    fn classified_dedup_handles_redundant_decimal_digits() {
        use crate::commodity_value::CommodityValue;
        use crate::transaction::posting::Posting;
        use std::io::Cursor;

        // Existing entry stored with minimal precision (as written to the journal).
        let existing_tx = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE BRACKLEY".to_string(),
            vec![
                Posting::new(
                    "assets:bank:hsbc".to_string(),
                    Some(CommodityValue::from_str("-25 GBP").unwrap()),
                ),
                Posting::new("expenses:food:groceries".to_string(), None),
            ],
        );
        let existing = vec![HashedTransaction {
            functional_hash: existing_tx.functional_hash(),
            partial_hash: existing_tx.partial_hash(),
            transaction: existing_tx,
        }];

        // CSV candidate carries redundant trailing zeros (-25.00 instead of -25).
        let candidate_tx = transaction::Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE BRACKLEY".to_string(),
            vec![
                Posting::new(
                    "assets:bank:hsbc".to_string(),
                    Some(CommodityValue::from_str("-25.00 GBP").unwrap()),
                ),
                Posting::new("expenses:food:groceries".to_string(), None),
            ],
        );

        let result = deduplicate_transactions(
            existing,
            vec![ImportCandidate::Classified(candidate_tx)],
            &mut Cursor::new(b""),
            &mut Vec::new(),
        );

        assert_eq!(
            result.len(),
            0,
            "-25.00 GBP should be treated as identical to -25 GBP for deduplication"
        );
    }

    // -------------------------------------------------------------------------
    // import_transactions_from_csv: same CSV imported twice adds nothing new
    // -------------------------------------------------------------------------

    /// End-to-end check: importing an all-classified CSV twice must leave the
    /// journal with exactly the same number of transactions after each run.
    #[test]
    fn import_same_csv_twice_only_adds_once() {
        let journal = TempJournal::new_empty();
        let parser = default_parser::DefaultParser::new(
            "assets:bank:hsbc".to_string(),
            "GBP".to_string(),
            rule_sheet_path("valid_rules.toml"),
            ',',
            false,
            0,
            "%d/%m/%Y".to_string(),
            vec![1],
            2,
            None,
            Some(','),
            '.',
        );

        import_transactions_from_csv(
            &parser,
            &csv_path("hsbc_classified.csv"),
            journal.path(),
            &mut std::io::Cursor::new(b""),
            &mut Vec::new(),
        )
        .unwrap();
        let after_first = journal.transaction_count();

        import_transactions_from_csv(
            &parser,
            &csv_path("hsbc_classified.csv"),
            journal.path(),
            &mut std::io::Cursor::new(b""),
            &mut Vec::new(),
        )
        .unwrap();
        let after_second = journal.transaction_count();

        assert_eq!(
            after_first, 2,
            "first import should add both classified transactions"
        );
        assert_eq!(
            after_second, after_first,
            "second import of the same CSV should not add any new transactions"
        );
    }

    // -------------------------------------------------------------------------
    // import_transactions_from_csv: partial match works with a different description
    // -------------------------------------------------------------------------

    /// First import hsbc_mixed.csv (one classified, one unclassified).
    /// Second import hsbc_mixed_alt_desc.csv — same amounts/dates but the unclassified
    /// transaction has a different description.  The partial match (date + first posting)
    /// must still fire and the user can skip it, leaving the journal unchanged.
    #[test]
    fn import_mixed_csv_twice_partial_match_with_different_description() {
        let journal = TempJournal::new_empty();
        let parser = default_parser::DefaultParser::new(
            "assets:bank:hsbc".to_string(),
            "GBP".to_string(),
            rule_sheet_path("valid_rules.toml"),
            ',',
            false,
            0,
            "%d/%m/%Y".to_string(),
            vec![1],
            2,
            None,
            Some(','),
            '.',
        );

        // First import: classified transaction added automatically; unclassified one
        // requires the user to supply an account.
        import_transactions_from_csv(
            &parser,
            &csv_path("hsbc_mixed.csv"),
            journal.path(),
            &mut std::io::Cursor::new(b"expenses:misc\n"),
            &mut Vec::new(),
        )
        .unwrap();
        let after_first = journal.transaction_count();

        // Second import uses the same amounts and dates but a different description for
        // the unclassified transaction.  The partial hash should still match the existing
        // journal entry, and the user confirms the match with "y".
        import_transactions_from_csv(
            &parser,
            &csv_path("hsbc_mixed_alt_desc.csv"),
            journal.path(),
            &mut std::io::Cursor::new(b"y\n"),
            &mut Vec::new(),
        )
        .unwrap();
        let after_second = journal.transaction_count();

        assert_eq!(after_first, 2, "first import should add both transactions");
        assert_eq!(
            after_second, after_first,
            "re-importing with a different unclassified description should not add duplicates"
        );
    }
}
