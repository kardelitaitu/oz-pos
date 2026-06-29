//! `AppError` — the single error type returned by every Tauri command.
//!
//! Marked `#[serde(tag = "kind", rename_all = "camelCase")]` so the
//! TypeScript side sees a `kind` discriminator field, and `non_exhaustive`
//! so new variants can be added without breaking semver.
//!
//! On the front-end, `ui/src/types/domain.ts` mirrors this shape.
//!
//! # TODO(structured-payload)
//!
//! The current `From<oz_core::CoreError>` / `From<oz_hal::HalError>`
//! impls call `.to_string()` and lose the typed variant. The next
//! refactor should carry a structured sub-payload — e.g.
//! `AppError::Core { kind: CoreErrorKind, message: String }` — so the
//! front-end can branch on both the top-level `kind` and a typed
//! sub-discriminator. The `tauri-ipc` skill calls this out.

use serde::Serialize;
use thiserror::Error;

/// Discriminated error returned by every `#[tauri::command]`.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum AppError {
    /// Wraps any `oz_core::CoreError` (DB, money, currency mismatch, …).
    #[error("core error: {0}")]
    Core(String),

    /// Wraps any `oz_hal::HalError` (device not found, USB timeout, …).
    #[error("hardware error: {0}")]
    Hardware(String),

    /// A Tauri-level error (state missing, invalid argument, …).
    #[error("invalid request: {0}")]
    Invalid(String),

    /// Catch-all for unexpected internal errors. Logged with full context.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<oz_core::CoreError> for AppError {
    fn from(e: oz_core::CoreError) -> Self {
        Self::Core(e.to_string())
    }
}

impl From<oz_hal::HalError> for AppError {
    fn from(e: oz_hal::HalError) -> Self {
        Self::Hardware(e.to_string())
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
        Self::Core(format!("sqlite: {e}"))
    }
}
