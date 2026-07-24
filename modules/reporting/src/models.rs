//! Reporting domain models.

use foundation::Money;
use serde::{Deserialize, Serialize};

/// A daily sales summary report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyReport {
    /// Date of the report (YYYY-MM-DD).
    pub date: String,
    /// Total sales count.
    pub total_sales_count: i64,
    /// Total revenue in minor units.
    pub total_revenue: Money,
    /// Total tax collected in minor units.
    pub total_tax: Money,
    /// ISO-8601 creation timestamp.
    pub generated_at: String,
}
