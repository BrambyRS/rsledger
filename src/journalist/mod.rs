pub mod parser;
pub mod writer;

use crate::types;

/// JOURNAL
/// Currently only supports storing transactions and prices
pub struct Journal {
    pub transactions: Vec<types::transaction::Transaction>,
    pub prices: Vec<types::price::PriceDirective>,
}
