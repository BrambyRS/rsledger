use super::fixed_decimal::FixedDecimal;
use std::hash::Hash;

/// Represents a monetary or commodity amount with a fixed-precision integer representation.
///
/// The numeric amount is stored as a [`FixedDecimal`] to avoid floating-point precision
/// issues. For example, `123.45 SEK` stores `amount = FixedDecimal { 12345, 2 }`.
#[derive(Clone, Debug, Hash)]
pub struct CommodityValue {
    /// The scaled decimal amount.
    amount: FixedDecimal,
    /// Name of the commodity (e.g. `SEK`, `GBP`, `Gold Bar`). Always stored without quotes.
    commodity: String,
}

impl core::fmt::Display for CommodityValue {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Print with quotes if the commodity contains a space for hledger compatibility
        if self.commodity.contains(' ') {
            write!(f, "{}  \"{}\"", self.amount, self.commodity)
        } else {
            write!(f, "{}  {}", self.amount, self.commodity)
        }
    }
}

impl CommodityValue {
    /// Parses a `CommodityValue` from a string of the form `"<amount> <commodity>"`.
    ///
    /// The commodity name may contain spaces (e.g. `"Gold Bar"`). The amount may
    /// include a decimal point; if omitted, precision is set to `0`.
    ///
    /// # Errors
    /// Returns an `Err` if the string is missing a commodity, has multiple decimal
    /// points, or contains a non-numeric amount.
    ///
    /// # Examples
    /// ```
    /// let cv = CommodityValue::from_str("123.45 SEK").unwrap();
    /// ```
    pub fn from_str(amount_str: &str) -> Result<Self, String> {
        // First split the string into the amount part and the commodity part
        // The commodity part can have spaces,
        let parts: Vec<&str> = amount_str.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!(
                "Invalid amount format: '{}'. Expected format: '<amount> <commodity>'.",
                amount_str
            ));
        }

        let amount_part: &str = parts[0];
        let commodity_part: String = parts[1..].join(" ");

        // Strip surrounding quotes if present (e.g. when reading back from a journal file).
        // The commodity is always stored unquoted; quotes are added at display time.
        let commodity_part = if commodity_part.starts_with('"')
            && commodity_part.ends_with('"')
            && commodity_part.len() >= 2
        {
            commodity_part[1..commodity_part.len() - 1].to_string()
        } else {
            commodity_part
        };

        let amount = FixedDecimal::from_str(amount_part)
            .map_err(|_| format!("Invalid amount format: '{}'.", amount_part))?;

        Ok(CommodityValue {
            amount,
            commodity: commodity_part,
        })
    }

    /// Returns `true` if both values share the same commodity name.
    pub fn same_commodity(&self, other: &Self) -> bool {
        self.commodity == other.commodity
    }

    /// Returns `true` if both values have exactly the same raw amount and precision.
    ///
    /// Unlike `PartialEq`, this does **not** normalize precision, so `1.0` and `1.00`
    /// are considered different.
    pub fn same_amount(&self, other: &Self) -> bool {
        self.amount.raw_amount() == other.amount.raw_amount()
            && self.amount.precision() == other.amount.precision()
    }

    /// Returns a reference to the underlying [`FixedDecimal`] amount.
    pub fn amount(&self) -> &FixedDecimal {
        &self.amount
    }

    /// Returns the commodity name.
    pub fn commodity(&self) -> &str {
        &self.commodity
    }
}

/// Implements `+=` for `CommodityValue`, delegating to `Add`.
///
/// # Panics
/// Panics if the two values have different commodities.
impl std::ops::AddAssign<&CommodityValue> for CommodityValue {
    fn add_assign(&mut self, other: &Self) {
        *self = &*self + other;
    }
}

/// Adds two `CommodityValue`s. Precision is aligned automatically.
///
/// # Panics
/// Panics if the two values have different commodities.
impl std::ops::Add for &CommodityValue {
    type Output = CommodityValue;

