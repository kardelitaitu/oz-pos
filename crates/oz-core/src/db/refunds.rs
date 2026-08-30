//! Refund CRUD — create, list, and query refunds.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice B5 part 4: refunds deep read)
crate: oz-core | status: SAFE | lint: CLEAN
findings: refund stock restoration per ADR-19 §5.3 is well built (FIFO full / reverse partial crediting via deduction_locations JSON, qty<=deducted guard, legacy fallback with warn audit, audit row inside the same tx); COR-25 MEDIUM: the over-refund guard runs OUTSIDE the transaction AND reads cumulative refunded with .unwrap_or(0) — a DB error reads as zero refunds and bypasses the guard (fail-open on a MONEY guard; same class as COR-11), and the check-then-act is race-safe only under the single-connection mutex; COR-26 LOW: refund currency never compared to the sale currency (comment defers to caller's checked_add, nothing enforces) — a cross-currency refund passes the over-refund guard against the wrong unit
next: move the guard inside the tx, propagate SUM errors, compare currencies (COR-25/COR-26) | perf: N/A
*/
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
            std::str::from_utf8(&refund.total.currency.0).map_err(|e| CoreError::Validation {
                field: "currency",
                message: format!("invalid UTF-8 in currency bytes: {e}"),
            })?;

        // ── 0. Over-refund guard ──────────────────────────────────
        // A sale may be refunded AT MOST its original total. The sale stays
        // 'completed' (nothing transitions it to 'refunded'), so without
        // this check the same sale could be refunded unlimited times and
        // stock credited each time. Reject when the cumulative refunded
        // amount plus this refund would exceed the sale's total.
        let (sale_total, sale_currency): (i64, String) = match self.conn.query_row(
            "SELECT total_minor, currency FROM sales WHERE id = ?1",
            params![refund.sale_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) {
            Ok(pair) => pair,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(CoreError::NotFound {
                    entity: "sale",
                    id: refund.sale_id.clone(),
                });
            }
            Err(e) => return Err(CoreError::Db(e)),
        };
        // COR-26: the refund must be denominated in the sale's own currency.
        // The per-currency SUM below only bounds refunds that share the
        // sale's unit; a foreign-currency refund would compare minor units
        // against the wrong total (and could be repeated once per currency
        // to bypass the guard). Enforced here so no caller has to be
        // trusted to have folded with `Money::zero(sale.currency)`.
        if cur_str != sale_currency {
            return Err(CoreError::CurrencyMismatch(
                sale_currency,
                cur_str.to_owned(),
            ));
        }
        let already_refunded: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(total_minor), 0) FROM refunds WHERE sale_id = ?1 AND currency = ?2",
                params![refund.sale_id, cur_str],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let after = already_refunded
            .checked_add(refund.total.minor_units)
            .ok_or_else(|| CoreError::Validation {
                field: "total",
                message: "refund total overflow".into(),
            })?;
        if after > sale_total {
            return Err(CoreError::Validation {
                field: "total",
                message: format!(
                    "refund total {} exceeds refundable balance {} for sale {} (already refunded {})",
                    refund.total.minor_units,
                    sale_total - already_refunded,
                    refund.sale_id,
                    already_refunded
                ),
            });
        }
        let tx = self.conn.unchecked_transaction()?;

        // ── 1. Persist refund + lines ──────────────────────────────
        tx.execute(
            "INSERT INTO refunds (id, sale_id, total_minor, currency, reason, note, processed_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![refund.id, refund.sale_id, refund.total.minor_units, cur_str, refund.reason, refund.note, refund.processed_by, refund.created_at],
        )?;

        for line in &refund.lines {
            let line_cur = std::str::from_utf8(&line.unit_price.currency.0).map_err(|e| {
                CoreError::Validation {
                    field: "currency",
                    message: format!("invalid UTF-8 in currency bytes: {e}"),
                }
            })?;
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
        match deduction_locations_json.as_deref() {
            None | Some("null") | Some("") => {
                self.credit_refund_to_default_location(&tx, refund)?;
            }
            Some(locations) => {
                self.credit_refund_from_deduction_locations(&tx, refund, locations)?;
            }
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

        // Emit a warning audit entry for the legacy fallback. This targets
        // the refund (not the sale) so it does not shadow the primary
        // `sale.refund` audit entry written by `create_refund`.
        tx.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                uuid::Uuid::now_v7().to_string(),
                refund.processed_by,
                "sale.refund.legacy",
                "refund",
                &refund.id,
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
                        currency: cur_str.parse::<Currency>().map_err(|e| {
                            rusqlite::Error::ToSqlConversionFailure(
                                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                                    .into(),
                            )
                        })?,
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
    ///
    /// Returns `Money::zero` in the sale's currency when no refunds exist
    /// (callers use this as a balance check). Only refunds in the SALE's
    /// currency are summed — a cross-currency refund line would not be
    /// comparable and is excluded from the balance.
    pub fn total_refunded_for_sale(&self, sale_id: &str) -> Result<Money, CoreError> {
        let row = self.conn.query_row(
            "SELECT total_minor, currency FROM sales WHERE id = ?1",
            params![sale_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        );
        let (sale_total_unused, sale_currency_str) = match row {
            Ok(pair) => pair,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(CoreError::NotFound {
                    entity: "sale",
                    id: sale_id.to_owned(),
                });
            }
            Err(e) => return Err(CoreError::Db(e)),
        };
        let _ = sale_total_unused;
        let sale_currency: Currency =
            sale_currency_str
                .parse()
                .map_err(|e| CoreError::Validation {
                    field: "currency",
                    message: format!("invalid sale currency: {e}"),
                })?;
        let total: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(total_minor), 0) FROM refunds WHERE sale_id = ?1 AND currency = ?2",
                params![sale_id, sale_currency_str],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(Money {
            minor_units: total,
            currency: sale_currency,
        })
    }

    fn row_to_refund_line(row: &rusqlite::Row) -> rusqlite::Result<RefundLine> {
        let cur_str: String = row.get("currency")?;
        let currency: Currency = cur_str.parse::<Currency>().map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
            )
        })?;
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
#[path = "refunds_tests.rs"]
mod tests;
