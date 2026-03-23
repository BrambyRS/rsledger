pub mod commodity_value;
pub mod fixed_decimal;

/// Represents a single line in a [`Transaction`], associating an account with an optional amount.
///
/// When `amount` is `None`, the posting is an auto-balancing entry whose value is
/// inferred when resolving the transaction. At most one posting per transaction may
/// have a `None` amount.
pub struct Posting {
    /// The account name (e.g. `"assets:bank"`, `"expenses:food"`).
    account: String,
    /// The commodity amount to post. `None` indicates an auto-balancing posting.
    amount: Option<commodity_value::CommodityValue>,
}

/// Formats the posting as `"<account> <amount>"`, or just `"<account>"` when the
/// amount is `None`.
impl core::fmt::Display for Posting {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match &self.amount {
            Some(amount) => write!(f, "{} {}", self.account, amount),
            None => write!(f, "{}", self.account),
        }
    }
}

impl Posting {
    /// Creates a new `Posting` with the given account name and optional amount.
    ///
    /// Pass `None` for `amount` to create an auto-balancing posting.
    pub fn new(account: String, amount: Option<commodity_value::CommodityValue>) -> Self {
        Posting {
            account,
            amount,
        }
    }

    /// Returns a reference to the posting's amount, or `None` if it is an auto-balancing posting.
    pub fn get_amount(&self) -> Option<&commodity_value::CommodityValue> {
        self.amount.as_ref()
    }
}

/// Represents a financial transaction with a date, description, and multiple posts (account and amount pairs).
pub struct Transaction {
    /// Date of the transaction in YYYY-MM-DD format.
    date: String,
    /// Description of the transaction.
    description: String,
    /// Account and amount pairs. For a simple double-entry transaction, there would be two posts with opposite amounts.
    postings: Vec<Posting>,
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
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        }
        write!(f, "\n")
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
    ///         Posting::new("expenses:food".to_string(), Some(CommodityValue::from_str("50.00 SEK").unwrap())),
    ///         Posting::new("assets:bank".to_string(), None),
    ///     ],
    /// );
    /// ```
    pub fn new(date: String, description: String, postings: Vec<Posting>) -> Self {
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
        let mut totals_per_commodity: std::collections::HashMap<String, fixed_decimal::FixedDecimal> = std::collections::HashMap::with_capacity(self.postings.len());
        for post in &self.postings {
            if let Some(amount) = post.get_amount() {
                let this_commodity: String = amount.commodity().to_string();
                let this_amount: fixed_decimal::FixedDecimal = amount.amount().clone();
                totals_per_commodity.entry(this_commodity.clone())
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap())),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("-123.45 SEK").unwrap())),
            ],
        );

        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1 123.45 SEK\n\tAccount 2 -123.45 SEK\n\n";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    #[test]
    fn test_transaction_display_multiple_postings() {
        let transaction: Transaction = Transaction::new(
            "2024-01-01".to_string(),
            "Test Transaction".to_string(),
            vec![
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap())),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
                Posting::new("Account 3".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
            ],
        );

        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1 100 GBP\n\tAccount 2 -50 GBP\n\tAccount 3 -50 GBP\n\n";
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap())),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
                Posting::new("Account 3".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("100.00 SEK").unwrap())),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("-30.00 SEK").unwrap())),
                Posting::new("Account 3".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap())),
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap())),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
                Posting::new("Account 3".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
                Posting::new("Account 4".to_string(), Some(commodity_value::CommodityValue::from_str("200.00 SEK").unwrap())),
                Posting::new("Account 5".to_string(), Some(commodity_value::CommodityValue::from_str("-200.00 SEK").unwrap())),
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("100.00 GBP").unwrap())),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
                Posting::new("Account 3".to_string(), Some(commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap())),
                Posting::new("Account 4".to_string(), Some(commodity_value::CommodityValue::from_str("200.00 SEK").unwrap())),
                Posting::new("Account 5".to_string(), Some(commodity_value::CommodityValue::from_str("-150.00 SEK").unwrap())),
            ],
        );
        assert!(!transaction.validate());
    }

    // -------------------------------------------------------------------------
    // Display tests: None amount
    // -------------------------------------------------------------------------

    #[test]
    fn test_posting_display_no_amount() {
        let posting = Posting::new("Account 1".to_string(), None);
        assert_eq!(format!("{}", posting), "Account 1");
    }

    #[test]
    fn test_transaction_display_last_posting_no_amount() {
        let transaction: Transaction = Transaction::new(
            "2024-01-01".to_string(),
            "Test Transaction".to_string(),
            vec![
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap())),
                Posting::new("Account 2".to_string(), None),
            ],
        );
        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1 123.45 SEK\n\tAccount 2\n\n";
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap())),
                Posting::new("Account 2".to_string(), None),
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
                Posting::new("Account 1".to_string(), None),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("-123.45 SEK").unwrap())),
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("123.45 SEK").unwrap())),
                Posting::new("Account 2".to_string(), None),
                Posting::new("Account 3".to_string(), None),
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
                Posting::new("Account 1".to_string(), Some(commodity_value::CommodityValue::from_str("100.00 SEK").unwrap())),
                Posting::new("Account 2".to_string(), Some(commodity_value::CommodityValue::from_str("50.00 SEK").unwrap())),
                Posting::new("Account 3".to_string(), None),
            ],
        );
        assert!(transaction.validate());
    }
}