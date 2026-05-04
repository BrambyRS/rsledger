//! This module defines the error types for use across the codebase.

#[derive(Debug, thiserror::Error)]
pub enum RsledgerError {
    /// Error when something cannot be parsed correctly
    /// ParseError(Item, Reason)
    #[error("Could not parse {0}: {1}")]
    ParseError(String, String),

    // Error with IO operations
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Validation errors
    /// ValidationError(Item, Reason)
    #[error("Validation error in {0}: {1}")]
    ValidationError(String, String),

    /// Invalid arguments passed from CLI
    /// CliError(Reason)
    #[error("CLI error: {0}")]
    CliError(String),
}
