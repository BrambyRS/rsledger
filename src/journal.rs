//! The journal module defines the Journal struct, which represents the ledger journal in code.
//! Currently, the journal struct only supports transactions and price directives.

use crate::price;
use crate::transaction;

/// JOURNAL
/// Currently only supports storing transactions and prices
pub struct Journal {
    pub transactions: Vec<transaction::Transaction>,
    pub prices: Vec<price::PriceDirective>,
}
