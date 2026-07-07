//! Command-line tools for OZ-POS — migrations, backup, export, smoke tests.
//!
//! `oz-cli` exposes the maintenance operations a merchant or operator runs
//! from a terminal: `oz migrate`, `oz backup`, `oz export`, `oz smoke`.
//!
//! The library target holds all business logic so `cargo-llvm-cov` can
//! attribute coverage to the crate.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod cli;
pub mod commands;
pub mod error;

pub use cli::*;
pub use commands::run;
pub use error::CliError;
