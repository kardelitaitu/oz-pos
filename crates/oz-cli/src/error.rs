//! Error type for the `oz-cli` binary.

use thiserror::Error;

/// Errors that can originate in the `oz` command-line tool.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CliError {
    /// A subcommand returned a non-zero status.
    #[error("subcommand `{0}` failed: {1}")]
    Subcommand(&'static str, String),

    /// The local database could not be opened.
    #[error("could not open database: {0}")]
    OpenDatabase(#[from] rusqlite::Error),

    /// Bad command-line arguments.
    #[error("invalid arguments: {0}")]
    Args(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subcommand_display() {
        let err = CliError::Subcommand("migrate", "connection refused".into());
        assert_eq!(
            err.to_string(),
            "subcommand `migrate` failed: connection refused"
        );
    }

    #[test]
    fn args_display() {
        let err = CliError::Args("missing required argument".into());
        assert_eq!(
            err.to_string(),
            "invalid arguments: missing required argument"
        );
    }

    #[test]
    fn open_database_display() {
        let err = CliError::OpenDatabase(rusqlite::Error::InvalidParameterName("bad".into()));
        let msg = err.to_string();
        assert!(msg.starts_with("could not open database:"));
        // The rusqlite::Error's Display impl is appended after the prefix.
        assert!(msg.len() > "could not open database: ".len());
    }

    #[test]
    fn from_rusqlite_error() {
        let db_err = rusqlite::Error::InvalidParameterName("x".into());
        let cli_err: CliError = db_err.into();
        assert!(matches!(cli_err, CliError::OpenDatabase(_)));
        assert!(cli_err.to_string().contains("could not open database"));
    }

    #[test]
    fn implements_std_error() {
        let err = CliError::Args("test".into());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn debug_output() {
        let err = CliError::Subcommand("backup", "disk full".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Subcommand"));
        assert!(debug.contains("backup"));
        assert!(debug.contains("disk full"));
    }

    #[test]
    fn empty_args() {
        let err = CliError::Args("".into());
        assert_eq!(err.to_string(), "invalid arguments: ");
    }
}
