//! This module implements the `account` type.
//! The `account` type represents hledger style, hierarchical accounts with one
//! of five possible "root" accounts: Assets, Liabilities, Equity, Income, and Expenses.

use crate::error;
use std::hash::Hash;

/// Represents the five possible root accounts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RootAccount {
    Assets,
    Liabilities,
    Equity,
    Income,
    Expenses,
}

impl std::fmt::Display for RootAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RootAccount::Assets => write!(f, "assets"),
            RootAccount::Liabilities => write!(f, "liabilities"),
            RootAccount::Equity => write!(f, "equity"),
            RootAccount::Income => write!(f, "income"),
            RootAccount::Expenses => write!(f, "expenses"),
        }
    }
}

/// Represents a hierarchical account with a root account and optional sub-accounts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Account {
    root_account: RootAccount,
    sub_accounts: Vec<String>,
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.root_account)?;
        for sub_account in &self.sub_accounts {
            write!(f, ":{}", sub_account)?;
        }
        Ok(())
    }
}

impl Account {
    pub fn new(root_account: RootAccount, sub_accounts: Vec<String>) -> Self {
        Account {
            root_account,
            sub_accounts,
        }
    }
    /// Parses a colon-separated account string into an Account struct.
    ///
    /// Parses a string on the form `root:sub1:sub2:...` into an Account struct.
    /// The root account has to be one of the five valid root accounts defined in the RootAccount enum:
    /// - `assets`
    /// - `liabilities`
    /// - `equity`
    /// - `income`
    /// - `expenses`
    ///
    /// FROM_STR will return a Result<Self, RsledgerError> where:
    /// - Ok(Account) if the parsing is successful
    /// - Err(RsledgerError) if the parsing fails (including empty input)
    ///
    /// Empty accounts, sub-accounts, or invalid root accounts will all result in an error.
    pub fn from_str(account_str: &str) -> Result<Self, error::RsledgerError> {
        let parts: Vec<&str> = account_str.split(':').collect();

        // Fail if empty
        if parts.is_empty() || parts[0].is_empty() {
            return Err(error::RsledgerError::ParseError(
                account_str.to_string(),
                "Account cannot be empty".to_string(),
            ));
        }

        let root_account = match parts[0] {
            "assets" => RootAccount::Assets,
            "liabilities" => RootAccount::Liabilities,
            "equity" => RootAccount::Equity,
            "income" => RootAccount::Income,
            "expenses" => RootAccount::Expenses,
            _ => {
                return Err(error::RsledgerError::ParseError(
                    account_str.to_string(),
                    "Invalid root account".to_string(),
                ));
            }
        };

        let sub_accounts: Vec<String> = parts[1..].iter().map(|s| s.trim().to_string()).collect();
        // Fail if any sub-account is empty
        if sub_accounts.iter().any(|s| s.is_empty()) {
            return Err(error::RsledgerError::ParseError(
                account_str.to_string(),
                "Sub-account cannot be empty".to_string(),
            ));
        }
        Ok(Account::new(root_account, sub_accounts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // from_str: valid root accounts (depth 0)
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_root_assets() {
        let account = Account::from_str("assets").unwrap();
        assert_eq!(account.root_account, RootAccount::Assets);
        assert!(account.sub_accounts.is_empty());
    }

    #[test]
    fn from_str_root_liabilities() {
        let account = Account::from_str("liabilities").unwrap();
        assert_eq!(account.root_account, RootAccount::Liabilities);
        assert!(account.sub_accounts.is_empty());
    }

    #[test]
    fn from_str_root_equity() {
        let account = Account::from_str("equity").unwrap();
        assert_eq!(account.root_account, RootAccount::Equity);
        assert!(account.sub_accounts.is_empty());
    }

    #[test]
    fn from_str_root_income() {
        let account = Account::from_str("income").unwrap();
        assert_eq!(account.root_account, RootAccount::Income);
        assert!(account.sub_accounts.is_empty());
    }

    #[test]
    fn from_str_root_expenses() {
        let account = Account::from_str("expenses").unwrap();
        assert_eq!(account.root_account, RootAccount::Expenses);
        assert!(account.sub_accounts.is_empty());
    }

    // -------------------------------------------------------------------------
    // from_str: valid accounts at different depth levels
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_depth_one() {
        let account = Account::from_str("assets:bank").unwrap();
        assert_eq!(account.root_account, RootAccount::Assets);
        assert_eq!(account.sub_accounts, vec!["bank"]);
    }

    #[test]
    fn from_str_depth_two() {
        let account = Account::from_str("assets:bank:checking").unwrap();
        assert_eq!(account.root_account, RootAccount::Assets);
        assert_eq!(account.sub_accounts, vec!["bank", "checking"]);
    }

    #[test]
    fn from_str_depth_three() {
        let account = Account::from_str("expenses:food:dining:restaurants").unwrap();
        assert_eq!(account.root_account, RootAccount::Expenses);
        assert_eq!(account.sub_accounts, vec!["food", "dining", "restaurants"]);
    }

    #[test]
    fn from_str_liabilities_with_sub_accounts() {
        let account = Account::from_str("liabilities:credit-card").unwrap();
        assert_eq!(account.root_account, RootAccount::Liabilities);
        assert_eq!(account.sub_accounts, vec!["credit-card"]);
    }

    #[test]
    fn from_str_income_with_sub_accounts() {
        let account = Account::from_str("income:salary:bonus").unwrap();
        assert_eq!(account.root_account, RootAccount::Income);
        assert_eq!(account.sub_accounts, vec!["salary", "bonus"]);
    }

    #[test]
    fn from_str_equity_with_sub_accounts() {
        let account = Account::from_str("equity:opening-balance").unwrap();
        assert_eq!(account.root_account, RootAccount::Equity);
        assert_eq!(account.sub_accounts, vec!["opening-balance"]);
    }

    // -------------------------------------------------------------------------
    // from_str: display roundtrip
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_display_roundtrip_root_only() {
        let input = "expenses";
        let account = Account::from_str(input).unwrap();
        assert_eq!(account.to_string(), input);
    }

    #[test]
    fn from_str_display_roundtrip_with_sub_accounts() {
        let input = "assets:bank:checking";
        let account = Account::from_str(input).unwrap();
        assert_eq!(account.to_string(), input);
    }

    #[test]
    fn from_str_display_roundtrip_deep() {
        let input = "expenses:food:dining:restaurants";
        let account = Account::from_str(input).unwrap();
        assert_eq!(account.to_string(), input);
    }

    // -------------------------------------------------------------------------
    // from_str: invalid root account
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_invalid_root_returns_error() {
        assert!(Account::from_str("foobar").is_err());
    }

    #[test]
    fn from_str_invalid_root_with_sub_accounts_returns_error() {
        assert!(Account::from_str("foobar:bank").is_err());
    }

    #[test]
    fn from_str_common_synonym_revenue_is_invalid() {
        // hledger uses "income", not "revenue"
        assert!(Account::from_str("revenue:sales").is_err());
    }

    #[test]
    fn from_str_common_synonym_asset_singular_is_invalid() {
        assert!(Account::from_str("asset:bank").is_err());
    }

    // -------------------------------------------------------------------------
    // from_str: case sensitivity
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_uppercase_root_is_invalid() {
        assert!(Account::from_str("ASSETS").is_err());
    }

    #[test]
    fn from_str_title_case_root_is_invalid() {
        assert!(Account::from_str("Assets").is_err());
    }

    #[test]
    fn from_str_mixed_case_root_is_invalid() {
        assert!(Account::from_str("EXPENSES:food").is_err());
    }

    // -------------------------------------------------------------------------
    // from_str: empty and blank inputs
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_empty_string_is_error() {
        assert!(Account::from_str("").is_err());
    }

    #[test]
    fn from_str_whitespace_only_is_error() {
        assert!(Account::from_str("   ").is_err());
    }

    // -------------------------------------------------------------------------
    // from_str: malformed separators
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_double_colon_is_error() {
        // "assets::bank" has an empty component between the colons
        assert!(Account::from_str("assets::bank").is_err());
    }

    #[test]
    fn from_str_double_colon_mid_path_is_error() {
        assert!(Account::from_str("assets:bank::checking").is_err());
    }

    #[test]
    fn from_str_trailing_colon_is_error() {
        assert!(Account::from_str("assets:").is_err());
    }

    #[test]
    fn from_str_leading_colon_is_error() {
        assert!(Account::from_str(":assets").is_err());
    }

    #[test]
    fn from_str_leading_colon_with_path_is_error() {
        assert!(Account::from_str(":assets:bank").is_err());
    }

    // -------------------------------------------------------------------------
    // from_str: whitespace in components
    // -------------------------------------------------------------------------

    #[test]
    fn from_str_leading_space_on_root_is_error() {
        assert!(Account::from_str(" assets").is_err());
    }

    #[test]
    fn from_str_trailing_space_on_root_is_error() {
        assert!(Account::from_str("assets ").is_err());
    }

    #[test]
    fn from_str_leading_space_in_sub_account_is_trimmed() {
        let account = Account::from_str("assets: bank").unwrap();
        assert_eq!(account.root_account, RootAccount::Assets);
        assert_eq!(account.sub_accounts, vec!["bank"]);
    }

    #[test]
    fn from_str_trailing_space_in_sub_account_is_trimmed() {
        let account = Account::from_str("assets:bank ").unwrap();
        assert_eq!(account.root_account, RootAccount::Assets);
        assert_eq!(account.sub_accounts, vec!["bank"]);
    }
}
