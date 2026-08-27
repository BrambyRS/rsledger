use crate::journal::account;
use crate::journal::commodity_value;

use std::hash::Hash;

/// Represents a single line in a [`Transaction`], associating an account with an optional amount.
///
/// When `amount` is `None`, the posting is an auto-balancing entry whose value is
/// inferred when resolving the transaction. At most one posting per transaction may
/// have a `None` amount.
#[derive(Hash, Clone)]
pub struct Posting {
    /// The account
    account: account::Account,
    /// The commodity amount to post. `None` indicates an auto-balancing posting.
    amount: Option<commodity_value::CommodityValue>,
}

/// Formats the posting as `"<account>  <amount>"` (two or more spaces), or just `"<account>"` when the
/// amount is `None`.
impl core::fmt::Display for Posting {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match &self.amount {
            Some(amount) => write!(f, "{}  {}", self.account, amount),
            None => write!(f, "{}", self.account),
        }
    }
}

impl Posting {
    /// Creates a new `Posting` with the given account name and optional amount.
    ///
    /// Pass `None` for `amount` to create an auto-balancing posting.
    pub fn new(account: account::Account, amount: Option<commodity_value::CommodityValue>) -> Self {
        return Posting { account, amount };
    }

    /// Parses a posting string of the form `"<account>    <amount> <commodity>"` into a `Posting`.
    /// Requires at least two whitespace characters between the account and amount. This is so that
    /// the account can contain spaces, but the amount and currency must not contain spaces.
    ///
    /// # Examples
    /// ```
    /// let posting = Posting::from_str("expenses:food  50.00 SEK").unwrap();
    /// assert_eq!(posting.account().to_string(), "expenses:food");
    /// assert_eq!(posting.amount().unwrap().to_string(), "50.00 SEK");
    /// ```
    pub fn from_str(posting_str: &str) -> crate::Result<Posting> {
        // Split into account and amount parts
        let account_str: &str;
        let amount_str: Option<&str>;

        // Find the first occurrence of two or more whitespace characters
        let mut account_end_index: usize = 0;

        for (i, c) in posting_str.char_indices() {
            if c.is_whitespace() {
                // Check if the next character is also whitespace
                match posting_str[i + 1..].chars().next() {
                    Some(next_c) if next_c.is_whitespace() => {
                        account_end_index = i;
                        break;
                    }
                    _ => continue,
                }
            }
        }

        // If account_end_index has not been set yet, then we must assume that
        // the posting has no amount.
        if account_end_index == 0 {
            account_str = posting_str.trim();
            let account = match account::Account::from_str(account_str) {
                Ok(a) => a,
                Err(e) => {
                    return Err(crate::error::RsledgerError::ParseError(
                        posting_str.to_string(),
                        format!("Failed to parse account: {}", e),
                    ));
                }
            };

            return Ok(Posting {
                account,
                amount: None,
            });
        } else {
            account_str = &posting_str[..account_end_index];
            amount_str = Some(&posting_str[account_end_index..].trim());
            let account = match account::Account::from_str(account_str) {
                Ok(a) => a,
                Err(e) => {
                    return Err(crate::error::RsledgerError::ParseError(
                        posting_str.to_string(),
                        format!("Failed to parse account: {}", e),
                    ));
                }
            };

            let amount = match amount_str {
                Some(s) => match commodity_value::CommodityValue::from_str(s) {
                    Ok(a) => Some(a),
                    Err(e) => {
                        return Err(crate::error::RsledgerError::ParseError(
                            posting_str.to_string(),
                            format!("Failed to parse amount: {}", e),
                        ));
                    }
                },
                None => None,
            };

            return Ok(Posting { account, amount });
        }
    }

    #[allow(dead_code)]
    /// Account getter function
    pub fn account(&self) -> &account::Account {
        return &self.account;
    }

    #[allow(dead_code)]
    /// Returns a reference to the posting's amount, or `None` if it is an auto-balancing posting.
    pub fn amount(&self) -> Option<&commodity_value::CommodityValue> {
        return self.amount.as_ref();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posting_from_str() {
        let posting_str = "income:salary  3000 GBP";
        let posting = Posting::from_str(posting_str).unwrap();
        assert_eq!(posting.account().to_string(), "income:salary");
        assert_eq!(posting.amount().unwrap().to_string(), "3000 GBP");
    }

    #[test]
    fn test_posting_from_str_no_amount() {
        let posting_str = "income:salary";
        let posting = Posting::from_str(posting_str).unwrap();
        assert_eq!(posting.account().to_string(), "income:salary");
        assert!(posting.amount().is_none());
    }

    #[test]
    fn test_posting_from_str_account_with_space() {
        let posting_str = "expenses:food and drink  50 GBP";
        let posting = Posting::from_str(posting_str).unwrap();
        assert_eq!(posting.account().to_string(), "expenses:food and drink");
        assert_eq!(posting.amount().unwrap().to_string(), "50 GBP");
    }

    #[test]
    fn test_posting_from_str_autobalancing() {
        let posting_str = "expenses:food";
        let posting = Posting::from_str(posting_str).unwrap();
        assert_eq!(posting.account().to_string(), "expenses:food");
        assert!(posting.amount().is_none());
    }

    #[test]
    fn test_posting_from_str_autobalancing_with_space_in_account() {
        let posting_str = "expenses:food and drink";
        let posting = Posting::from_str(posting_str).unwrap();
        assert_eq!(posting.account().to_string(), "expenses:food and drink");
        assert!(posting.amount().is_none());
    }
}
