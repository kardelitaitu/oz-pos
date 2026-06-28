//! Structured logging facade for OZ-POS.
//!
//! `oz-logging` wraps the `tracing` ecosystem with a context-tagged
//! record format, a file + stdout writer, and a small CLI to inspect
//! recent log output. Levels follow the `rust-backend` skill's
//! conventions (error/warn/info/debug).

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::LoggingError;

/// Initialise structured logging via `tracing-subscriber`.
///
/// Reads `RUST_LOG` from the environment; falls back to `info` if unset.
/// Call this once, early in `main` / `run`, before any `tracing` macro
/// is hit.
///
/// # Panics
///
/// Panics if the global subscriber has already been set (e.g. by a test
/// harness). Production code must call this exactly once.
pub fn init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
