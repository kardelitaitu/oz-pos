/*
last audited 25-07-26 by RSA-Agent (modules-reporting slice A: error verified)
crate: modules-reporting | status: SAFE | lint: CLEAN
findings: clean thiserror reporting error taxonomy
next: none | perf: N/A
*/
//! Error type for the reporting domain.

use thiserror::Error;

/// Errors that can originate in the reporting domain.
#[derive(Debug, Error)]
pub enum ReportingError {
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

impl ReportingError {
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
    fn reporting_error_validation_message() {
        let err = ReportingError::validation("date", "must be a valid date");
        assert!(matches!(
            err,
            ReportingError::Validation { field, .. } if field == "date"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on date: must be a valid date"
        );
    }

    #[test]
    fn reporting_error_not_found_message() {
        let err = ReportingError::NotFound {
            entity: "report",
            id: "2025-13-01".into(),
        };
        assert_eq!(format!("{err}"), "not found: report 2025-13-01");
    }

    #[test]
    fn reporting_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = ReportingError::from(rusqlite_err);
        assert!(matches!(err, ReportingError::Db(_)));
    }
}
