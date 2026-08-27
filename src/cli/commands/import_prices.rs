use crate::csv_importer::avanza_prices::AvanzaPricesImporter;
use crate::csv_importer::{EntryImporter, ImportCandidate};
use crate::journal;

/// Imports price directives from an Avanza positions CSV into the prices journal.
///
/// Deduplicates against existing prices using the price hash and appends only
/// new entries to the journal file.
pub fn run_import_prices(
    mut journal_file: journal::JournalFile,
    csv_file: &std::path::PathBuf,
) -> crate::Result<()> {
    let candidates = AvanzaPricesImporter::new().import_csv(csv_file.clone())?;

    // Load the existing prices once and build a hash-set for O(1) lookup.
    let journal = journal_file.load()?;
    let existing: std::collections::HashSet<u64> =
        journal.prices.iter().map(|(hash, _)| *hash).collect();

    for candidate in candidates {
        // All Avanza price candidates are Classified; the match is exhaustive for safety.
        let price = match candidate {
            ImportCandidate::Classified(p) | ImportCandidate::Unclassified(p) => p,
        };
        if !existing.contains(&price.price_hash()) {
            journal_file.add_entry(&price)?;
        }
    }

    Ok(())
}
