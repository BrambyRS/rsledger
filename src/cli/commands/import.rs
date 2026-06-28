use crate::cli::args::ParserOptions;
use crate::journal;

use std::io::{BufRead, Write};

/// Stub: imports transactions from a CSV. Not yet implemented.
pub fn run_import(
    _journal_file: journal::JournalFile,
    _csv_file: &std::path::PathBuf,
    _parser_opt: ParserOptions,
    _rule_sheet: &str,
    _accept_partial_matches: bool,
    _reader: &mut impl BufRead,
    _writer: &mut impl Write,
) -> crate::Result<()> {
    return Err(crate::error::RsledgerError::CliError(
        "Import command is not yet implemented.".to_string(),
    ));
}
