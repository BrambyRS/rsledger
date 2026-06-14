//! The `journalist` module handles reading of- and writing to journal files, for which
//! it provides the `parser` and `writer` submodules.
//! It defines the `Journal` struct which represents the journal in code.

pub mod parser;
pub mod writer;

use crate::types;

/// JOURNAL
/// Currently only supports storing transactions and prices
pub struct Journal {
    pub transactions: Vec<types::transaction::Transaction>,
    pub prices: Vec<types::price::PriceDirective>,
}
