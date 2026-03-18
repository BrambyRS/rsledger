pub mod commodity_value;

/*
DoubleEntry struct
*/
pub struct DoubleEntry {
    date: String,
    description: String,
    account_1: String,
    amount_1: commodity_value::CommodityValue,
    account_2: String,
    amount_2: commodity_value::CommodityValue,
}

impl core::fmt::Display for DoubleEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} {}\n\t{} {}\n\t{} {}\n\n", self.date, self.description, self.account_1, self.amount_1, self.account_2, self.amount_2)
    }
}

impl DoubleEntry {
    pub fn new (date: String, description: String, account_1: String, amount_1: commodity_value::CommodityValue, account_2: String, amount_2: commodity_value::CommodityValue) -> Self {
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

    // DoubleEntry tests
    #[test]
    fn test_double_entry_display() {
        let double_entry: DoubleEntry = DoubleEntry::new(
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