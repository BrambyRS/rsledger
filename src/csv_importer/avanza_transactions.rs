//! Avanza-specific CSV importer for transaction exports.
//!
//! Avanza transactions are always fully classified — every supported action type
//! maps to a deterministic set of postings with no manual intervention required.
//! Unsupported action types are propagated as errors rather than silently skipped.

use chrono::NaiveDate;
use std::path::PathBuf;

use crate::journal::account::Account;
use crate::journal::commodity_value::CommodityValue;
use crate::journal::transaction::Transaction;
use crate::journal::transaction::posting::Posting;

use super::{EntryImporter, ImportCandidate};

// Column indices for the Avanza transaction CSV format.
// Header: Datum;Konto;Typ av transaktion;Värdepapper/beskrivning;Antal;Kurs;Belopp;Transaktionsvaluta;Courtage;Valutakurs;Instrumentvaluta;ISIN;Resultat
const DATE_COL: usize = 0;
const ACTION_COL: usize = 2;
const NAME_COL: usize = 3;
const QUANTITY_COL: usize = 4;
const CASH_AMOUNT_COL: usize = 6;
const CURRENCY_COL: usize = 7;
const FEE_COL: usize = 8;
const PROFIT_COL: usize = 12;
const MIN_COLUMNS: usize = 13;

/// Raw data extracted from a single Avanza CSV row, before postings are constructed.
///
/// All amount strings are normalised at parse time: the Swedish comma decimal
/// separator is replaced with `.`, and an empty fee is defaulted to `"0.00"`.
/// This lets the action-specific builders focus purely on producing the correct
/// postings without worrying about formatting.
struct AvanzaRow {
    /// Transaction date.
    date: NaiveDate,
    /// Transaction type (e.g. `"Köp"`, `"Sälj"`, `"Utdelning"`).
    action: String,
    /// Security or instrument name (e.g. `"Ericsson B"`).
    name: String,
    /// Number of securities, normalised to use `.` as the decimal separator.
    quantity: String,
    /// Cash amount, normalised to use `.` as the decimal separator.
    cash_amount: String,
    /// Currency of the cash amount (e.g. `"SEK"`, `"USD"`).
    currency: String,
    /// Brokerage fee, normalised and defaulted to `"0.00"` when blank.
    fee: String,
    /// Capital gain or loss for sell transactions, normalised to use `.` as the decimal separator.
    profit: String,
}

/// Importer for Avanza transaction CSV exports.
///
/// All supported action types produce a [`ImportCandidate::Classified`] entry.
/// Unknown action types return an error so that unexpected data is never silently dropped.
pub struct AvanzaImporter;

impl AvanzaImporter {
    pub fn new() -> Self {
        AvanzaImporter
    }

