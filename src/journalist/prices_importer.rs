use crate::commodity_value::{CommodityValue, commodity, fixed_decimal};
use crate::journalist::{self, add_price_to_file};
use crate::price;

use chrono::NaiveDate;
use std::fs::File;
use std::io::{BufRead, Lines};
use std::iter::Peekable;

/// HASHED PRICE
/// A price directive paired with its hash for efficient deduplication.
struct HashedPrice {
    hash: u64,
    #[allow(dead_code)]
    price: price::PriceDirective,
}

/// READ_PRICES_FROM_JOURNAL
/// Opens the journal at `journal_path`, parses it, and returns all price directives
/// wrapped in `HashedPrice` for deduplication. Returns `None` on IO or parse error.
fn read_prices_from_journal(journal_path: std::path::PathBuf) -> Option<Vec<HashedPrice>> {
    let file = match std::fs::File::open(&journal_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening journal file: {}", e);
            return None;
        }
    };

    let mut lines: Peekable<Lines<std::io::BufReader<std::fs::File>>> =
        std::io::BufReader::new(file).lines().peekable();

    let journal = match journalist::journal_parser::parse_journal(&mut lines) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Error parsing journal: {}", e);
            return None;
        }
    };

    Some(
        journal
            .prices
            .into_iter()
            .map(|p| HashedPrice {
                hash: p.price_hash(),
                price: p,
            })
            .collect(),
    )
}

/// DEDUPLICATE_PRICES
/// Filters `candidates` to only those whose `price_hash` does not already appear
/// in `existing_prices`.
fn deduplicate_prices(
    existing_prices: Vec<HashedPrice>,
    candidates: Vec<price::PriceDirective>,
) -> Vec<price::PriceDirective> {
    candidates
        .into_iter()
        .filter(|c| {
            let h = c.price_hash();
            !existing_prices.iter().any(|e| e.hash == h)
        })
        .collect()
}

/// IMPORT_CSV
/// Reads the an Avanza-style positions CSV and returns a vector of PriceDirective entries
/// corresponding to the data in the CSV file.
pub fn import_csv(csv_path: &std::path::PathBuf) -> std::io::Result<Vec<price::PriceDirective>> {
    let file: File = File::open(csv_path)?;
    let mut lines: Peekable<Lines<std::io::BufReader<File>>> =
        std::io::BufReader::new(file).lines().peekable();

    let mut prices: Vec<price::PriceDirective> = Vec::new();

    // Get the date from the CSV name (first 10 charactersshould be YYYY-MM-DD)
    let file_name = csv_path.file_name().unwrap().to_str().unwrap();
    let date: NaiveDate = match NaiveDate::parse_from_str(&file_name[0..10], "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Could not parse date from file name '{}'. Expected format: YYYY-MM-DD.",
                    file_name
                ),
            ));
        }
    };

    // First line is the header, so skip it
    lines.next();

    loop {
        match lines.peek() {
            Some(Ok(line)) => {
                if line.trim().len() == 0 {
                    // Skip empty lines
                    lines.next();
                } else {
                    // The column order is
                    // Namn;Kortnamn;Volym;Marknadsvärde;GAV (SEK);GAV;Valuta;Land;ISIN;Marknad;Typ
                    // The date comes from the file name
                    // The name will be used as the commodity
                    // The price in SEK will have to be calculated as Marknadsvärde / Volym

                    let sek_commodity = commodity::Commodity {
                        name: "SEK".to_string(),
                    };

                    let parts: Vec<&str> = line.split(';').collect();
                    if parts.len() == 6 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!(
                                "Unexpected number of columns in line '{}'. Expected 6 columns.",
                                line
                            ),
                        ));
                    }

                    let commodity_name = parts[0].trim();
                    let commodity: commodity::Commodity = commodity::Commodity {
                        name: commodity_name.to_string(),
                    };

                    let volume_str = parts[2].trim().replace(",", ".");
                    let volume =
                        fixed_decimal::FixedDecimal::from_str(&volume_str).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!(
                                    "Could not parse volume '{}' as a number: {}",
                                    volume_str, e
                                ),
                            )
                        })?;

                    let market_value_str = parts[3].trim().replace(",", ".");
                    let market_value = fixed_decimal::FixedDecimal::from_str(&market_value_str)
                        .map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!(
                                    "Could not parse market value '{}' as a number: {}",
                                    market_value_str, e
                                ),
                            )
                        })?;

                    // Protect against 0 volume by skipping
                    if volume.raw_amount() == 0 {
                        println!(
                            "Skipping line with zero volume for commodity '{}'.",
                            commodity_name
                        );
                        lines.next();
                    } else {
                        let amount: fixed_decimal::FixedDecimal = &market_value / &volume;
                        let value = CommodityValue::new(amount, sek_commodity.clone());

                        prices.push(price::PriceDirective {
                            date,
                            commodity,
                            value,
                        });
                        lines.next();
                    }
                }
            }
            Some(Err(_)) => {
                // If there's an error reading the line, skip it
                lines.next();
            }
            None => {
                // End of file
                return Ok(prices);
            }
        }
    }
}

