pub(crate) mod avanza_prices;
pub(crate) mod rules;

use std::fmt::Display;
use std::hash::Hash;

/// Enum to represent an import candidate with its classification status.
/// Some candidates can be automatically classified while others may require manual classification.
pub(crate) enum ImportCandidate<T: Display + Hash> {
    Classified(T),
    Unclassified(T),
}

/// Defines the IO-behaviour of an entry importer, allowing the underlying
/// importing logic to be arbitrarily complex as long as it adheres to this interface.
trait EntryImporter<T: Display + Hash> {
    fn import_csv(&self, csv_path: std::path::PathBuf) -> Vec<ImportCandidate<T>>;
}

/// Import items into the journal file.
/// This function handles deduplication of items and promps for manual classification where needed.
fn import_items<T: Display + Hash>(
    items: Vec<T>,
    existing_journal: crate::journal::Journal,
) -> crate::Result<()> {
    return Err(crate::error::RsledgerError::CliError(
        "Import command is not yet implemented.".to_string(),
    ));
}
