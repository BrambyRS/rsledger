pub mod commodity_value;

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
    posts: Vec<Posting>,
}

impl core::fmt::Display for Transaction {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} {}\n", self.date, self.description)?;
        for post in &self.posts {
            match write!(f, "\t{}\n", post) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        }
        write!(f, "\n")
    }
}

impl Transaction {
    pub fn new (date: String, description: String, account_1: String, amount_1: commodity_value::CommodityValue, account_2: String, amount_2: commodity_value::CommodityValue) -> Self {
        Transaction {
            date,
            description,
            posts: vec![
                Posting::new(account_1, amount_1),
                Posting::new(account_2, amount_2),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Transaction tests
    #[test]
    fn test_double_entry_display() {
        let double_entry: Transaction = Transaction::new(
            "2024-01-01".to_string(),
            "Test Transaction".to_string(),
            "Account 1".to_string(),
            commodity_value::CommodityValue::from_str("123.45 SEK").unwrap(),
            "Account 2".to_string(),
            commodity_value::CommodityValue::from_str("-123.45 SEK").unwrap(),
        );

        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1 123.45 SEK\n\tAccount 2 -123.45 SEK\n\n";
        assert_eq!(format!("{}", double_entry), expected_display);
    }
}