/// IMPORT_PRICES
/// Reads the an Avanza-style positions CSV and appends PriceDirective entries
/// corresponding to the data in the CSV file to the journal file as price directives.
/// Deduplicates against prices already present in the journal.
pub fn import_prices(
    csv_path: &std::path::PathBuf,
    journal_file: &std::path::PathBuf,
) -> std::io::Result<()> {
    let price_directives = match import_csv(csv_path) {
        Ok(prices) => prices,
        Err(e) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to import prices from CSV: {}", e),
            ));
        }
    };

    let existing_prices = match read_prices_from_journal(journal_file.clone()) {
        Some(p) => p,
        None => {
            eprintln!(
                "Error reading existing prices from {}. Aborting import.",
                journal_file.display()
            );
            return Ok(());
        }
    };

    let new_prices = deduplicate_prices(existing_prices, price_directives);

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(journal_file)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Failed to open journal file {}: {}",
                    journal_file.display(),
                    e
                ),
            )
        })?;

    for price in new_prices {
        add_price_to_file(&mut file, &price)?;
    }

    Ok(())
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

    // -------------------------------------------------------------------------
    // Parsing and row count
    // -------------------------------------------------------------------------

    #[test]
    fn import_csv_returns_correct_number_of_prices() {
        // Gamma AB has zero volume and is skipped, so 3 prices expected.
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn import_csv_parses_date_from_filename() {
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        for price in &result {
            assert_eq!(price.date, expected);
        }
    }

    #[test]
    fn import_csv_first_entry_commodity_name() {
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
        assert_eq!(result[0].commodity.name, "Acme Corp");
    }

    #[test]
    fn import_csv_first_entry_price_is_market_value_divided_by_volume() {
        // Acme Corp: 5000.00 / 10 = 500 SEK
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
        assert_eq!(format!("{}", result[0].value), "500 SEK");
    }

    #[test]
    fn import_csv_fund_entry_with_fractional_volume() {
        // Beta Fund: 10100.00 / 50.5 = 200 SEK
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
        assert_eq!(result[1].commodity.name, "Beta Fund");
        assert_eq!(format!("{}", result[1].value), "200 SEK");
    }

    #[test]
    fn import_csv_skips_zero_volume_entry() {
        // Gamma AB has volume 0 and must not appear in output.
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
        assert!(!result.iter().any(|p| p.commodity.name == "Gamma AB"));
    }

    #[test]
    fn import_csv_non_divisible_price_rounds_to_six_decimal_places() {
        // Delta International: 3333.33 / 4 = 833.3325 SEK
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
        assert_eq!(result[2].commodity.name, "Delta International");
        assert_eq!(format!("{}", result[2].value), "833.3325 SEK");
    }

    #[test]
    fn import_csv_price_value_commodity_is_sek() {
        let result = import_csv(&csv_path("2026-01-15_positions.csv")).unwrap();
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
    fn import_csv_invalid_date_in_filename_returns_error() {
        // File name does not start with a valid YYYY-MM-DD date.
        let path = std::path::PathBuf::from("not-a-date_positions.csv");
        assert!(import_csv(&path).is_err());
    }

    // -------------------------------------------------------------------------
    // Deduplication
    // -------------------------------------------------------------------------

    /// Creates a temporary journal file with the given content and returns its path.
    /// The caller must delete the file after the test.
    fn write_temp_journal(content: &str) -> std::path::PathBuf {
        use std::io::Write;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let path = std::env::temp_dir().join(format!("rsledger_test_{}.journal", nanos));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    /// Reads all lines from a file and returns them as a String.
    fn read_journal(path: &std::path::PathBuf) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn import_prices_adds_all_to_empty_journal() {
        // Empty journal → all 3 prices from the CSV should be appended.
        let journal = write_temp_journal("");
        import_prices(&csv_path("2026-01-15_positions.csv"), &journal).unwrap();
        let contents = read_journal(&journal);
        std::fs::remove_file(&journal).unwrap();

        assert!(contents.contains("Acme Corp"));
        assert!(contents.contains("Beta Fund"));
        assert!(contents.contains("Delta International"));
        assert_eq!(contents.lines().count(), 3);
    }

    #[test]
    fn import_prices_skips_exact_duplicate() {
        // Journal already has Acme Corp → only Beta Fund and Delta International should be added.
        let journal = write_temp_journal("P 2026-01-15 \"Acme Corp\" 500 SEK\n");
        import_prices(&csv_path("2026-01-15_positions.csv"), &journal).unwrap();
        let contents = read_journal(&journal);
        std::fs::remove_file(&journal).unwrap();

        assert_eq!(
            contents.lines().filter(|l| l.contains("Acme Corp")).count(),
            1,
            "Acme Corp should appear only once"
        );
        assert!(contents.contains("Beta Fund"));
        assert!(contents.contains("Delta International"));
        assert_eq!(contents.lines().count(), 3);
    }

    #[test]
    fn import_prices_skips_all_when_all_present() {
        // Journal already has all 3 prices → nothing should be appended.
        let seed = "P 2026-01-15 \"Acme Corp\" 500 SEK\n\
                    P 2026-01-15 \"Beta Fund\" 200 SEK\n\
                    P 2026-01-15 \"Delta International\" 833.3325 SEK\n";
        let journal = write_temp_journal(seed);
        import_prices(&csv_path("2026-01-15_positions.csv"), &journal).unwrap();
        let contents = read_journal(&journal);
        std::fs::remove_file(&journal).unwrap();

        assert_eq!(contents.lines().count(), 3, "no new lines should be added");
    }

    #[test]
    fn import_prices_adds_only_new_prices() {
        // Journal has Beta Fund → Acme Corp and Delta International should be added.
        let journal = write_temp_journal("P 2026-01-15 \"Beta Fund\" 200 SEK\n");
        import_prices(&csv_path("2026-01-15_positions.csv"), &journal).unwrap();
        let contents = read_journal(&journal);
        std::fs::remove_file(&journal).unwrap();

        assert!(contents.contains("Acme Corp"));
        assert_eq!(
            contents.lines().filter(|l| l.contains("Beta Fund")).count(),
            1,
            "Beta Fund should appear only once"
        );
        assert!(contents.contains("Delta International"));
        assert_eq!(contents.lines().count(), 3);
    }
}
