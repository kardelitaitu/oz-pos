//! Structured logging facade for OZ-POS.
//!
//! `oz-logging` wraps the `tracing` ecosystem with a context-tagged
//! record format, a file + stdout writer, and a small CLI to inspect
//! recent log output. Levels follow the `rust-backend` skill's
//! conventions (error/warn/info/debug).
//!
//! This crate is a scaffold — the file logger and context plumbing
//! land in a follow-up.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::LoggingError;
