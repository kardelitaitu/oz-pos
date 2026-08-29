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

pub mod daily_summary;
pub mod error;
pub mod margin;
pub mod menu_engineering;
#[cfg(feature = "metrics")]
pub mod metrics;

pub use daily_summary::{
    DailySummaryResult, DailySummaryRow, HourlySalesRow, TopProductRow, query_daily_summary,
    query_sales_by_hour, query_top_products,
};
pub use error::ReportingError;
#[cfg(feature = "metrics")]
pub use metrics::*;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
