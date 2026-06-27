//! Command-line tools for OZ-POS — migrations, backup, export, smoke tests.
//!
//! `oz-cli` is a binary crate (see `src/main.rs`) that exposes the
//! maintenance operations a merchant or operator runs from a terminal:
//! - `oz migrate` — apply SQL migrations
//! - `oz backup` — snapshot the local SQLite store
//! - `oz export` — write a CSV report
//! - `oz smoke` — run a self-test against the local stack
//!
//! The library side of the crate (this file) holds the shared error
//! type so both `main.rs` and the subcommand modules can use it.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::CliError;
