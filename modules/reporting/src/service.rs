//! Reporting Service — sales and operational reporting workflows.

use crate::models::DailyReport;
use crate::repository::ReportingRepository;
use rusqlite::Connection;

/// Service encapsulating report generation workflows.
pub struct ReportingService;

impl ReportingService {
    /// Generate daily report for date.
    pub fn generate_daily_report(
        conn: &Connection,
        date: &str,
    ) -> Result<DailyReport, anyhow::Error> {
        let repo = ReportingRepository::new(conn);
        repo.generate_daily_report(date)
    }
}
