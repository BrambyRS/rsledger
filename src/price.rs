use crate::commodity_value;

/// PRICE DIRECTIVE
/// Struct to hold exchange rates between commodities at a certain date
pub struct PriceDirective {
    pub date: chrono::NaiveDate,
    pub commodity: commodity_value::commodity::Commodity,
    pub value: commodity_value::CommodityValue,
}

/// Implement display for PriceDirective to write it in the format "P YYYY-MM-DD COMMODITY_1 VALUE COMMODITY_2"
impl core::fmt::Display for PriceDirective {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        return write!(f, "P {} {} {}", self.date, self.commodity, self.value);
    }
}

impl PriceDirective {
    /// FROM STR
    /// Parses a price directive from a string in the format "P YYYY-MM-DD COMMODITY_1 VALUE COMMODITY_2"
    /// COMMODITY_1 may be quoted (e.g. "Gold Bar") if it contains spaces, matching hledger's format.
    pub fn from_str(s: &str) -> Result<PriceDirective, Box<dyn std::error::Error>> {
        let tokens: Vec<&str> = s.split_whitespace().collect();
        // Minimum: P date commodity_1 value commodity_2 → 5 tokens
        if tokens.len() < 5 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid price directive format: '{}'. Expected format: 'P YYYY-MM-DD COMMODITY_1 VALUE COMMODITY_2'",
                    s
                ),
            )));
        }

        let date = chrono::NaiveDate::parse_from_str(tokens[1], "%Y-%m-%d")?;

        // Parse COMMODITY_1: if it starts with a quote, accumulate tokens until the closing quote.
        // This mirrors Commodity::fmt, which adds quotes when the name contains a space.
        let (commodity_name, value_start) = if tokens[2].starts_with('"') {
            let mut end = 2;
            while end < tokens.len() && !tokens[end].ends_with('"') {
                end += 1;
            }
            if end >= tokens.len() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "Unclosed quote in commodity name in price directive: '{}'",
                        s
                    ),
                )));
            }
            let joined = tokens[2..=end].join(" ");
            let name = joined[1..joined.len() - 1].to_string();
            (name, end + 1)
        } else {
            (tokens[2].to_string(), 3)
        };

        let commodity = commodity_value::commodity::Commodity {
            name: commodity_name,
        };
        // The remaining tokens form the value (e.g. "1234.56 SEK" or "1234.56 \"Gold Bar\"").
        // CommodityValue::from_str already handles quoted commodity names on that side.
        let value_str = tokens[value_start..].join(" ");
        let value = commodity_value::CommodityValue::from_str(&value_str).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                as Box<dyn std::error::Error>
        })?;

        Ok(PriceDirective {
            date,
            commodity,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_simple() {
        let p = PriceDirective::from_str("P 2026-01-01 SEK 10.00 USD").unwrap();
        assert_eq!(format!("{}", p), "P 2026-01-01 SEK 10 USD");
    }

    #[test]
    fn test_from_str_quoted_commodity_1() {
        // COMMODITY_1 contains a space and is quoted
        let p = PriceDirective::from_str("P 2026-01-01 \"Gold Bar\" 1234.56 SEK").unwrap();
        assert_eq!(format!("{}", p), "P 2026-01-01 \"Gold Bar\" 1234.56 SEK");
    }

    #[test]
    fn test_from_str_quoted_commodity_2() {
        // COMMODITY_2 contains a space and is quoted (handled by CommodityValue::from_str)
        let p = PriceDirective::from_str("P 2026-01-01 USD 10.50 \"Gold Bar\"").unwrap();
        assert_eq!(format!("{}", p), "P 2026-01-01 USD 10.5 \"Gold Bar\"");
    }

    #[test]
    fn test_from_str_both_quoted() {
        // Both commodities contain spaces
        let p = PriceDirective::from_str("P 2026-01-01 \"Gold Bar\" 2.5 \"Silver Coin\"").unwrap();
        assert_eq!(
            format!("{}", p),
            "P 2026-01-01 \"Gold Bar\" 2.5 \"Silver Coin\""
        );
    }

    #[test]
    fn test_from_str_roundtrip() {
        // Formatting and re-parsing should produce the same result
        let original = PriceDirective::from_str("P 2026-01-01 \"Gold Bar\" 1234.56 SEK").unwrap();
        let roundtripped = PriceDirective::from_str(&format!("{}", original)).unwrap();
        assert_eq!(format!("{}", original), format!("{}", roundtripped));
    }

    #[test]
    fn test_from_str_too_few_tokens() {
        assert!(PriceDirective::from_str("P 2026-01-01 SEK").is_err());
        assert!(PriceDirective::from_str("P 2026-01-01").is_err());
    }

    #[test]
    fn test_from_str_unclosed_quote() {
        assert!(PriceDirective::from_str("P 2026-01-01 \"Gold Bar 1234.56 SEK").is_err());
    }

    #[test]
    fn test_from_str_invalid_date() {
        assert!(PriceDirective::from_str("P not-a-date SEK 10.00 USD").is_err());
    }
}
