/*
CommodityValue struct
*/
#[derive(Clone)]
pub struct CommodityValue {
    amount: i64, // We save the amount as an integer to avoid floating point precision issues.
    precision: u8, // Number of decimal places for the value
    commodity: String, // Name of the commodity
}

impl core::fmt::Display for CommodityValue {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Format the amount as a string with the correct number of decimal places based in the precision
        let amount_str = if self.precision == 0 {
            self.amount.to_string()
        } else {
            let int_part = self.amount / 10_i64.pow(self.precision as u32);
            let decimal_part = (self.amount.abs() % 10_i64.pow(self.precision as u32)).abs();
            format!("{}.{}", int_part, format!("{:0width$}", decimal_part, width = self.precision as usize))
        };
        write!(f, "{} {}", amount_str, self.commodity)
    }
}

impl std::ops::Neg for CommodityValue {
    type Output = Self;

    fn neg(self) -> Self::Output {
        CommodityValue {
            amount: -self.amount,
            precision: self.precision,
            commodity: self.commodity,
        }
    }
}

impl PartialEq for CommodityValue {
    fn eq(&self, other: &Self) -> bool {
        if self.precision > other.precision {
            let factor = 10_i64.pow((self.precision - other.precision) as u32);
            self.amount == other.amount * factor && self.commodity == other.commodity
        } else if self.precision < other.precision {
            let factor = 10_i64.pow((other.precision - self.precision) as u32);
            self.amount * factor == other.amount && self.commodity == other.commodity
        } else {
            self.amount == other.amount && self.commodity == other.commodity
        }
    }
}

impl CommodityValue {
    pub fn from_str(amount_str: &str) -> Result<Self, String> {
        // First split the string into the amount part and the commodity part
        // The commodity part can have spaces, 
        let parts: Vec<&str> = amount_str.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!("Invalid amount format: '{}'. Expected format: '<amount> <commodity>'.", amount_str));
        }

        let amount_part: &str = parts[0];
        let commodity_part: String = parts[1..].join(" ");

        // Split the amount_part at the decimal point if it exists
        let amount_parts: Vec<&str> = amount_part.split('.').collect();
        let amount_int: i64;
        let precision: u8;
        // If there's no decimal point, save the integer and set precision to 0
        if amount_parts.len() == 1 {
            (amount_int, precision) = match amount_parts[0].parse::<i64>() {
                Ok(val) => (val, 0),
                Err(_) => return Err(format!("Invalid amount format: '{}'.", amount_part)),
            };
        // If there are two parts, save as an integer with non-zero precision
        } else if amount_parts.len() == 2 {
            let int_str = amount_parts[..].join("");
            precision = amount_parts[1].len() as u8;
            amount_int = match int_str.parse::<i64>() {
                Ok(val) => val,
                Err(_) => return Err(format!("Invalid amount format: '{}'.", amount_part)),
            };
        // If there are more than two parts, something is wrong!
        } else {
            return Err(format!("Invalid amount format: '{}'.", amount_part));
        }

        Ok(CommodityValue {
            amount: amount_int,
            precision,
            commodity: commodity_part,
        })
    }

    pub fn same_commodity(&self, other: &Self) -> bool {
        self.commodity == other.commodity
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
        assert_eq!(transaction_amount.precision, 2);
        assert_eq!(transaction_amount.commodity, "SEK");
    }

    #[test]
    fn test_transaction_amount_from_str_no_decimal() {
        let amount_str_no_decimal: &str = "123 SEK";
        let transaction_amount_no_decimal: CommodityValue = CommodityValue::from_str(amount_str_no_decimal).unwrap();
        assert_eq!(transaction_amount_no_decimal.amount, 123);
        assert_eq!(transaction_amount_no_decimal.precision, 0);
        assert_eq!(transaction_amount_no_decimal.commodity, "SEK");
    }

    #[test]
    fn test_transaction_amount_from_str_different_precision() {
        let amount_str_different_precision: &str = "123.4567 USD";
        let transaction_amount_different_precision: CommodityValue = CommodityValue::from_str(amount_str_different_precision).unwrap();
        assert_eq!(transaction_amount_different_precision.amount, 1234567);
        assert_eq!(transaction_amount_different_precision.precision, 4);
        assert_eq!(transaction_amount_different_precision.commodity, "USD");
    }

    #[test]
    fn test_transaction_amount_from_str_commodity_with_spaces() {
        let amount_str_commodity_with_spaces: &str = "123.45 Gold Bar";
        let transaction_amount_commodity_with_spaces: CommodityValue = CommodityValue::from_str(amount_str_commodity_with_spaces).unwrap();
        assert_eq!(transaction_amount_commodity_with_spaces.amount, 12345);
        assert_eq!(transaction_amount_commodity_with_spaces.precision, 2);
        assert_eq!(transaction_amount_commodity_with_spaces.commodity, "Gold Bar");
    }

    #[test]
    fn test_transaction_amount_from_str_negative() {
        let amount_str_negative: &str = "-123.45 SEK";
        let transaction_amount_negative: CommodityValue = CommodityValue::from_str(amount_str_negative).unwrap();
        assert_eq!(transaction_amount_negative.amount, -12345);
        assert_eq!(transaction_amount_negative.precision, 2);
        assert_eq!(transaction_amount_negative.commodity, "SEK");
    }

    #[test]
    fn test_transaction_amount_from_str_invalid_format() {
        let amount_str_invalid_format: &str = "123.45.67 SEK";
        let transaction_amount_invalid_format: Result<CommodityValue, String> = CommodityValue::from_str(amount_str_invalid_format);
        assert!(transaction_amount_invalid_format.is_err());
    }

    #[test]
    fn test_transaction_amount_from_str_invalid() {
        let amount_str_invalid: &str = "invalid_amount";
        let transaction_amount_invalid: Result<CommodityValue, String> = CommodityValue::from_str(amount_str_invalid);
        assert!(transaction_amount_invalid.is_err());
    }

    #[test]
    fn test_commodity_value_display() {
        let commodity_value: CommodityValue = match CommodityValue::from_str("123.45 SEK") {
            Ok(val) => val,
            Err(e) => panic!("Failed to parse amount string: {}", e),
        };
        let expected_display = "123.45 SEK";
        assert_eq!(format!("{}", commodity_value), expected_display);
    }

    #[test]
    fn test_commodity_value_display_no_decimal() {
        let commodity_value_no_decimal: CommodityValue = match CommodityValue::from_str("123 SEK") {
            Ok(val) => val,
            Err(e) => panic!("Failed to parse amount string: {}", e),
        };
        let expected_display_no_decimal = "123 SEK";
        assert_eq!(format!("{}", commodity_value_no_decimal), expected_display_no_decimal);
    }

    #[test]
    fn test_commodity_value_display_different_precision() {
        let commodity_value_different_precision: CommodityValue = match CommodityValue::from_str("123.4567 Gold Bar") {
            Ok(val) => val,
            Err(e) => panic!("Failed to parse amount string: {}", e),
        };
        let expected_display_different_precision = "123.4567 Gold Bar";
        assert_eq!(format!("{}", commodity_value_different_precision), expected_display_different_precision);
    }

    // DoubleEntry tests
    #[test]
    fn test_double_entry_display() {
        let double_entry: DoubleEntry = DoubleEntry::new(
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