//! Error type for the CRM domain.

use thiserror::Error;

/// Errors that can originate in the customer relationship management domain.
#[derive(Debug, Error)]
pub enum CrmError {
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

impl CrmError {
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
    fn crm_error_validation_message() {
        let err = CrmError::validation("name", "must not be empty");
        assert!(matches!(
            err,
            CrmError::Validation { field, .. } if field == "name"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on name: must not be empty"
        );
    }

    #[test]
    fn crm_error_not_found_message() {
        let err = CrmError::NotFound {
            entity: "customer",
            id: "bad-id".into(),
        };
        assert_eq!(format!("{err}"), "not found: customer bad-id");
    }

    #[test]
    fn crm_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = CrmError::from(rusqlite_err);
        assert!(matches!(err, CrmError::Db(_)));
    }
}