    fn add(self, other: Self) -> CommodityValue {
        if self.commodity != other.commodity {
            panic!("Cannot add CommodityValues with different commodities.");
        }
        CommodityValue {
            amount: &self.amount + &other.amount,
            commodity: self.commodity.clone(),
        }
    }
}

/// Subtracts one `CommodityValue` from another. Precision is aligned automatically.
///
/// # Panics
/// Panics if the two values have different commodities.
impl std::ops::Sub for &CommodityValue {
    type Output = CommodityValue;

    fn sub(self, other: Self) -> CommodityValue {
        if self.commodity != other.commodity {
            panic!("Cannot subtract CommodityValues with different commodities.");
        }
        CommodityValue {
            amount: &self.amount - &other.amount,
            commodity: self.commodity.clone(),
        }
    }
}

/// Negates a `CommodityValue` by flipping the sign of its amount.
impl std::ops::Neg for &CommodityValue {
    type Output = CommodityValue;

    fn neg(self) -> Self::Output {
        CommodityValue {
            amount: -&self.amount,
            commodity: self.commodity.clone(),
        }
    }
}

/// Two `CommodityValue`s are equal when they share the same commodity and their
/// amounts are equal after normalizing to the same precision (e.g. `1.4` == `1.40`).
impl PartialEq for CommodityValue {
    fn eq(&self, other: &Self) -> bool {
        if self.commodity != other.commodity {
            return false;
        }
        self.amount == other.amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // from_str parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_commodity_value_from_str_nominal_format() {
        let cv = CommodityValue::from_str("123.45 SEK").unwrap();
        assert_eq!(cv.amount.raw_amount(), 12345);
        assert_eq!(cv.amount.precision(), 2);
    }

    #[test]
    fn test_commodity_value_from_str_no_decimal() {
        let cv = CommodityValue::from_str("123 SEK").unwrap();
        assert_eq!(cv.amount.raw_amount(), 123);
        assert_eq!(cv.amount.precision(), 0);
    }

    #[test]
    fn test_commodity_value_from_str_high_precision() {
        let cv = CommodityValue::from_str("123.4567 USD").unwrap();
        assert_eq!(cv.amount.raw_amount(), 1234567);
        assert_eq!(cv.amount.precision(), 4);
    }

    #[test]
    fn test_commodity_value_from_str_commodity_with_spaces() {
        let cv = CommodityValue::from_str("123.45 Gold Bar").unwrap();
        assert_eq!(cv.amount.raw_amount(), 12345);
        assert_eq!(cv.amount.precision(), 2);
    }

    #[test]
    fn test_commodity_value_from_str_negative() {
        let cv = CommodityValue::from_str("-123.45 SEK").unwrap();
        assert_eq!(cv.amount.raw_amount(), -12345);
        assert_eq!(cv.amount.precision(), 2);
    }

    #[test]
    fn test_commodity_value_from_str_invalid_format() {
        assert!(CommodityValue::from_str("123.45.67 SEK").is_err());
    }

    #[test]
    fn test_commodity_value_from_str_invalid() {
        assert!(CommodityValue::from_str("invalid_amount").is_err());
    }

