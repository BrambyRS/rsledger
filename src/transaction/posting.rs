use crate::transaction::commodity_value;

use std::hash::Hash;

/// Represents a single line in a [`Transaction`], associating an account with an optional amount.
///
/// When `amount` is `None`, the posting is an auto-balancing entry whose value is
/// inferred when resolving the transaction. At most one posting per transaction may
/// have a `None` amount.
#[derive(Hash)]
pub struct Posting {
    /// The account name (e.g. `"assets:bank"`, `"expenses:food"`).
    account: String,
    /// The commodity amount to post. `None` indicates an auto-balancing posting.
    amount: Option<commodity_value::CommodityValue>,
}

/// Formats the posting as `"<account> <amount>"`, or just `"<account>"` when the
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
    pub fn new(account: String, amount: Option<commodity_value::CommodityValue>) -> Self {
        Posting { account, amount }
    }

    /// Returns a reference to the posting's amount, or `None` if it is an auto-balancing posting.
    pub fn get_amount(&self) -> Option<&commodity_value::CommodityValue> {
        self.amount.as_ref()
    }
}
