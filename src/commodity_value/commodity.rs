use std::hash::Hash;

/// Simple struct to hold and format the name of a commodity
#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Commodity {
    pub name: String,
}

impl core::fmt::Display for Commodity {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Print with quotes if the commodity contains a space for hledger compatibility
        if self.name.contains(' ') {
            write!(f, "\"{}\"", self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}
