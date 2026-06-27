//! Error type for `oz-reporting`.

use thiserror::Error;

/// Errors that can originate in a reporting query or export.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReportingError {
    /// The underlying SQLite query failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// The requested time window is invalid (e.g., end before start).
    #[error("invalid time window: {0}")]
    InvalidWindow(String),

    /// A CSV export could not be written to disk.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}
