use std::hash::Hash;

/// FIXED DECIMAL
/// A fixed-precision decimal number stored as a scaled integer.
///
/// Arithmetic operations on `FixedDecimal` are independent of any commodity.
/// For example, `123.45` is stored as `amount = 12345`, `precision = 2`.
#[derive(Clone, Debug, Hash)]
pub struct FixedDecimal {
    /// The scaled integer amount. Divide by `10^precision` to get the real value.
    amount: i64,
    /// Number of decimal places (e.g. `2` means the amount is in hundredths).
    precision: u8,
}

impl FixedDecimal {
    /// NEW
    /// Constructs a `FixedDecimal` directly from its raw components.
    ///
    /// Reduces the value to the lowest possible precision by removing trailing
    /// fractional zeros. For example, `new(140, 2)` is stored as `amount = 14`,
    /// `precision = 1`. Zero is always stored as `amount = 0`, `precision = 0`.
    pub fn new(amount: i64, precision: u8) -> Self {
        if amount == 0 {
            return FixedDecimal {
                amount: 0,
                precision: 0,
            };
        }
        let mut a = amount;
        let mut p = precision;
        while p > 0 && a % 10 == 0 {
            a /= 10;
            p -= 1;
        }
        FixedDecimal {
            amount: a,
            precision: p,
        }
    }

    /// FROM_STR
    /// Parses a `FixedDecimal` from a bare number string such as `"123.45"` or `"-10"`.
    ///
    /// Reduces the value to the lowest possible precision by removing any trailing zeros
    /// after the decimal point. For example, `"1.40"` is parsed as `amount = 14`, `precision = 1`.
    ///
    /// # Errors
    /// Returns an `Err` if the string has multiple decimal points or contains
    /// non-numeric characters.
    ///
    /// # Examples
    /// ```
    /// let fd = FixedDecimal::from_str("123.45").unwrap();
    /// ```
    pub fn from_str(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        match parts.len() {
            1 => {
                let amount = parts[0]
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid decimal format: '{}'.", s))?;
                Ok(FixedDecimal::new(amount, 0))
            }
            2 => {
                let precision = parts[1].len() as u8;
                let joined = format!("{}{}", parts[0], parts[1]);
                let amount = joined
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid decimal format: '{}'.", s))?;
                Ok(FixedDecimal::new(amount, precision))
            }
            _ => Err(format!("Invalid decimal format: '{}'.", s)),
        }
    }

    /// RAW_AMOUNT (getter)
    /// The raw scaled integer. Divide by `10^precision()` to get the real value.
    pub fn raw_amount(&self) -> i64 {
        return self.amount;
    }

    /// PRECISION (getter)
    /// Number of decimal places used in the scaled representation.
    pub fn precision(&self) -> u8 {
        return self.precision;
    }

    /// ALIGN_PRECISION
    /// Aligns the precision of `self` and `other` to the same scale.
    ///
    /// Returns `(self_amount, other_amount, max_precision)` where both amounts
    /// have been scaled up to `max_precision` decimal places.
    fn align_precision(&self, other: &Self) -> (i64, i64, u8) {
        let max_precision = std::cmp::max(self.precision, other.precision);
        let self_aligned = self.amount * 10_i64.pow((max_precision - self.precision) as u32);
        let other_aligned = other.amount * 10_i64.pow((max_precision - other.precision) as u32);

        return (self_aligned, other_aligned, max_precision);
    }
}

/// DISPLAY
impl core::fmt::Display for FixedDecimal {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        if self.precision == 0 {
            return write!(f, "{}", self.amount);
        } else {
            let int_part = self.amount / 10_i64.pow(self.precision as u32);
            let decimal_part = (self.amount.abs() % 10_i64.pow(self.precision as u32)).abs();
            return write!(
                f,
                "{}.{}",
                int_part,
                format!("{:0width$}", decimal_part, width = self.precision as usize)
            );
        }
    }
}

/// PARTIAL EQ
/// Two `FixedDecimal`s are equal when their amounts are equal after normalizing
/// to the same precision (e.g. `1.4` == `1.40`).
impl PartialEq for FixedDecimal {
    fn eq(&self, other: &Self) -> bool {
        let (self_aligned, other_aligned, _) = self.align_precision(other);
        self_aligned == other_aligned
    }
}

/// ADD
/// Adds two `FixedDecimal`s, aligning precision automatically.
impl std::ops::Add for &FixedDecimal {
    type Output = FixedDecimal;

    fn add(self, other: Self) -> FixedDecimal {
        let (self_aligned, other_aligned, max_precision) = self.align_precision(other);
        FixedDecimal {
            amount: self_aligned + other_aligned,
            precision: max_precision,
        }
    }
}

