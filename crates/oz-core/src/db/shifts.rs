//! Shift management — open/close shifts, cash reconciliation.

use rusqlite::params;

use crate::Shift;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// Open a new shift for a user.
    ///
    /// Validates that the user exists and is active, and that there is no
    /// other open shift for the same user.
    pub fn open_shift(
        &self,
        user_id: &str,
        terminal_id: Option<&str>,
        opening_balance_minor: i64,
    ) -> Result<Shift, CoreError> {
        if user_id.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "user_id",
                message: "user_id must not be empty".into(),
            });
        }
        if opening_balance_minor < 0 {
            return Err(CoreError::Validation {
                field: "opening_balance_minor",
                message: "opening_balance_minor must be ≥ 0".into(),
            });
        }

        // Verify the user exists and is active.
        let active: bool = self
            .conn
            .query_row(
                "SELECT is_active FROM users WHERE id = ?1",
                params![user_id.trim()],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v != 0)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => CoreError::Validation {
                    field: "user_id",
                    message: "user not found".into(),
                },
                _ => CoreError::Db(e),
            })?;

        if !active {
            return Err(CoreError::Validation {
                field: "user_id",
                message: "user account is deactivated".into(),
            });
        }

        // Ensure no duplicate open shift for this user.
        let open_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM shifts WHERE user_id = ?1 AND status = 'open'",
            params![user_id.trim()],
            |row| row.get(0),
        )?;
        if open_count > 0 {
            return Err(CoreError::Validation {
                field: "user_id",
                message: "user already has an open shift".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let id = uuid::Uuid::now_v7().to_string();

        self.conn.execute(
            "INSERT INTO shifts (id, user_id, terminal_id, opening_balance_minor, opened_at, created_at, updated_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'open')",
            params![id, user_id.trim(), terminal_id, opening_balance_minor, now, now, now],
        )?;

        self.get_shift(&id)?.ok_or_else(|| CoreError::NotFound {
            entity: "shift",
            id: id.to_owned(),
        })
    }

    /// Close an active shift with a counted closing balance and optional notes.
    ///
    /// Calculates `expected_cash_minor` (opening + cash sales) and
    /// `cash_difference_minor` (closing - expected). Updates all aggregated
    /// sales fields from the sales table.
    ///
    /// All reads and the final write run inside a single SQLite transaction
    /// to prevent concurrent close operations from observing inconsistent
    /// intermediate state.
    pub fn close_shift(
        &self,
        id: &str,
        closing_balance_minor: i64,
        notes: Option<&str>,
    ) -> Result<Shift, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tx = self.conn.unchecked_transaction()?;

        // Verify the shift exists and is open.
        let shift: Shift = {
            let mut stmt = tx.prepare(
                "SELECT id, user_id, terminal_id, opened_at, closed_at,
                        opening_balance_minor, closing_balance_minor,
                        expected_cash_minor, cash_difference_minor,
                        total_sales_minor, total_cash_minor, total_card_minor,
                        total_other_minor, total_voids_minor, total_refunds_minor,
                        total_payouts_minor,
                        notes, status, created_at, updated_at
                 FROM shifts WHERE id = ?1",
            )?;
            let result = stmt.query_row(params![id], Self::row_to_shift);
            match result {
                Ok(s) => s,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(CoreError::NotFound {
                        entity: "shift",
                        id: id.to_owned(),
                    });
                }
                Err(e) => return Err(CoreError::Db(e)),
            }
        };

        if shift.is_closed() {
            return Err(CoreError::Validation {
                field: "status",
                message: "shift is already closed".into(),
            });
        }

        // Calculate sales totals from the sales table for sales made during this shift.
        let (total_sales, total_cash, total_card, total_other, total_voids): (i64, i64, i64, i64, i64) = tx.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN status = 'completed' THEN total_minor ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'completed' AND payment_method = 'cash' THEN total_minor ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'completed' AND payment_method = 'card' THEN total_minor ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'completed' AND payment_method NOT IN ('cash', 'card') THEN total_minor ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'voided' THEN total_minor ELSE 0 END), 0)
             FROM sales WHERE user_id = ?1 AND created_at >= ?2 AND created_at <= ?3",
            params![shift.user_id, shift.opened_at, now],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;

        // Calculate total refunds for sales made by this user during the shift.
        let total_refunds: i64 = tx.query_row(
            "SELECT COALESCE(SUM(r.total_minor), 0)
             FROM refunds r
             JOIN sales s ON r.sale_id = s.id
             WHERE s.user_id = ?1 AND r.created_at >= ?2 AND r.created_at <= ?3",
            params![shift.user_id, shift.opened_at, now],
            |row| row.get(0),
        )?;

        // Include cash payouts (safe drops) in the expected cash calculation.
        let total_payouts: i64 = tx.query_row(
            "SELECT COALESCE(SUM(amount_minor), 0) FROM cash_payouts WHERE shift_id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        let expected_cash = shift.opening_balance_minor + total_cash - total_payouts;
        let cash_difference = closing_balance_minor - expected_cash;

        tx.execute(
            "UPDATE shifts SET
                closed_at = ?1, closing_balance_minor = ?2, expected_cash_minor = ?3,
                cash_difference_minor = ?4, total_sales_minor = ?5, total_cash_minor = ?6,
                total_card_minor = ?7, total_other_minor = ?8, total_voids_minor = ?9,
                total_refunds_minor = ?10, total_payouts_minor = ?11,
                notes = ?12, status = 'closed', updated_at = ?13
             WHERE id = ?14",
            params![
                now,
                closing_balance_minor,
                expected_cash,
                cash_difference,
                total_sales,
                total_cash,
                total_card,
                total_other,
                total_voids,
                total_refunds,
                total_payouts,
                notes.unwrap_or(""),
                now,
                id,
            ],
        )?;

        tx.commit()?;

        self.get_shift(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "shift",
            id: id.to_owned(),
        })
    }

    /// Get the currently open shift for a user, if any.
    pub fn get_active_shift(&self, user_id: &str) -> Result<Option<Shift>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, terminal_id, opened_at, closed_at,
                    opening_balance_minor, closing_balance_minor,
                    expected_cash_minor, cash_difference_minor,
                    total_sales_minor, total_cash_minor, total_card_minor,
                    total_other_minor, total_voids_minor, total_refunds_minor,
                    total_payouts_minor,
                    notes, status, created_at, updated_at
             FROM shifts WHERE user_id = ?1 AND status = 'open'
             ORDER BY opened_at DESC LIMIT 1",
        )?;
        let result = stmt.query_row(params![user_id], Self::row_to_shift);
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all shifts, ordered by opened_at DESC (most recent first).
    pub fn list_shifts(&self) -> Result<Vec<Shift>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, terminal_id, opened_at, closed_at,
                    opening_balance_minor, closing_balance_minor,
                    expected_cash_minor, cash_difference_minor,
                    total_sales_minor, total_cash_minor, total_card_minor,
                    total_other_minor, total_voids_minor, total_refunds_minor,
                    total_payouts_minor,
                    notes, status, created_at, updated_at
             FROM shifts ORDER BY opened_at DESC",
        )?;
        let rows = stmt.query_map([], Self::row_to_shift)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get a single shift by id.
    pub fn get_shift(&self, id: &str) -> Result<Option<Shift>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, terminal_id, opened_at, closed_at,
                    opening_balance_minor, closing_balance_minor,
                    expected_cash_minor, cash_difference_minor,
                    total_sales_minor, total_cash_minor, total_card_minor,
                    total_other_minor, total_voids_minor, total_refunds_minor,
                    total_payouts_minor,
                    notes, status, created_at, updated_at
             FROM shifts WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], Self::row_to_shift);
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Generate a comprehensive report for a single shift.
    ///
    /// Returns the shift's aggregated totals plus payment-method and hourly
    /// breakdowns computed from the `sales` and `payments` tables within the
    /// shift's time window.
    pub fn get_shift_report(&self, shift_id: &str) -> Result<ShiftReport, CoreError> {
        let shift = self
            .get_shift(shift_id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "shift",
                id: shift_id.to_owned(),
            })?;

        let start = &shift.opened_at;
        let now_str = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let end = shift.closed_at.as_deref().unwrap_or(&now_str);

        let user = &shift.user_id;

        // Payment method breakdown within the shift window.
        let payment_breakdown: Vec<ShiftPaymentBreakdown> = {
            let mut stmt = self.conn.prepare(
                "SELECT p.method, COUNT(*) AS cnt, COALESCE(SUM(p.amount_minor), 0) AS tot
                 FROM payments p
                 JOIN sales s ON p.sale_id = s.id
                 WHERE s.user_id = ?1 AND s.created_at >= ?2 AND s.created_at <= ?3
                   AND s.status = 'completed'
                 GROUP BY p.method
                 ORDER BY tot DESC",
            )?;
            let rows = stmt.query_map(params![user, start, end], |row| {
                Ok(ShiftPaymentBreakdown {
                    method: row.get("method")?,
                    count: row.get("cnt")?,
                    total_minor: row.get("tot")?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        // Hourly sales breakdown within the shift window (from sales table).
        let hourly_breakdown: Vec<ShiftSalesByHour> = {
            let mut stmt = self.conn.prepare(
                "SELECT CAST(strftime('%H', created_at) AS INTEGER) AS hour,
                        SUM(total_minor) AS total_minor,
                        COUNT(*) AS sale_count
                 FROM sales
                 WHERE user_id = ?1 AND created_at >= ?2 AND created_at <= ?3
                   AND status = 'completed'
                 GROUP BY hour ORDER BY hour",
            )?;
            let rows = stmt.query_map(params![user, start, end], |row| {
                Ok(ShiftSalesByHour {
                    hour: row.get("hour")?,
                    total_minor: row.get("total_minor")?,
                    sale_count: row.get("sale_count")?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        // Sale and void counts within the shift window.
        let (sale_count, void_count): (i64, i64) = self.conn.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'voided' THEN 1 ELSE 0 END), 0)
             FROM sales WHERE user_id = ?1 AND created_at >= ?2 AND created_at <= ?3",
            params![user, start, end],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Refund count from refunds table.
        let refund_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM refunds r
             JOIN sales s ON r.sale_id = s.id
             WHERE s.user_id = ?1 AND r.created_at >= ?2 AND r.created_at <= ?3",
            params![user, start, end],
            |row| row.get(0),
        )?;

        // Cash payouts for this shift.
        let cash_payouts = self.list_cash_payouts(shift_id)?;

        // ── Gross profit (HPP) ────────────────────────────────────────
        // Revenue is the completed-sale totals (same source as the hourly
        // breakdown and the shift's stored total). COGS is the sum of
        // current product cost × qty over the completed-sale lines, matching
        // the reporting layer's cost semantics (costs are not snapshotted
        // per line). Lines whose product is unknown fall back to a zero cost.
        let gross_revenue_minor: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_minor), 0) FROM sales
             WHERE user_id = ?1 AND created_at >= ?2 AND created_at <= ?3
               AND status = 'completed'",
            params![user, start, end],
            |r| r.get(0),
        )?;
        let cogs_minor: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(COALESCE(p.cost_minor, 0) * sl.qty), 0)
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             LEFT JOIN products p ON sl.sku = p.sku
             WHERE s.user_id = ?1 AND s.created_at >= ?2 AND s.created_at <= ?3
               AND s.status = 'completed'",
            params![user, start, end],
            |r| r.get(0),
        )?;
        let gross_profit_minor = gross_revenue_minor - cogs_minor;
        let gross_margin_percent = if gross_revenue_minor > 0 {
            gross_profit_minor as f64 / gross_revenue_minor as f64 * 100.0
        } else {
            0.0
        };

        Ok(ShiftReport {
            shift,
            payment_breakdown,
            hourly_breakdown,
            cash_payouts,
            sale_count,
            void_count,
            refund_count,
            cogs_minor,
            gross_profit_minor,
            gross_margin_percent,
        })
    }

    fn row_to_shift(row: &rusqlite::Row) -> rusqlite::Result<Shift> {
        Ok(Shift {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            terminal_id: row.get("terminal_id")?,
            opened_at: row.get("opened_at")?,
            closed_at: row.get("closed_at")?,
            opening_balance_minor: row.get("opening_balance_minor")?,
            closing_balance_minor: row.get("closing_balance_minor")?,
            expected_cash_minor: row.get("expected_cash_minor")?,
            cash_difference_minor: row.get("cash_difference_minor")?,
            total_sales_minor: row.get("total_sales_minor")?,
            total_cash_minor: row.get("total_cash_minor")?,
            total_card_minor: row.get("total_card_minor")?,
            total_other_minor: row.get("total_other_minor")?,
            total_voids_minor: row.get("total_voids_minor")?,
            total_refunds_minor: row.get("total_refunds_minor")?,
            total_payouts_minor: row.get("total_payouts_minor")?,
            notes: row.get("notes")?,
            status: row.get("status")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

// ── Shift Report types ────────────────────────────────────────────────

/// Comprehensive report for a single shift, including breakdowns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShiftReport {
    /// The shift record itself.
    pub shift: Shift,
    /// Payment method breakdown during this shift.
    pub payment_breakdown: Vec<ShiftPaymentBreakdown>,
    /// Hourly sales breakdown during this shift.
    pub hourly_breakdown: Vec<ShiftSalesByHour>,
    /// Cash payouts (safe drops) recorded during this shift.
    pub cash_payouts: Vec<crate::CashPayout>,
    /// Number of completed sales in this shift.
    pub sale_count: i64,
    /// Number of voided sales in this shift.
    pub void_count: i64,
    /// Number of refund transactions in this shift.
    pub refund_count: i64,
    /// Cost of goods sold in minor units (Σ current product cost × qty over
    /// completed-sale lines). 0 when no lines or costs are recorded.
    pub cogs_minor: i64,
    /// Gross profit in minor units: completed-sale revenue − COGS.
    pub gross_profit_minor: i64,
    /// Gross margin as a percentage of revenue; 0.0 when revenue is 0.
    pub gross_margin_percent: f64,
}

/// Payment method totals within a shift's time window.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShiftPaymentBreakdown {
    /// Payment method name (e.g. "cash", "card").
    pub method: String,
    /// Number of payments using this method.
    pub count: i64,
    /// Total amount in minor units.
    pub total_minor: i64,
}

/// Hourly sales aggregate within a shift's time window.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShiftSalesByHour {
    /// Hour of day (0–23).
    pub hour: i64,
    /// Total value in minor units.
    pub total_minor: i64,
    /// Number of sales in this hour.
    pub sale_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn seed_user(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-staff', 'staff', 'Staff', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
                ('user-1', 'alice', 'hash', 'Alice', 'role-staff', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    fn seed_inactive_user(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-inact', 'inactive_role', 'Inactive', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-inactive', 'inactive', 'hash', 'Inactive', 'role-inact', 0, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    #[test]
    fn open_shift_with_inactive_user_rejected() {
        let conn = fresh();
        seed_inactive_user(&conn);
        let s = store(&conn);
        let err = s.open_shift("user-inactive", None, 100).unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { field, .. } if field == "user_id"),
            "expected Validation error for inactive user, got: {err}"
        );
    }

    #[test]
    fn open_shift_duplicate_rejected() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        s.open_shift("user-1", None, 100).unwrap();
        let err = s.open_shift("user-1", None, 200).unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { field, .. } if field == "user_id"),
            "expected Validation error for duplicate shift, got: {err}"
        );
    }

    #[test]
    fn open_shift_succeeds_after_previous_closed() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 100).unwrap();
        s.close_shift(&shift.id, 150, None).unwrap();

        // Should be allowed to open a new shift after the previous one is closed.
        let shift2 = s.open_shift("user-1", None, 200).unwrap();
        assert_eq!(shift2.opening_balance_minor, 200);
        assert!(shift2.is_open());
    }

    #[test]
    fn open_shift_creates_open_shift() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 500).unwrap();
        assert_eq!(shift.user_id, "user-1");
        assert_eq!(shift.opening_balance_minor, 500);
        assert!(shift.is_open());
        assert!(shift.terminal_id.is_none());
        assert!(!shift.id.is_empty());
        assert!(shift.opened_at.contains('T'));
    }

    #[test]
    fn open_shift_with_terminal() {
        let conn = fresh();
        seed_user(&conn);
        conn.execute_batch(
            "INSERT INTO terminals (id, name, device_id, created_at, updated_at) VALUES
             ('term-1', 'Front Register', 'dev-001', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')"
        ).unwrap();
        let s = store(&conn);

        let shift = s.open_shift("user-1", Some("term-1"), 500).unwrap();
        assert_eq!(shift.terminal_id.as_deref(), Some("term-1"));
    }

    #[test]
    fn open_shift_empty_user_rejected() {
        let conn = fresh();
        let err = store(&conn).open_shift("", None, 0).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "user_id"));
    }

    #[test]
    fn open_shift_negative_balance_rejected() {
        let conn = fresh();
        seed_user(&conn);
        let err = store(&conn).open_shift("user-1", None, -1).unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { field, .. } if field == "opening_balance_minor")
        );
    }

    #[test]
    fn close_shift_sets_closed_fields() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 100).unwrap();
        let closed = s.close_shift(&shift.id, 500, Some("All good")).unwrap();

        assert!(closed.is_closed());
        assert!(closed.closed_at.is_some());
        assert_eq!(closed.closing_balance_minor, Some(500));
        assert_eq!(closed.notes, "All good");
    }

    #[test]
    fn close_shift_calculates_cash_difference() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        // Open with 100, close with 150, no sales → expected = 100, diff = 50.
        let shift = s.open_shift("user-1", None, 100).unwrap();
        let closed = s.close_shift(&shift.id, 150, None).unwrap();

        assert_eq!(closed.expected_cash_minor, Some(100)); // opening + 0 cash sales
        assert_eq!(closed.cash_difference_minor, Some(50)); // 150 - 100
    }

    #[test]
    fn close_shift_already_closed_rejected() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 100).unwrap();
        s.close_shift(&shift.id, 200, None).unwrap();

        let err = s.close_shift(&shift.id, 300, None).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn close_shift_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .close_shift("nonexistent", 100, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "shift"));
    }

    #[test]
    fn get_active_shift_returns_none_when_no_open_shift() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let active = s.get_active_shift("user-1").unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn get_active_shift_returns_open_shift() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 100).unwrap();
        let active = s.get_active_shift("user-1").unwrap().unwrap();
        assert_eq!(active.id, shift.id);
        assert!(active.is_open());
    }

    #[test]
    fn get_active_shift_returns_none_after_close() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 100).unwrap();
        s.close_shift(&shift.id, 200, None).unwrap();

        let active = s.get_active_shift("user-1").unwrap();
        assert!(active.is_none(), "no open shift after close");
    }

    #[test]
    fn list_shifts_ordered_by_opened_at_desc() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let s1 = s.open_shift("user-1", None, 100).unwrap();
        s.close_shift(&s1.id, 150, None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let s2 = s.open_shift("user-1", None, 200).unwrap();

        let shifts = s.list_shifts().unwrap();
        assert_eq!(shifts.len(), 2);
        assert_eq!(shifts[0].id, s2.id, "most recent first");
        assert_eq!(shifts[1].id, s1.id);
    }

    #[test]
    fn list_shifts_empty_db() {
        let conn = fresh();
        let shifts = store(&conn).list_shifts().unwrap();
        assert!(shifts.is_empty());
    }

    #[test]
    fn get_shift_found() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 500).unwrap();
        let loaded = s.get_shift(&shift.id).unwrap().unwrap();
        assert_eq!(loaded.id, shift.id);
        assert_eq!(loaded.opening_balance_minor, 500);
    }

    #[test]
    fn get_shift_not_found() {
        let conn = fresh();
        let shift = store(&conn).get_shift("nonexistent").unwrap();
        assert!(shift.is_none());
    }

    // ── Shift report tests ───────────────────────────────────────

    #[test]
    fn get_shift_report_not_found() {
        let conn = fresh();
        let err = store(&conn).get_shift_report("nonexistent").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "shift"));
    }

    #[test]
    fn get_shift_report_with_payouts() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 200).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Add a cash payout.
        s.create_cash_payout(&shift.id, 300, "safe drop").unwrap();

        // Insert a cash sale.
        conn.execute_batch(&format!(
            "INSERT INTO sales (id, user_id, status, total_minor, payment_method, currency, line_count, created_at, updated_at) VALUES
             ('sale-p1', 'user-1', 'completed', 500, 'cash', 'USD', 1, '{now}', '{now}');"
        )).unwrap();

        // Close the shift.
        let closed = s.close_shift(&shift.id, 500, None).unwrap();

        // Expected cash = opening(200) + cash_sales(500) - payouts(300) = 400
        assert_eq!(closed.expected_cash_minor, Some(400));
        assert_eq!(closed.total_payouts_minor, 300);
        assert_eq!(closed.cash_difference_minor, Some(100)); // 500 - 400

        let report = s.get_shift_report(&shift.id).unwrap();
        assert_eq!(report.cash_payouts.len(), 1);
        assert_eq!(report.cash_payouts[0].amount_minor, 300);
    }

    #[test]
    fn get_shift_report_with_sales() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 200).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Insert sales with different payment methods.
        conn.execute_batch(&format!(
            "INSERT INTO sales (id, user_id, status, total_minor, payment_method, currency, line_count, created_at, updated_at) VALUES
             ('sale-c1', 'user-1', 'completed', 500, 'cash', 'USD', 1, '{now}', '{now}'),
             ('sale-c2', 'user-1', 'completed', 300, 'card', 'USD', 1, '{now}', '{now}'),
             ('sale-c3', 'user-1', 'completed', 200, 'mobile_wallet', 'USD', 1, '{now}', '{now}'),
             ('sale-v1', 'user-1', 'voided', 100, 'cash', 'USD', 1, '{now}', '{now}');
             INSERT INTO payments (id, sale_id, method, amount_minor, currency, created_at) VALUES
             ('pmt-1', 'sale-c1', 'cash', 500, 'USD', '{now}'),
             ('pmt-2', 'sale-c2', 'card', 300, 'USD', '{now}'),
             ('pmt-3', 'sale-c3', 'mobile_wallet', 200, 'USD', '{now}');"
        )).unwrap();

        // Close the shift so totals are stored.
        s.close_shift(&shift.id, 800, None).unwrap();

        let report = s.get_shift_report(&shift.id).unwrap();

        // Verify the shift identity is included.
        assert_eq!(report.shift.id, shift.id);
        assert_eq!(report.shift.total_sales_minor, 1000);

        // Payment breakdown from payments table.
        assert_eq!(report.payment_breakdown.len(), 3);
        assert_eq!(report.payment_breakdown[0].method, "cash");
        assert_eq!(report.payment_breakdown[0].count, 1);
        assert_eq!(report.payment_breakdown[0].total_minor, 500);
        assert_eq!(report.payment_breakdown[1].method, "card");
        assert_eq!(report.payment_breakdown[2].method, "mobile_wallet");

        // Counts.
        assert_eq!(report.sale_count, 3, "completed sales");
        assert_eq!(report.void_count, 1, "voided sales");
        assert_eq!(report.refund_count, 0, "no refunds");

        // Hourly breakdown should have the sales grouped by hour.
        assert!(
            !report.hourly_breakdown.is_empty(),
            "should have hourly data"
        );
        let total_from_hours: i64 = report.hourly_breakdown.iter().map(|h| h.total_minor).sum();
        assert_eq!(total_from_hours, 1000, "hourly totals match sales");

        // Gross profit: no sale lines were inserted, so COGS is 0 and the
        // profit equals the completed-sale revenue (1000).
        assert_eq!(report.cogs_minor, 0);
        assert_eq!(report.gross_profit_minor, 1000);
        assert_eq!(report.gross_margin_percent, 100.0);
    }

    #[test]
    fn get_shift_report_gross_profit_from_product_costs() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 200).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Two products with known costs; a completed sale of 2× STEAK
        // (2500 − 800) and a voided sale that must NOT contribute COGS.
        conn.execute_batch(&format!(
            "INSERT INTO products (id, sku, name, price_minor, currency, cost_minor, created_at, updated_at) VALUES
             ('p-1', 'STEAK', 'Steak', 2500, 'USD', 800, '{now}', '{now}'),
             ('p-2', 'SODA',  'Soda',  300,  'USD', 100, '{now}', '{now}');
             INSERT INTO sales (id, user_id, status, total_minor, payment_method, currency, line_count, created_at, updated_at) VALUES
             ('sale-g1', 'user-1', 'completed', 5000, 'cash', 'USD', 1, '{now}', '{now}'),
             ('sale-g2', 'user-1', 'completed', 900,  'card', 'USD', 1, '{now}', '{now}'),
             ('sale-g3', 'user-1', 'voided',    500,  'cash', 'USD', 1, '{now}', '{now}');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
             ('sl-1', 'sale-g1', 'STEAK', 2, 2500, 5000, 'USD', 1),
             ('sl-2', 'sale-g2', 'SODA',  3, 300,  900,  'USD', 1),
             ('sl-3', 'sale-g3', 'STEAK', 1, 2500, 2500, 'USD', 1);"
        ))
        .unwrap();

        s.close_shift(&shift.id, 800, None).unwrap();
        let report = s.get_shift_report(&shift.id).unwrap();

        // Revenue = completed sales only: 5000 + 900 = 5900.
        // COGS = (800 × 2) + (100 × 3) = 1900 (the voided line is excluded).
        // Gross profit = 5900 − 1900 = 4000 (~67.8% margin).
        assert_eq!(report.sale_count, 2);
        assert_eq!(report.cogs_minor, 1900);
        assert_eq!(report.gross_profit_minor, 4000);
        let expected_margin = 4000.0 / 5900.0 * 100.0;
        assert!(
            (report.gross_margin_percent - expected_margin).abs() < 1e-9,
            "margin was {}",
            report.gross_margin_percent
        );
    }

    #[test]
    fn get_shift_report_open_shift() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        // Open a shift but don't close it.
        let shift = s.open_shift("user-1", None, 100).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        conn.execute_batch(&format!(
            "INSERT INTO sales (id, user_id, status, total_minor, payment_method, currency, line_count, created_at, updated_at) VALUES
             ('sale-1', 'user-1', 'completed', 250, 'cash', 'USD', 1, '{now}', '{now}');"
        )).unwrap();

        // Report should still work for an open shift (uses current time as end).
        let report = s.get_shift_report(&shift.id).unwrap();
        assert_eq!(report.shift.status, "open");
        assert!(report.shift.closed_at.is_none());
        assert_eq!(report.sale_count, 1);
        assert_eq!(
            report.payment_breakdown.len(),
            0,
            "no payments table entries"
        );
    }

    #[test]
    fn close_shift_atomic_within_transaction() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 200).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Insert a cash sale and a payout.
        conn.execute_batch(&format!(
            "INSERT INTO sales (id, user_id, status, total_minor, payment_method, currency, line_count, created_at, updated_at) VALUES
             ('sale-tx', 'user-1', 'completed', 1000, 'cash', 'USD', 1, '{now}', '{now}');"
        )).unwrap();
        s.create_cash_payout(&shift.id, 300, "safe drop").unwrap();

        // Close the shift — should see both the sale and the payout.
        let closed = s.close_shift(&shift.id, 1000, None).unwrap();

        // expected_cash = opening(200) + cash_sales(1000) - payouts(300) = 900
        assert_eq!(closed.expected_cash_minor, Some(900));
        assert_eq!(closed.cash_difference_minor, Some(100)); // 1000 - 900
        assert_eq!(closed.total_cash_minor, 1000);
        assert_eq!(closed.total_payouts_minor, 300);
    }

    #[test]
    fn get_shift_report_empty_shift() {
        let conn = fresh();
        seed_user(&conn);
        let s = store(&conn);

        let shift = s.open_shift("user-1", None, 100).unwrap();

        let report = s.get_shift_report(&shift.id).unwrap();
        assert_eq!(report.sale_count, 0);
        assert_eq!(report.void_count, 0);
        assert_eq!(report.refund_count, 0);
        assert!(report.payment_breakdown.is_empty());
        assert!(report.hourly_breakdown.is_empty());
    }
}
