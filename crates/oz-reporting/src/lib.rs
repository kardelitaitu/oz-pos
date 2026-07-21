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

pub mod daily_summary;
pub mod error;
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
mod tests {
    use super::*;

    #[test]
    fn db_error_display() {
        let inner = rusqlite::Error::InvalidParameterName("x".into());
        let err = ReportingError::Db(inner);
        assert!(err.to_string().contains("database error"));
    }

    #[test]
    fn invalid_window_display() {
        let err = ReportingError::InvalidWindow("end before start".into());
        assert_eq!(err.to_string(), "invalid time window: end before start");
    }

    #[test]
    fn io_error_display() {
        let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = ReportingError::Io(inner);
        assert!(err.to_string().contains("i/o error"));
    }

    #[test]
    fn error_is_debug() {
        let err = ReportingError::InvalidWindow("x".into());
        assert!(!format!("{err:?}").is_empty());
    }
}