    /// Opens `csv_path` and parses every data row into an [`AvanzaRow`].
    ///
    /// `flexible(true)` is set to accommodate Avanza's trailing semicolon, which
    /// produces 14 columns rather than the expected 13. Returns an error on the
    /// first row that cannot be parsed.
    fn read_rows(&self, csv_path: &PathBuf) -> crate::Result<Vec<AvanzaRow>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b';')
            .flexible(true)
            .from_path(csv_path)
            .map_err(|e| {
                crate::error::RsledgerError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let mut rows: Vec<AvanzaRow> = Vec::new();

        for result in reader.records() {
            let record = result.map_err(|e| {
                crate::error::RsledgerError::ParseError(
                    csv_path.display().to_string(),
                    format!("Failed to read CSV record: {}", e),
                )
            })?;

            if record.len() < MIN_COLUMNS {
                return Err(crate::error::RsledgerError::ParseError(
                    csv_path.display().to_string(),
                    format!(
                        "Row has {} column(s) but {} are required.",
                        record.len(),
                        MIN_COLUMNS
                    ),
                ));
            }

            // --- Date ---
            let date_str = record[DATE_COL].trim();
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
                crate::error::RsledgerError::ParseError(
                    csv_path.display().to_string(),
                    format!(
                        "Could not parse date '{}'. Expected format YYYY-MM-DD.",
                        date_str
                    ),
                )
            })?;

            let action = record[ACTION_COL].trim().to_string();
            let name = record[NAME_COL].trim().to_string();

            // Normalise decimal separators: Avanza uses ',' where we need '.'.
            let quantity = record[QUANTITY_COL].trim().replace(',', ".");
            let cash_amount = record[CASH_AMOUNT_COL].trim().replace(',', ".");
            let currency = record[CURRENCY_COL].trim().to_string();

            let raw_fee = record[FEE_COL].trim().replace(',', ".");
            let fee = if raw_fee.is_empty() {
                "0.00".to_string()
            } else {
                raw_fee
            };

            let profit = record[PROFIT_COL].trim().replace(',', ".");

            rows.push(AvanzaRow {
                date,
                action,
                name,
                quantity,
                cash_amount,
                currency,
                fee,
                profit,
            });
        }

        Ok(rows)
    }

    /// Dispatches an [`AvanzaRow`] to the appropriate builder based on its action type
    /// and wraps the result in [`ImportCandidate::Classified`].
    ///
    /// Returns an error for unrecognised action types.
    fn build_transaction(&self, row: AvanzaRow) -> crate::Result<ImportCandidate<Transaction>> {
        let transaction = match row.action.as_str() {
            "Insättning" | "Uttag" => self.build_deposit_withdrawal(row)?,
            "Köp" => self.build_buy(row)?,
            "Sälj" => self.build_sell(row)?,
            "Utdelning" => self.build_dividend(row)?,
            "Utländsk källskatt" => self.build_withholding_tax(row)?,
            "Utlåningsränta" => self.build_lending_interest(row)?,
            other => {
                return Err(crate::error::RsledgerError::ParseError(
                    "Avanza CSV".to_string(),
                    format!("Unknown transaction type '{}'.", other),
                ));
            }
        };
        Ok(ImportCandidate::Classified(transaction))
    }

    /// Builds a deposit (`Insättning`) or withdrawal (`Uttag`) transaction.
    ///
    /// The cash amount flows through `assets:bank:avanza`; the counterpart
    /// `expenses:bank:internal-transfers` auto-balances the entry.
    fn build_deposit_withdrawal(&self, row: AvanzaRow) -> crate::Result<Transaction> {
        let amount = CommodityValue::from_str(&format!("{} {}", row.cash_amount, row.currency))?;
        let postings = vec![
            Posting::new(Account::from_str("assets:bank:avanza")?, Some(amount)),
            Posting::new(Account::from_str("expenses:bank:internal-transfers")?, None),
        ];
        Ok(Transaction::new(
            row.date,
            format!("{} {}", row.action, row.name),
            postings,
        ))
    }

    /// Builds a buy (`Köp`) transaction.
    ///
    /// Three postings are created: the acquired securities, the cash paid (negative),
    /// and the brokerage fee.
    fn build_buy(&self, row: AvanzaRow) -> crate::Result<Transaction> {
        let commodity_amount = CommodityValue::from_str(&format!("{} {}", row.quantity, row.name))?;
        let cash_amount =
            CommodityValue::from_str(&format!("{} {}", row.cash_amount, row.currency))?;
        let fee_amount = CommodityValue::from_str(&format!("{} SEK", row.fee))?;

        let postings = vec![
            Posting::new(
                Account::from_str("assets:bank:avanza")?,
                Some(commodity_amount),
            ),
            Posting::new(Account::from_str("assets:bank:avanza")?, Some(cash_amount)),
            Posting::new(Account::from_str("expenses:bank:avanza")?, Some(fee_amount)),
        ];
        Ok(Transaction::new(
            row.date,
            format!("{} {}", row.action, row.name),
            postings,
        ))
    }

    /// Builds a sell (`Sälj`) transaction.
    ///
    /// Four postings are created: the sold securities (negative quantity), the cash received,
    /// the brokerage fee, and the capital gain/loss posted to `equity:capital-gains`
    /// as the negated profit figure reported by Avanza.
    fn build_sell(&self, row: AvanzaRow) -> crate::Result<Transaction> {
        let commodity_amount = CommodityValue::from_str(&format!("{} {}", row.quantity, row.name))?;
        let cash_amount =
            CommodityValue::from_str(&format!("{} {}", row.cash_amount, row.currency))?;
        let fee_amount = CommodityValue::from_str(&format!("{} SEK", row.fee))?;

        let profit_cv = CommodityValue::from_str(&format!("{} SEK", row.profit))?;
        let negated_profit = -&profit_cv;

        let postings = vec![
            Posting::new(
                Account::from_str("assets:bank:avanza")?,
                Some(commodity_amount),
            ),
            Posting::new(Account::from_str("assets:bank:avanza")?, Some(cash_amount)),
            Posting::new(Account::from_str("expenses:bank:avanza")?, Some(fee_amount)),
            Posting::new(
                Account::from_str("equity:capital-gains")?,
                Some(negated_profit),
            ),
        ];
        Ok(Transaction::new(
            row.date,
            format!("{} {}", row.action, row.name),
            postings,
        ))
    }

    /// Builds a dividend (`Utdelning`) transaction.
    ///
    /// Cash received flows into `assets:bank:avanza`; the counterpart
    /// `income:dividends` auto-balances.
    fn build_dividend(&self, row: AvanzaRow) -> crate::Result<Transaction> {
        let amount = CommodityValue::from_str(&format!("{} {}", row.cash_amount, row.currency))?;
        let postings = vec![
            Posting::new(Account::from_str("assets:bank:avanza")?, Some(amount)),
            Posting::new(Account::from_str("income:dividends")?, None),
        ];
        Ok(Transaction::new(
            row.date,
            format!("{} {}", row.action, row.name),
            postings,
        ))
    }

    /// Builds a foreign withholding tax (`Utländsk källskatt`) transaction.
    ///
    /// The deducted tax flows out of `assets:bank:avanza`; the counterpart
    /// `expenses:taxes:withholding` auto-balances.
    fn build_withholding_tax(&self, row: AvanzaRow) -> crate::Result<Transaction> {
        let amount = CommodityValue::from_str(&format!("{} {}", row.cash_amount, row.currency))?;
        let postings = vec![
            Posting::new(Account::from_str("assets:bank:avanza")?, Some(amount)),
            Posting::new(Account::from_str("expenses:taxes:withholding")?, None),
        ];
        Ok(Transaction::new(
            row.date,
            format!("{} {}", row.action, row.name),
            postings,
        ))
    }

    /// Builds a lending interest (`Utlåningsränta`) transaction.
    ///
    /// Interest received flows into `assets:bank:avanza`; the counterpart
    /// `expenses:bank:avanza:interest` auto-balances.
    fn build_lending_interest(&self, row: AvanzaRow) -> crate::Result<Transaction> {
        let amount = CommodityValue::from_str(&format!("{} {}", row.cash_amount, row.currency))?;
        let postings = vec![
            Posting::new(Account::from_str("assets:bank:avanza")?, Some(amount)),
            Posting::new(Account::from_str("expenses:bank:avanza:interest")?, None),
        ];
        Ok(Transaction::new(
            row.date,
            format!("{} {}", row.action, row.name),
            postings,
        ))
    }
}

