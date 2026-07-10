//! Physical inventory / stock counting database operations.
//!
//! Provides CRUD for [`StockCount`], [`StockCountLine`], and
//! [`StockAdjustment`] records, plus the `complete_stock_count`
//! workflow that finalises a count, creates adjustment records,
//! and updates inventory quantities.

use rusqlite::params;

use crate::Store;
use crate::error::CoreError;
use crate::stock_count::{
    CountType, StockAdjustment, StockCount, StockCountLine, StockCountStatus,
};

impl Store<'_> {
    // ── Stock Count CRUD ───────────────────────────────────────────

    /// Create a new stock count record.
    pub fn create_stock_count(&self, count: &StockCount) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO stock_counts (id, count_number, status, count_type, notes, counted_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                count.id,
                count.count_number,
                count.status.as_str(),
                count.count_type.as_str(),
                count.notes,
                count.counted_by,
                count.created_at,
                count.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Fetch a single stock count by id.
    pub fn get_stock_count(&self, id: &str) -> Result<Option<StockCount>, CoreError> {
        let result = self.conn.query_row(
            "SELECT id, count_number, status, count_type, notes, counted_by, created_at, completed_at, updated_at
             FROM stock_counts WHERE id = ?1",
            params![id],
            |row| {
                let status_str: String = row.get("status")?;
                let type_str: String = row.get("count_type")?;
                Ok(StockCount {
                    id: row.get("id")?,
                    count_number: row.get("count_number")?,
                    status: StockCountStatus::from_db_str(&status_str).unwrap_or(StockCountStatus::Draft),
                    count_type: CountType::from_db_str(&type_str).unwrap_or(CountType::Full),
                    notes: row.get("notes")?,
                    counted_by: row.get("counted_by")?,
                    created_at: row.get("created_at")?,
                    completed_at: row.get("completed_at")?,
                    updated_at: row.get("updated_at")?,
                })
            },
        );
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all stock counts, newest first.
    pub fn list_stock_counts(&self) -> Result<Vec<StockCount>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, count_number, status, count_type, notes, counted_by, created_at, completed_at, updated_at
             FROM stock_counts ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get("status")?;
            let type_str: String = row.get("count_type")?;
            Ok(StockCount {
                id: row.get("id")?,
                count_number: row.get("count_number")?,
                status: StockCountStatus::from_db_str(&status_str)
                    .unwrap_or(StockCountStatus::Draft),
                count_type: CountType::from_db_str(&type_str).unwrap_or(CountType::Full),
                notes: row.get("notes")?,
                counted_by: row.get("counted_by")?,
                created_at: row.get("created_at")?,
                completed_at: row.get("completed_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Update a stock count's details (status, notes, counted_by, etc.).
    pub fn update_stock_count(&self, count: &StockCount) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE stock_counts SET status = ?1, count_type = ?2, notes = ?3, counted_by = ?4, completed_at = ?5, updated_at = ?6
             WHERE id = ?7",
            params![
                count.status.as_str(),
                count.count_type.as_str(),
                count.notes,
                count.counted_by,
                count.completed_at,
                count.updated_at,
                count.id,
            ],
        )?;
        Ok(())
    }

    // ── Count Lines ─────────────────────────────────────────────────

    /// Add a line to a stock count.
    pub fn add_count_line(&self, line: &StockCountLine) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO stock_count_lines (id, count_id, sku, product_name, expected_qty, counted_qty, difference, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                line.id, line.count_id, line.sku, line.product_name,
                line.expected_qty, line.counted_qty, line.difference, line.notes,
            ],
        )?;
        Ok(())
    }

    /// Update a count line (counted_qty, notes, difference).
    pub fn update_count_line(&self, line: &StockCountLine) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE stock_count_lines SET counted_qty = ?1, difference = ?2, notes = ?3 WHERE id = ?4",
            params![line.counted_qty, line.difference, line.notes, line.id],
        )?;
        Ok(())
    }

    /// Remove a line from a stock count.
    pub fn remove_count_line(&self, line_id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM stock_count_lines WHERE id = ?1",
            params![line_id],
        )?;
        Ok(())
    }

    /// Get a single count line by id.
    pub fn get_count_line_by_id(&self, line_id: &str) -> Result<Option<StockCountLine>, CoreError> {
        let result = self.conn.query_row(
            "SELECT id, count_id, sku, product_name, expected_qty, counted_qty, difference, notes
             FROM stock_count_lines WHERE id = ?1",
            params![line_id],
            |row| {
                Ok(StockCountLine {
                    id: row.get("id")?,
                    count_id: row.get("count_id")?,
                    sku: row.get("sku")?,
                    product_name: row.get("product_name")?,
                    expected_qty: row.get("expected_qty")?,
                    counted_qty: row.get("counted_qty")?,
                    difference: row.get("difference")?,
                    notes: row.get("notes")?,
                })
            },
        );
        match result {
            Ok(l) => Ok(Some(l)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all lines for a stock count.
    pub fn get_count_lines(&self, count_id: &str) -> Result<Vec<StockCountLine>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, count_id, sku, product_name, expected_qty, counted_qty, difference, notes
             FROM stock_count_lines WHERE count_id = ?1 ORDER BY sku",
        )?;
        let rows = stmt.query_map(params![count_id], |row| {
            Ok(StockCountLine {
                id: row.get("id")?,
                count_id: row.get("count_id")?,
                sku: row.get("sku")?,
                product_name: row.get("product_name")?,
                expected_qty: row.get("expected_qty")?,
                counted_qty: row.get("counted_qty")?,
                difference: row.get("difference")?,
                notes: row.get("notes")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    // ── Complete ─────────────────────────────────────────────────────

    /// Finalise a stock count: generate stock adjustments, update inventory
    /// quantities, and mark the count as completed.
    ///
    /// Returns the list of adjustments that were created.
    pub fn complete_stock_count(
        &self,
        count_id: &str,
        completed_by: Option<&str>,
    ) -> Result<Vec<StockAdjustment>, CoreError> {
        let count = self
            .get_stock_count(count_id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "stock_count",
                id: count_id.to_owned(),
            })?;

        if count.status != StockCountStatus::InProgress && count.status != StockCountStatus::Draft {
            return Err(CoreError::Validation {
                field: "status",
                message: format!(
                    "cannot complete count with status {}",
                    count.status.as_str()
                ),
            });
        }

        let lines = self.get_count_lines(count_id)?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut adjustments: Vec<StockAdjustment> = Vec::new();

        let tx = self.conn.unchecked_transaction()?;

        for line in &lines {
            let counted_qty = line.counted_qty.unwrap_or(line.expected_qty);
            if counted_qty == line.expected_qty {
                continue;
            }

            let product_id = match self.product_id_by_sku(&line.sku)? {
                Some(pid) => pid,
                None => continue,
            };

            let previous_qty = self.get_stock(&product_id).unwrap_or(0);
            let delta = counted_qty - previous_qty;

            // Update inventory.
            {
                let new_qty = previous_qty
                    .checked_add(delta)
                    .filter(|&v| v >= 0)
                    .unwrap_or(0);

                tx.execute(
                    "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty,
                                                             updated_at = excluded.updated_at",
                    params![product_id, new_qty, now],
                )?;
            }

            let adj_id = uuid::Uuid::now_v7().to_string();
            let adjustment = StockAdjustment {
                id: adj_id.clone(),
                count_id: Some(count_id.to_owned()),
                sku: line.sku.clone(),
                product_name: line.product_name.clone(),
                previous_qty,
                adjusted_qty: counted_qty,
                reason: format!("stock count {} ({})", count.count_number, line.notes),
                created_by: completed_by.map(|s| s.to_owned()),
                created_at: now.clone(),
            };

            tx.execute(
                "INSERT INTO stock_adjustments (id, count_id, sku, product_name, previous_qty, adjusted_qty, reason, created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    adj_id, count_id, line.sku, line.product_name,
                    previous_qty, counted_qty, adjustment.reason, completed_by, now,
                ],
            )?;

            adjustments.push(adjustment);
        }

        // Mark the count as completed.
        tx.execute(
            "UPDATE stock_counts SET status = 'completed', completed_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![now, now, count_id],
        )?;

        tx.commit()?;

        Ok(adjustments)
    }

    // ── Adjustments ─────────────────────────────────────────────────

    /// List all stock adjustments, newest first.
    pub fn list_stock_adjustments(&self) -> Result<Vec<StockAdjustment>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, count_id, sku, product_name, previous_qty, adjusted_qty, reason, created_by, created_at
             FROM stock_adjustments ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StockAdjustment {
                id: row.get("id")?,
                count_id: row.get("count_id")?,
                sku: row.get("sku")?,
                product_name: row.get("product_name")?,
                previous_qty: row.get("previous_qty")?,
                adjusted_qty: row.get("adjusted_qty")?,
                reason: row.get("reason")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Generate a human-readable count number like "CNT-20260706-001".
    pub fn next_count_number(&self) -> Result<String, CoreError> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("CNT-{today}-");
        let max_seq: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(CAST(SUBSTR(count_number, ?2) AS INTEGER)), 0) FROM stock_counts WHERE count_number LIKE ?1",
                params![format!("{prefix}%"), prefix.len() + 1],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(format!("{}{:03}", prefix, max_seq + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::stock_count::{CountType, StockCountLine, StockCountStatus};
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    fn seed_product(conn: &Connection, sku: &str, name: &str) {
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES (?1, ?2, ?3, 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![uuid::Uuid::now_v7().to_string(), sku, name],
        ).unwrap();
    }

    fn seed_inventory(conn: &Connection, product_id: &str, qty: i64) {
        conn.execute(
            "INSERT OR IGNORE INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, '2025-01-01T00:00:00.000Z')",
            params![product_id, qty],
        ).unwrap();
    }

    fn seed_user(conn: &Connection, id: &str) {
        // The actual users schema (from 021_shifts.sql et al) uses
        // `username, pin_hash, display_name, role_id` rather than the
        // `name, pin, role` columns a casual reader might guess.
        // `complete_stock_count` writes `stock_adjustments.created_by`
        // with the caller's id, so the FK target row must exist.
        conn.execute(
            "INSERT OR IGNORE INTO roles (id, name, description, permissions, created_at, updated_at)
             VALUES ('role-owner', 'Owner', 'Owner role', '[\"*\"]',
                     '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id,
                                created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, 'role-owner',
                     '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![id, id, id],
        )
        .unwrap();
    }

    #[test]
    fn create_and_get_stock_count() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let count = StockCount {
            id: id.clone(),
            count_number: "CNT-TEST-001".into(),
            status: StockCountStatus::Draft,
            count_type: CountType::Full,
            notes: "Test count".into(),
            counted_by: None,
            created_at: now.clone(),
            completed_at: None,
            updated_at: now.clone(),
        };
        store.create_stock_count(&count).unwrap();

        let fetched = store.get_stock_count(&id).unwrap().expect("should exist");
        assert_eq!(fetched.count_number, "CNT-TEST-001");
        assert_eq!(fetched.status, StockCountStatus::Draft);
        assert_eq!(fetched.count_type, CountType::Full);
    }

    #[test]
    fn list_stock_counts_ordered() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let c1 = StockCount {
            id: uuid::Uuid::now_v7().to_string(),
            count_number: "CNT-001".into(),
            status: StockCountStatus::Draft,
            count_type: CountType::Full,
            notes: "".into(),
            counted_by: None,
            created_at: "2025-01-02T00:00:00.000Z".into(),
            completed_at: None,
            updated_at: now.clone(),
        };
        let c2 = StockCount {
            id: uuid::Uuid::now_v7().to_string(),
            count_number: "CNT-002".into(),
            status: StockCountStatus::Completed,
            count_type: CountType::Cyclic,
            notes: "".into(),
            counted_by: None,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            completed_at: Some(now.clone()),
            updated_at: now.clone(),
        };

        store.create_stock_count(&c1).unwrap();
        store.create_stock_count(&c2).unwrap();

        let list = store.list_stock_counts().unwrap();
        assert_eq!(list.len(), 2);
        // Newest first.
        assert_eq!(list[0].count_number, "CNT-001");
    }

    #[test]
    fn add_and_get_count_lines() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let count_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let count = StockCount {
            id: count_id.clone(),
            count_number: "CNT-LINES".into(),
            status: StockCountStatus::Draft,
            count_type: CountType::Full,
            notes: "".into(),
            counted_by: None,
            created_at: now.clone(),
            completed_at: None,
            updated_at: now.clone(),
        };
        store.create_stock_count(&count).unwrap();

        let line = StockCountLine {
            id: uuid::Uuid::now_v7().to_string(),
            count_id: count_id.clone(),
            sku: "TEST-SKU".into(),
            product_name: "Test Product".into(),
            expected_qty: 10,
            counted_qty: None,
            difference: 0,
            notes: "".into(),
        };
        store.add_count_line(&line).unwrap();

        let lines = store.get_count_lines(&count_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].sku, "TEST-SKU");
        assert_eq!(lines[0].expected_qty, 10);
    }

    #[test]
    fn update_count_line() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let count_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let count = StockCount {
            id: count_id.clone(),
            count_number: "CNT-UPDATE".into(),
            status: StockCountStatus::InProgress,
            count_type: CountType::Spot,
            notes: "".into(),
            counted_by: None,
            created_at: now.clone(),
            completed_at: None,
            updated_at: now.clone(),
        };
        store.create_stock_count(&count).unwrap();

        let line = StockCountLine {
            id: uuid::Uuid::now_v7().to_string(),
            count_id: count_id.clone(),
            sku: "UPDATE-SKU".into(),
            product_name: "Update Product".into(),
            expected_qty: 10,
            counted_qty: None,
            difference: 0,
            notes: "".into(),
        };
        store.add_count_line(&line).unwrap();

        let updated = StockCountLine {
            id: line.id.clone(),
            count_id: count_id.clone(),
            sku: "UPDATE-SKU".into(),
            product_name: "Update Product".into(),
            expected_qty: 10,
            counted_qty: Some(8),
            difference: -2,
            notes: "Found 2 missing".into(),
        };
        store.update_count_line(&updated).unwrap();

        let lines = store.get_count_lines(&count_id).unwrap();
        assert_eq!(lines[0].counted_qty, Some(8));
        assert_eq!(lines[0].difference, -2);
    }

    #[test]
    fn complete_stock_count_creates_adjustments() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let count_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Seed user (FK target on stock_adjustments.created_by), product,
        // and inventory rows before the test exercises the workflow.
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-A", "Product A");
        let pid: String = conn
            .query_row("SELECT id FROM products WHERE sku='SKU-A'", [], |r| {
                r.get(0)
            })
            .unwrap();
        seed_inventory(&conn, &pid, 10);

        let count = StockCount {
            id: count_id.clone(),
            count_number: "CNT-COMPLETE".into(),
            status: StockCountStatus::InProgress,
            count_type: CountType::Cyclic,
            notes: "".into(),
            counted_by: None,
            created_at: now.clone(),
            completed_at: None,
            updated_at: now.clone(),
        };
        store.create_stock_count(&count).unwrap();

        let line = StockCountLine {
            id: uuid::Uuid::now_v7().to_string(),
            count_id: count_id.clone(),
            sku: "SKU-A".into(),
            product_name: "Product A".into(),
            expected_qty: 10,
            counted_qty: Some(8),
            difference: -2,
            notes: "".into(),
        };
        store.add_count_line(&line).unwrap();

        // Update count status to in_progress.
        let mut update = count.clone();
        update.status = StockCountStatus::InProgress;
        store.update_stock_count(&update).unwrap();

        let adjustments = store
            .complete_stock_count(&count_id, Some("user-1"))
            .unwrap();
        assert_eq!(adjustments.len(), 1);
        assert_eq!(adjustments[0].sku, "SKU-A");
        assert_eq!(adjustments[0].previous_qty, 10);
        assert_eq!(adjustments[0].adjusted_qty, 8);

        // Verify inventory was updated.
        let new_qty: i64 = conn
            .query_row(
                "SELECT qty FROM inventory WHERE product_id=?1",
                params![pid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(new_qty, 8);

        // Count should be completed.
        let updated_count = store.get_stock_count(&count_id).unwrap().unwrap();
        assert_eq!(updated_count.status, StockCountStatus::Completed);
    }

    #[test]
    fn next_count_number_generates_sequential() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let n1 = store.next_count_number().unwrap();
        assert!(n1.starts_with("CNT-"));

        // Create a count with that number.
        let count = StockCount {
            id: uuid::Uuid::now_v7().to_string(),
            count_number: n1.clone(),
            status: StockCountStatus::Draft,
            count_type: CountType::Full,
            notes: "".into(),
            counted_by: None,
            created_at: now.clone(),
            completed_at: None,
            updated_at: now.clone(),
        };
        store.create_stock_count(&count).unwrap();

        let n2 = store.next_count_number().unwrap();
        assert_ne!(n1, n2);
        assert!(n2 > n1);
    }

    #[test]
    fn remove_count_line() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let count_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let count = StockCount {
            id: count_id.clone(),
            count_number: "CNT-REMOVE".into(),
            status: StockCountStatus::Draft,
            count_type: CountType::Full,
            notes: "".into(),
            counted_by: None,
            created_at: now.clone(),
            completed_at: None,
            updated_at: now.clone(),
        };
        store.create_stock_count(&count).unwrap();

        let line = StockCountLine {
            id: uuid::Uuid::now_v7().to_string(),
            count_id: count_id.clone(),
            sku: "RM-SKU".into(),
            product_name: "Remove Me".into(),
            expected_qty: 5,
            counted_qty: None,
            difference: 0,
            notes: "".into(),
        };
        store.add_count_line(&line).unwrap();
        assert_eq!(store.get_count_lines(&count_id).unwrap().len(), 1);

        store.remove_count_line(&line.id).unwrap();
        assert!(store.get_count_lines(&count_id).unwrap().is_empty());
    }

    #[test]
    fn complete_already_completed_count_rejected() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let count_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let count = StockCount {
            id: count_id.clone(),
            count_number: "CNT-COMPLETED".into(),
            status: StockCountStatus::Completed,
            count_type: CountType::Full,
            notes: "".into(),
            counted_by: None,
            created_at: now.clone(),
            completed_at: Some(now.clone()),
            updated_at: now.clone(),
        };
        store.create_stock_count(&count).unwrap();

        let err = store.complete_stock_count(&count_id, None).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn get_count_line_by_id_not_found_returns_none() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let result = store.get_count_line_by_id("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_stock_count_not_found_returns_none() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let result = store.get_stock_count("no-such-count").unwrap();
        assert!(result.is_none());
    }
}
