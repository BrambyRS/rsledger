use crate::journalist::journal_parser;
use crate::transaction;

use std::io::BufRead;
use std::io::Lines;
use std::iter::Peekable;

// TODO:
// - [x] Read and hash transactions in current journal
//     - [x] Implement hashing of transactions
//     - [x] Implement comparison of transaction hashes
//     - [x] Implement journal parser
// - [ ] Implement rules for classifying transactions
// - [ ] Implement rules for skipping transactions I want to enter explicitly
// - [ ] Implement interactive prompt for manual classification when auto fails
// - [ ] Implement skipping of pre-existing transactions
// - [ ] Implement matching of transactions between accounts to remove duplicates
// - [ ] Implement reading of prices sheet for stock prices

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
