/// Represents a monetary or commodity amount with a fixed-precision integer representation.
///
/// The amount is stored as a scaled integer to avoid floating-point precision issues.
/// For example, `123.45 SEK` is stored as `amount = 12345`, `precision = 2`.
#[derive(Clone, Debug)]
pub struct CommodityValue {
    /// The scaled integer amount. Divide by `10^precision` to get the real value.
    amount: i64,
    /// Number of decimal places (e.g. `2` means the amount is in hundredths).
    precision: u8,
    /// Name of the commodity (e.g. `"SEK"`, `"GBP"`, `"Gold Bar"`).
    commodity: String,
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

    /// Aligns the precision of `self` and `other` to the same scale for arithmetic
    /// and comparisons.
    ///
    /// Returns `(self_amount, other_amount, max_precision)` where both amounts have
    /// been scaled up to `max_precision` decimal places.
    fn align_precision(&self, other: &Self) -> (i64, i64, u8) {
        let max_precision: u8 = std::cmp::max(self.precision, other.precision);
        let self_amount_aligned = self.amount * 10_i64.pow((max_precision - self.precision) as u32);
        let other_amount_aligned = other.amount * 10_i64.pow((max_precision - other.precision) as u32);
        return (self_amount_aligned, other_amount_aligned, max_precision);
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
        self.amount == other.amount && self.precision == other.precision
    }
}

/// Adds two `CommodityValue`s. Precision is aligned automatically.
///
/// # Panics
/// Panics if the two values have different commodities.
impl std::ops::Add for CommodityValue {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if self.commodity != other.commodity {
            panic!("Cannot add CommodityValues with different commodities.");
        }

        let (self_amount_aligned, other_amount_aligned, max_precision) = self.align_precision(&other);

        CommodityValue {
            amount: self_amount_aligned + other_amount_aligned,
            precision: max_precision,
            commodity: self.commodity,
        }
    }
}

/// Subtracts one `CommodityValue` from another. Precision is aligned automatically.
///
/// # Panics
/// Panics if the two values have different commodities.
impl std::ops::Sub for CommodityValue {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if self.commodity != other.commodity {
            panic!("Cannot subtract CommodityValues with different commodities.");
        }

        let (self_amount_aligned, other_amount_aligned, max_precision) = self.align_precision(&other);

        CommodityValue {
            amount: self_amount_aligned - other_amount_aligned,
            precision: max_precision,
            commodity: self.commodity,
        }
    }
}

/// Negates a `CommodityValue` by flipping the sign of its amount.
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

/// Two `CommodityValue`s are equal when they share the same commodity and their
/// amounts are equal after normalizing to the same precision (e.g. `1.4` == `1.40`).
impl PartialEq for CommodityValue {
    fn eq(&self, other: &Self) -> bool {
        if self.commodity != other.commodity {
            return false;
        }

        let (self_amount_aligned, other_amount_aligned, _) = self.align_precision(other);
        return self_amount_aligned == other_amount_aligned;
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
        let commodity_value = CommodityValue::from_str("123.45 SEK").unwrap();
        assert_eq!(commodity_value.amount, 12345);
        assert_eq!(commodity_value.precision, 2);
        assert_eq!(commodity_value.commodity, "SEK");
    }

    #[test]
    fn test_commodity_value_from_str_no_decimal() {
        let commodity_value = CommodityValue::from_str("123 SEK").unwrap();
        assert_eq!(commodity_value.amount, 123);
        assert_eq!(commodity_value.precision, 0);
        assert_eq!(commodity_value.commodity, "SEK");
    }

    #[test]
    fn test_commodity_value_from_str_high_precision() {
        let commodity_value = CommodityValue::from_str("123.4567 USD").unwrap();
        assert_eq!(commodity_value.amount, 1234567);
        assert_eq!(commodity_value.precision, 4);
        assert_eq!(commodity_value.commodity, "USD");
    }

    #[test]
    fn test_commodity_value_from_str_commodity_with_spaces() {
        let commodity_value = CommodityValue::from_str("123.45 Gold Bar").unwrap();
        assert_eq!(commodity_value.amount, 12345);
        assert_eq!(commodity_value.precision, 2);
        assert_eq!(commodity_value.commodity, "Gold Bar");
    }

    #[test]
    fn test_commodity_value_from_str_negative() {
        let commodity_value = CommodityValue::from_str("-123.45 SEK").unwrap();
        assert_eq!(commodity_value.amount, -12345);
        assert_eq!(commodity_value.precision, 2);
        assert_eq!(commodity_value.commodity, "SEK");
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
        assert_eq!(-cv, CommodityValue::from_str("-123.45 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_same_precision() {
        let cv1 = CommodityValue::from_str("100.50 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.75 SEK").unwrap();
        assert_eq!(cv1 + cv2, CommodityValue::from_str("124.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_different_precision() {
        let cv1 = CommodityValue::from_str("100.5 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.75 SEK").unwrap();
        assert_eq!(cv1 + cv2, CommodityValue::from_str("124.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_to_zero() {
        let cv1 = CommodityValue::from_str("50.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("-50.00 SEK").unwrap();
        assert_eq!(cv1 + cv2, CommodityValue::from_str("0.00 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_addition_negative() {
        let cv1 = CommodityValue::from_str("-10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("-5.00 SEK").unwrap();
        assert_eq!(cv1 + cv2, CommodityValue::from_str("-15.00 SEK").unwrap());
    }

    #[test]
    #[should_panic]
    fn test_commodity_value_addition_different_commodities_panics() {
        let cv1 = CommodityValue::from_str("10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("10.00 USD").unwrap();
        let _ = cv1 + cv2;
    }

    #[test]
    fn test_commodity_value_subtraction_same_precision() {
        let cv1 = CommodityValue::from_str("100.50 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.25 SEK").unwrap();
        assert_eq!(cv1 - cv2, CommodityValue::from_str("77.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_subtraction_different_precision() {
        let cv1 = CommodityValue::from_str("100.5 SEK").unwrap();
        let cv2 = CommodityValue::from_str("23.25 SEK").unwrap();
        assert_eq!(cv1 - cv2, CommodityValue::from_str("77.25 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_subtraction_to_zero() {
        let cv1 = CommodityValue::from_str("50.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("50.00 SEK").unwrap();
        assert_eq!(cv1 - cv2, CommodityValue::from_str("0.00 SEK").unwrap());
    }

    #[test]
    fn test_commodity_value_subtraction_to_negative() {
        let cv1 = CommodityValue::from_str("10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("25.00 SEK").unwrap();
        assert_eq!(cv1 - cv2, CommodityValue::from_str("-15.00 SEK").unwrap());
    }

    #[test]
    #[should_panic]
    fn test_commodity_value_subtraction_different_commodities_panics() {
        let cv1 = CommodityValue::from_str("10.00 SEK").unwrap();
        let cv2 = CommodityValue::from_str("10.00 USD").unwrap();
        let _ = cv1 - cv2;
    }
}