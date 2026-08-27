pub(crate) mod avanza_prices;
pub(crate) mod avanza_transactions;
pub(crate) mod generic_importer;
pub(crate) mod rules;

use std::fmt::Display;
use std::hash::Hash;

/// Enum to represent an import candidate with its classification status.
/// Some candidates can be automatically classified while others may require manual classification.
pub(crate) enum ImportCandidate<T: Display + Hash> {
    Classified(T),
    Unclassified(T),
}

/// Defines the IO-behaviour of an entry importer, allowing the underlying
/// importing logic to be arbitrarily complex as long as it adheres to this interface.
pub(crate) trait EntryImporter<T: Display + Hash> {
    /// Reads the CSV at `csv_path` and returns a list of import candidates.
    ///
    /// Returns an error if the file cannot be opened or a row cannot be parsed.
    fn import_csv(&self, csv_path: std::path::PathBuf) -> crate::Result<Vec<ImportCandidate<T>>>;
}

/// Imports transaction candidates into a journal file, deduplicating against existing entries.
///
/// **Classified** candidates are compared against existing transactions using
/// the *functional hash* (date + postings, description excluded). A candidate
/// whose functional hash matches an existing transaction is silently skipped.
///
/// **Unclassified** candidates (single-posting entries whose payee is unknown)
/// are compared by *partial hash* (date + first posting). On a hit the user is
/// shown both entries and asked to confirm the duplicate. On a miss, the user is
/// prompted for a counterpart account so the transaction can be completed before
/// being written to the journal.
///
/// Set `accept_partial_matches` to `true` (the CLI `-y` flag) to auto-confirm all
/// partial-match queries without prompting.
pub(crate) fn import_entries(
    candidates: Vec<ImportCandidate<crate::journal::transaction::Transaction>>,
    journal_file: &mut crate::journal::JournalFile,
    accept_partial_matches: bool,
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> crate::Result<()> {
    use crate::journal::transaction::Transaction;
    use crate::journal::transaction::posting::Posting;

    let journal = journal_file.load()?;

    // Build a lookup of existing transactions keyed by their functional hash
    // (date + postings, excluding description) and partial hash (date + first posting).
    // We recompute these from the loaded transactions rather than using the stored
    // DefaultHasher hashes, because functional_hash deliberately excludes description
    // to catch re-imports where the CSV payee name differs from the journal description.
    let existing: Vec<(u64, u64, &Transaction)> = journal
        .transactions
        .iter()
        .map(|(_, t)| (t.functional_hash(), t.partial_hash(), t))
        .collect();

    for candidate in candidates {
        match candidate {
            ImportCandidate::Classified(t) => {
                // Skip if the journal already has a transaction with the same date
                // and postings, regardless of description.
                if existing.iter().any(|(fh, _, _)| *fh == t.functional_hash()) {
                    continue;
                }
                journal_file.append_transaction(&t)?;
            }

            ImportCandidate::Unclassified(u) => {
                let partial = u.partial_hash();
                let mut skip = false;

                // Search for an existing transaction whose partial hash matches.
                for (_, eph, existing_t) in &existing {
                    if *eph != partial {
                        continue;
                    }
                    if accept_partial_matches {
                        skip = true;
                        break;
                    }
                    // Present both entries to the user and ask for confirmation.
                    writeln!(
                        writer,
                        "Found a potential match for the unclassified transaction:"
                    )?;
                    writeln!(writer, "{}\n", u)?;
                    writeln!(writer, "With the existing transaction:")?;
                    writeln!(writer, "{}\n", existing_t)?;
                    let input = crate::cli::utils::prompt_input(
                        "Do you want to classify this transaction as the existing one? (y/n) ",
                        reader,
                        writer,
                    )?;
                    if input.to_lowercase() == "y" {
                        skip = true;
                        break;
                    }
                }

                if !skip {
                    // No match confirmed — prompt the user for the counterpart account
                    // so the transaction can be balanced and written.
                    writeln!(
                        writer,
                        "This transaction could not be automatically classified:\n{}\n",
                        u
                    )?;
                    let account = crate::cli::utils::prompt_for_account(
                        "Please enter the account to balance this transaction against \
                         (e.g. 'expenses:food'): ",
                        reader,
                        writer,
                    )?;
                    let second_posting = Posting::new(account, None);
                    let classified = Transaction::new(
                        *u.date(),
                        u.description().clone(),
                        vec![u.postings()[0].clone(), second_posting],
                    );
                    journal_file.append_transaction(&classified)?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::JournalFile;
    use crate::journal::account::Account;
    use crate::journal::commodity_value::CommodityValue;
    use crate::journal::transaction::Transaction;
    use crate::journal::transaction::posting::Posting;
    use chrono::NaiveDate;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn journal_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("journals")
            .join(filename)
    }

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

    // -------------------------------------------------------------------------
    // Journal loading tests
    // -------------------------------------------------------------------------

    #[test]
    fn loading_basic_transactions_returns_all_entries() {
        let result = JournalFile::new(journal_path("basic_transactions.journal"))
            .load()
            .unwrap();
        assert_eq!(result.transactions.len(), 15);
    }

    #[test]
    fn journal_functional_hashes_are_deterministic() {
        let first = JournalFile::new(journal_path("basic_transactions.journal"))
            .load()
            .unwrap();
        let second = JournalFile::new(journal_path("basic_transactions.journal"))
            .load()
            .unwrap();
        for ((_, a), (_, b)) in first.transactions.iter().zip(second.transactions.iter()) {
            assert_eq!(a.functional_hash(), b.functional_hash());
            assert_eq!(a.partial_hash(), b.partial_hash());
        }
    }

    #[test]
    fn basic_transactions_functional_hashes_are_unique() {
        let result = JournalFile::new(journal_path("basic_transactions.journal"))
            .load()
            .unwrap();
        let mut seen = std::collections::HashSet::new();
        for (_, t) in &result.transactions {
            assert!(
                seen.insert(t.functional_hash()),
                "duplicate functional_hash found"
            );
        }
    }

    #[test]
    fn spot_check_salary_january_hashes() {
        let result = JournalFile::new(journal_path("basic_transactions.journal"))
            .load()
            .unwrap();
        let (_, loaded_t) = &result.transactions[1];

        // Transaction index 1 in basic_transactions.journal: "2026-01-25 * Salary January"
        let expected = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 1, 25).unwrap(),
            "* Salary January".to_string(),
            vec![
                Posting::new(
                    Account::from_str("assets:bank:checking").unwrap(),
                    Some(CommodityValue::from_str("35000.00 SEK").unwrap()),
                ),
                Posting::new(
                    Account::from_str("income:salary").unwrap(),
                    Some(CommodityValue::from_str("-35000.00 SEK").unwrap()),
                ),
            ],
        );
        assert_eq!(loaded_t.functional_hash(), expected.functional_hash());
        assert_eq!(loaded_t.partial_hash(), expected.partial_hash());
    }

    #[test]
    fn spot_check_spotify_autobalance_hashes() {
        let result = JournalFile::new(journal_path("basic_transactions.journal"))
            .load()
            .unwrap();
        let (_, loaded_t) = &result.transactions[6];

        // Transaction index 6: "2026-02-01 Spotify AB | Monthly subscription"
        let expected = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            "Spotify AB | Monthly subscription".to_string(),
            vec![
                Posting::new(
                    Account::from_str("expenses:entertainment").unwrap(),
                    Some(CommodityValue::from_str("119.00 SEK").unwrap()),
                ),
                Posting::new(Account::from_str("assets:bank:checking").unwrap(), None),
            ],
        );
        assert_eq!(loaded_t.functional_hash(), expected.functional_hash());
        assert_eq!(loaded_t.partial_hash(), expected.partial_hash());
    }

    // -------------------------------------------------------------------------
    // TempJournal helper
    // -------------------------------------------------------------------------

    static TEMP_JOURNAL_COUNTER: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    struct TempJournal(PathBuf);

    impl TempJournal {
        fn new_empty() -> Self {
            let id = TEMP_JOURNAL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("rsledger_import_test_{}.journal", id));
            std::fs::write(&path, "").unwrap();
            TempJournal(path)
        }

        fn new_with_transaction(t: &Transaction) -> Self {
            let temp = Self::new_empty();
            JournalFile::new(temp.0.clone())
                .append_transaction(t)
                .unwrap();
            temp
        }

        fn journal_file(&self) -> JournalFile {
            JournalFile::new(self.0.clone())
        }

        fn transaction_count(&self) -> usize {
            JournalFile::new(self.0.clone())
                .load()
                .map(|j| j.transactions.len())
                .unwrap_or(0)
        }
    }

    impl Drop for TempJournal {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    // -------------------------------------------------------------------------
    // import_entries: classified dedup ignores description
    // -------------------------------------------------------------------------

    /// Functional hash covers date and postings only, not description.
    /// A classified candidate with a different description but identical date/postings
    /// must be treated as a duplicate and not added.
    #[test]
    fn classified_dedup_ignores_description() {
        let existing_tx = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE (journal description)".to_string(),
            vec![
                Posting::new(
                    Account::from_str("assets:bank:hsbc").unwrap(),
                    Some(CommodityValue::from_str("-25 GBP").unwrap()),
                ),
                Posting::new(Account::from_str("expenses:food:groceries").unwrap(), None),
            ],
        );
        let temp = TempJournal::new_with_transaction(&existing_tx);

        let candidate = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE BRACKLEY (different CSV description)".to_string(),
            vec![
                Posting::new(
                    Account::from_str("assets:bank:hsbc").unwrap(),
                    Some(CommodityValue::from_str("-25 GBP").unwrap()),
                ),
                Posting::new(Account::from_str("expenses:food:groceries").unwrap(), None),
            ],
        );
        import_entries(
            vec![ImportCandidate::Classified(candidate)],
            &mut temp.journal_file(),
            false,
            &mut Cursor::new(b""),
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(
            temp.transaction_count(),
            1,
            "classified transaction should be deduplicated even when descriptions differ"
        );
    }

    // -------------------------------------------------------------------------
    // import_entries: unclassified partial match ignores description
    // -------------------------------------------------------------------------

    /// Partial hash covers date and first posting only.
    /// An unclassified candidate whose description differs from the existing journal entry
    /// must still be offered as a partial match, and skipped when the user confirms.
    #[test]
    fn unclassified_partial_match_ignores_description() {
        let existing_tx = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
            "SOME UNKNOWN SHOP original".to_string(),
            vec![
                Posting::new(
                    Account::from_str("assets:bank:hsbc").unwrap(),
                    Some(CommodityValue::from_str("-15.50 GBP").unwrap()),
                ),
                Posting::new(Account::from_str("expenses:misc").unwrap(), None),
            ],
        );
        let temp = TempJournal::new_with_transaction(&existing_tx);

        let candidate = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
            "SOME UNKNOWN SHOP re-import different description".to_string(),
            vec![Posting::new(
                Account::from_str("assets:bank:hsbc").unwrap(),
                Some(CommodityValue::from_str("-15.50 GBP").unwrap()),
            )],
        );
        // User confirms the partial match → transaction should be skipped.
        import_entries(
            vec![ImportCandidate::Unclassified(candidate)],
            &mut temp.journal_file(),
            false,
            &mut Cursor::new(b"y\n"),
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(
            temp.transaction_count(),
            1,
            "unclassified transaction should be skipped when user confirms partial match"
        );
    }

    // -------------------------------------------------------------------------
    // import_entries: redundant trailing zeros are normalised
    // -------------------------------------------------------------------------

    /// FixedDecimal strips trailing zeros on parse, so "-25.00 GBP" and "-25 GBP"
    /// produce identical hashes and must be treated as duplicates.
    #[test]
    fn classified_dedup_handles_redundant_decimal_digits() {
        let existing_tx = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE BRACKLEY".to_string(),
            vec![
                Posting::new(
                    Account::from_str("assets:bank:hsbc").unwrap(),
                    Some(CommodityValue::from_str("-25 GBP").unwrap()),
                ),
                Posting::new(Account::from_str("expenses:food:groceries").unwrap(), None),
            ],
        );
        let temp = TempJournal::new_with_transaction(&existing_tx);

        let candidate = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(),
            "GROCERY STORE BRACKLEY".to_string(),
            vec![
                Posting::new(
                    Account::from_str("assets:bank:hsbc").unwrap(),
                    Some(CommodityValue::from_str("-25.00 GBP").unwrap()),
                ),
                Posting::new(Account::from_str("expenses:food:groceries").unwrap(), None),
            ],
        );
        import_entries(
            vec![ImportCandidate::Classified(candidate)],
            &mut temp.journal_file(),
            false,
            &mut Cursor::new(b""),
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(
            temp.transaction_count(),
            1,
            "-25.00 GBP should be treated as identical to -25 GBP for deduplication"
        );
    }

    // -------------------------------------------------------------------------
    // import_entries: end-to-end CSV import
    // -------------------------------------------------------------------------

    fn hsbc_importer(rule_sheet: &str) -> generic_importer::GenericImporter {
        generic_importer::GenericImporter::new(
            Account::from_str("assets:bank:hsbc").unwrap(),
            "GBP".to_string(),
            Some(rule_sheet_path(rule_sheet)),
            ',',
            false,
            0,
            "%d/%m/%Y".to_string(),
            vec![1],
            2,
            None,
            Some(','),
            '.',
        )
        .unwrap()
    }

    /// End-to-end: importing the same CSV twice must add transactions only once.
    #[test]
    fn import_same_csv_twice_only_adds_once() {
        let temp = TempJournal::new_empty();
        let importer = hsbc_importer("valid_rules.toml");

        let c1 = importer
            .import_csv(csv_path("hsbc_classified.csv"))
            .unwrap();
        import_entries(
            c1,
            &mut temp.journal_file(),
            false,
            &mut Cursor::new(b""),
            &mut Vec::new(),
        )
        .unwrap();
        let after_first = temp.transaction_count();

        let c2 = importer
            .import_csv(csv_path("hsbc_classified.csv"))
            .unwrap();
        import_entries(
            c2,
            &mut temp.journal_file(),
            false,
            &mut Cursor::new(b""),
            &mut Vec::new(),
        )
        .unwrap();
        let after_second = temp.transaction_count();

        assert_eq!(
            after_first, 2,
            "first import should add both classified transactions"
        );
        assert_eq!(
            after_second, after_first,
            "second import should not add duplicates"
        );
    }

    /// End-to-end: re-importing with a different description for the unclassified entry
    /// must still fire the partial-match check and leave the journal unchanged when confirmed.
    #[test]
    fn import_mixed_csv_twice_partial_match_with_different_description() {
        let temp = TempJournal::new_empty();
        let importer = hsbc_importer("valid_rules.toml");

        // First import: classified added automatically; unclassified needs manual account.
        let c1 = importer.import_csv(csv_path("hsbc_mixed.csv")).unwrap();
        import_entries(
            c1,
            &mut temp.journal_file(),
            false,
            &mut Cursor::new(b"expenses:misc\n"),
            &mut Vec::new(),
        )
        .unwrap();
        let after_first = temp.transaction_count();

        // Second import: same amounts/dates but different description on the unclassified row.
        // Partial hash still fires; user confirms with "y".
        let c2 = importer
            .import_csv(csv_path("hsbc_mixed_alt_desc.csv"))
            .unwrap();
        import_entries(
            c2,
            &mut temp.journal_file(),
            false,
            &mut Cursor::new(b"y\n"),
            &mut Vec::new(),
        )
        .unwrap();
        let after_second = temp.transaction_count();

        assert_eq!(after_first, 2, "first import should add both transactions");
        assert_eq!(
            after_second, after_first,
            "re-importing with a different unclassified description should not add duplicates"
        );
    }
}
