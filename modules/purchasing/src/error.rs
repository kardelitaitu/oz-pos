/*
last audited 25-07-26 by RSA-Agent (modules-purchasing slice A: error verified)
crate: modules-purchasing | status: SAFE | lint: CLEAN
findings: clean thiserror purchasing error taxonomy (stub)
next: none | perf: N/A
*/
//! Error type for the purchasing domain.
//!
//! Mirrors the shape used by the other module crates (`Db`, `NotFound`,
//! `Validation`) so that promoting this stub to an owning module does not
//! change the error surface its callers already match on.

use thiserror::Error;

/// Errors that can originate in the purchasing domain.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PurchasingError {
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

impl PurchasingError {
    /// Create a validation error for a specific field.
    pub fn validation(field: &'static str, message: impl Into<String>) -> Self {
        Self::Validation {
            field,
            message: message.into(),
        }
    }
}
