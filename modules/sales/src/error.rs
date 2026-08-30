/*
last audited 25-07-26 by RSA-Agent (modules-sales slice B: error verified)
crate: modules-sales | status: SAFE | lint: CLEAN
findings: clean thiserror sales error taxonomy with validation helper
next: none | perf: N/A
*/
//! Error type for the sales domain.

use thiserror::Error;

/// Errors that can originate in the sales domain.
#[derive(Debug, Error)]
pub enum SalesError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// A serialization error.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// An invalid sale status transition.
    #[error("invalid transition: {0}")]
    InvalidTransition(#[from] foundation::InvalidTransition),

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

impl SalesError {
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
    fn sales_error_validation_message() {
        let err = SalesError::validation("currency", "invalid currency code");
        assert!(matches!(
            err,
            SalesError::Validation { field, .. } if field == "currency"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on currency: invalid currency code"
        );
    }

    #[test]
    fn sales_error_not_found_message() {
        let err = SalesError::NotFound {
            entity: "sale",
            id: "bad-id".into(),
        };
        assert_eq!(format!("{err}"), "not found: sale bad-id");
    }

    #[test]
    fn sales_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = SalesError::from(rusqlite_err);
        assert!(matches!(err, SalesError::Db(_)));
    }
}
