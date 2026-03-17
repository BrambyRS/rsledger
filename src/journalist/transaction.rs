/*
CommodityValue struct
*/
#[derive(Clone)]
pub struct CommodityValue {
    amount: i32, // We save the amount as an integer (100x the amount in the journal) to avoid floating point issues.
    currency: String,
}

impl core::fmt::Display for CommodityValue {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Format the amount as a decimal with 2 places by placing a decimal point before the last two digits.
        let amount_str = format!("{}.{:02}", self.amount / 100, self.amount.abs() % 100);
        write!(f, "{} {}", amount_str, self.currency)
    }
}

impl std::ops::Neg for CommodityValue {
    type Output = Self;

    fn neg(self) -> Self::Output {
        CommodityValue {
            amount: -self.amount,
            currency: self.currency,
        }
    }
}

impl PartialEq for CommodityValue {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount && self.currency == other.currency
    }
}

impl CommodityValue {
    pub fn from_str(amount_str: &str) -> Option<Self> {
        // Split the amount string into the numeric part and the currency part.
        let parts: Vec<&str> = amount_str.split_whitespace().collect();
        if parts.len() != 2 {
            return None;
        }

        let amount_part = parts[0];
        let currency_part = parts[1].to_string();

        // Remove the decimal point from the amount part and convert it to an integer.
        // If there is no decimal point, we can just parse it as an integer and multiply by 100.
        let amount_int: i32;
        if amount_part.contains('.') {
            let amount_int_str = amount_part.replace('.', "");
            amount_int = match amount_int_str.parse::<i32>() {
                Ok(val) => val,
                Err(_) => return None,
            };
        } else {
            let whole_part = match amount_part.parse::<i32>() {
                Ok(val) => val,
                Err(_) => return None,
            };
            amount_int = whole_part * 100; // Multiply by 100
        }

        Some(CommodityValue {
            amount: amount_int,
            currency: currency_part,
        })
    }

    pub fn same_currency(&self, other: &Self) -> bool {
        self.currency == other.currency
    }

    pub fn same_amount(&self, other: &Self) -> bool {
        self.amount == other.amount
    }
}

/*
DoubleEntry struct
*/
pub struct DoubleEntry {
    date: String,
    description: String,
    account_1: String,
    amount_1: CommodityValue,
    account_2: String,
    amount_2: CommodityValue,
}

impl core::fmt::Display for DoubleEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} {}\n\t{} {}\n\t{} {}\n\n", self.date, self.description, self.account_1, self.amount_1, self.account_2, self.amount_2)
    }
}

impl DoubleEntry {
    pub fn new (date: String, description: String, account_1: String, amount_1: CommodityValue, account_2: String, amount_2: CommodityValue) -> Self {
        DoubleEntry {
            date,
            description,
            account_1,
            amount_1,
            account_2,
            amount_2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CommodityValue tests
    #[test]
    fn test_transaction_amount_from_str() {
        let amount_str = "123.45 SEK";
        let transaction_amount = CommodityValue::from_str(amount_str).unwrap();
        assert_eq!(transaction_amount.amount, 12345);
        assert_eq!(transaction_amount.currency, "SEK");
    }

    #[test]
    fn test_transaction_amount_from_str_no_decimal() {
        let amount_str_no_decimal = "123 SEK";
        let transaction_amount_no_decimal = CommodityValue::from_str(amount_str_no_decimal).unwrap();
        assert_eq!(transaction_amount_no_decimal.amount, 12300);
        assert_eq!(transaction_amount_no_decimal.currency, "SEK");
    }

    #[test]
    fn test_transaction_amount_from_str_negative() {
        let amount_str_negative = "-123.45 SEK";
        let transaction_amount_negative = CommodityValue::from_str(amount_str_negative).unwrap();
        assert_eq!(transaction_amount_negative.amount, -12345);
        assert_eq!(transaction_amount_negative.currency, "SEK");
    }

    #[test]
    fn test_transaction_amount_from_str_invalid() {
        let amount_str_invalid = "invalid_amount";
        let transaction_amount_invalid = CommodityValue::from_str(amount_str_invalid);
        assert!(transaction_amount_invalid.is_none());
    }

    // DoubleEntry tests
    #[test]
    fn test_double_entry_display() {
        let double_entry = DoubleEntry::new(
            "2024-01-01".to_string(),
            "Test Transaction".to_string(),
            "Account 1".to_string(),
            CommodityValue::from_str("123.45 SEK").unwrap(),
            "Account 2".to_string(),
            CommodityValue::from_str("-123.45 SEK").unwrap(),
        );

        let expected_display = "2024-01-01 Test Transaction\n\tAccount 1 123.45 SEK\n\tAccount 2 -123.45 SEK\n\n";
        assert_eq!(format!("{}", double_entry), expected_display);
    }
}