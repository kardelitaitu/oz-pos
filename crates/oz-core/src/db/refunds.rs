//! Refund CRUD — create, list, and query refunds.
//!
//! ADR-19 §5.3: On refund, stock is credited back to the original deduction
//! source locations in FIFO order (oldest deduction first for full refunds;
//! reverse-chronological for partial refunds). The `deduction_locations` JSON
//! column on the `sales` table records the per-line, per-location breakdown.

use rusqlite::params;

use crate::error::CoreError;
use crate::money::Currency;
use crate::{Money, Refund, RefundLine};

use super::Store;

impl Store<'_> {
    /// Process a refund — persist refund + lines inside a transaction
    /// and restore stock to the original deduction sources.
    ///
    /// **Stock restoration (ADR-19 §5.3):**
    /// - Reads the sale's `deduction_locations` JSON column.
    /// - For each refund line, matches it to a sale line and credits stock
    ///   back to the original deduction locations.
    /// - Full refund of a line: iterates deductions forward (FIFO oldest first).
    /// - Partial refund of a line (qty < original line qty): iterates
    ///   deductions in REVERSE, crediting the most recently deducted location
    ///   first, stopping when the refund qty is satisfied.
    pub fn create_refund(&self, refund: &Refund) -> Result<(), CoreError> {
        let cur_str =
            std::str::from_utf8(&refund.total.currency.0).expect("currency bytes are valid UTF-8");

        let tx = self.conn.unchecked_transaction()?;

        // ── 1. Persist refund + lines ──────────────────────────────
        tx.execute(
            "INSERT INTO refunds (id, sale_id, total_minor, currency, reason, note, processed_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![refund.id, refund.sale_id, refund.total.minor_units, cur_str, refund.reason, refund.note, refund.processed_by, refund.created_at],
        )?;

        for line in &refund.lines {
            let line_cur = std::str::from_utf8(&line.unit_price.currency.0)
                .expect("currency bytes are valid UTF-8");
            tx.execute(
                "INSERT INTO refund_lines (id, refund_id, sale_line_id, sku, qty, unit_minor, line_minor, currency, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![line.id, line.refund_id, line.sale_line_id, line.sku, line.qty,
                        line.unit_price.minor_units, line.line_total.minor_units, line_cur, line.created_at],
            )?;
        }

        // ── 2. Read deduction_locations from the sale ──────────────
        let deduction_locations_json: Option<String> = match tx.query_row(
            "SELECT deduction_locations FROM sales WHERE id = ?1",
            params![refund.sale_id],
            |row| row.get(0),
        ) {
            Ok(j) => j,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(CoreError::NotFound {
                    entity: "sale",
                    id: refund.sale_id.clone(),
                });
            }
            Err(e) => return Err(CoreError::Db(e)),
        };

        // If deduction_locations is NULL (pre-093 legacy sale), fall
        // back to crediting the canonical default location.
        if deduction_locations_json.as_deref().unwrap_or("").is_empty()
            || deduction_locations_json.as_deref() == Some("null")
        {
            self.credit_refund_to_default_location(&tx, refund)?;
        } else {
            self.credit_refund_from_deduction_locations(
                &tx,
                refund,
                deduction_locations_json.as_deref().unwrap(),
            )?;
        }

        // ── 3. Write audit log inside the same transaction ─────────
        tx.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                uuid::Uuid::now_v7().to_string(),
                refund.processed_by,
                "sale.refund",
                "sale",
                refund.sale_id,
                serde_json::json!({
                    "refund_id": refund.id,
                    "reason": refund.reason,
                    "total_minor": refund.total.minor_units,
                    "currency": cur_str,
                    "line_count": refund.lines.len(),
                }).to_string(),
                "success",
                refund.created_at,
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Credit stock back to original deduction sources per ADR-19 §5.3 FIFO.
    ///
    /// For each refund line:
    /// - Matches `sale_line_id` in the `deduction_locations` JSON.
    /// - If the refund qty equals or exceeds the original line qty (full
    ///   refund), iterates deductions forward — oldest deduction first.
    /// - If the refund qty is less than the original line qty (partial
    ///   refund), iterates deductions in REVERSE — most recent deduction
    ///   first — crediting `min(entry.qty, remaining)` until satisfied.
    fn credit_refund_from_deduction_locations(
        &self,
        tx: &rusqlite::Transaction<'_>,
        refund: &Refund,
        deduction_locations_json: &str,
    ) -> Result<(), CoreError> {
        let v: serde_json::Value =
            serde_json::from_str(deduction_locations_json).map_err(|e| CoreError::Validation {
                field: "deduction_locations",
                message: e.to_string(),
            })?;

        let lines_array = v["lines"].as_array().ok_or_else(|| CoreError::Validation {
            field: "deduction_locations.lines",
            message: "expected an array".into(),
        })?;

        for refund_line in &refund.lines {
            // Find the matching line in deduction_locations by sale_line_id.
            let dl_line = lines_array
                .iter()
                .find(|l| l["sale_line_id"].as_str() == Some(&refund_line.sale_line_id))
                .ok_or_else(|| CoreError::Validation {
                    field: "deduction_locations",
                    message: format!(
                        "sale_line_id {} not found in deduction_locations",
                        refund_line.sale_line_id
                    ),
                })?;

            let deductions =
                dl_line["deductions"]
                    .as_array()
                    .ok_or_else(|| CoreError::Validation {
                        field: "deduction_locations.deductions",
                        message: "expected an array".into(),
                    })?;

            // Determine if this is a full or partial refund of the line.
            let total_deducted: i64 = deductions.iter().filter_map(|d| d["qty"].as_i64()).sum();
            let refund_qty = refund_line.qty;

            if refund_qty <= 0 {
                continue;
            }

            if refund_qty > total_deducted {
                return Err(CoreError::Validation {
                    field: "refund_line.qty",
                    message: format!(
                        "refund qty {} exceeds original deduction qty {} for line {}",
                        refund_qty, total_deducted, refund_line.sale_line_id
                    ),
                });
            }

            // ── Credit stock per ADR-19 §5.3 FIFO ─────────────
            let sku = dl_line["sku"].as_str().unwrap_or(&refund_line.sku);
            let mut remaining = refund_qty;

            if refund_qty >= total_deducted {
                // Full refund: iterate forward (oldest deduction first).
                for d in deductions {
                    let loc_id =
                        d["location_id"]
                            .as_str()
                            .ok_or_else(|| CoreError::Validation {
                                field: "location_id",
                                message: "missing location_id in deductions".into(),
                            })?;
                    let qty = d["qty"].as_i64().ok_or_else(|| CoreError::Validation {
                        field: "qty",
                        message: "missing qty in deductions".into(),
                    })?;
                    self.adjust_stock_at_location_with_reason(
                        tx,
                        sku,
                        qty,
                        &crate::inventory::LocationId::from(loc_id),
                        Some("refund"),
                        None,
                        None,
                        None,
                    )?;
                }
            } else {
                // Partial refund: iterate REVERSE (most recent deduction first).
                for d in deductions.iter().rev() {
                    if remaining <= 0 {
                        break;
                    }
                    let loc_id =
                        d["location_id"]
                            .as_str()
                            .ok_or_else(|| CoreError::Validation {
                                field: "location_id",
                                message: "missing location_id in deductions".into(),
                            })?;
                    let entry_qty = d["qty"].as_i64().ok_or_else(|| CoreError::Validation {
                        field: "qty",
                        message: "missing qty in deductions".into(),
                    })?;
                    let credit = entry_qty.min(remaining);
                    self.adjust_stock_at_location_with_reason(
                        tx,
                        sku,
                        credit,
                        &crate::inventory::LocationId::from(loc_id),
                        Some("refund"),
                        None,
                        None,
                        None,
                    )?;
                    remaining -= credit;
                }
            }
        }

        Ok(())
    }

    /// Fallback for pre-093 legacy sales: credit refund qty to the canonical
    /// default location and emit a warning audit log entry.
    fn credit_refund_to_default_location(
        &self,
        tx: &rusqlite::Transaction<'_>,
        refund: &Refund,
    ) -> Result<(), CoreError> {
        let default_loc =
            crate::inventory::LocationId::from("01926b3a-0000-7000-8000-000000000001");

        for refund_line in &refund.lines {
            self.adjust_stock_at_location_with_reason(
                tx,
                &refund_line.sku,
                refund_line.qty,
                &default_loc,
                Some("refund"),
                None,
                None,
                None,
            )?;
        }

        // Emit a warning audit entry for the legacy fallback.
        tx.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                uuid::Uuid::now_v7().to_string(),
                refund.processed_by,
                "sale.refund.legacy",
                "sale",
                refund.sale_id,
                serde_json::json!({
                    "refund_id": refund.id,
                    "note": "deduction_locations was NULL; credited to default location",
                }).to_string(),
                "warn",
                refund.created_at,
            ],
        )?;

        Ok(())
    }

    /// List all refunds for a given sale.
    pub fn list_refunds_for_sale(&self, sale_id: &str) -> Result<Vec<Refund>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sale_id, total_minor, currency, reason, note, processed_by, created_at
             FROM refunds WHERE sale_id = ?1 ORDER BY created_at ASC",
        )?;
        let refunds: Vec<Refund> = stmt
            .query_map(params![sale_id], |row| {
                let cur_str: String = row.get("currency")?;
                Ok(Refund {
                    id: row.get("id")?,
                    sale_id: row.get("sale_id")?,
                    total: Money {
                        minor_units: row.get("total_minor")?,
                        currency: cur_str.parse().expect("valid currency in DB"),
                    },
                    reason: row.get("reason")?,
                    note: row.get("note")?,
                    processed_by: row.get("processed_by")?,
                    created_at: row.get("created_at")?,
                    lines: Vec::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut line_stmt = self.conn.prepare(
            "SELECT id, refund_id, sale_line_id, sku, qty, unit_minor, line_minor, currency, created_at
             FROM refund_lines WHERE refund_id = ?1 ORDER BY created_at ASC"
        )?;
        let mut result: Vec<Refund> = Vec::new();
        for mut r in refunds {
            let lines: Vec<RefundLine> = line_stmt
                .query_map(params![r.id], Self::row_to_refund_line)?
                .collect::<Result<Vec<_>, _>>()?;
            r.lines = lines;
            result.push(r);
        }

        Ok(result)
    }

    /// Get total refunded amount for a sale.
    pub fn total_refunded_for_sale(&self, sale_id: &str) -> Result<Money, CoreError> {
        let row = self.conn.query_row(
            "SELECT COALESCE(SUM(total_minor), 0) AS total, currency FROM refunds WHERE sale_id = ?1 GROUP BY currency",
            params![sale_id],
            |row| {
                Ok((row.get::<_, i64>("total")?, row.get::<_, String>("currency")?))
            },
        );
        match row {
            Ok((total, cur_str)) => {
                let currency: Currency = cur_str.parse().expect("valid currency in DB");
                Ok(Money {
                    minor_units: total,
                    currency,
                })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(CoreError::NotFound {
                entity: "refund",
                id: sale_id.to_owned(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    fn row_to_refund_line(row: &rusqlite::Row) -> rusqlite::Result<RefundLine> {
        let cur_str: String = row.get("currency")?;
        let currency: Currency = cur_str.parse().expect("valid currency in DB");
        Ok(RefundLine {
            id: row.get("id")?,
            refund_id: row.get("refund_id")?,
            sale_line_id: row.get("sale_line_id")?,
            sku: row.get("sku")?,
            qty: row.get("qty")?,
            unit_price: Money {
                minor_units: row.get("unit_minor")?,
                currency,
            },
            line_total: Money {
                minor_units: row.get("line_minor")?,
                currency,
            },
            created_at: row.get("created_at")?,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::{Refund, RefundLine};
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn seed_completed_sale(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('ref-p1', 'COFFEE', 'Coffee', 350, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at,
                                deduction_locations) VALUES
                ('ref-sale-1', 700, 'USD', 2, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z',
                 '{\"version\":1,\"lines\":[{\"sale_line_id\":\"ref-sl-1\",\"sku\":\"COFFEE\",\"deductions\":[{\"location_id\":\"01926b3a-0000-7000-8000-000000000001\",\"qty\":2}]}]}');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                ('ref-sl-1', 'ref-sale-1', 'COFFEE', 2, 350, 700, 'USD', 1);"
        ).unwrap();
    }

    /// Seed a sale with multi-location split deductions for FIFO testing.
    /// Loc A gets 2, Loc B gets 3.
    fn seed_split_location_sale(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO inventory_locations (id, name, type) VALUES
                ('loc-store', 'Store Inventory', 'store'),
                ('loc-wh-a', 'Warehouse A', 'warehouse');
             INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('p-cho', 'CHO-001', 'Choco Bar', 500, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at,
                                deduction_locations) VALUES
                ('split-sale-1', 2500, 'USD', 1, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z',
                 '{\"version\":1,\"lines\":[{\"sale_line_id\":\"split-sl-1\",\"sku\":\"CHO-001\",\"deductions\":[{\"location_id\":\"loc-store\",\"qty\":2,\"sold_at\":\"2026-07-19T10:00:00Z\"},{\"location_id\":\"loc-wh-a\",\"qty\":3,\"sold_at\":\"2026-07-19T10:00:01Z\"}]}]}');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                ('split-sl-1', 'split-sale-1', 'CHO-001', 5, 500, 2500, 'USD', 1);"
        ).unwrap();
    }

    #[test]
    fn create_refund_persists() {
        let conn = fresh();
        seed_completed_sale(&conn);
        let s = store(&conn);

        let line = RefundLine::new("ref-sl-1", "COFFEE", 2, price(350), price(700));
        let refund = Refund::new(
            "ref-sale-1",
            price(700),
            "customer changed mind",
            "",
            "user-1",
            vec![line],
        );

        s.create_refund(&refund).unwrap();

        let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
        assert_eq!(refunds.len(), 1);
        assert_eq!(refunds[0].total.minor_units, 700);
        assert_eq!(refunds[0].total.currency, usd());
        assert_eq!(refunds[0].reason, "customer changed mind");
        assert_eq!(refunds[0].processed_by, "user-1");
        assert_eq!(refunds[0].lines.len(), 1);
        assert_eq!(refunds[0].lines[0].sku, "COFFEE");
        assert_eq!(refunds[0].lines[0].qty, 2);
    }

    #[test]
    fn create_refund_nonexistent_sale_fails() {
        let conn = fresh();
        let s = store(&conn);

        let line = RefundLine::new("sl-x", "COFFEE", 1, price(350), price(350));
        let refund = Refund::new("nonexistent", price(350), "test", "", "user-1", vec![line]);

        let result = s.create_refund(&refund);
        assert!(result.is_err());
    }

    #[test]
    fn list_refunds_empty_for_sale() {
        let conn = fresh();
        seed_completed_sale(&conn);
        let s = store(&conn);
        let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
        assert!(refunds.is_empty());
    }

    #[test]
    fn total_refunded_for_sale_no_refunds() {
        let conn = fresh();
        seed_completed_sale(&conn);
        let s = store(&conn);
        let result = s.total_refunded_for_sale("ref-sale-1");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::NotFound { .. }));
    }

    #[test]
    fn multiple_partial_refunds() {
        let conn = fresh();
        seed_completed_sale(&conn);
        let s = store(&conn);

        // First refund: 1 item.
        let line1 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
        let r1 = Refund::new(
            "ref-sale-1",
            price(350),
            "partial",
            "",
            "user-1",
            vec![line1],
        );
        s.create_refund(&r1).unwrap();

        // Second refund: 1 item.
        let line2 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
        let r2 = Refund::new(
            "ref-sale-1",
            price(350),
            "partial",
            "",
            "user-1",
            vec![line2],
        );
        s.create_refund(&r2).unwrap();

        let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
        assert_eq!(refunds.len(), 2);
        assert_eq!(refunds[0].total.minor_units, 350);
        assert_eq!(refunds[1].total.minor_units, 350);

        // Verify audit log entries.
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.refund' AND target_id = 'ref-sale-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    // ── ADR-19 §5.3 FIFO refund stock restoration tests ─────────────

    fn get_stock_at(conn: &Connection, sku: &str, location_id: &str) -> i64 {
        conn.query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary
             WHERE item_id = (SELECT id FROM products WHERE sku = ?1)
             AND location_id = ?2",
            rusqlite::params![sku, location_id],
            |row| row.get(0),
        )
        .unwrap_or(0)
    }

    /// Full refund of a split-location sale — stock should be credited
    /// forward (oldest deduction first): loc-store gets +2, loc-wh-a gets +3.
    #[test]
    fn refund_credits_split_location_full_refund_forward_fifo() {
        let conn = fresh();
        seed_split_location_sale(&conn);
        let s = store(&conn);

        // Initial stock is 0 at both locations.
        assert_eq!(get_stock_at(&conn, "CHO-001", "loc-store"), 0);
        assert_eq!(get_stock_at(&conn, "CHO-001", "loc-wh-a"), 0);

        // Full refund of all 5 units.
        let line = RefundLine::new("split-sl-1", "CHO-001", 5, price(500), price(2500));
        let refund = Refund::new(
            "split-sale-1",
            price(2500),
            "full refund",
            "",
            "user-1",
            vec![line],
        );
        s.create_refund(&refund).unwrap();

        // Stock credited forward: 2 to loc-store, 3 to loc-wh-a.
        assert_eq!(
            get_stock_at(&conn, "CHO-001", "loc-store"),
            2,
            "store gets 2 (oldest deduction first)"
        );
        assert_eq!(
            get_stock_at(&conn, "CHO-001", "loc-wh-a"),
            3,
            "warehouse gets 3 (second deduction)"
        );

        // Verify audit log.
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.refund' AND target_id = 'split-sale-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 1);
    }

    /// Partial refund of a split-location line — stock should be credited
    /// in REVERSE order (most recent deduction first): loc-wh-a gets credited
    /// before loc-store.
    #[test]
    fn refund_credits_split_location_partial_refund_reverse_order() {
        let conn = fresh();
        seed_split_location_sale(&conn);
        let s = store(&conn);

        // Refund 2 of 5 units (partial).
        let line = RefundLine::new("split-sl-1", "CHO-001", 2, price(500), price(1000));
        let refund = Refund::new(
            "split-sale-1",
            price(1000),
            "partial refund 2",
            "",
            "user-1",
            vec![line],
        );
        s.create_refund(&refund).unwrap();

        // Stock credited in reverse: loc-wh-a (most recent) gets 2,
        // loc-store gets 0 (remaining = 0 after warehouse covers it).
        assert_eq!(
            get_stock_at(&conn, "CHO-001", "loc-store"),
            0,
            "store gets 0 (partial refund credits most recent deduction first)"
        );
        assert_eq!(
            get_stock_at(&conn, "CHO-001", "loc-wh-a"),
            2,
            "warehouse gets 2 (most recent deduction credited first)"
        );
    }

    /// Partial refund that spans two deduction locations — credit crosses
    /// from most recent to oldest.
    #[test]
    fn refund_credits_across_two_locations_partial() {
        let conn = fresh();
        seed_split_location_sale(&conn);
        let s = store(&conn);

        // Refund 4 of 5 units — should exhaust loc-wh-a (3) and take 1 from loc-store.
        let line = RefundLine::new("split-sl-1", "CHO-001", 4, price(500), price(2000));
        let refund = Refund::new(
            "split-sale-1",
            price(2000),
            "partial refund 4",
            "",
            "user-1",
            vec![line],
        );
        s.create_refund(&refund).unwrap();

        assert_eq!(
            get_stock_at(&conn, "CHO-001", "loc-wh-a"),
            3,
            "warehouse gets full 3 (most recent deduction first)"
        );
        assert_eq!(
            get_stock_at(&conn, "CHO-001", "loc-store"),
            1,
            "store gets remaining 1 after warehouse exhausted"
        );
    }

    /// Refund with qty larger than original deduction should fail.
    #[test]
    fn refund_qty_exceeds_original_deduction_fails() {
        let conn = fresh();
        seed_split_location_sale(&conn);
        let s = store(&conn);

        let line = RefundLine::new("split-sl-1", "CHO-001", 99, price(500), price(49500));
        let refund = Refund::new(
            "split-sale-1",
            price(49500),
            "excessive refund",
            "",
            "user-1",
            vec![line],
        );
        let result = s.create_refund(&refund);
        assert!(result.is_err(), "refund exceeding original qty should fail");
        match result.unwrap_err() {
            CoreError::Validation { field, .. } => {
                assert_eq!(field, "refund_line.qty");
            }
            other => panic!("expected Validation error, got: {other:?}"),
        }
    }

    /// Legacy sale (NULL deduction_locations) falls back to default location.
    #[test]
    fn refund_legacy_sale_with_null_deduction_locations() {
        let conn = fresh();
        // The default location (01926b3a-...-001) is seeded by migration 078.
        // Use the old seed that sets deduction_locations = NULL explicitly.
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('legacy-p1', 'LEGACY', 'Legacy Item', 100, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
                ('legacy-sale-1', 200, 'USD', 1, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                ('legacy-sl-1', 'legacy-sale-1', 'LEGACY', 2, 100, 200, 'USD', 1);"
        ).unwrap();
        let s = store(&conn);

        let line = RefundLine::new("legacy-sl-1", "LEGACY", 2, price(100), price(200));
        let refund = Refund::new(
            "legacy-sale-1",
            price(200),
            "legacy refund",
            "",
            "user-1",
            vec![line],
        );
        s.create_refund(&refund).unwrap();

        // Stock should be credited to default location.
        assert_eq!(
            get_stock_at(&conn, "LEGACY", "01926b3a-0000-7000-8000-000000000001"),
            2,
            "legacy refund credits to default location"
        );

        // Audit log should have the legacy warning entry.
        let warn_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.refund.legacy' AND target_id = 'legacy-sale-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            warn_count, 1,
            "legacy fallback should emit a warning audit entry"
        );
    }

    /// Verify that a refund correctly updates the stock_movements ledger.
    #[test]
    fn refund_creates_positive_stock_movements() {
        let conn = fresh();
        seed_split_location_sale(&conn);
        let s = store(&conn);

        let line = RefundLine::new("split-sl-1", "CHO-001", 3, price(500), price(1500));
        let refund = Refund::new(
            "split-sale-1",
            price(1500),
            "partial refund 3",
            "",
            "user-1",
            vec![line],
        );
        s.create_refund(&refund).unwrap();

        // Check that stock_movements has positive entries with reason 'refund' and location_id set.
        let movement_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stock_movements
                 WHERE item_id = (SELECT id FROM products WHERE sku = 'CHO-001')
                 AND reason = 'refund' AND delta > 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            movement_count, 1,
            "should have one positive movement (wh-a gets 3)"
        );

        let total_delta: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(delta), 0) FROM stock_movements
                 WHERE item_id = (SELECT id FROM products WHERE sku = 'CHO-001')
                 AND reason = 'refund'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            total_delta, 3,
            "total credited delta should match refund qty"
        );
    }
}
