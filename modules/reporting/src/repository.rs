//! Reporting Repository — database query layer for reports.

use crate::models::DailyReport;
use foundation::{Currency, Money};
use rusqlite::Connection;

/// Database access repository for sales and inventory reports.
pub struct ReportingRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ReportingRepository<'a> {
    /// Create a new `ReportingRepository`.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Generate daily report for date.
    pub fn generate_daily_report(&self, date: &str) -> Result<DailyReport, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(total_minor), 0), COALESCE(SUM(tax_minor), 0)
             FROM sales WHERE strftime('%Y-%m-%d', created_at) = ?1 AND status = 'completed'",
        )?;

        let mut rows = stmt.query([date])?;
        let (count, rev_minor, tax_minor): (i64, i64, i64) = if let Some(row) = rows.next()? {
            (row.get(0)?, row.get(1)?, row.get(2)?)
        } else {
            (0, 0, 0)
        };

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Ok(DailyReport {
            date: date.to_string(),
            total_sales_count: count,
            total_revenue: Money {
                minor_units: rev_minor,
                currency: Currency(*b"USD"),
            },
            total_tax: Money {
                minor_units: tax_minor,
                currency: Currency(*b"USD"),
            },
            generated_at: now,
        })
    }
}
