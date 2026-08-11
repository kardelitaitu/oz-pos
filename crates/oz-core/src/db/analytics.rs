//! Per-staff analytics over a date range (analytics:view).
//!
//! Aggregates shift history and completed sales from the store-scoped DB,
//! grouped per staff member. Shift days use `opened_at` (the shift's start),
//! sales days use `created_at`; both are matched with SQLite `DATE()`
//! against an inclusive `[from, to]` `YYYY-MM-DD` range, mirroring the
//! reports module. All money is integer minor units (`i64`).
//!
//! The commands layer enforces the `analytics:view` permission and the
//! caller's scoped assignment (ADR #35 D5 / spec 0048); this module only
//! shapes the data. Sales without a `user_id` (pre-014 rows) are excluded.

use rusqlite::params;

use crate::error::CoreError;

use super::Store;

/// Per-staff aggregate over a date range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffAnalyticsSummary {
    /// The staff member (cashier) id.
    pub user_id: String,
    /// Number of shifts opened in the range (open or closed).
    pub shift_count: i64,
    /// Number of those shifts already closed.
    pub closed_shift_count: i64,
    /// Sum of `shifts.total_sales_minor` for the range.
    pub shift_sales_minor: i64,
    /// Number of completed sales in the range attributed to this staff.
    pub sale_count: i64,
    /// Sum of `sales.total_minor` for the completed sales in the range.
    pub sale_total_minor: i64,
}

/// Per-day series for one staff member over a date range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffAnalyticsDaily {
    /// `YYYY-MM-DD` (SQLite `DATE()`).
    pub day: String,
    /// Completed sales attributed to the staff member that day.
    pub sale_count: i64,
    /// Sum of `sales.total_minor` for those sales.
    pub sale_total_minor: i64,
    /// Shifts opened that day.
    pub shift_count: i64,
    /// Sum of `shifts.total_sales_minor` for those shifts.
    pub shift_sales_minor: i64,
}

