//! Error type for the staff domain.

use thiserror::Error;

/// Errors that can originate in the staff/user domain.
#[derive(Debug, Error)]
pub enum StaffError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// A platform infrastructure error.
    #[error("platform error: {0}")]
    Platform(#[from] platform_core::PlatformError),

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

impl StaffError {
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
    fn staff_error_validation_message() {
        let err = StaffError::validation("username", "must not be empty");
        assert!(matches!(
            err,
            StaffError::Validation { field, .. } if field == "username"
        ));
        assert_eq!(
            format!("{err}"),
            "validation error on username: must not be empty"
        );
    }

    #[test]
    fn staff_error_not_found_message() {
        let err = StaffError::NotFound {
            entity: "user",
            id: "bad-id".into(),
        };
        assert_eq!(format!("{err}"), "not found: user bad-id");
    }

    #[test]
    fn staff_error_from_rusqlite() {
        let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
        let err = StaffError::from(rusqlite_err);
        assert!(matches!(err, StaffError::Db(_)));
    }

    #[test]
    fn staff_error_from_platform_error() {
        let platform_err = platform_core::PlatformError::Internal("test".into());
        let err = StaffError::from(platform_err);
        assert!(matches!(err, StaffError::Platform(_)));
    }
}
