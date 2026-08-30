/*
last audited 25-07-26 by RSA-Agent (modules-terminal slice A: error verified)
crate: modules-terminal | status: SAFE | lint: CLEAN
findings: clean thiserror terminal error taxonomy
next: none | perf: N/A
*/
//! Error type for the terminal domain.

use thiserror::Error;

/// Errors that can originate in the terminal domain.
#[derive(Debug, Error)]
pub enum TerminalError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// A lookup by id returned no row.
    #[error("not found: {entity} {id}")]
    NotFound {
        /// The kind of entity that was being looked up.
        entity: &'static str,
        /// The id that was looked up.
        id: String,
    },

    /// Input validation failure.
    #[error("validation error on {field}: {message}")]
    Validation {
        /// The field that failed validation.
        field: &'static str,
        /// Human-readable description of the failure.
        message: String,
    },
}

impl TerminalError {
    /// Create a validation error for a specific field.
    pub fn validation(field: &'static str, message: impl Into<String>) -> Self {
        Self::Validation {
            field,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_error_validation_message() {
        let err = TerminalError::validation("name", "must not be empty");
        assert!(matches!(
            err,
            TerminalError::Validation { field, .. } if field == "name"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on name: must not be empty"
        );
    }

    #[test]
    fn terminal_error_not_found_message() {
        let err = TerminalError::NotFound {
            entity: "terminal",
            id: "bad-id".into(),
        };
        assert_eq!(format!("{err}"), "not found: terminal bad-id");
    }

    #[test]
    fn terminal_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = TerminalError::from(rusqlite_err);
        assert!(matches!(err, TerminalError::Db(_)));
    }
}
