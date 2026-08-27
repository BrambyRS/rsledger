//! Avanza-specific CSV importer for positions/prices exports.
//!
//! Provides [`AvanzaPricesImporter`], which reads an Avanza positions CSV and
//! produces [`PriceDirective`] entries by dividing each holding's market value by
//! its volume. The date is taken from the first 10 characters of the filename
//! (expected format: `YYYY-MM-DD`).

use chrono::NaiveDate;
use std::path::PathBuf;

use crate::journal::price::PriceDirective;

use super::{EntryImporter, ImportCandidate};

/// Importer for Avanza positions CSV exports.
///
/// All prices are expressed in SEK and are always fully classified.
/// Rows with zero volume are skipped since no price can be derived from them.
pub struct AvanzaPricesImporter;

impl AvanzaPricesImporter {
    pub fn new() -> Self {
        AvanzaPricesImporter
    }
}

impl EntryImporter<PriceDirective> for AvanzaPricesImporter {
    fn import_csv(&self, csv_path: PathBuf) -> crate::Result<Vec<ImportCandidate<PriceDirective>>> {
        // The date is encoded in the first 10 characters of the filename (YYYY-MM-DD).
        let file_name = csv_path.file_name().unwrap().to_str().unwrap();
        let date: NaiveDate =
            NaiveDate::parse_from_str(&file_name[0..10], "%Y-%m-%d").map_err(|_| {
                crate::error::RsledgerError::ParseError(
                    "Avanza CSV".to_string(),
                    format!(
                        "Could not parse date from file name '{}'. Expected format: YYYY-MM-DD.",
                        file_name
                    ),
                )
            })?;

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b';')
            .from_path(&csv_path)
            .map_err(|e| {
                crate::error::RsledgerError::ParseError(
                    "Avanza CSV".to_string(),
                    format!("Failed to read CSV file '{}': {}", csv_path.display(), e),
                )
            })?;

        let sek = crate::journal::commodity_value::commodity::Commodity {
            name: "SEK".to_string(),
        };

        // Column order: Namn;Kortnamn;Volym;Marknadsvärde;GAV (SEK);GAV;Valuta;Land;ISIN;Marknad;Typ
        // Price = Marknadsvärde / Volym, always expressed in SEK.

        let mut import_candidates: Vec<ImportCandidate<PriceDirective>> = Vec::new();

        for result in rdr.records() {
            let record = result.map_err(|e| {
                crate::error::RsledgerError::ParseError(
                    "Avanza CSV".to_string(),
                    format!(
                        "Failed to read a record in CSV file '{}': {}",
                        csv_path.display(),
                        e
                    ),
                )
            })?;

            let commodity_name = record.get(0).unwrap_or("").trim();
            let volume_str = record.get(2).unwrap_or("").trim().replace(',', ".");
            let market_value_str = record.get(3).unwrap_or("").trim().replace(',', ".");

            let volume =
                crate::journal::commodity_value::fixed_decimal::FixedDecimal::from_str(&volume_str)
                    .map_err(|e| {
                        crate::error::RsledgerError::ParseError(
                            "Avanza CSV".to_string(),
                            format!("Could not parse volume '{}': {}", volume_str, e),
                        )
                    })?;

            // Rows with zero volume cannot produce a price — skip them.
            if volume.raw_amount() == 0 {
                println!(
                    "Skipping line with zero volume for commodity '{}'.",
                    commodity_name
                );
                continue;
            }

            let market_value =
                crate::journal::commodity_value::fixed_decimal::FixedDecimal::from_str(
                    &market_value_str,
                )
                .map_err(|e| {
                    crate::error::RsledgerError::ParseError(
                        "Avanza CSV".to_string(),
                        format!("Could not parse market value '{}': {}", market_value_str, e),
                    )
                })?;

            let price_amount = &market_value / &volume;
            let value =
                crate::journal::commodity_value::CommodityValue::new(price_amount, sek.clone());
            let commodity = crate::journal::commodity_value::commodity::Commodity {
                name: commodity_name.to_string(),
            };

            import_candidates.push(ImportCandidate::Classified(PriceDirective {
                date,
                commodity,
                value,
            }));
        }

        Ok(import_candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("csvs")
            .join(filename)
    }

    /// Unwraps `ImportCandidate` wrappers, returning the inner `PriceDirective` for each entry.
    fn extract_prices(candidates: Vec<ImportCandidate<PriceDirective>>) -> Vec<PriceDirective> {
        candidates
            .into_iter()
            .map(|c| match c {
                ImportCandidate::Classified(p) | ImportCandidate::Unclassified(p) => p,
            })
            .collect()
    }

    fn import_prices(filename: &str) -> Vec<PriceDirective> {
        extract_prices(
            AvanzaPricesImporter::new()
                .import_csv(csv_path(filename))
                .unwrap(),
        )
    }

    // -------------------------------------------------------------------------
    // Parsing and row count
    // -------------------------------------------------------------------------

    #[test]
    fn returns_correct_number_of_prices() {
        // Gamma AB has zero volume and is skipped, so 3 prices expected.
        assert_eq!(import_prices("2026-01-15_positions.csv").len(), 3);
    }

    #[test]
    fn parses_date_from_filename() {
        let expected = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        for price in import_prices("2026-01-15_positions.csv") {
            assert_eq!(price.date, expected);
        }
    }

    #[test]
    fn first_entry_commodity_name() {
        assert_eq!(
            import_prices("2026-01-15_positions.csv")[0].commodity.name,
            "Acme Corp"
        );
    }

    #[test]
    fn first_entry_price_is_market_value_divided_by_volume() {
        // Acme Corp: 5000.00 / 10 = 500 SEK
        assert_eq!(
            format!("{}", import_prices("2026-01-15_positions.csv")[0].value),
            "500 SEK"
        );
    }

    #[test]
    fn fund_entry_with_fractional_volume() {
        // Beta Fund: 10100.00 / 50.5 = 200 SEK
        let prices = import_prices("2026-01-15_positions.csv");
        assert_eq!(prices[1].commodity.name, "Beta Fund");
        assert_eq!(format!("{}", prices[1].value), "200 SEK");
    }

    #[test]
    fn skips_zero_volume_entry() {
        // Gamma AB has volume 0 and must not appear in output.
        assert!(
            !import_prices("2026-01-15_positions.csv")
                .iter()
                .any(|p| p.commodity.name == "Gamma AB")
        );
    }

    #[test]
    fn non_divisible_price_rounds_correctly() {
        // Delta International: 3333.33 / 4 = 833.3325 SEK
        let prices = import_prices("2026-01-15_positions.csv");
        assert_eq!(prices[2].commodity.name, "Delta International");
        assert_eq!(format!("{}", prices[2].value), "833.3325 SEK");
    }

    #[test]
    fn price_value_commodity_is_sek() {
        for price in import_prices("2026-01-15_positions.csv") {
            assert!(
                format!("{}", price.value).ends_with(" SEK"),
                "expected SEK commodity but got: {}",
                price.value
            );
        }
    }

    // -------------------------------------------------------------------------
    // Error cases
    // -------------------------------------------------------------------------

    #[test]
    fn invalid_date_in_filename_returns_error() {
        let path = PathBuf::from("not-a-date_positions.csv");
        assert!(AvanzaPricesImporter::new().import_csv(path).is_err());
    }
}
