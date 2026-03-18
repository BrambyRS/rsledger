pub mod commodity_value;
pub mod fixed_decimal;

pub struct Posting {
    account: String,
    amount: commodity_value::CommodityValue,
}

impl core::fmt::Display for Posting {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} {}", self.account, self.amount)
    }
}

impl Posting {
    pub fn new(account: String, amount: commodity_value::CommodityValue) -> Self {
        Posting {
            account,
            amount,
        }
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
    pub fn new (date: String, description: String, postings: Vec<Posting>) -> Self {
        Transaction {
            date,
            description,
            postings,
        }
    }

    /// Validate that the transaction is balanced for each commodity.
    pub fn validate(&self) -> bool {
        // Sum amounts by commodity
        // Total possible number of unique commodities is equal to the number of postings, so we can set the initial capacity of the HashMap to that.
        let mut totals_per_commodity: std::collections::HashMap<String, fixed_decimal::FixedDecimal> = std::collections::HashMap::with_capacity(self.postings.len());
        for post in &self.postings {
            let this_commodity: String = post.amount.commodity().to_string();
            let this_amount: fixed_decimal::FixedDecimal = post.amount.amount().clone();
            totals_per_commodity.entry(this_commodity.clone())
                .and_modify(|total| *total += &this_amount)
                .or_insert(this_amount);
        }

        // Check that all totals are zero
        for (commodity, total) in totals_per_commodity {
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

    // Transaction tests
    #[test]
    fn test_transaction_display_two_postings() {
        let transaction: Transaction = Transaction::new(
            "2024-01-01".to_string(),
            "Test Transaction".to_string(),
            vec![
                Posting::new("Account 1".to_string(), commodity_value::CommodityValue::from_str("123.45 SEK").unwrap()),
                Posting::new("Account 2".to_string(), commodity_value::CommodityValue::from_str("-123.45 SEK").unwrap()),
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
                Posting::new("Account 1".to_string(), commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                Posting::new("Account 2".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                Posting::new("Account 3".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
            ],
        );

        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1 100.00 GBP\n\tAccount 2 -50.00 GBP\n\tAccount 3 -50.00 GBP\n\n";
        assert_eq!(format!("{}", transaction), expected_display);
    }

    #[test]
    fn test_transaction_validate_balanced_single_commodity() {
        let transaction: Transaction = Transaction::new(
            "2024-01-01".to_string(),
            "Test Transaction".to_string(),
            vec![
                Posting::new("Account 1".to_string(), commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                Posting::new("Account 2".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                Posting::new("Account 3".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
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
                Posting::new("Account 1".to_string(), commodity_value::CommodityValue::from_str("100.00 SEK").unwrap()),
                Posting::new("Account 2".to_string(), commodity_value::CommodityValue::from_str("-30.00 SEK").unwrap()),
                Posting::new("Account 3".to_string(), commodity_value::CommodityValue::from_str("-50.00 SEK").unwrap()),
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
                Posting::new("Account 1".to_string(), commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                Posting::new("Account 2".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                Posting::new("Account 3".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                Posting::new("Account 4".to_string(), commodity_value::CommodityValue::from_str("200.00 SEK").unwrap()),
                Posting::new("Account 5".to_string(), commodity_value::CommodityValue::from_str("-200.00 SEK").unwrap()),
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
                Posting::new("Account 1".to_string(), commodity_value::CommodityValue::from_str("100.00 GBP").unwrap()),
                Posting::new("Account 2".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                Posting::new("Account 3".to_string(), commodity_value::CommodityValue::from_str("-50.00 GBP").unwrap()),
                Posting::new("Account 4".to_string(), commodity_value::CommodityValue::from_str("200.00 SEK").unwrap()),
                Posting::new("Account 5".to_string(), commodity_value::CommodityValue::from_str("-150.00 SEK").unwrap()),
            ],
        );
        assert!(!transaction.validate());
    }
}