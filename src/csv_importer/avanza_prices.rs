//! Module for importing stock prices from Avanza CSV files.

use chrono::NaiveDate;

use super::ImportCandidate;

fn parse_avanza_prices(
    csv_file: &std::path::PathBuf,
) -> crate::Result<Vec<ImportCandidate<crate::journal::price::PriceDirective>>> {
    // Extract the date from the leading part of the file name, which is expected to be in the format "YYYY-MM-DD".
    let file_name = csv_file.file_name().unwrap().to_str().unwrap();
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

    let mut rdr = match csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b';')
        .from_path(csv_file)
    {
        Ok(rdr) => rdr,
        Err(e) => {
            return Err(crate::error::RsledgerError::ParseError(
                "Avanza CSV".to_string(),
                format!("Failed to read CSV file '{}': {}", csv_file.display(), e),
            ));
        }
    };

    let sek = crate::journal::commodity_value::commodity::Commodity {
        name: "SEK".to_string(),
    };

    let mut import_candidates: Vec<ImportCandidate<crate::journal::price::PriceDirective>> =
        Vec::new();
    for result in rdr.records() {
        let this_record: csv::StringRecord = match result {
            Ok(record) => record,
            Err(e) => {
                return Err(crate::error::RsledgerError::ParseError(
                    "Avanza CSV".to_string(),
                    format!(
                        "Failed to read a record in CSV file '{}': {}",
                        csv_file.display(),
                        e
                    ),
                ));
            }
        };

        // Column order: Namn;Kortnamn;Volym;Marknadsvärde;GAV (SEK);GAV;Valuta;Land;ISIN;Marknad;Typ
        // Price is calculated as Marknadsvärde / Volym, always expressed in SEK.
        let commodity_name = this_record.get(0).unwrap_or("").trim();
        let volume_str = this_record.get(2).unwrap_or("").trim().replace(',', ".");
        let market_value_str = this_record.get(3).unwrap_or("").trim().replace(',', ".");

        let volume =
            crate::journal::commodity_value::fixed_decimal::FixedDecimal::from_str(&volume_str)
                .map_err(|e| {
                    crate::error::RsledgerError::ParseError(
                        "Avanza CSV".to_string(),
                        format!("Could not parse volume '{}': {}", volume_str, e),
                    )
                })?;

        if volume.raw_amount() == 0 {
            println!(
                "Skipping line with zero volume for commodity '{}'.",
                commodity_name
            );
            continue;
        }

        let market_value = crate::journal::commodity_value::fixed_decimal::FixedDecimal::from_str(
            &market_value_str,
        )
        .map_err(|e| {
            crate::error::RsledgerError::ParseError(
                "Avanza CSV".to_string(),
                format!("Could not parse market value '{}': {}", market_value_str, e),
            )
        })?;

        let price_amount = &market_value / &volume;
        let value = crate::journal::commodity_value::CommodityValue::new(price_amount, sek.clone());
        let commodity = crate::journal::commodity_value::commodity::Commodity {
            name: commodity_name.to_string(),
        };

        import_candidates.push(ImportCandidate::Classified(
            crate::journal::price::PriceDirective {
                date,
                commodity,
                value,
            },
        ));
    }

    return Ok(import_candidates);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_path(filename: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("csvs")
            .join(filename)
    }

    /// Unwraps `ImportCandidate` wrappers, returning the inner `PriceDirective` for each entry.
    fn extract_prices(
        candidates: Vec<ImportCandidate<crate::journal::price::PriceDirective>>,
    ) -> Vec<crate::journal::price::PriceDirective> {
        candidates
            .into_iter()
            .map(|c| match c {
                ImportCandidate::Classified(p) | ImportCandidate::Unclassified(p) => p,
            })
            .collect()
    }

    // -------------------------------------------------------------------------
    // Parsing and row count
    // -------------------------------------------------------------------------

    #[test]
    fn parse_avanza_prices_returns_correct_number_of_prices() {
        // Gamma AB has zero volume and is skipped, so 3 prices expected.
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn parse_avanza_prices_parses_date_from_filename() {
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        for price in &result {
            assert_eq!(price.date, expected);
        }
    }

    #[test]
    fn parse_avanza_prices_first_entry_commodity_name() {
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        assert_eq!(result[0].commodity.name, "Acme Corp");
    }

    #[test]
    fn parse_avanza_prices_first_entry_price_is_market_value_divided_by_volume() {
        // Acme Corp: 5000.00 / 10 = 500 SEK
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        assert_eq!(format!("{}", result[0].value), "500 SEK");
    }

    #[test]
    fn parse_avanza_prices_fund_entry_with_fractional_volume() {
        // Beta Fund: 10100.00 / 50.5 = 200 SEK
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        assert_eq!(result[1].commodity.name, "Beta Fund");
        assert_eq!(format!("{}", result[1].value), "200 SEK");
    }

    #[test]
    fn parse_avanza_prices_skips_zero_volume_entry() {
        // Gamma AB has volume 0 and must not appear in output.
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        assert!(!result.iter().any(|p| p.commodity.name == "Gamma AB"));
    }

    #[test]
    fn parse_avanza_prices_non_divisible_price_rounds_to_six_decimal_places() {
        // Delta International: 3333.33 / 4 = 833.3325 SEK
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        assert_eq!(result[2].commodity.name, "Delta International");
        assert_eq!(format!("{}", result[2].value), "833.3325 SEK");
    }

    #[test]
    fn parse_avanza_prices_price_value_commodity_is_sek() {
        let result =
            extract_prices(parse_avanza_prices(&csv_path("2026-01-15_positions.csv")).unwrap());
        for price in &result {
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
    fn parse_avanza_prices_invalid_date_in_filename_returns_error() {
        // File name does not start with a valid YYYY-MM-DD date.
        let path = std::path::PathBuf::from("not-a-date_positions.csv");
        assert!(parse_avanza_prices(&path).is_err());
    }
}