impl Store<'_> {
    /// Per-staff shift + completed-sales aggregates over `[from, to]`.
    pub fn staff_analytics_summary(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<StaffAnalyticsSummary>, CoreError> {
        let shifts = self.analytics_shift_rows(from, to)?;
        let sales = self.analytics_sale_rows(from, to)?;

        let mut out = Vec::new();
        for s in shifts {
            out.push(StaffAnalyticsSummary {
                user_id: s.user_id.clone(),
                shift_count: s.shift_count,
                closed_shift_count: s.closed_shift_count,
                shift_sales_minor: s.shift_sales_minor,
                sale_count: 0,
                sale_total_minor: 0,
            });
        }
        for s in sales {
            match out.iter_mut().find(|o| o.user_id == s.user_id) {
                Some(row) => {
                    row.sale_count = s.sale_count;
                    row.sale_total_minor = s.sale_total_minor;
                }
                None => out.push(StaffAnalyticsSummary {
                    user_id: s.user_id.clone(),
                    shift_count: 0,
                    closed_shift_count: 0,
                    shift_sales_minor: 0,
                    sale_count: s.sale_count,
                    sale_total_minor: s.sale_total_minor,
                }),
            }
        }
        out.sort_by(|a, b| a.user_id.cmp(&b.user_id));
        Ok(out)
    }

    /// Per-day shift + completed-sales series for one staff member.
    pub fn staff_analytics_daily(
        &self,
        user_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<StaffAnalyticsDaily>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DATE(s.created_at) AS day,
                    COUNT(*) AS sale_count,
                    COALESCE(SUM(s.total_minor), 0) AS sale_total_minor
             FROM sales s
             WHERE s.status = 'completed'
               AND s.user_id IS NOT NULL
               AND s.user_id = ?1
               AND DATE(s.created_at) BETWEEN ?2 AND ?3
             GROUP BY DATE(s.created_at)",
        )?;
        let sales = stmt
            .query_map(params![user_id, from, to], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT DATE(sh.opened_at) AS day,
                    COUNT(*) AS shift_count,
                    COALESCE(SUM(sh.total_sales_minor), 0) AS shift_sales_minor
             FROM shifts sh
             WHERE sh.user_id = ?1
               AND DATE(sh.opened_at) BETWEEN ?2 AND ?3
             GROUP BY DATE(sh.opened_at)",
        )?;
        let shifts = stmt
            .query_map(params![user_id, from, to], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut out = Vec::new();
        for (day, count, total) in sales {
            out.push(StaffAnalyticsDaily {
                day: day.clone(),
                sale_count: count,
                sale_total_minor: total,
                shift_count: 0,
                shift_sales_minor: 0,
            });
        }
        for (day, count, total) in shifts {
            match out.iter_mut().find(|o| o.day == day) {
                Some(row) => {
                    row.shift_count = count;
                    row.shift_sales_minor = total;
                }
                None => out.push(StaffAnalyticsDaily {
                    day: day.clone(),
                    sale_count: 0,
                    sale_total_minor: 0,
                    shift_count: count,
                    shift_sales_minor: total,
                }),
            }
        }
        out.sort_by(|a, b| a.day.cmp(&b.day));
        Ok(out)
    }

    /// Shift rows: per-user count + closed count + sales total in range.
    fn analytics_shift_rows(&self, from: &str, to: &str) -> Result<Vec<ShiftAggregate>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT sh.user_id,
                    COUNT(*) AS shift_count,
                    SUM(CASE WHEN sh.status = 'closed' THEN 1 ELSE 0 END) AS closed_count,
                    COALESCE(SUM(sh.total_sales_minor), 0) AS shift_sales_minor
             FROM shifts sh
             WHERE DATE(sh.opened_at) BETWEEN ?1 AND ?2
             GROUP BY sh.user_id",
        )?;
        let rows = stmt
            .query_map(params![from, to], |row| {
                Ok(ShiftAggregate {
                    user_id: row.get(0)?,
                    shift_count: row.get(1)?,
                    closed_shift_count: row.get(2)?,
                    shift_sales_minor: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Sale rows: per-user count + total of completed sales in range.
    fn analytics_sale_rows(&self, from: &str, to: &str) -> Result<Vec<SaleAggregate>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT s.user_id,
                    COUNT(*) AS sale_count,
                    COALESCE(SUM(s.total_minor), 0) AS sale_total_minor
             FROM sales s
             WHERE s.status = 'completed'
               AND s.user_id IS NOT NULL
               AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY s.user_id",
        )?;
        let rows = stmt
            .query_map(params![from, to], |row| {
                Ok(SaleAggregate {
                    user_id: row.get(0)?,
                    sale_count: row.get(1)?,
                    sale_total_minor: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

/// Per-user shift aggregate (internal).
struct ShiftAggregate {
    user_id: String,
    shift_count: i64,
    closed_shift_count: i64,
    shift_sales_minor: i64,
}

/// Per-user completed-sale aggregate (internal).
struct SaleAggregate {
    user_id: String,
    sale_count: i64,
    sale_total_minor: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn seed(conn: &rusqlite::Connection) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at)
             VALUES ('role-staff', 'Staff', 'Staff', '[]', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
                ('u-alice', 'alice', 'h', 'Alice', 'role-staff', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'),
                ('u-bob',   'bob',   'h', 'Bob',   'role-staff', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, user_id, created_at) VALUES
                ('s1', 10000, 'USD', 1, 'completed', 'u-alice', '2026-07-10T09:00:00Z'),
                ('s2', 25000, 'USD', 1, 'completed', 'u-alice', '2026-07-10T14:00:00Z'),
                ('s3', 5000,  'USD', 1, 'completed', 'u-bob',   '2026-07-11T10:00:00Z'),
                -- pending + voided are excluded, and the no-cashier sale too
                ('s4', 90000, 'USD', 1, 'pending',   'u-alice', '2026-07-10T15:00:00Z'),
                ('s5', 70000, 'USD', 1, 'voided',    'u-bob',   '2026-07-11T11:00:00Z'),
                ('s6', 40000, 'USD', 1, 'completed', NULL,      '2026-07-10T16:00:00Z'),
                -- outside the range
                ('s7', 80000, 'USD', 1, 'completed', 'u-alice', '2026-08-01T09:00:00Z');
             INSERT INTO shifts (id, user_id, opened_at, closed_at, status, total_sales_minor, created_at, updated_at) VALUES
                ('sh1', 'u-alice', '2026-07-10T08:00:00Z', '2026-07-10T16:00:00Z', 'closed',  30000, '2026-07-10T08:00:00Z', '2026-07-10T16:00:00Z'),
                ('sh2', 'u-alice', '2026-07-11T08:00:00Z', NULL,                  'open',    5000,  '2026-07-11T08:00:00Z', '2026-07-11T08:00:00Z'),
                ('sh3', 'u-bob',   '2026-07-12T08:00:00Z', '2026-07-12T16:00:00Z', 'closed',  9000,  '2026-07-12T08:00:00Z', '2026-07-12T16:00:00Z'),
                -- outside the range
                ('sh4', 'u-bob',   '2026-08-01T08:00:00Z', '2026-08-01T16:00:00Z', 'closed', 1000, '2026-08-01T08:00:00Z', '2026-08-01T16:00:00Z');",
        )
        .unwrap();
    }

    #[test]
    fn summary_aggregates_shifts_and_sales_per_user() {
        let conn = migrations::fresh_db();
        seed(&conn);
        let store = Store::new(&conn);

        let rows = store
            .staff_analytics_summary("2026-07-01", "2026-07-31")
            .unwrap();
        let alice = rows.iter().find(|r| r.user_id == "u-alice").unwrap();
        assert_eq!(alice.shift_count, 2);
        assert_eq!(alice.closed_shift_count, 1);
        assert_eq!(alice.shift_sales_minor, 35000);
        assert_eq!(alice.sale_count, 2);
        assert_eq!(alice.sale_total_minor, 35000);

        let bob = rows.iter().find(|r| r.user_id == "u-bob").unwrap();
        assert_eq!(bob.shift_count, 1);
        assert_eq!(bob.closed_shift_count, 1);
        assert_eq!(bob.shift_sales_minor, 9000);
        assert_eq!(bob.sale_count, 1);
        assert_eq!(bob.sale_total_minor, 5000);
    }

    #[test]
    fn summary_excludes_pending_voided_and_no_cashier_sales() {
        let conn = migrations::fresh_db();
        seed(&conn);
        let store = Store::new(&conn);

        let rows = store
            .staff_analytics_summary("2026-07-01", "2026-07-31")
            .unwrap();
        let alice = rows.iter().find(|r| r.user_id == "u-alice").unwrap();
        // s4 (pending) and s6 (no cashier) must not count.
        assert_eq!(alice.sale_count, 2);
        assert_eq!(alice.sale_total_minor, 35000);
    }

    #[test]
    fn summary_respects_date_range() {
        let conn = migrations::fresh_db();
        seed(&conn);
        let store = Store::new(&conn);

        // Narrow to a single day: alice has 2 sales + 1 shift on 07-10.
        let rows = store
            .staff_analytics_summary("2026-07-10", "2026-07-10")
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].user_id, "u-alice");
        assert_eq!(rows[0].sale_count, 2);
        assert_eq!(rows[0].shift_count, 1);

        // An empty range yields nothing.
        assert!(
            store
                .staff_analytics_summary("2020-01-01", "2020-01-02")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn summary_zero_fills_the_missing_side() {
        let conn = migrations::fresh_db();
        // u-bob gets sales but no shifts; a third user gets only a shift.
        seed(&conn);
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at)
             VALUES ('u-cara', 'cara', 'h', 'Cara', 'role-staff', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, user_id, created_at)
             VALUES ('s8', 1234, 'USD', 1, 'completed', 'u-cara', '2026-07-20T09:00:00Z');
             INSERT INTO shifts (id, user_id, opened_at, status, total_sales_minor, created_at, updated_at)
             VALUES ('sh5', 'u-bob', '2026-07-13T08:00:00Z', 'closed', 1111, '2026-07-13T08:00:00Z', '2026-07-13T16:00:00Z');",
        )
        .unwrap();
        let store = Store::new(&conn);

        let rows = store
            .staff_analytics_summary("2026-07-01", "2026-07-31")
            .unwrap();
        // u-bob: 1 extra shift, no sales in July (his only sale is 07-11,
        // still counted above — adjust: assert zero-fill via u-cara instead).
        let cara = rows.iter().find(|r| r.user_id == "u-cara").unwrap();
        assert_eq!(cara.shift_count, 0);
        assert_eq!(cara.closed_shift_count, 0);
        assert_eq!(cara.shift_sales_minor, 0);
        assert_eq!(cara.sale_count, 1);
        assert_eq!(cara.sale_total_minor, 1234);

        let bob = rows.iter().find(|r| r.user_id == "u-bob").unwrap();
        assert_eq!(bob.shift_count, 2);
        assert_eq!(bob.sale_count, 1);
    }

    #[test]
    fn daily_series_groups_shifts_and_sales_by_day() {
        let conn = migrations::fresh_db();
        seed(&conn);
        let store = Store::new(&conn);

        let rows = store
            .staff_analytics_daily("u-alice", "2026-07-01", "2026-07-31")
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].day, "2026-07-10");
        assert_eq!(rows[0].sale_count, 2);
        assert_eq!(rows[0].sale_total_minor, 35000);
        assert_eq!(rows[0].shift_count, 1);
        assert_eq!(rows[0].shift_sales_minor, 30000);
        assert_eq!(rows[1].day, "2026-07-11");
        assert_eq!(rows[1].sale_count, 0);
        assert_eq!(rows[1].shift_count, 1);
        assert_eq!(rows[1].shift_sales_minor, 5000);
    }

    #[test]
    fn daily_series_excludes_non_completed_sales_and_no_cashier() {
        let conn = migrations::fresh_db();
        seed(&conn);
        let store = Store::new(&conn);

        let rows = store
            .staff_analytics_daily("u-bob", "2026-07-01", "2026-07-31")
            .unwrap();
        // bob: sale s3 (completed) on 07-11 and shift sh3 on 07-12; the
        // voided s5 must not count as a sale.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].day, "2026-07-11");
        assert_eq!(rows[0].sale_count, 1);
        assert_eq!(rows[0].sale_total_minor, 5000);
        assert_eq!(rows[0].shift_count, 0);
        assert_eq!(rows[1].day, "2026-07-12");
        assert_eq!(rows[1].sale_count, 0);
        assert_eq!(rows[1].shift_count, 1);
        assert_eq!(rows[1].shift_sales_minor, 9000);
    }

    #[test]
    fn daily_series_respects_date_range() {
        let conn = migrations::fresh_db();
        seed(&conn);
        let store = Store::new(&conn);

        assert!(
            store
                .staff_analytics_daily("u-alice", "2020-01-01", "2020-01-02")
                .unwrap()
                .is_empty()
        );
    }
}