/// SUB
/// Subtracts one `FixedDecimal` from another, aligning precision automatically.
impl std::ops::Sub for &FixedDecimal {
    type Output = FixedDecimal;

    fn sub(self, other: Self) -> FixedDecimal {
        let (self_aligned, other_aligned, max_precision) = self.align_precision(other);
        FixedDecimal {
            amount: self_aligned - other_aligned,
            precision: max_precision,
        }
    }
}

/// ADD ASSIGN
/// Implements `+=` for `FixedDecimal`, delegating to `Add`.
impl std::ops::AddAssign<&FixedDecimal> for FixedDecimal {
    fn add_assign(&mut self, other: &Self) {
        *self = &*self + other;
    }
}

/// SUB ASSIGN
/// Implements `-=` for `FixedDecimal`, delegating to `Sub`.
impl std::ops::SubAssign<&FixedDecimal> for FixedDecimal {
    fn sub_assign(&mut self, other: &Self) {
        *self = &*self - other;
    }
}

/// NEG
/// Negates a `FixedDecimal` by flipping the sign of its amount.
impl std::ops::Neg for &FixedDecimal {
    type Output = FixedDecimal;

    fn neg(self) -> FixedDecimal {
        FixedDecimal {
            amount: -self.amount,
            precision: self.precision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // from_str parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fixed_decimal_from_str_with_decimal() {
        let fd = FixedDecimal::from_str("123.45").unwrap();
        assert_eq!(fd.raw_amount(), 12345);
        assert_eq!(fd.precision(), 2);
    }

    #[test]
    fn test_fixed_decimal_from_str_no_decimal() {
        let fd = FixedDecimal::from_str("123").unwrap();
        assert_eq!(fd.raw_amount(), 123);
        assert_eq!(fd.precision(), 0);
    }

    #[test]
    fn test_fixed_decimal_from_str_high_precision() {
        let fd = FixedDecimal::from_str("1.4567").unwrap();
        assert_eq!(fd.raw_amount(), 14567);
        assert_eq!(fd.precision(), 4);
    }

    #[test]
    fn test_fixed_decimal_from_str_negative() {
        let fd = FixedDecimal::from_str("-123.45").unwrap();
        assert_eq!(fd.raw_amount(), -12345);
        assert_eq!(fd.precision(), 2);
    }

    #[test]
    fn test_fixed_decimal_from_str_trailing_zeros() {
        let fd = FixedDecimal::from_str("1.40").unwrap();
        assert_eq!(fd.raw_amount(), 14);
        assert_eq!(fd.precision(), 1);
    }

    #[test]
    fn test_fixed_decimal_multiple_trailing_zeros() {
        let fd = FixedDecimal::from_str("1.4000").unwrap();
        assert_eq!(fd.raw_amount(), 14);
        assert_eq!(fd.precision(), 1);
    }

    #[test]
    fn test_fixed_decimal_all_trailing_zeros() {
        let fd = FixedDecimal::from_str("1.000").unwrap();
        assert_eq!(fd.raw_amount(), 1);
        assert_eq!(fd.precision(), 0);
    }

    #[test]
    fn test_fixed_decimal_from_str_invalid_multiple_dots() {
        assert!(FixedDecimal::from_str("1.2.3").is_err());
    }

    #[test]
    fn test_fixed_decimal_from_str_invalid_non_numeric() {
        assert!(FixedDecimal::from_str("abc").is_err());
    }

    // -------------------------------------------------------------------------
    // Display formatting tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fixed_decimal_display_with_decimal() {
        let fd = FixedDecimal::from_str("123.45").unwrap();
        assert_eq!(format!("{}", fd), "123.45");
    }

    #[test]
    fn test_fixed_decimal_display_no_decimal() {
        let fd = FixedDecimal::from_str("123").unwrap();
        assert_eq!(format!("{}", fd), "123");
    }

    #[test]
    fn test_fixed_decimal_display_negative() {
        let fd = FixedDecimal::from_str("-123.45").unwrap();
        assert_eq!(format!("{}", fd), "-123.45");
    }

    #[test]
    fn test_fixed_decimal_display_trailing_zeros() {
        let fd = FixedDecimal::from_str("1.40").unwrap();
        assert_eq!(format!("{}", fd), "1.4");
    }

