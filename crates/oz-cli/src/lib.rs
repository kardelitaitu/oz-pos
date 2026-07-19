/*
last audited 19-07-26 by RSA-Agent
crate: oz-cli | status: SAFE | lint: CLEAN
findings: #![deny(unsafe_code)] at crate root. Pure CLI orchestration (migrations, backup, export, smoke)
  using rusqlite and serde_json. No FFI or unsafe blocks. 77 unit tests pass.
next: None | perf: CLI runs are ephemeral; no long-lived allocations.
*/
#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Command-line tools for OZ-POS — migrations, backup, export, smoke tests.
//!
//! `oz-cli` exposes the maintenance operations a merchant or operator runs
//! from a terminal: `oz migrate`, `oz backup`, `oz export`, `oz smoke`.
//!
//! The library target holds all business logic so `cargo-llvm-cov` can
//! attribute coverage to the crate.

pub mod cli;
pub mod commands;
pub mod error;

pub use cli::*;
pub use commands::run;
pub use error::CliError;