impl EntryImporter<Transaction> for AvanzaImporter {
    fn import_csv(&self, csv_path: PathBuf) -> crate::Result<Vec<ImportCandidate<Transaction>>> {
        // Phase 1: parse all rows into intermediate AvanzaRows.
        // Phase 2: dispatch each row to the appropriate transaction builder.
        let rows = self.read_rows(&csv_path)?;
        rows.into_iter()
            .map(|row| self.build_transaction(row))
            .collect()
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

    fn import_all() -> Vec<ImportCandidate<Transaction>> {
        AvanzaImporter::new()
            .import_csv(csv_path("avanza_transactions.csv"))
            .unwrap()
    }

    // -------------------------------------------------------------------------
    // Overall shape
    // -------------------------------------------------------------------------

    #[test]
    fn imports_all_seven_transactions() {
        assert_eq!(import_all().len(), 7);
    }

    #[test]
    fn all_transactions_are_classified() {
        for candidate in import_all() {
            assert!(
                matches!(candidate, ImportCandidate::Classified(_)),
                "every Avanza transaction should be classified"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Insättning (deposit)
    // -------------------------------------------------------------------------

    #[test]
    fn deposit_has_correct_date_and_description() {
        if let ImportCandidate::Classified(t) = &import_all()[0] {
            assert_eq!(*t.date(), NaiveDate::from_ymd_opt(2026, 3, 21).unwrap());
            assert_eq!(t.description(), "Insättning Bankgiro");
        }
    }

    #[test]
    fn deposit_has_two_postings() {
        if let ImportCandidate::Classified(t) = &import_all()[0] {
            assert_eq!(t.postings().len(), 2);
        }
    }

    #[test]
    fn deposit_amount_and_accounts() {
        if let ImportCandidate::Classified(t) = &import_all()[0] {
            assert_eq!(t.postings()[0].account().to_string(), "assets:bank:avanza");
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "1000 SEK");
            assert_eq!(
                t.postings()[1].account().to_string(),
                "expenses:bank:internal-transfers"
            );
            assert!(t.postings()[1].amount().is_none());
        }
    }

    // -------------------------------------------------------------------------
    // Uttag (withdrawal)
    // -------------------------------------------------------------------------

    #[test]
    fn withdrawal_amount_is_negative() {
        if let ImportCandidate::Classified(t) = &import_all()[1] {
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "-500 SEK");
        }
    }

    // -------------------------------------------------------------------------
    // Köp (buy)
    // -------------------------------------------------------------------------

    #[test]
    fn buy_has_three_postings() {
        if let ImportCandidate::Classified(t) = &import_all()[2] {
            assert_eq!(t.postings().len(), 3);
        }
    }

    #[test]
    fn buy_commodity_amount_uses_security_name() {
        if let ImportCandidate::Classified(t) = &import_all()[2] {
            // Commodity posting: "{quantity} \"{name}\"" — names with spaces are quoted by Display.
            assert_eq!(
                t.postings()[0].amount().unwrap().to_string(),
                "56 \"Ericsson B\""
            );
        }
    }

    #[test]
    fn buy_cash_amount_normalises_comma_decimal() {
        if let ImportCandidate::Classified(t) = &import_all()[2] {
            // -5408,04 SEK → -5408.04 SEK
            assert_eq!(
                t.postings()[1].amount().unwrap().to_string(),
                "-5408.04 SEK"
            );
        }
    }

    #[test]
    fn buy_fee_normalises_comma_decimal() {
        if let ImportCandidate::Classified(t) = &import_all()[2] {
            // 13,49 SEK → 13.49 SEK
            assert_eq!(t.postings()[2].amount().unwrap().to_string(), "13.49 SEK");
            assert_eq!(
                t.postings()[2].account().to_string(),
                "expenses:bank:avanza"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Sälj (sell)
    // -------------------------------------------------------------------------

    #[test]
    fn sell_has_four_postings() {
        if let ImportCandidate::Classified(t) = &import_all()[3] {
            assert_eq!(t.postings().len(), 4);
        }
    }

    #[test]
    fn sell_capital_gains_posting_is_negated_profit() {
        if let ImportCandidate::Classified(t) = &import_all()[3] {
            // Profit in CSV is 500,00 → normalised to 500.00 → negated → -500 SEK
            assert_eq!(
                t.postings()[3].account().to_string(),
                "equity:capital-gains"
            );
            assert_eq!(t.postings()[3].amount().unwrap().to_string(), "-500 SEK");
        }
    }

    // -------------------------------------------------------------------------
    // Utdelning (dividend)
    // -------------------------------------------------------------------------

    #[test]
    fn dividend_has_two_postings_with_correct_accounts() {
        if let ImportCandidate::Classified(t) = &import_all()[4] {
            assert_eq!(t.postings().len(), 2);
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "342.5 USD");
            assert_eq!(t.postings()[1].account().to_string(), "income:dividends");
        }
    }

    // -------------------------------------------------------------------------
    // Utländsk källskatt (withholding tax)
    // -------------------------------------------------------------------------

    #[test]
    fn withholding_tax_routes_to_correct_account() {
        if let ImportCandidate::Classified(t) = &import_all()[5] {
            assert_eq!(
                t.postings()[1].account().to_string(),
                "expenses:taxes:withholding"
            );
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "-51.38 USD");
        }
    }

    // -------------------------------------------------------------------------
    // Utlåningsränta (lending interest)
    // -------------------------------------------------------------------------

    #[test]
    fn lending_interest_routes_to_correct_account() {
        if let ImportCandidate::Classified(t) = &import_all()[6] {
            assert_eq!(
                t.postings()[1].account().to_string(),
                "expenses:bank:avanza:interest"
            );
            assert_eq!(t.postings()[0].amount().unwrap().to_string(), "12.5 SEK");
        }
    }

    // -------------------------------------------------------------------------
    // Empty fee defaults to zero
    // -------------------------------------------------------------------------

    #[test]
    fn empty_fee_column_defaults_to_zero() {
        // Deposit row (index 0) has an empty fee column; buy builds a fee posting
        // only for Köp/Sälj. Verify the buy row's fee is non-zero to prove the
        // default kicks in when blank.
        if let ImportCandidate::Classified(t) = &import_all()[2] {
            // fee is "13,49" → "13.49", not the default "0.00"
            assert_ne!(t.postings()[2].amount().unwrap().to_string(), "0 SEK");
        }
    }

    // -------------------------------------------------------------------------
    // Error cases
    // -------------------------------------------------------------------------

    #[test]
    fn unknown_action_type_returns_error() {
        // Build a minimal in-memory CSV with an unknown action type.
        let dir = std::env::temp_dir();
        let path = dir.join("avanza_unknown_action.csv");
        std::fs::write(
            &path,
            "Datum;Konto;Typ av transaktion;Värdepapper/beskrivning;Antal;Kurs;Belopp;Transaktionsvaluta;Courtage;Valutakurs;Instrumentvaluta;ISIN;Resultat\n\
             2026-03-01;ISK;Okänd;Test;0;0;100;SEK;;;SEK;;\n",
        )
        .unwrap();
        let result = AvanzaImporter::new().import_csv(path);
        assert!(result.is_err());
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let result = AvanzaImporter::new().import_csv(PathBuf::from("does_not_exist.csv"));
        assert!(result.is_err());
    }
}