    // -------------------------------------------------------------------------
    // Display formatting tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_commodity_value_display_nominal_format() {
        let cv = CommodityValue::from_str("123.45 SEK").unwrap();
        assert_eq!(format!("{}", cv), "123.45 SEK");
    }

    #[test]
    fn test_commodity_value_display_no_decimal() {
        let cv = CommodityValue::from_str("123 SEK").unwrap();
        assert_eq!(format!("{}", cv), "123 SEK");
    }

    #[test]
    fn test_commodity_value_display_different_precision() {
        let cv = CommodityValue::from_str("123.4567 Gold Bar").unwrap();
        assert_eq!(format!("{}", cv), "123.4567 Gold Bar");
    }

    #[test]
    fn test_commodity_value_display_negative() {
        let cv = CommodityValue::from_str("-123.45 SEK").unwrap();
        assert_eq!(format!("{}", cv), "-123.45 SEK");
    }

    // -------------------------------------------------------------------------
    // Equality tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_commodity_value_equality_same_precision() {
        let cv1 = CommodityValue::from_str("123.45 SEK").unwrap();
        let cv2 = CommodityValue::from_str("123.45 SEK").unwrap();
        assert_eq!(cv1, cv2);
    }

    #[test]
    fn test_commodity_value_equality_different_precision() {
        let cv1 = CommodityValue::from_str("123.4 SEK").unwrap();
        let cv2 = CommodityValue::from_str("123.40 SEK").unwrap();
        assert_eq!(cv1, cv2);
    }

    #[test]
    fn test_commodity_value_equality_different_commodities() {
        let cv1 = CommodityValue::from_str("123.45 SEK").unwrap();
        let cv2 = CommodityValue::from_str("123.45 USD").unwrap();
        assert_ne!(cv1, cv2);
    }

    #[test]
    fn test_commodity_value_equality_different_precision_and_commodities() {
        let cv1 = CommodityValue::from_str("123.4 SEK").unwrap();
        let cv2 = CommodityValue::from_str("123.40 USD").unwrap();
        assert_ne!(cv1, cv2);
    }

    // -------------------------------------------------------------------------
    // Arithmetic tests: negation, addition, subtraction
    // -------------------------------------------------------------------------

    #[test]
    fn test_commodity_value_negation() {
        let cv = CommodityValue::from_str("123.45 SEK").unwrap();
        assert_eq!(-&cv, CommodityValue::from_str("-123.45 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_same_precision() {
        let cv1 = CommodityValue::from_str("100.50 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.75 SEK").unwrap();
        assert_eq!(&cv1 + &cv2, CommodityValue::from_str("124.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_different_precision() {
        let cv1 = CommodityValue::from_str("100.5 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.75 SEK").unwrap();
        assert_eq!(&cv1 + &cv2, CommodityValue::from_str("124.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_to_zero() {
        let cv1 = CommodityValue::from_str("50.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("-50.00 SEK").unwrap();
        assert_eq!(&cv1 + &cv2, CommodityValue::from_str("0.00 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_negative() {
        let cv1 = CommodityValue::from_str("-10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("-5.00 SEK").unwrap();
        assert_eq!(&cv1 + &cv2, CommodityValue::from_str("-15.00 SEK").unwrap());
    }

    #[test]
    #[should_panic]
    fn test_commodity_value_addition_different_commodities_panics() {
        let cv1 = CommodityValue::from_str("10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("10.00 USD").unwrap();
        let _ = &cv1 + &cv2;
    }

    #[test]
    fn test_commodity_value_subtraction_same_precision() {
        let cv1 = CommodityValue::from_str("100.50 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.25 SEK").unwrap();
        assert_eq!(&cv1 - &cv2, CommodityValue::from_str("77.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_subtraction_different_precision() {
        let cv1 = CommodityValue::from_str("100.5 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.25 SEK").unwrap();
        assert_eq!(&cv1 - &cv2, CommodityValue::from_str("77.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_subtraction_to_zero() {
        let cv1 = CommodityValue::from_str("50.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("50.00 SEK").unwrap();
        assert_eq!(&cv1 - &cv2, CommodityValue::from_str("0.00 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_subtraction_to_negative() {
        let cv1 = CommodityValue::from_str("10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("25.00 SEK").unwrap();
        assert_eq!(&cv1 - &cv2, CommodityValue::from_str("-15.00 SEK").unwrap());
    }

    #[test]
    #[should_panic]
    fn test_commodity_value_subtraction_different_commodities_panics() {
        let cv1 = CommodityValue::from_str("10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("10.00 USD").unwrap();
        let _ = &cv1 - &cv2;
    }
}