    // -------------------------------------------------------------------------
    // Equality tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fixed_decimal_equality_same_precision() {
        let fd1 = FixedDecimal::from_str("123.45").unwrap();
        let fd2 = FixedDecimal::from_str("123.45").unwrap();
        assert_eq!(fd1, fd2);
    }

    #[test]
    fn test_fixed_decimal_equality_different_precision() {
        let fd1 = FixedDecimal::from_str("1.4").unwrap();
        let fd2 = FixedDecimal::from_str("1.40").unwrap();
        assert_eq!(fd1, fd2);
    }

    #[test]
    fn test_fixed_decimal_inequality() {
        let fd1 = FixedDecimal::from_str("1.4").unwrap();
        let fd2 = FixedDecimal::from_str("1.5").unwrap();
        assert_ne!(fd1, fd2);
    }

    // -------------------------------------------------------------------------
    // Arithmetic tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fixed_decimal_negation() {
        let fd = FixedDecimal::from_str("123.45").unwrap();
        assert_eq!(-&fd, FixedDecimal::from_str("-123.45").unwrap());
    }

    #[test]
    fn test_fixed_decimal_add_same_precision() {
        let fd1 = FixedDecimal::from_str("100.50").unwrap();
        let fd2 = FixedDecimal::from_str("23.75").unwrap();
        assert_eq!(&fd1 + &fd2, FixedDecimal::from_str("124.25").unwrap());
    }

    #[test]
    fn test_fixed_decimal_add_different_precision() {
        let fd1 = FixedDecimal::from_str("100.5").unwrap();
        let fd2 = FixedDecimal::from_str("23.75").unwrap();
        assert_eq!(&fd1 + &fd2, FixedDecimal::from_str("124.25").unwrap());
    }

    #[test]
    fn test_fixed_decimal_add_to_zero() {
        let fd1 = FixedDecimal::from_str("50.00").unwrap();
        let fd2 = FixedDecimal::from_str("-50.00").unwrap();
        assert_eq!(&fd1 + &fd2, FixedDecimal::from_str("0.00").unwrap());
    }

    #[test]
    fn test_fixed_decimal_sub_same_precision() {
        let fd1 = FixedDecimal::from_str("100.50").unwrap();
        let fd2 = FixedDecimal::from_str("23.25").unwrap();
        assert_eq!(&fd1 - &fd2, FixedDecimal::from_str("77.25").unwrap());
    }

    #[test]
    fn test_fixed_decimal_sub_to_negative() {
        let fd1 = FixedDecimal::from_str("10.00").unwrap();
        let fd2 = FixedDecimal::from_str("25.00").unwrap();
        assert_eq!(&fd1 - &fd2, FixedDecimal::from_str("-15.00").unwrap());
    }

    #[test]
    fn test_fixed_decimal_add_assign() {
        let mut fd1 = FixedDecimal::from_str("100.50").unwrap();
        let fd2 = FixedDecimal::from_str("23.75").unwrap();
        fd1 += &fd2;
        assert_eq!(fd1, FixedDecimal::from_str("124.25").unwrap());
    }

    #[test]
    fn test_fixed_decimal_sub_assign() {
        let mut fd1 = FixedDecimal::from_str("100.50").unwrap();
        let fd2 = FixedDecimal::from_str("23.25").unwrap();
        fd1 -= &fd2;
        assert_eq!(fd1, FixedDecimal::from_str("77.25").unwrap());
    }

    // -------------------------------------------------------------------------
    // new() normalization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_new_already_canonical() {
        let fd = FixedDecimal::new(12345, 2);
        assert_eq!(fd.raw_amount(), 12345);
        assert_eq!(fd.precision(), 2);
    }

    #[test]
    fn test_new_strips_one_trailing_zero() {
        let fd = FixedDecimal::new(140, 2);
        assert_eq!(fd.raw_amount(), 14);
        assert_eq!(fd.precision(), 1);
    }

    #[test]
    fn test_new_strips_all_trailing_zeros_to_integer() {
        let fd = FixedDecimal::new(5000, 3);
        assert_eq!(fd.raw_amount(), 5);
        assert_eq!(fd.precision(), 0);
    }

    #[test]
    fn test_new_zero_canonical() {
        let fd = FixedDecimal::new(0, 6);
        assert_eq!(fd.raw_amount(), 0);
        assert_eq!(fd.precision(), 0);
    }

    #[test]
    fn test_new_negative_strips_trailing_zeros() {
        let fd = FixedDecimal::new(-1200, 2);
        assert_eq!(fd.raw_amount(), -12);
        assert_eq!(fd.precision(), 0);
    }

    #[test]
    fn test_new_precision_zero_unchanged() {
        let fd = FixedDecimal::new(42, 0);
        assert_eq!(fd.raw_amount(), 42);
        assert_eq!(fd.precision(), 0);
    }
}
