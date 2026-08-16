//! Error type for the settings domain.

use thiserror::Error;

/// Errors that can originate in the settings domain.
#[derive(Debug, Error)]
pub enum SettingsError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// A lookup by key returned no row.
    #[error("not found: {entity} {id}")]
    NotFound {
        /// The kind of entity that was being looked up.
        entity: &'static str,
        /// The key that was looked up.
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

impl SettingsError {
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
    fn settings_error_validation_message() {
        let err = SettingsError::validation("key", "must not be empty");
        assert!(matches!(
            err,
            SettingsError::Validation { field, .. } if field == "key"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on key: must not be empty"
        );
    }

    #[test]
    fn settings_error_not_found_message() {
        let err = SettingsError::NotFound {
            entity: "setting",
            id: "missing_key".into(),
        };
        assert_eq!(format!("{err}"), "not found: setting missing_key");
    }

    #[test]
    fn settings_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = SettingsError::from(rusqlite_err);
        assert!(matches!(err, SettingsError::Db(_)));
    }
}
