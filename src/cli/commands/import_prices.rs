use crate::journal;

/// Stub: imports prices from a positions CSV. Not yet implemented.
pub fn run_import_prices(
    _journal_file: journal::JournalFile,
    _csv_file: &std::path::PathBuf,
) -> crate::Result<()> {
    return Err(crate::error::RsledgerError::CliError(
        "Import prices command is not yet implemented.".to_string(),
    ));
}
