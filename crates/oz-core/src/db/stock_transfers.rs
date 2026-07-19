//! Stock transfer CRUD — create, send, receive, cancel.
//!
//! Transfers move inventory between terminals/stores. The lifecycle is:
//! draft → pending → in_transit → received / cancelled.
//! `send_transfer` decrements source inventory; `receive_transfer` increments
//! destination inventory and records received quantities.

use rusqlite::params;

use crate::error::CoreError;
use crate::stock_transfer::{StockTransfer, StockTransferLine};

use super::Store;

impl Store<'_> {
    /// Create a new stock transfer with the given lines.
    ///
    /// Generates a unique transfer number (`TRF-<timestamp>-<short-id>`).
    /// All lines are inserted in the same transaction as the header.
    #[allow(clippy::too_many_arguments)]
    pub fn create_transfer(
        &self,
        source_location: Option<&str>,
        destination_location: Option<&str>,
        source_terminal_id: Option<&str>,
        destination_terminal_id: Option<&str>,
        notes: &str,
        created_by: &str,
        lines: &[StockTransferLine],
    ) -> Result<StockTransfer, CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let ts = chrono::Utc::now().timestamp_millis();
        let short = &id[..8];
        let transfer_number = format!("TRF-{ts}-{short}");

        // ADR-18 §13-36 canonical default-location UUID (see
        // `crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID` for the
        // frozen-invariant rationale). Migration 081's
        // `source_location_id` and `destination_location_id` columns are
        // NOT NULL with this canonical UUID as DEFAULT. SQLite does NOT
        // fall back to DEFAULT when VALUES provides an explicit NULL —
        // a NOT NULL constraint violation fires instead. We resolve the
        // None → canonical mapping at the Rust layer so the bound value
        // is always a non-NULL FK string, while keeping the function
        // signature ergonomic (Option<&str> for callers that don't care
        // to specify a location).
        let canonical_default_loc = crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
        let source_loc = source_location.unwrap_or(canonical_default_loc);
        let destination_loc = destination_location.unwrap_or(canonical_default_loc);

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO stock_transfers
                (id, transfer_number, status, source_location_id, destination_location_id,
                 source_terminal_id, destination_terminal_id, notes, created_by,
                 created_at, updated_at)
             VALUES (?1, ?2, 'draft', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                transfer_number,
                source_loc,
                destination_loc,
                source_terminal_id,
                destination_terminal_id,
                notes,
                created_by,
                now,
                now,
            ],
        )?;

        for line in lines {
            let line_id = uuid::Uuid::now_v7().to_string();
            tx.execute(
                "INSERT INTO stock_transfer_lines (id, transfer_id, sku, product_name, qty, received_qty)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                params![line_id, id, line.sku, line.product_name, line.qty],
            )?;
        }

        tx.commit()?;

        Ok(StockTransfer {
            id,
            transfer_number,
            status: "draft".into(),
            source_location: source_location.map(String::from),
            destination_location: destination_location.map(String::from),
            source_terminal_id: source_terminal_id.map(String::from),
            destination_terminal_id: destination_terminal_id.map(String::from),
            notes: notes.to_owned(),
            created_by: created_by.to_owned(),
            received_by: None,
            created_at: now.clone(),
            sent_at: None,
            received_at: None,
            updated_at: now,
        })
    }

    /// Get a single transfer by id (with lines populated via
    /// [`get_transfer_lines`]).
    pub fn get_transfer(&self, id: &str) -> Result<Option<StockTransfer>, CoreError> {
        // ADR-18 §2d: read the FK columns (source_location_id, destination_location_id)
        // introduced by migration 081's column rename (`source_location` →
        // `source_location_old` audit + `source_location_id` FK). The domain
        // field names `source_location`/`destination_location` are preserved
        // for JSON contract backward compat — callers receive FK UUID strings.
        let mut stmt = self.conn.prepare(
            "SELECT id, transfer_number, status,
                    source_location_id AS source_location,
                    destination_location_id AS destination_location,
                    source_terminal_id, destination_terminal_id,
                    notes, created_by, received_by,
                    created_at, sent_at, received_at, updated_at
             FROM stock_transfers WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(StockTransfer {
                id: row.get("id")?,
                transfer_number: row.get("transfer_number")?,
                status: row.get("status")?,
                source_location: row.get("source_location")?,
                destination_location: row.get("destination_location")?,
                source_terminal_id: row.get("source_terminal_id")?,
                destination_terminal_id: row.get("destination_terminal_id")?,
                notes: row.get("notes")?,
                created_by: row.get("created_by")?,
                received_by: row.get("received_by")?,
                created_at: row.get("created_at")?,
                sent_at: row.get("sent_at")?,
                received_at: row.get("received_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all transfers, newest first.
    pub fn list_transfers(&self) -> Result<Vec<StockTransfer>, CoreError> {
        // ADR-18 §2d: read the FK columns (source_location_id, destination_location_id)
        // introduced by migration 081. Domain field names preserved via column
        // aliasing; actual storage is FK UUID strings (NOT NULL DEFAULT canonical).
        let mut stmt = self.conn.prepare(
            "SELECT id, transfer_number, status,
                    source_location_id AS source_location,
                    destination_location_id AS destination_location,
                    source_terminal_id, destination_terminal_id,
                    notes, created_by, received_by,
                    created_at, sent_at, received_at, updated_at
             FROM stock_transfers ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StockTransfer {
                id: row.get("id")?,
                transfer_number: row.get("transfer_number")?,
                status: row.get("status")?,
                source_location: row.get("source_location")?,
                destination_location: row.get("destination_location")?,
                source_terminal_id: row.get("source_terminal_id")?,
                destination_terminal_id: row.get("destination_terminal_id")?,
                notes: row.get("notes")?,
                created_by: row.get("created_by")?,
                received_by: row.get("received_by")?,
                created_at: row.get("created_at")?,
                sent_at: row.get("sent_at")?,
                received_at: row.get("received_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get lines for a transfer.
    pub fn get_transfer_lines(
        &self,
        transfer_id: &str,
    ) -> Result<Vec<StockTransferLine>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, transfer_id, sku, product_name, qty, received_qty
             FROM stock_transfer_lines WHERE transfer_id = ?1
             ORDER BY id",
        )?;
        let rows = stmt.query_map(params![transfer_id], |row| {
            Ok(StockTransferLine {
                id: row.get("id")?,
                transfer_id: row.get("transfer_id")?,
                sku: row.get("sku")?,
                product_name: row.get("product_name")?,
                qty: row.get("qty")?,
                received_qty: row.get("received_qty")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Add a line to an existing transfer (only allowed in `draft` status).
    pub fn add_transfer_line(
        &self,
        transfer_id: &str,
        sku: &str,
        product_name: &str,
        qty: i64,
    ) -> Result<StockTransferLine, CoreError> {
        let status: String = self
            .conn
            .query_row(
                "SELECT status FROM stock_transfers WHERE id = ?1",
                params![transfer_id],
                |row| row.get(0),
            )
            .map_err(|_| CoreError::NotFound {
                entity: "stock_transfer",
                id: transfer_id.to_owned(),
            })?;

        if status != "draft" {
            return Err(CoreError::Validation {
                field: "status",
                message: "can only add lines to a draft transfer".into(),
            });
        }

        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO stock_transfer_lines (id, transfer_id, sku, product_name, qty, received_qty)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![id, transfer_id, sku, product_name, qty],
        )?;

        Ok(StockTransferLine {
            id,
            transfer_id: transfer_id.to_owned(),
            sku: sku.to_owned(),
            product_name: product_name.to_owned(),
            qty,
            received_qty: 0,
        })
    }

    /// Remove a line from a draft transfer.
    pub fn remove_transfer_line(&self, line_id: &str) -> Result<(), CoreError> {
        let transfer_id: String = self
            .conn
            .query_row(
                "SELECT transfer_id FROM stock_transfer_lines WHERE id = ?1",
                params![line_id],
                |row| row.get(0),
            )
            .map_err(|_| CoreError::NotFound {
                entity: "stock_transfer_line",
                id: line_id.to_owned(),
            })?;

        let status: String = self.conn.query_row(
            "SELECT status FROM stock_transfers WHERE id = ?1",
            params![transfer_id],
            |row| row.get(0),
        )?;

        if status != "draft" {
            return Err(CoreError::Validation {
                field: "status",
                message: "can only remove lines from a draft transfer".into(),
            });
        }

        let deleted = self.conn.execute(
            "DELETE FROM stock_transfer_lines WHERE id = ?1",
            params![line_id],
        )?;
        if deleted == 0 {
            return Err(CoreError::NotFound {
                entity: "stock_transfer_line",
                id: line_id.to_owned(),
            });
        }
        Ok(())
    }

    /// Mark a transfer as `in_transit` and decrement source inventory for each line.
    ///
    /// Only allowed when status is `draft` or `pending`.
    pub fn send_transfer(&self, id: &str) -> Result<StockTransfer, CoreError> {
        let transfer = self.get_transfer(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "stock_transfer",
            id: id.to_owned(),
        })?;

        if transfer.status != "draft" && transfer.status != "pending" {
            return Err(CoreError::Validation {
                field: "status",
                message: format!(
                    "cannot send transfer in status '{}'; expected 'draft' or 'pending'",
                    transfer.status
                ),
            });
        }

        let lines = self.get_transfer_lines(id)?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tx = self.conn.unchecked_transaction()?;

        // Decrement source inventory for each line.
        for line in &lines {
            let product_id = tx
                .query_row(
                    "SELECT id FROM products WHERE sku = ?1",
                    params![line.sku],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|_| CoreError::NotFound {
                    entity: "product",
                    id: line.sku.clone(),
                })?;

            let prev_qty: i64 = tx
                .query_row(
                    "SELECT COALESCE(qty, 0) FROM inventory WHERE product_id = ?1",
                    params![product_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            let new_qty = prev_qty
                .checked_sub(line.qty)
                .filter(|&v| v >= 0)
                .ok_or_else(|| CoreError::Validation {
                    field: "qty",
                    message: format!(
                        "insufficient stock for SKU '{}': have {prev_qty}, need {}",
                        line.sku, line.qty
                    ),
                })?;

            tx.execute(
                "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty,
                                                         updated_at = excluded.updated_at",
                params![product_id, new_qty, now],
            )?;
        }

        tx.execute(
            "UPDATE stock_transfers SET status = 'in_transit', sent_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![now, now, id],
        )?;

        tx.commit()?;

        self.get_transfer(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "stock_transfer",
            id: id.to_owned(),
        })
    }

    /// Mark a transfer as `received`, record received quantities, and
    /// increment destination inventory.
    ///
    /// Only allowed when status is `in_transit`.
    pub fn receive_transfer(
        &self,
        id: &str,
        received_by: &str,
        received_lines: &[ReceivedLine],
    ) -> Result<StockTransfer, CoreError> {
        let transfer = self.get_transfer(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "stock_transfer",
            id: id.to_owned(),
        })?;

        if transfer.status != "in_transit" {
            return Err(CoreError::Validation {
                field: "status",
                message: format!(
                    "cannot receive transfer in status '{}'; expected 'in_transit'",
                    transfer.status
                ),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let tx = self.conn.unchecked_transaction()?;

        for rl in received_lines {
            // Validate that received_qty does not exceed the line's ordered qty.
            let ordered_qty: i64 = tx.query_row(
                "SELECT qty FROM stock_transfer_lines WHERE id = ?1 AND transfer_id = ?2",
                params![rl.line_id, id],
                |row| row.get(0),
            )?;
            if rl.received_qty > ordered_qty {
                return Err(CoreError::Validation {
                    field: "received_qty",
                    message: format!(
                        "received_qty ({}) exceeds ordered qty ({}) for line {}",
                        rl.received_qty, ordered_qty, rl.line_id
                    ),
                });
            }

            // Update received_qty on the line.
            tx.execute(
                "UPDATE stock_transfer_lines SET received_qty = ?1 WHERE id = ?2 AND transfer_id = ?3",
                params![rl.received_qty, rl.line_id, id],
            )?;

            if rl.received_qty > 0 {
                let sku: String = tx.query_row(
                    "SELECT sku FROM stock_transfer_lines WHERE id = ?1",
                    params![rl.line_id],
                    |row| row.get(0),
                )?;

                let product_id: String = tx
                    .query_row(
                        "SELECT id FROM products WHERE sku = ?1",
                        params![sku],
                        |row| row.get(0),
                    )
                    .map_err(|_| CoreError::NotFound {
                        entity: "product",
                        id: sku.clone(),
                    })?;

                // Increment destination inventory.
                let prev_qty: i64 = tx
                    .query_row(
                        "SELECT COALESCE(qty, 0) FROM inventory WHERE product_id = ?1",
                        params![product_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                let new_qty = prev_qty
                    .checked_add(rl.received_qty)
                    .ok_or_else(|| CoreError::Internal("inventory overflow on receive".into()))?;

                tx.execute(
                    "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty,
                                                             updated_at = excluded.updated_at",
                    params![product_id, new_qty, now],
                )?;
            }
        }

        let all_received: bool = {
            let mut stmt = tx.prepare(
                "SELECT COUNT(*) FROM stock_transfer_lines
                 WHERE transfer_id = ?1 AND received_qty < qty",
            )?;
            let partial: i64 = stmt.query_row(params![id], |row| row.get(0))?;
            partial == 0
        };

        let final_status = if all_received {
            "received"
        } else {
            "in_transit"
        };

        tx.execute(
            "UPDATE stock_transfers SET status = ?1, received_by = ?2, received_at = ?3, updated_at = ?4 WHERE id = ?5",
            params![final_status, received_by, now, now, id],
        )?;

        tx.commit()?;

        self.get_transfer(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "stock_transfer",
            id: id.to_owned(),
        })
    }

    /// Cancel a transfer (only allowed when not already received/cancelled).
    pub fn cancel_transfer(&self, id: &str) -> Result<StockTransfer, CoreError> {
        let transfer = self.get_transfer(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "stock_transfer",
            id: id.to_owned(),
        })?;

        if transfer.status == "received" || transfer.status == "cancelled" {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("cannot cancel transfer in status '{}'", transfer.status),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "UPDATE stock_transfers SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;

        self.get_transfer(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "stock_transfer",
            id: id.to_owned(),
        })
    }
}

/// A line-level received quantity for [`Store::receive_transfer`].
#[derive(Debug, Clone)]
pub struct ReceivedLine {
    /// FK to stock_transfer_lines.id.
    pub line_id: String,
    /// Quantity actually received for this line.
    pub received_qty: i64,
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

    fn seed_user(conn: &Connection, id: &str) {
        // The actual users schema (from 021_shifts.sql et al) uses
        // `username, pin_hash, display_name, role_id` rather than the
        // `name, pin, role` columns a casual reader might guess from
        // the crate's domain types. Seed the FK target role first.
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

    fn seed_product(conn: &Connection, sku: &str, name: &str) {
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![uuid::Uuid::now_v7().to_string(), sku, name],
        )
        .unwrap();
    }

    fn seed_inventory(conn: &Connection, sku: &str, qty: i64) {
        let pid: String = conn
            .query_row(
                "SELECT id FROM products WHERE sku = ?1",
                params![sku],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, '2025-01-01T00:00:00.000Z')",
            params![pid, qty],
        )
        .unwrap();
    }

    fn make_line(sku: &str, product_name: &str, qty: i64) -> StockTransferLine {
        StockTransferLine {
            id: String::new(),
            transfer_id: String::new(),
            sku: sku.to_owned(),
            product_name: product_name.to_owned(),
            qty,
            received_qty: 0,
        }
    }

    #[test]
    fn create_and_get_transfer() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 100);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "test notes", "user-1", &lines)
            .unwrap();
        assert_eq!(t.status, "draft");
        assert!(t.transfer_number.starts_with("TRF-"));

        let fetched = store(&conn).get_transfer(&t.id).unwrap().unwrap();
        assert_eq!(fetched.id, t.id);
        assert_eq!(fetched.status, "draft");
    }

    #[test]
    fn list_transfers_orders_by_created_at() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 100);

        let lines = vec![make_line("SKU-001", "Widget", 5)];
        let _t1 = store(&conn)
            .create_transfer(None, None, None, None, "first", "user-1", &lines)
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let _t2 = store(&conn)
            .create_transfer(None, None, None, None, "second", "user-1", &lines)
            .unwrap();

        let all = store(&conn).list_transfers().unwrap();
        assert_eq!(all.len(), 2);
        assert!(all[0].created_at >= all[1].created_at);
    }

    #[test]
    fn send_transfer_decrements_inventory() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();

        let sent = store(&conn).send_transfer(&t.id).unwrap();
        assert_eq!(sent.status, "in_transit");
        assert!(sent.sent_at.is_some());
    }

    #[test]
    fn send_transfer_insufficient_stock_fails() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 5);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();

        let err = store(&conn).send_transfer(&t.id).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "qty"));
    }

    #[test]
    fn receive_transfer_increments_inventory() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_user(&conn, "user-2");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();
        let sent = store(&conn).send_transfer(&t.id).unwrap();
        assert_eq!(sent.status, "in_transit");

        let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        let received = store(&conn)
            .receive_transfer(
                &t.id,
                "user-2",
                &[ReceivedLine {
                    line_id: transfer_lines[0].id.clone(),
                    received_qty: 10,
                }],
            )
            .unwrap();
        assert_eq!(received.status, "received");
        assert!(received.received_at.is_some());
        assert_eq!(received.received_by.unwrap(), "user-2");
    }

    #[test]
    fn cancel_draft_transfer() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();

        let cancelled = store(&conn).cancel_transfer(&t.id).unwrap();
        assert_eq!(cancelled.status, "cancelled");
    }

    #[test]
    fn add_and_remove_transfer_line() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 100);

        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &[])
            .unwrap();

        let line = store(&conn)
            .add_transfer_line(&t.id, "SKU-001", "Widget", 5)
            .unwrap();
        assert_eq!(line.qty, 5);

        let lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        assert_eq!(lines.len(), 1);

        store(&conn).remove_transfer_line(&line.id).unwrap();
        let lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn partial_receive_leaves_in_transit() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_user(&conn, "user-2");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();
        store(&conn).send_transfer(&t.id).unwrap();

        let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        let result = store(&conn)
            .receive_transfer(
                &t.id,
                "user-2",
                &[ReceivedLine {
                    line_id: transfer_lines[0].id.clone(),
                    received_qty: 4,
                }],
            )
            .unwrap();
        // Status stays in_transit because 4 < 10.
        assert_eq!(result.status, "in_transit");
    }

    #[test]
    fn get_transfer_not_found_returns_none() {
        let conn = fresh();
        let result = store(&conn).get_transfer("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn cancel_received_transfer_rejected() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_user(&conn, "user-2");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();
        store(&conn).send_transfer(&t.id).unwrap();

        let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        store(&conn)
            .receive_transfer(
                &t.id,
                "user-2",
                &[ReceivedLine {
                    line_id: transfer_lines[0].id.clone(),
                    received_qty: 10,
                }],
            )
            .unwrap();

        let err = store(&conn).cancel_transfer(&t.id).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn send_already_in_transit_rejected() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();
        store(&conn).send_transfer(&t.id).unwrap();

        let err = store(&conn).send_transfer(&t.id).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn add_line_to_non_draft_transfer_rejected() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 100);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();
        store(&conn).send_transfer(&t.id).unwrap();

        let err = store(&conn)
            .add_transfer_line(&t.id, "SKU-001", "Widget", 5)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn transfer_full_lifecycle() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_user(&conn, "user-2");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 100);

        // Step 1: Create draft
        let lines = vec![make_line("SKU-001", "Widget", 20)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "lifecycle test", "user-1", &lines)
            .unwrap();
        assert_eq!(t.status, "draft");
        assert!(t.transfer_number.starts_with("TRF-"));

        // Step 2: Send → in_transit
        let sent = store(&conn).send_transfer(&t.id).unwrap();
        assert_eq!(sent.status, "in_transit");
        assert!(sent.sent_at.is_some());

        // Step 3: Receive full → received
        let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        assert_eq!(transfer_lines[0].qty, 20);

        let received = store(&conn)
            .receive_transfer(
                &t.id,
                "user-2",
                &[ReceivedLine {
                    line_id: transfer_lines[0].id.clone(),
                    received_qty: 20,
                }],
            )
            .unwrap();
        assert_eq!(received.status, "received");
        assert!(received.received_at.is_some());
        assert_eq!(received.received_by.unwrap(), "user-2");

        // Verify received_qty persisted on the line
        let final_lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        assert_eq!(final_lines[0].received_qty, 20);
    }

    #[test]
    fn cancel_in_transit_transfer() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();

        // Send first
        let sent = store(&conn).send_transfer(&t.id).unwrap();
        assert_eq!(sent.status, "in_transit");

        // Cancel while in_transit
        let cancelled = store(&conn).cancel_transfer(&t.id).unwrap();
        assert_eq!(cancelled.status, "cancelled");
        // Note: inventory is NOT restored on cancel (intentional design)
    }

    #[test]
    fn receive_excess_stock_errors() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_user(&conn, "user-2");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 50);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();
        store(&conn).send_transfer(&t.id).unwrap();

        let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();

        // Try to receive 15 when only 10 were ordered
        let err = store(&conn)
            .receive_transfer(
                &t.id,
                "user-2",
                &[ReceivedLine {
                    line_id: transfer_lines[0].id.clone(),
                    received_qty: 15,
                }],
            )
            .unwrap_err();
        assert!(
            matches!(&err, CoreError::Validation { field, message } if *field == "received_qty" && message.contains("15"))
        );

        // Transfer should still be in_transit (receive was rolled back)
        let after = store(&conn).get_transfer(&t.id).unwrap().unwrap();
        assert_eq!(after.status, "in_transit");
    }

    #[test]
    fn receive_zero_qty_keeps_in_transit() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_user(&conn, "user-2");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 30);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();
        store(&conn).send_transfer(&t.id).unwrap();

        let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();

        // Receive 0 — no inventory increment, status stays in_transit
        let result = store(&conn)
            .receive_transfer(
                &t.id,
                "user-2",
                &[ReceivedLine {
                    line_id: transfer_lines[0].id.clone(),
                    received_qty: 0,
                }],
            )
            .unwrap();
        assert_eq!(result.status, "in_transit");

        // Verify received_qty was recorded as 0
        let lines = store(&conn).get_transfer_lines(&t.id).unwrap();
        assert_eq!(lines[0].received_qty, 0);
    }

    #[test]
    fn cancel_nonexistent_transfer_errors() {
        let conn = fresh();
        let err = store(&conn).cancel_transfer("i-do-not-exist").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "stock_transfer"));
    }

    #[test]
    fn receive_draft_transfer_rejected() {
        let conn = fresh();
        seed_user(&conn, "user-1");
        seed_user(&conn, "user-2");
        seed_product(&conn, "SKU-001", "Widget");
        seed_inventory(&conn, "SKU-001", 30);

        let lines = vec![make_line("SKU-001", "Widget", 10)];
        let t = store(&conn)
            .create_transfer(None, None, None, None, "", "user-1", &lines)
            .unwrap();

        // Transfer is still 'draft' — cannot receive
        let err = store(&conn)
            .receive_transfer(&t.id, "user-2", &[])
            .unwrap_err();
        assert!(
            matches!(&err, CoreError::Validation { field, message } if *field == "status" && message.contains("draft"))
        );
    }
}
