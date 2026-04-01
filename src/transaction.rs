pub mod commodity_value;
pub mod fixed_decimal;
pub mod posting;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Represents a financial transaction with a date, description, and multiple posts (account and amount pairs).
pub struct Transaction {
    /// Date of the transaction in YYYY-MM-DD format.
    date: String,
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
        for post in &self.postings {
            match write!(f, "\t{}\n", post) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        write!(f, "\n")
    }
}

impl Hash for Transaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Don't include the description as it has no real function
        // We want to be able to detect duplicates even if the descriptions differ
        self.date.hash(state);
        // Hash each posting independently then sort the sub-hashes so that
        // posting order does not affect the transaction hash.
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
            h.hash(state);
        }
    }
}

impl Transaction {
    /// Creates a new `Transaction` with the given date, description, and postings.
    ///
    /// # Examples
    /// ```
    /// let t = Transaction::new(
    ///     "2024-01-01".to_string(),
    ///     "Groceries".to_string(),
    ///     vec![
    ///         posting::Posting::new("expenses:food".to_string(), Some(CommodityValue::from_str("50.00 SEK").unwrap())),
    ///         posting::Posting::new("assets:bank".to_string(), None),
    ///     ],
    /// );
    /// ```
    pub fn new(date: String, description: String, postings: Vec<posting::Posting>) -> Self {
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
    /// - All postings have explicit amounts and the sum for every commodity is zero.
    ///
    /// Returns `false` if more than one posting has a `None` amount, or if any
    /// commodity's postings do not sum to zero.
    pub fn validate(&self) -> bool {
        // If there is a None amount, the transaction is auto balanced
        // More than a single None amount makes the transaction invalid
        let mut none_amount_count: usize = 0;
        for post in &self.postings {
            if post.get_amount().is_none() {
                none_amount_count += 1;
                if none_amount_count > 1 {
                    return false;
                }
            }
        }
        if none_amount_count == 1 {
            return true;
        }

        // If no conclusion was reached, check that the transaction is balanced for each commodity.
        // Sum amounts by commodity
        // Total possible number of unique commodities is equal to the number of postings, so we can set the initial capacity of the HashMap to that.
        let mut totals_per_commodity: std::collections::HashMap<
            String,
            fixed_decimal::FixedDecimal,
        > = std::collections::HashMap::with_capacity(self.postings.len());
        for post in &self.postings {
            if let Some(amount) = post.get_amount() {
                let this_commodity: String = amount.commodity().to_string();
                let this_amount: fixed_decimal::FixedDecimal = amount.amount().clone();
                totals_per_commodity
                    .entry(this_commodity.clone())
                    .and_modify(|total| *total += &this_amount)
                    .or_insert(this_amount);
            }
        }

        // Check that all totals are zero
        for (_, total) in totals_per_commodity {
            if total.raw_amount() != 0 {
                return false;
            }
        }

        return true;
    }

    /// Returns a hash of only part of the transaction's data
    ///
    /// This is used for hashing a transaction based only on the date, description,
    /// and first posting. This is useful for identifying duplicate transactions during
    /// CSV import in cases where it can't be fully classified and compared to the full
    /// transaction.
    fn partial_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.date.hash(&mut hasher);
        self.description.hash(&mut hasher);
        if let Some(first_post) = self.postings.first() {
            first_post.hash(&mut hasher);
        }
        hasher.finish()
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
            "2024-01-01".to_string(),
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
            "2024-01-01 Test Transaction\n\tAccount 1  123.45 SEK\n\tAccount 2  -123.45 SEK\n\n";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    #[test]
    fn test_transaction_display_multiple_postings() {
        let transaction: Transaction = Transaction::new(
            "2024-01-01".to_string(),
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

        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1  100 GBP\n\tAccount 2  -50 GBP\n\tAccount 3  -50 GBP\n\n";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    // -------------------------------------------------------------------------
    // Validate tests: explicit amounts
    // -------------------------------------------------------------------------

    #[test]
    fn test_transaction_validate_balanced_single_commodity() {
        let transaction: Transaction = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
            "Test Transaction".to_string(),
            vec![
                posting::Posting::new(
                    "Account 1".to_string(),
                    Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap()),
                ),
                posting::Posting::new("Account 2".to_string(), None),
            ],
        );
        let expected_display =
            "2024-01-01 Test Transaction\n\tAccount 1  123.45 SEK\n\tAccount 2\n\n";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    // -------------------------------------------------------------------------
    // Validate tests: None amount (auto-balance)
    // -------------------------------------------------------------------------

    #[test]
    fn test_transaction_validate_single_none_is_valid() {
        let transaction: Transaction = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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

    fn hash_of<T: Hash>(value: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_transaction_hash_same_input_is_stable() {
        let make = || {
            Transaction::new(
                "2024-01-01".to_string(),
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
        assert_eq!(hash_of(&make()), hash_of(&make()));
    }

    #[test]
    fn test_transaction_hash_description_ignored() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
        assert_eq!(hash_of(&t1), hash_of(&t2));
    }

    #[test]
    fn test_transaction_hash_different_date_differs() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-02-01".to_string(),
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
        assert_ne!(hash_of(&t1), hash_of(&t2));
    }

    #[test]
    fn test_transaction_hash_different_account_differs() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
        assert_ne!(hash_of(&t1), hash_of(&t2));
    }

    #[test]
    fn test_transaction_hash_different_amount_differs() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
        assert_ne!(hash_of(&t1), hash_of(&t2));
    }

    #[test]
    fn test_transaction_hash_different_commodity_differs() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
        assert_ne!(hash_of(&t1), hash_of(&t2));
    }

    #[test]
    fn test_transaction_hash_different_posting_order() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
        assert_eq!(hash_of(&t1), hash_of(&t2));
    }

    #[test]
    fn test_transaction_hash_none_amount_differs_from_explicit() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
        assert_ne!(hash_of(&t1), hash_of(&t2));
    }

    // -------------------------------------------------------------------------
    // Partial hash tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_partial_hash_same_input_is_stable() {
        let make = || {
            Transaction::new(
                "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
            "Test".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        let t2 = Transaction::new(
            "2024-02-01".to_string(),
            "Test".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        assert_ne!(t1.partial_hash(), t2.partial_hash());
    }

    #[test]
    fn test_partial_hash_description_included() {
        // Unlike the full Hash impl, partial_hash includes the description.
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
            "Description A".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        let t2 = Transaction::new(
            "2024-01-01".to_string(),
            "Description B".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        assert_ne!(t1.partial_hash(), t2.partial_hash());
    }

    #[test]
    fn test_partial_hash_different_first_posting_differs() {
        let t1 = Transaction::new(
            "2024-01-01".to_string(),
            "Test".to_string(),
            vec![posting::Posting::new(
                "expenses:food".to_string(),
                Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap()),
            )],
        );
        let t2 = Transaction::new(
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
            "2024-01-01".to_string(),
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
        let t1 = Transaction::new("2024-01-01".to_string(), "Test".to_string(), vec![]);
        let t2 = Transaction::new("2024-01-01".to_string(), "Test".to_string(), vec![]);
        assert_eq!(t1.partial_hash(), t2.partial_hash());
    }
}
