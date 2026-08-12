//! Error type for the loyalty domain.

use thiserror::Error;

/// Errors that can originate in the loyalty/gift-card domain.
#[derive(Debug, Error)]
pub enum LoyaltyError {
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

impl LoyaltyError {
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
    fn loyalty_error_validation_message() {
        let err = LoyaltyError::validation("card_number", "must not be empty");
        assert!(matches!(
            err,
            LoyaltyError::Validation { field, .. } if field == "card_number"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on card_number: must not be empty"
        );
    }

    #[test]
    fn loyalty_error_not_found_message() {
        let err = LoyaltyError::NotFound {
            entity: "loyalty_account",
            id: "cust-xxx".into(),
        };
        assert_eq!(format!("{err}"), "not found: loyalty_account cust-xxx");
    }

    #[test]
    fn loyalty_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = LoyaltyError::from(rusqlite_err);
        assert!(matches!(err, LoyaltyError::Db(_)));
    }
}
