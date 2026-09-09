use crate::csv_importer::avanza_prices::AvanzaPricesImporter;
use crate::csv_importer::{EntryImporter, import_price_entries};
use crate::journal;

/// Imports price directives from an Avanza positions CSV into the prices journal.
pub fn run_import_prices(
    mut journal_file: journal::JournalFile,
    csv_file: &std::path::PathBuf,
) -> crate::Result<()> {
    let candidates = AvanzaPricesImporter::new().import_csv(csv_file.clone())?;
    return import_price_entries(candidates, &mut journal_file);
}
