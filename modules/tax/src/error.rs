/*
last audited 25-07-26 by RSA-Agent (modules-tax slice A: error verified)
crate: modules-tax | status: SAFE | lint: CLEAN
findings: clean thiserror tax error taxonomy
next: none | perf: N/A
*/
//! Error type for the tax domain.

use thiserror::Error;

/// Errors that can originate in the tax domain.
#[derive(Debug, Error)]
pub enum TaxError {
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

impl TaxError {
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
    fn tax_error_validation_message() {
        let err = TaxError::validation("rate_bps", "rate must be positive");
        assert!(matches!(
            err,
            TaxError::Validation { field, .. } if field == "rate_bps"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on rate_bps: rate must be positive"
        );
    }

    #[test]
    fn tax_error_not_found_message() {
        let err = TaxError::NotFound {
            entity: "tax_rate",
            id: "bad-id".into(),
        };
        assert_eq!(format!("{err}"), "not found: tax_rate bad-id");
    }

    #[test]
    fn tax_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = TaxError::from(rusqlite_err);
        assert!(matches!(err, TaxError::Db(_)));
    }
}
