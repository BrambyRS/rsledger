pub mod posting;

use crate::commodity_value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Represents a financial transaction with a date, description, and multiple posts (account and amount pairs).
#[derive(Hash)]
pub struct Transaction {
    /// Date of the transaction in YYYY-MM-DD format.
    date: chrono::NaiveDate,
    /// Description of the transaction.
    description: String,
    /// Account and amount pairs. For a simple double-entry transaction, there would be two posts with opposite amounts.
    postings: Vec<posting::Posting>,
}

/// Formats the transaction as a journal entry:
///
/// ```text
/// YYYY-MM-DD Description
///     Account1 123.45 SEK
///     Account2 -123.45 SEK
/// ```
impl core::fmt::Display for Transaction {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} {}\n", self.date, self.description)?;
        let mut i: usize = 0;
        let num_postings: usize = self.postings.len();
        for posting in &self.postings {
            match write!(f, "\t{}", posting) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }

            if i < num_postings - 1 {
                match write!(f, "\n") {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            i += 1;
        }
        return Ok(());
    }
}

impl Transaction {
    /// Creates a new `Transaction` with the given date, description, and postings.
    ///
    /// # Examples
    /// ```
    /// let t = Transaction::new(
    ///     chrono::NaiveDate::from_ymd(2024, 1, 1),
    ///     "Groceries".to_string(),
    ///     vec![
    ///         posting::Posting::new("expenses:food".to_string(), Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap())),
    ///         posting::Posting::new("assets:bank".to_string(), None),
    ///     ],
    /// );
    /// ```
    pub fn new(
        date: chrono::NaiveDate,
        description: String,
        postings: Vec<posting::Posting>,
    ) -> Self {
        Transaction {
            date,
            description,
            postings,
        }
    }

    /// Returns `true` if the transaction is balanced.
    ///
    /// A transaction is considered balanced when either:
    /// - Exactly one posting has a `None` amount (auto-balancing entry), or
    /// - All postings have explicit amounts with the same commodity, and their sum is zero
    /// - A transaction comprising multiple commodities must either:
    ///     - Have the sum for each commodity equal to zero
    ///     - Have two or more commodities unbalanced, at least one positive and one negative, indicating an exchange is taking place.
    ///
    /// Returns `false` if more than one posting has a `None` amount, or if any
    /// commodity's postings do not sum to zero.
    pub fn validate(&self) -> bool {
        // If there is a None amount, the transaction is auto balanced
        // More than a single None amount makes the transaction invalid
        let mut none_amount_count: usize = 0;
        for posting in &self.postings {
            if posting.get_amount().is_none() {
                none_amount_count += 1;
                if none_amount_count > 1 {
                    return false;
                }
            }
        }
        if none_amount_count == 1 {
            return true;
        }

        // Sum amounts by commodity
        // Total possible number of unique commodities is equal to the number of postings, so we can set the initial capacity of the HashMap to that.
        let mut totals_per_commodity: std::collections::HashMap<
            String,
            commodity_value::fixed_decimal::FixedDecimal,
        > = std::collections::HashMap::with_capacity(self.postings.len());
        for posting in &self.postings {
            if let Some(amount) = posting.get_amount() {
                let this_commodity: String = amount.commodity().to_string();
                let this_amount: commodity_value::fixed_decimal::FixedDecimal =
                    amount.amount().clone();
                totals_per_commodity
                    .entry(this_commodity.clone())
                    .and_modify(|total| *total += &this_amount)
                    .or_insert(this_amount);
            }
        }

        // Collect commodities whose postings do not sum to zero.
        let mut has_positive = false;
        let mut has_negative = false;
        let mut unbalanced_count: usize = 0;
        for total in totals_per_commodity.values() {
            if total.raw_amount() != 0 {
                unbalanced_count += 1;
                if total.raw_amount() > 0 {
                    has_positive = true;
                } else {
                    has_negative = true;
                }
            }
        }

        // If all commodities are balanced the transaction is valid.
        if unbalanced_count == 0 {
            return true;
        }

        // If there are multiple unbalanced commodities where they flow both in and out it is valid.
        if unbalanced_count >= 2 && has_positive && has_negative {
            return true;
        }

        return false;
    }

    /// Returns a hash of the transaction based on the date and all postings.
    ///
    /// This is used for comparing if two transactions are *functionally identical*
    /// (same date, accounts, and amounts) even if they have different descriptions
    /// or different posting order. This is useful for identifying duplicate transactions.
    pub fn functional_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.date.hash(&mut hasher);
        let mut posting_hashes: Vec<u64> = self
            .postings
            .iter()
            .map(|p| {
                let mut h = DefaultHasher::new();
                p.hash(&mut h);
                h.finish()
            })
            .collect();
        posting_hashes.sort_unstable();
        for h in posting_hashes {
            h.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Returns a hash of only part of the transaction's data.
    ///
    /// This is used for hashing a transaction based only on the date and first posting.
    /// This is useful for identifying duplicate transactions during
    /// CSV import in cases where it can't be fully classified and compared to the full
    /// transaction.
    pub fn partial_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.date.hash(&mut hasher);
        if let Some(first_post) = self.postings.first() {
            first_post.hash(&mut hasher);
        }
        hasher.finish()
    }

