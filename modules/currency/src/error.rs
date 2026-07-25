//! Error type for the currency/exchange-rate domain.

use thiserror::Error;

/// Errors that can originate in the currency/exchange-rate domain.
#[derive(Debug, Error)]
pub enum CurrencyError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// Input validation failure.
    #[error("validation error on {field}: {message}")]
    Validation {
        /// The field that failed validation.
        field: &'static str,
        /// Human-readable description of the failure.
        message: String,
    },

    /// A lookup by id returned no row.
    #[error("not found: {entity} {id}")]
    NotFound {
        /// The kind of entity that was being looked up.
        entity: &'static str,
        /// The id that was looked up.
        id: String,
    },
}

impl CurrencyError {
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
    fn currency_error_validation_message() {
        let err = CurrencyError::validation("rate_millionths", "rate must be positive");
        assert!(matches!(err, CurrencyError::Validation { field, .. } if field == "rate_millionths"));
        assert_eq!(format!("{err}"), "validation error on rate_millionths: rate must be positive");
    }

    #[test]
    fn currency_error_not_found_message() {
        let err = CurrencyError::NotFound {
            entity: "exchange_rate",
            id: "bad-id".into(),
        };
        assert_eq!(format!("{err}"), "not found: exchange_rate bad-id");
    }

    #[test]
    fn currency_error_from_rusqlite() {
        // Build a rusqlite error without needing a real DB.
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = CurrencyError::from(rusqlite_err);
        assert!(matches!(err, CurrencyError::Db(_)));
    }
}
