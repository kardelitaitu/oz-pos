//! Command-line tools for OZ-POS — migrations, backup, export, smoke tests.
//!
//! `oz-cli` is a binary crate (see `src/main.rs`) that exposes the
//! maintenance operations a merchant or operator runs from a terminal:
//! - `oz migrate` — apply SQL migrations
//! - `oz backup` — snapshot the local SQLite store
//! - `oz export` — write a CSV report
//! - `oz smoke` — run a self-test against the local stack
//!
//! The library side of the crate (this file) holds the shared error
//! type so both `main.rs` and the subcommand modules can use it.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::CliError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subcommand_error_display() {
        let err = CliError::Subcommand("migrate", "failed".into());
        assert_eq!(err.to_string(), "subcommand `migrate` failed: failed");
    }

    #[test]
    fn open_database_error_display() {
        let inner = rusqlite::Error::InvalidParameterName("x".into());
        let err = CliError::OpenDatabase(inner);
        assert!(err.to_string().contains("could not open database"));
    }

    #[test]
    fn args_error_display() {
        let err = CliError::Args("missing <sku>".into());
        assert_eq!(err.to_string(), "invalid arguments: missing <sku>");
    }

    #[test]
    fn error_is_debug() {
        let err = CliError::Args("test".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn open_database_from_rusqlite() {
        let inner = rusqlite::Error::InvalidParameterName("foo".into());
        let err: CliError = inner.into();
        assert!(err.to_string().contains("could not open database"));
    }
}
