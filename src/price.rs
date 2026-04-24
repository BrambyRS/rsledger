use crate::commodity_value;

/// PRICE DIRECTIVE
/// Struct to hold exchange rates between commodities at a certain date
pub struct PriceDirective {
    pub date: chrono::NaiveDate,
    pub commodity: commodity_value::commodity::Commodity,
    pub value: commodity_value::CommodityValue,
}

impl core::fmt::Display for PriceDirective {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        return write!(f, "P {} {} {}", self.date, self.commodity, self.value);
    }
}
