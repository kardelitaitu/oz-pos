//! `AppError` — the single error type returned by every Tauri command.
//!
//! Marked `#[serde(tag = "kind", rename_all = "camelCase")]` so the
//! TypeScript side sees a `kind` discriminator field, and `non_exhaustive`
//! so new variants can be added without breaking semver.
//!
//! On the front-end, `ui/src/types/domain.ts` mirrors this shape.
//!
//! `Core` and `Hardware` variants carry a typed `sub_kind` discriminator
//! so the front-end can branch on the specific error variant without
//! parsing the message string.

use oz_core::CoreErrorKind;
use oz_hal::HalErrorKind;
use serde::Serialize;
use thiserror::Error;

/// Discriminated error returned by every `#[tauri::command]`.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum AppError {
    /// Wraps any `oz_core::CoreError` (DB, money, currency mismatch, …).
    #[error("core error: {message}")]
    Core {
        /// Typed sub-discriminator mirroring the `CoreError` variant.
        sub_kind: CoreErrorKind,
        /// Human-readable error message.
        message: String,
    },

    /// Wraps any `oz_hal::HalError` (device not found, USB timeout, …).
    #[error("hardware error: {message}")]
    Hardware {
        /// Typed sub-discriminator mirroring the `HalError` variant.
        sub_kind: HalErrorKind,
        /// Human-readable error message.
        message: String,
    },

    /// A Tauri-level error (state missing, invalid argument, …).
    #[error("invalid request: {0}")]
    Invalid(String),

    /// The caller's role does not have the required permission.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Catch-all for unexpected internal errors. Logged with full context.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<oz_core::CoreError> for AppError {
    fn from(e: oz_core::CoreError) -> Self {
        Self::Core {
            sub_kind: e.kind(),
            message: e.to_string(),
        }
    }
}

impl From<oz_hal::HalError> for AppError {
    fn from(e: oz_hal::HalError) -> Self {
        Self::Hardware {
            sub_kind: e.kind(),
            message: e.to_string(),
        }
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(format!("{e:#}"))
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Core {
            sub_kind: CoreErrorKind::Db,
            message: format!("sqlite: {e}"),
        }
    }
}