    // Getters

    pub fn get_date(&self) -> &chrono::NaiveDate {
        &self.date
    }

    pub fn get_description(&self) -> &String {
        &self.description
    }

    pub fn get_postings(&self) -> &Vec<posting::Posting> {
        &self.postings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Display formatting tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_transaction_display_two_postings() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-123.45 SEK").unwrap()),
                ),
            ],
        );

        let expected_display =
            "2024-01-01 Test Transaction\n\tAccount 1  123.45 SEK\n\tAccount 2  -123.45 SEK";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    #[test]
    fn test_transaction_display_multiple_postings() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 3".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
            ],
        );

        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1  100 GBP\n\tAccount 2  -50 GBP\n\tAccount 3  -50 GBP";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    // -------------------------------------------------------------------------
    // Validate tests: explicit amounts
    // -------------------------------------------------------------------------

    #[test]
    fn test_transaction_validate_balanced_single_commodity() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 3".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_unbalanced_single_commodity() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-30.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "Account 3".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        assert!(!transaction.validate());
    }

    #[test]
    fn test_transaction_validate_balanced_multiple_commodities() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 3".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 4".to_string(),
                    Some(commodity_value::CommodityValue::from_str("200.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "Account 5".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-200.00 SEK").unwrap()),
                ),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_unbalanced_multiple_commodities() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 3".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "Account 4".to_string(),
                    Some(commodity_value::CommodityValue::from_str("200.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "Account 5".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-150.00 SEK").unwrap()),
                ),
            ],
        );
        assert!(!transaction.validate());
    }

    #[test]
    fn test_transaction_validate_exchange_two_commodities() {
        // Two unbalanced commodities with opposite signs → valid exchange
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Currency Exchange".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-1500.00 SEK").unwrap()),
                ),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_exchange_three_commodities_one_balanced() {
        // Two unbalanced commodities (opposite signs) + one balanced → valid exchange
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Currency Exchange".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-1500.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "expenses:fees".to_string(),
                    Some(commodity_value::CommodityValue::from_str("5.00 EUR").unwrap()),
                ),
                posting::Posting::new(
                    "assets:eur".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-5.00 EUR").unwrap()),
                ),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_exchange_three_commodities_all_unbalanced() {
        // Three unbalanced commodities, at least one positive and one negative → valid exchange
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Multi-Currency Exchange".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-1500.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:eur".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 EUR").unwrap()),
                ),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_three_commodities_all_balanced() {
        // All three commodities individually sum to zero → valid
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "expenses:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("200.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "expenses:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-200.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:eur".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 EUR").unwrap()),
                ),
                posting::Posting::new(
                    "expenses:eur".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 EUR").unwrap()),
                ),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_multi_commodity_both_positive_unbalanced() {
        // Two unbalanced commodities, both positive → invalid (not a valid exchange)
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("200.00 SEK").unwrap()),
                ),
            ],
        );
        assert!(!transaction.validate());
    }

    #[test]
    fn test_transaction_validate_multi_commodity_both_negative_unbalanced() {
        // Two unbalanced commodities, both negative → invalid (not a valid exchange)
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-200.00 SEK").unwrap()),
                ),
            ],
        );
        assert!(!transaction.validate());
    }

    #[test]
    fn test_transaction_validate_three_commodities_all_positive_unbalanced() {
        // Three unbalanced commodities, all positive → invalid
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("200.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:eur".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 EUR").unwrap()),
                ),
            ],
        );
        assert!(!transaction.validate());
    }

    #[test]
    fn test_transaction_validate_three_commodities_all_negative_unbalanced() {
        // Three unbalanced commodities, all negative → invalid
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "assets:gbp".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-100.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:sek".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-200.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:eur".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 EUR").unwrap()),
                ),
            ],
        );
        assert!(!transaction.validate());
    }

    // -------------------------------------------------------------------------
    // Display tests: None amount
    // -------------------------------------------------------------------------

    #[test]
    fn test_posting_display_no_amount() {
        let posting = posting::Posting::new("Account 1".to_string(), None);
        assert_eq!(format!("{}", posting), "Account 1");
    }

    #[test]
    fn test_transaction_display_last_posting_no_amount() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap()),
                ),
                posting::Posting::new("Account 2".to_string(), None),
            ],
        );
        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1  123.45 SEK\n\tAccount 2";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    // -------------------------------------------------------------------------
    // Validate tests: None amount (auto-balance)
    // -------------------------------------------------------------------------

    #[test]
    fn test_transaction_validate_single_none_is_valid() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap()),
                ),
                posting::Posting::new("Account 2".to_string(), None),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_none_not_required_to_be_last() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new("Account 1".to_string(), None),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-123.45 SEK").unwrap()),
                ),
            ],
        );
        assert!(transaction.validate());
    }

    #[test]
    fn test_transaction_validate_two_none_postings_is_invalid() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap()),
                ),
                posting::Posting::new("Account 2".to_string(), None),
                posting::Posting::new("Account 3".to_string(), None),
            ],
        );
        assert!(!transaction.validate());
    }

    #[test]
    fn test_transaction_validate_single_none_among_many_postings_is_valid() {
        let transaction: Transaction = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("100.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "Account 2".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new("Account 3".to_string(), None),
            ],
        );
        assert!(transaction.validate());
    }

    // -------------------------------------------------------------------------
    // Hashing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_transaction_hash_same_input_is_stable() {
        let make = || {
            Transaction::new(
                chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                "Groceries".to_string(),
                vec![
                    posting::Posting::new(
                        "expenses:food".to_string(),
                        Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                    ),
                    posting::Posting::new(
                        "assets:bank".to_string(),
                        Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                    ),
                ],
            )
        };
        assert_eq!(make().functional_hash(), make().functional_hash());
    }

    #[test]
    fn test_transaction_hash_description_ignored() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Description A".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Description B".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        assert_eq!(t1.functional_hash(), t2.functional_hash());
    }

    #[test]
    fn test_transaction_hash_different_date_differs() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        assert_ne!(t1.functional_hash(), t2.functional_hash());
    }

    #[test]
    fn test_transaction_hash_different_account_differs() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:other".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        assert_ne!(t1.functional_hash(), t2.functional_hash());
    }

    #[test]
    fn test_transaction_hash_different_amount_differs() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("99.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-99.00 SEK").unwrap()),
                ),
            ],
        );
        assert_ne!(t1.functional_hash(), t2.functional_hash());
    }

    #[test]
    fn test_transaction_hash_different_commodity_differs() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 GBP").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                ),
            ],
        );
        assert_ne!(t1.functional_hash(), t2.functional_hash());
    }

    #[test]
    fn test_transaction_hash_different_posting_order() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
            ],
        );
        assert_eq!(t1.functional_hash(), t2.functional_hash());
    }

    #[test]
    fn test_transaction_hash_none_amount_differs_from_explicit() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new("assets:bank".to_string(), None),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        assert_ne!(t1.functional_hash(), t2.functional_hash());
    }

    // -------------------------------------------------------------------------
    // Partial hash tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_partial_hash_same_input_is_stable() {
        let make = || {
            Transaction::new(
                chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                "Groceries".to_string(),
                vec![
                    posting::Posting::new(
                        "expenses:food".to_string(),
                        Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                    ),
                    posting::Posting::new(
                        "assets:bank".to_string(),
                        Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                    ),
                ],
            )
        };
        assert_eq!(make().partial_hash(), make().partial_hash());
    }

    #[test]
    fn test_partial_hash_different_date_differs() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            "Test".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        assert_ne!(t1.partial_hash(), t2.partial_hash());
    }

    #[test]
    fn test_partial_hash_description_ignored() {
        // Like the full functional_hash, partial_hash excludes the description.
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Description A".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Description B".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        assert_eq!(t1.partial_hash(), t2.partial_hash());
    }

    #[test]
    fn test_partial_hash_matches_with_different_description() {
        // partial_hash should match even when descriptions differ, as long as
        // date and first posting are the same.
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Supermarket ACME".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:groceries".to_string(),
                    Some(commodity_value::CommodityValue::from_str("120.00 SEK").unwrap()),
                ),
                posting::Posting::new("assets:checking".to_string(), None),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "ACME Store Purchase".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:groceries".to_string(),
                    Some(commodity_value::CommodityValue::from_str("120.00 SEK").unwrap()),
                ),
                posting::Posting::new("assets:checking".to_string(), None),
            ],
        );
        assert_eq!(t1.partial_hash(), t2.partial_hash());
    }

    #[test]
    fn test_partial_hash_different_first_posting_differs() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("99.00 SEK").unwrap()),
            )],
        );
        assert_ne!(t1.partial_hash(), t2.partial_hash());
    }

    #[test]
    fn test_partial_hash_only_first_posting_considered() {
        // Same date, description, and first posting — second posting differs.
        // partial_hash should be identical.
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:bank".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![
                posting::Posting::new(
                    "expenses:food".to_string(),
                    Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
                ),
                posting::Posting::new(
                    "assets:savings".to_string(),
                    Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
                ),
            ],
        );
        assert_eq!(t1.partial_hash(), t2.partial_hash());
    }

    #[test]
    fn test_partial_hash_no_postings_is_stable() {
        let t1 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![],
        );
        let t2 = Transaction::new(
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            "Test".to_string(),
            vec![],
        );
        assert_eq!(t1.partial_hash(), t2.partial_hash());
    }
}
