//! Analytics and CSV export engine for OZ-POS.
//!
//! `oz-reporting` aggregates data from the local SQLite store and
//! produces daily summaries, sales-by-hour, inventory movement, and
//! CSV exports. Reports are computed on the device to keep the
//! offline-first guarantee; cloud sync of pre-aggregated reports is
//! planned as a separate service.
//!
//! This crate is a scaffold — reports are added once the cart, sale,
//! payment, and inventory tables stabilize.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::ReportingError;
