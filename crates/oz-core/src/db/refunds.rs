//! Refund CRUD — create, list, and query refunds.

use rusqlite::params;

use crate::error::CoreError;
use crate::money::Currency;
use crate::{Money, Refund, RefundLine};

use super::Store;

impl Store<'_> {
    /// Process a refund — persist refund + lines inside a transaction.
    pub fn create_refund(&self, refund: &Refund) -> Result<(), CoreError> {
        let cur_str =
            std::str::from_utf8(&refund.total.currency.0).expect("currency bytes are valid UTF-8");

        let tx = self.conn.unchecked_transaction()?;

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

        // Write audit log inside the same transaction.
        tx.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                uuid::Uuid::new_v4().to_string(),
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
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
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
             INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
                ('ref-sale-1', 700, 'USD', 2, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                ('ref-sl-1', 'ref-sale-1', 'COFFEE', 2, 350, 700, 'USD', 1);"
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
}
