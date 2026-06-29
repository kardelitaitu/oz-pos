//! Cash payout (safe drop) database operations.

use rusqlite::params;

use crate::error::CoreError;
use crate::CashPayout;

use super::Store;

impl Store<'_> {
    /// Record a cash payout (safe drop) against an open shift.
    ///
    /// Returns `CoreError::Validation` if the shift is not found or is
    /// already closed, or if `amount_minor ≤ 0`.
    pub fn create_cash_payout(
        &self,
        shift_id: &str,
        amount_minor: i64,
        reason: &str,
    ) -> Result<CashPayout, CoreError> {
        if amount_minor <= 0 {
            return Err(CoreError::Validation {
                field: "amount_minor",
                message: "amount_minor must be > 0".into(),
            });
        }

        // Verify the shift exists and is open.
        let shift = self.get_shift(shift_id)?
            .ok_or_else(|| CoreError::NotFound { entity: "shift", id: shift_id.to_owned() })?;
        if shift.is_closed() {
            return Err(CoreError::Validation {
                field: "status",
                message: "cannot add payout to a closed shift".into(),
            });
        }

        let payout = CashPayout::new(shift_id, amount_minor, reason);
        let now = &payout.created_at;

        self.conn.execute(
            "INSERT INTO cash_payouts (id, shift_id, amount_minor, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![payout.id, shift_id, amount_minor, reason, now],
        )?;

        Ok(payout)
    }

    /// List all cash payouts for a shift, ordered by created_at ASC.
    pub fn list_cash_payouts(&self, shift_id: &str) -> Result<Vec<CashPayout>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, shift_id, amount_minor, reason, created_at
             FROM cash_payouts WHERE shift_id = ?1
             ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map(params![shift_id], |row| {
            Ok(CashPayout {
                id: row.get("id")?,
                shift_id: row.get("shift_id")?,
                amount_minor: row.get("amount_minor")?,
                reason: row.get("reason")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get the total amount of all cash payouts for a shift (0 if none).
    pub fn get_total_payouts_for_shift(&self, shift_id: &str) -> Result<i64, CoreError> {
        let total: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(amount_minor), 0) FROM cash_payouts WHERE shift_id = ?1",
            params![shift_id],
            |row| row.get(0),
        )?;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    fn seed_user_and_shift(conn: &Connection) -> (Store<'_>, String) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-c', 'cashier', 'C', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
                ('u1', 'alice', 'h', 'Alice', 'role-c', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
        let s = Store::new(conn);
        let shift = s.open_shift("u1", None, 1000).unwrap();
        (s, shift.id)
    }

    #[test]
    fn create_payout_on_open_shift() {
        let conn = fresh();
        let (s, shift_id) = seed_user_and_shift(&conn);

        let payout = s.create_cash_payout(&shift_id, 5000, "bank drop").unwrap();
        assert_eq!(payout.shift_id, shift_id);
        assert_eq!(payout.amount_minor, 5000);
        assert_eq!(payout.reason, "bank drop");
        assert!(!payout.id.is_empty());
    }

    #[test]
    fn create_payout_closed_shift_rejected() {
        let conn = fresh();
        let (s, shift_id) = seed_user_and_shift(&conn);
        s.close_shift(&shift_id, 2000, None).unwrap();

        let err = s.create_cash_payout(&shift_id, 1000, "drop").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn create_payout_not_found_shift() {
        let conn = fresh();
        let s = Store::new(&conn);
        let err = s.create_cash_payout("nonexistent", 1000, "drop").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "shift"));
    }

    #[test]
    fn create_payout_zero_or_negative_rejected() {
        let conn = fresh();
        let (s, shift_id) = seed_user_and_shift(&conn);

        let err = s.create_cash_payout(&shift_id, 0, "zero").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "amount_minor"));

        let err = s.create_cash_payout(&shift_id, -100, "neg").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "amount_minor"));
    }

    #[test]
    fn list_payouts_for_shift() {
        let conn = fresh();
        let (s, shift_id) = seed_user_and_shift(&conn);

        s.create_cash_payout(&shift_id, 3000, "drop 1").unwrap();
        s.create_cash_payout(&shift_id, 7000, "drop 2").unwrap();

        let payouts = s.list_cash_payouts(&shift_id).unwrap();
        assert_eq!(payouts.len(), 2);
        assert_eq!(payouts[0].amount_minor, 3000);
        assert_eq!(payouts[1].amount_minor, 7000);
    }

    #[test]
    fn list_payouts_empty() {
        let conn = fresh();
        let (s, shift_id) = seed_user_and_shift(&conn);

        let payouts = s.list_cash_payouts(&shift_id).unwrap();
        assert!(payouts.is_empty());
    }

    #[test]
    fn total_payouts_for_shift() {
        let conn = fresh();
        let (s, shift_id) = seed_user_and_shift(&conn);

        assert_eq!(s.get_total_payouts_for_shift(&shift_id).unwrap(), 0);

        s.create_cash_payout(&shift_id, 3000, "drop").unwrap();
        s.create_cash_payout(&shift_id, 7000, "drop").unwrap();

        assert_eq!(s.get_total_payouts_for_shift(&shift_id).unwrap(), 10000);
    }
}
