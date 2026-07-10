//! Sale CRUD — create, list, get, update status, held carts, exports.

use rusqlite::params;

use crate::error::CoreError;
use crate::money::Currency;
use crate::tax_rate::TaxRate;
use crate::{AuditEntry, Money, Sale, SaleLine, SaleStatus};

use super::Store;

/// Input for cart-level tax computation (used by IPC command).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CartLineTaxInput {
    /// Product SKU for rate lookup.
    pub sku: String,
    /// Quantity in this line.
    pub qty: i64,
    /// Unit price in minor units.
    pub unit_price_minor: i64,
}

// ── Export types ─────────────────────────────────────────────────────

/// Row returned by [`Store::export_daily_summary`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailySummaryRow {
    /// Sale unique identifier.
    pub sale_id: String,
    /// Total sale amount in minor units (e.g. cents).
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Number of line items in the sale.
    pub line_count: i64,
    /// Sale status (e.g. "active", "completed", "voided").
    pub status: String,
    /// RFC-3339 timestamp of when the sale was created.
    pub created_at: String,
}

/// Row returned by [`Store::export_sales_by_hour`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct SalesByHourRow {
    /// Hour of day (0–23).
    pub hour: i64,
    /// Total value of all sales in this hour, in minor units.
    pub total_minor: i64,
    /// Number of sales transacted in this hour.
    pub sale_count: i64,
}

/// Summary row for a held (parked) cart.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeldCartRow {
    /// Held cart unique identifier.
    pub id: String,
    /// User-assigned label for the cart.
    pub label: String,
    /// Number of line items in the cart.
    pub item_count: i64,
    /// Cart total in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// RFC-3339 timestamp of when the cart was parked.
    pub created_at: String,
    /// Type of cart: 'hold' or 'open_bill'.
    pub bill_type: String,
    /// Customer name (set when bill_type = 'open_bill').
    pub customer_name: Option<String>,
}

/// Full held cart data including the JSON cart_data blob.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeldCartFull {
    /// Held cart unique identifier.
    pub id: String,
    /// User-assigned label for the cart.
    pub label: String,
    /// JSON-encoded cart state (line items, discounts, etc.).
    pub cart_data: String,
    /// Number of line items in the cart.
    pub item_count: i64,
    /// Cart total in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// RFC-3339 timestamp of when the cart was parked.
    pub created_at: String,
    /// Type of cart: 'hold' or 'open_bill'.
    pub bill_type: String,
    /// Customer name (set when bill_type = 'open_bill').
    pub customer_name: Option<String>,
}

// ── Sale CRUD ────────────────────────────────────────────────────

impl Store<'_> {
    fn row_to_sale_line(row: &rusqlite::Row) -> rusqlite::Result<SaleLine> {
        let unit_cur_str: String = row.get("currency")?;
        let currency: Currency = unit_cur_str.parse().expect("valid currency in DB");
        Ok(SaleLine {
            id: row.get("id")?,
            sale_id: row.get("sale_id")?,
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
            line_position: row.get("line_position")?,
            tax_amount: Money {
                minor_units: row.get("tax_minor")?,
                currency,
            },
            tax_rate_id: row.get("tax_rate_id")?,
            serial_number: row.get("serial_number")?,
        })
    }

    /// Persist a [`Sale`] (header + all line items) inside a single transaction.
    pub fn create_sale(&self, sale: &Sale) -> Result<(), CoreError> {
        let cur_str =
            std::str::from_utf8(&sale.currency.0).expect("currency bytes are valid UTF-8");
        let status_str = sale.status.as_stored_str();

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor,
                                discount_percent, discount_label, user_id, created_at, updated_at,
                                subtotal_minor, tax_total_minor, customer_id, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1)",
            params![
                sale.id, sale.total.minor_units, cur_str, sale.line_count,
                status_str, sale.payment_method, sale.tendered_minor,
                sale.discount_percent, sale.discount_label, sale.user_id,
                sale.created_at, sale.updated_at,
                sale.subtotal.minor_units, sale.tax_total.minor_units,
                sale.customer_id,
            ],
        )?;

        for line in &sale.lines {
            let unit_cur = std::str::from_utf8(&line.unit_price.currency.0)
                .expect("currency bytes are valid UTF-8");
            tx.execute(
                "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position,
                                        tax_minor, tax_rate_id, serial_number)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    line.id, line.sale_id, line.sku, line.qty,
                    line.unit_price.minor_units, line.line_total.minor_units,
                    unit_cur, line.line_position,
                    line.tax_amount.minor_units, line.tax_rate_id,
                    line.serial_number,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// List all sales ordered by creation date (most recent first), without line items.
    pub fn list_sales(&self) -> Result<Vec<Sale>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version
             FROM sales ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse().expect("valid currency in DB");
            let status = SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency,
                },
                line_count: row.get("line_count")?,
                currency,
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row
                    .get::<_, Option<i64>>("discount_percent")
                    .unwrap_or(Some(0))
                    .unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(),
                subtotal: Money {
                    minor_units: row.get("subtotal_minor")?,
                    currency,
                },
                tax_total: Money {
                    minor_units: row.get("tax_total_minor")?,
                    currency,
                },
                customer_id: row.get("customer_id")?,
                version: row.get("version").unwrap_or(1),
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single sale by id, including all line items.
    pub fn get_sale(&self, id: &str) -> Result<Option<Sale>, CoreError> {
        let mut sale_stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version
             FROM sales WHERE id = ?1",
        )?;

        let sale_result = sale_stmt.query_row(params![id], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse().expect("valid currency in DB");
            let status = SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency,
                },
                line_count: row.get("line_count")?,
                currency,
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row
                    .get::<_, Option<i64>>("discount_percent")
                    .unwrap_or(Some(0))
                    .unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(),
                subtotal: Money {
                    minor_units: row.get("subtotal_minor")?,
                    currency,
                },
                tax_total: Money {
                    minor_units: row.get("tax_total_minor")?,
                    currency,
                },
                customer_id: row.get("customer_id")?,
                version: row.get("version").unwrap_or(1),
            })
        });

        let mut sale = match sale_result {
            Ok(s) => s,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut line_stmt = self.conn.prepare(
            "SELECT id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position,
                    tax_minor, tax_rate_id, serial_number
             FROM sale_lines WHERE sale_id = ?1 ORDER BY line_position",
        )?;
        let line_rows = line_stmt.query_map(params![id], Self::row_to_sale_line)?;
        for line in line_rows {
            sale.lines.push(line?);
        }

        Ok(Some(sale))
    }

    /// Update the status of a sale, validating the state machine transition.
    pub fn update_sale_status(&self, id: &str, to: SaleStatus) -> Result<Sale, CoreError> {
        let result = self.conn.query_row(
            "SELECT status FROM sales WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        );

        let current_str = match result {
            Ok(s) => s,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(CoreError::NotFound {
                    entity: "sale",
                    id: id.to_owned(),
                });
            }
            Err(e) => return Err(e.into()),
        };

        let current =
            SaleStatus::from_stored_str(&current_str).ok_or_else(|| CoreError::Validation {
                field: "status",
                message: format!("invalid stored status: {current_str}"),
            })?;

        if !SaleStatus::can_transition_to(current, to) {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("cannot transition from {:?} to {:?}", current, to),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let status_str = to.as_stored_str();
        let rows = self.conn.execute(
            "UPDATE sales SET status = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![status_str, now, id],
        )?;
        if rows == 0 {
            return Err(CoreError::Conflict {
                entity: "sale",
                field: "version",
            });
        }

        self.get_sale(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: id.to_owned(),
        })
    }
}

// ── Export / Report queries ─────────────────────────────────────────

impl Store<'_> {
    /// Query all sales for today, ordered chronologically.
    pub fn export_daily_summary(&self) -> Result<Vec<DailySummaryRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status, created_at
             FROM sales WHERE date(created_at) = date('now') ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DailySummaryRow {
                sale_id: row.get("id")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                line_count: row.get("line_count")?,
                status: row.get("status")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Query sales volume grouped by hour of day (for today).
    pub fn export_sales_by_hour(&self) -> Result<Vec<SalesByHourRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%H', created_at) AS INTEGER) AS hour,
                    SUM(total_minor) AS total_minor, COUNT(*) AS sale_count
             FROM sales WHERE date(created_at) = date('now')
             GROUP BY hour ORDER BY hour",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SalesByHourRow {
                hour: row.get("hour")?,
                total_minor: row.get("total_minor")?,
                sale_count: row.get("sale_count")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }
}

// ── Held Carts ──────────────────────────────────────────────────────

impl Store<'_> {
    /// Persist a cart as a held (parked) order or open bill.
    ///
    /// `bill_type` should be `"hold"` or `"open_bill"`. When `customer_name`
    /// is set, it is stored alongside the cart for open-bill listing.
    #[allow(clippy::too_many_arguments)]
    pub fn hold_cart(
        &self,
        label: &str,
        cart_data: &str,
        item_count: i64,
        total_minor: i64,
        currency: &str,
        bill_type: &str,
        customer_name: Option<&str>,
    ) -> Result<String, CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO held_carts (id, label, cart_data, item_count, total_minor, currency, bill_type, customer_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                label.trim(),
                cart_data,
                item_count,
                total_minor,
                currency,
                bill_type,
                customer_name,
            ],
        )?;
        Ok(id)
    }

    /// List all held (parked) orders, most recent first.
    pub fn list_held_carts(&self) -> Result<Vec<HeldCartRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, item_count, total_minor, currency, created_at, bill_type, customer_name
             FROM held_carts ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HeldCartRow {
                id: row.get("id")?,
                label: row.get("label")?,
                item_count: row.get("item_count")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                created_at: row.get("created_at")?,
                bill_type: row.get("bill_type")?,
                customer_name: row.get("customer_name")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List only open bills (bill_type = 'open_bill'), most recent first.
    pub fn list_open_bills(&self) -> Result<Vec<HeldCartRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, item_count, total_minor, currency, created_at, bill_type, customer_name
             FROM held_carts WHERE bill_type = 'open_bill' ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HeldCartRow {
                id: row.get("id")?,
                label: row.get("label")?,
                item_count: row.get("item_count")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                created_at: row.get("created_at")?,
                bill_type: row.get("bill_type")?,
                customer_name: row.get("customer_name")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a held cart by id.
    pub fn get_held_cart(&self, id: &str) -> Result<Option<HeldCartFull>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, cart_data, item_count, total_minor, currency, created_at, bill_type, customer_name
             FROM held_carts WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(HeldCartFull {
                id: row.get("id")?,
                label: row.get("label")?,
                cart_data: row.get("cart_data")?,
                item_count: row.get("item_count")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                created_at: row.get("created_at")?,
                bill_type: row.get("bill_type")?,
                customer_name: row.get("customer_name")?,
            })
        });
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a held cart by id.
    pub fn delete_held_cart(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM held_carts WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "held_cart",
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}

// ── Receipt Barcodes ──────────────────────────────────────────────────

impl Store<'_> {
    /// Store a receipt barcode mapping for a sale.
    pub fn save_receipt_barcode(&self, sale_id: &str, barcode: &str) -> Result<(), CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO receipt_barcodes (id, sale_id, barcode) VALUES (?1, ?2, ?3)",
            params![id, sale_id, barcode],
        )?;
        Ok(())
    }

    /// Look up a sale by its receipt barcode.
    pub fn lookup_sale_by_receipt_barcode(&self, barcode: &str) -> Result<Option<Sale>, CoreError> {
        let sale_id: Option<String> = self
            .conn
            .query_row(
                "SELECT sale_id FROM receipt_barcodes WHERE barcode = ?1",
                params![barcode],
                |row| row.get(0),
            )
            .ok();

        match sale_id {
            Some(id) => self.get_sale(&id),
            None => Ok(None),
        }
    }
}

// ── Void Sale ───────────────────────────────────────────────────────

impl Store<'_> {
    /// Void a sale and restore stock for all line items.
    pub fn void_sale(&self, sale_id: &str, user_id: &str, reason: &str) -> Result<Sale, CoreError> {
        let sale = self.get_sale(sale_id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: sale_id.to_owned(),
        })?;

        if sale.status != SaleStatus::Active {
            return Err(CoreError::Validation {
                field: "status",
                message: format!(
                    "only active sales can be voided (current: {:?})",
                    sale.status
                ),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let tx = self.conn.unchecked_transaction()?;

        // 1. Update status to Voided with optimistic concurrency (ADR #6).
        let rows = tx.execute(
            "UPDATE sales SET status = 'voided', updated_at = ?1, version = version + 1 WHERE id = ?2",
            rusqlite::params![now, sale_id],
        )?;
        if rows == 0 {
            return Err(CoreError::Conflict {
                entity: "sale",
                field: "version",
            });
        }

        // 2. Restore stock for each line item.
        for line in &sale.lines {
            if let Some(product_id) = self.product_id_by_sku(&line.sku)? {
                let current_qty = self.get_stock(&product_id)?;
                let new_qty =
                    current_qty
                        .checked_add(line.qty)
                        .ok_or_else(|| CoreError::Validation {
                            field: "qty",
                            message: "stock overflow during void".into(),
                        })?;
                tx.execute(
                    "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty,
                                                            updated_at = excluded.updated_at",
                    rusqlite::params![product_id, new_qty, now],
                )?;
            }
        }

        // 3. Audit log entry.
        let details = serde_json::json!({
            "reason": reason,
            "total_minor": sale.total.minor_units,
        })
        .to_string();
        let audit = AuditEntry::new(
            user_id,
            "sale.void",
            Some("sale"),
            Some(sale_id),
            Some(details),
            "success",
        );
        self.log_audit(&audit)?;

        tx.commit()?;

        self.get_sale(sale_id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: sale_id.to_owned(),
        })
    }
}

// ── Tax Computation ───────────────────────────────────────────────────

impl Store<'_> {
    /// Compute tax breakdown for a sale in-place.
    ///
    /// For each line resolves ALL applicable tax rates via the chain:
    /// 1. Product-level tax rates (`get_product_tax_rates`)
    /// 2. Category-level tax rates (via the product's `category_id`)
    /// 3. Default store-wide tax rate (where `is_default = true`)
    ///
    /// `lua_overrides` — per-SKU tax rate overrides from plugins.
    /// When a SKU is present in `lua_overrides` its `(rate_bps, is_inclusive)`
    /// values are used instead of the DB-resolved rates for that line.
    ///
    /// All rates for a line contribute to its total tax. Stores the
    /// first rate's id in `tax_rate_id` for backward compatibility.
    /// Updates each line's `tax_amount`, then sets `sale.subtotal`
    /// and `sale.tax_total`.
    pub fn compute_sale_tax(
        &self,
        sale: &mut Sale,
        lua_overrides: &[(String, i64, bool)],
    ) -> Result<(), CoreError> {
        let currency = sale.currency;
        let mut total_tax: Option<Money> = None;
        let mut subtotal: Option<Money> = None;

        for line in &mut sale.lines {
            let line_subtotal = line.line_total;
            let mut line_tax = Money::zero(currency);

            // Check for a Lua plugin override first.
            let override_idx = lua_overrides
                .iter()
                .position(|(sku, _, _)| sku == &line.sku);

            if let Some(idx) = override_idx {
                let (_, rate_bps, is_inclusive) = &lua_overrides[idx];
                let rbps = *rate_bps;
                let tax = if *is_inclusive {
                    let divisor = 10_000 + rbps;
                    let tax_minor = line_subtotal.minor_units * rbps / divisor;
                    Money {
                        minor_units: tax_minor,
                        currency: line_subtotal.currency,
                    }
                } else {
                    let tax_minor = line_subtotal.minor_units * rbps / 10_000;
                    Money {
                        minor_units: tax_minor,
                        currency: line_subtotal.currency,
                    }
                };
                line_tax = line_tax
                    .checked_add(tax)
                    .unwrap_or_else(|| Money::zero(currency));
                // No DB tax_rate_id for override lines.
                line.tax_rate_id = None;
            } else {
                let rates = self.resolve_best_tax_rates_for_sku(&line.sku)?;

                for rate in &rates {
                    let tax = if rate.is_inclusive {
                        let divisor = 10_000 + rate.rate_bps;
                        let tax_minor = line_subtotal.minor_units * rate.rate_bps / divisor;
                        Money {
                            minor_units: tax_minor,
                            currency: line_subtotal.currency,
                        }
                    } else {
                        let tax_minor = line_subtotal.minor_units * rate.rate_bps / 10_000;
                        Money {
                            minor_units: tax_minor,
                            currency: line_subtotal.currency,
                        }
                    };
                    line_tax = line_tax
                        .checked_add(tax)
                        .unwrap_or_else(|| Money::zero(currency));
                }

                line.tax_rate_id = rates.first().map(|r| r.id.clone());
            }

            line.tax_amount = line_tax;

            total_tax = match total_tax {
                None => Some(line_tax),
                Some(acc) => acc.checked_add(line_tax),
            };

            subtotal = match subtotal {
                None => Some(line.line_total),
                Some(acc) => acc.checked_add(line.line_total),
            };
        }

        sale.subtotal = subtotal.unwrap_or_else(|| Money::zero(currency));
        sale.tax_total = total_tax.unwrap_or_else(|| Money::zero(currency));
        Ok(())
    }

    /// Compute the total tax for a set of cart lines (live preview).
    ///
    /// For each cart line resolves ALL applicable tax rates and sums
    /// their contributions. Returns the total tax amount.
    pub fn compute_cart_tax(
        &self,
        lines: &[CartLineTaxInput],
        currency: Currency,
    ) -> Result<Money, CoreError> {
        let mut total_tax: Option<Money> = None;

        for line in lines {
            let line_total_minor = line.qty * line.unit_price_minor;
            let rates = self.resolve_best_tax_rates_for_sku(&line.sku)?;

            for rate in &rates {
                let tax_minor = if rate.is_inclusive {
                    let divisor = 10_000 + rate.rate_bps;
                    line_total_minor * rate.rate_bps / divisor
                } else {
                    line_total_minor * rate.rate_bps / 10_000
                };
                let tax = Money {
                    minor_units: tax_minor,
                    currency,
                };
                total_tax = match total_tax {
                    None => Some(tax),
                    Some(acc) => acc.checked_add(tax),
                };
            }
        }

        Ok(total_tax.unwrap_or_else(|| Money::zero(currency)))
    }

    /// Resolve all applicable tax rates for a SKU using the chain:
    /// product rates → category rates → default rate.
    ///
    /// Returns ALL rates at the first matching level (e.g. all product-
    /// level rates). Returns an empty vec when no rate is configured.
    pub fn resolve_best_tax_rates_for_sku(&self, sku: &str) -> Result<Vec<TaxRate>, CoreError> {
        // 1. Product-level tax rates — return ALL assigned rates.
        let product_rate_ids = self.get_product_tax_rates(sku)?;
        if !product_rate_ids.is_empty() {
            let mut rates = Vec::with_capacity(product_rate_ids.len());
            for id in &product_rate_ids {
                if let Some(rate) = self.get_tax_rate(id)? {
                    rates.push(rate);
                }
            }
            if !rates.is_empty() {
                return Ok(rates);
            }
        }

        // 2. Category-level tax rates (via product.category_id).
        let product_id = self.product_id_by_sku(sku)?;
        if let Some(pid) = product_id {
            let category_id: Option<String> = self
                .conn
                .query_row(
                    "SELECT category_id FROM products WHERE id = ?1",
                    params![pid],
                    |row| row.get(0),
                )
                .ok()
                .and_then(|v| v);

            if let Some(cid) = category_id {
                let cat_rate_ids = self.get_category_tax_rates(&cid)?;
                if !cat_rate_ids.is_empty() {
                    let mut rates = Vec::with_capacity(cat_rate_ids.len());
                    for id in &cat_rate_ids {
                        if let Some(rate) = self.get_tax_rate(id)? {
                            rates.push(rate);
                        }
                    }
                    if !rates.is_empty() {
                        return Ok(rates);
                    }
                }
            }
        }

        // 3. Default store-wide tax rate.
        let all_rates = self.list_tax_rates()?;
        if let Some(default) = all_rates.into_iter().find(|r| r.is_default) {
            return Ok(vec![default]);
        }

        Ok(Vec::new())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::{Cart, CartLine, Sku};
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

    fn make_cart() -> Cart {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
            .unwrap();
        cart
    }

    // ── Sale CRUD ────────────────────────────────────────────────

    #[test]
    fn create_sale_persists_header() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.id, sale.id);
        assert_eq!(loaded.status, SaleStatus::Pending);
        assert_eq!(loaded.total.minor_units, 1150);
        assert_eq!(loaded.line_count, 2);
    }

    #[test]
    fn create_sale_persists_lines() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.lines.len(), 2);
        assert_eq!(loaded.lines[0].sku, "COFFEE");
        assert_eq!(loaded.lines[0].qty, 2);
        assert_eq!(loaded.lines[0].unit_price.minor_units, 350);
        assert_eq!(loaded.lines[0].line_total.minor_units, 700);
        assert_eq!(loaded.lines[0].line_position, 1);
        assert_eq!(loaded.lines[1].sku, "BAGEL");
        assert_eq!(loaded.lines[1].line_position, 2);
    }

    #[test]
    fn create_sale_empty_cart() {
        let conn = fresh();
        let cart = Cart::new(usd());
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();
        let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.line_count, 0);
        assert_eq!(loaded.lines.len(), 0);
        assert_eq!(loaded.total.minor_units, 0);
    }

    #[test]
    fn list_sales_empty_db() {
        let conn = fresh();
        let sales = store(&conn).list_sales().unwrap();
        assert!(sales.is_empty());
    }

    #[test]
    fn list_sales_returns_all() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let mut cart2 = Cart::new(usd());
        cart2
            .add_line(CartLine::new(Sku::new("TEA"), 1, price(200)))
            .unwrap();
        let sale2 = Sale::from_cart(&cart2).unwrap();
        store(&conn).create_sale(&sale2).unwrap();

        let sales = store(&conn).list_sales().unwrap();
        assert_eq!(sales.len(), 2);
        // Most recent first.
        assert_eq!(sales[0].id, sale2.id);
        assert_eq!(sales[1].id, sale.id);
        // Lines should be empty (not loaded).
        assert!(sales[0].lines.is_empty());
    }

    #[test]
    fn get_sale_not_found() {
        let conn = fresh();
        let result = store(&conn).get_sale("nope").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_sale_status_active() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let updated = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        assert_eq!(updated.status, SaleStatus::Active);
        assert!(!updated.updated_at.is_empty());
    }

    #[test]
    fn update_sale_status_full_flow() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        // Pending -> Active.
        let s = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        assert_eq!(s.status, SaleStatus::Active);

        // Active -> Completed.
        let s = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();
        assert_eq!(s.status, SaleStatus::Completed);

        // Terminal -> rejected (Completed -> Voided is invalid).
        let err = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Voided)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));
    }

    #[test]
    fn update_sale_status_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_sale_status("nope", SaleStatus::Active)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_sale_status_invalid_transition() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        // Pending -> Completed is invalid.
        let err = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));
    }

    // ── Export / Report ───────────────────────────────────────────

    #[test]
    fn export_daily_summary_empty() {
        let conn = fresh();
        let rows = store(&conn).export_daily_summary().unwrap();
        assert!(rows.is_empty(), "no sales today → empty");
    }

    #[test]
    fn export_sales_by_hour_empty() {
        let conn = fresh();
        let rows = store(&conn).export_sales_by_hour().unwrap();
        assert!(rows.is_empty());
    }

    // ── Held Carts ───────────────────────────────────────────────

    #[test]
    fn hold_cart_creates_and_list() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart("Cart 1", r#"{"items":[]}"#, 0, 0, "USD", "hold", None)
            .unwrap();
        assert!(!id.is_empty());

        let carts = s.list_held_carts().unwrap();
        assert_eq!(carts.len(), 1);
        assert_eq!(carts[0].label, "Cart 1");
        assert_eq!(carts[0].total_minor, 0);
    }

    #[test]
    fn hold_cart_with_items() {
        let conn = fresh();
        let s = store(&conn);
        s.hold_cart(
            "Active Cart",
            r#"{"lines":[{"sku":"COFFEE","qty":2}]}"#,
            2,
            700,
            "USD",
            "hold",
            None,
        )
        .unwrap();

        let carts = s.list_held_carts().unwrap();
        assert_eq!(carts.len(), 1);
        assert_eq!(carts[0].item_count, 2);
        assert_eq!(carts[0].total_minor, 700);
        assert_eq!(carts[0].currency, "USD");
    }

    #[test]
    fn get_held_cart_found() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart(
                "Test Cart",
                "{\"data\":\"value\"}",
                3,
                1500,
                "EUR",
                "hold",
                None,
            )
            .unwrap();

        let full = s.get_held_cart(&id).unwrap().unwrap();
        assert_eq!(full.label, "Test Cart");
        assert_eq!(full.cart_data, "{\"data\":\"value\"}");
        assert_eq!(full.item_count, 3);
        assert_eq!(full.total_minor, 1500);
        assert_eq!(full.currency, "EUR");
        assert!(!full.created_at.is_empty());
    }

    #[test]
    fn get_held_cart_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.get_held_cart("nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_held_carts_empty() {
        let conn = fresh();
        let s = store(&conn);
        let carts = s.list_held_carts().unwrap();
        assert!(carts.is_empty());
    }

    #[test]
    fn delete_held_cart_removes() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart("Delete Me", "{}", 0, 0, "USD", "hold", None)
            .unwrap();
        s.delete_held_cart(&id).unwrap();
        let result = s.get_held_cart(&id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_held_cart_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.delete_held_cart("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "held_cart"));
    }

    #[test]
    fn hold_cart_strips_label_whitespace() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart("  My Cart  ", "{}", 0, 0, "USD", "hold", None)
            .unwrap();
        let full = s.get_held_cart(&id).unwrap().unwrap();
        assert_eq!(full.label, "My Cart", "label should be trimmed");
    }

    // ── Open Bills ───────────────────────────────────────────────

    #[test]
    fn open_bill_persists_across_shifts() {
        let conn = fresh();
        let s = store(&conn);

        // Seed two users and a terminal.
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
               ('role-cashier', 'cashier', 'Cashier', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
               ('user-morning', 'alice', 'hash', 'Alice', 'role-cashier', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
               ('user-evening', 'bob', 'hash', 'Bob', 'role-cashier', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();

        // ── Morning shift ──
        let shift_morning = s.open_shift("user-morning", None, 200).unwrap();

        // Create an open bill.
        let _bill_id = s
            .hold_cart(
                "Table 4 — John",
                r#"{"lines":[{"sku":"STEAK","qty":1,"unit_price":1500}]}"#,
                1,
                1500,
                "USD",
                "open_bill",
                Some("John"),
            )
            .unwrap();

        // Open bill shows up immediately.
        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1, "open bill visible in same shift");
        assert_eq!(open[0].customer_name.as_deref(), Some("John"));

        // Close morning shift.
        s.close_shift(&shift_morning.id, 1700, None).unwrap();

        // ── Evening shift (different user) ──
        let _shift_evening = s.open_shift("user-evening", None, 500).unwrap();

        // The open bill is still listed — it is NOT scoped to a shift.
        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1, "open bill visible across shifts");
        assert_eq!(open[0].customer_name.as_deref(), Some("John"));
        assert_eq!(open[0].total_minor, 1500);
        assert_eq!(open[0].currency, "USD");
    }

    #[test]
    fn open_bill_list_excludes_hold_carts() {
        let conn = fresh();
        let s = store(&conn);

        s.hold_cart("Hold 1", "{}", 0, 0, "USD", "hold", None)
            .unwrap();
        s.hold_cart("Hold 2", "{}", 0, 0, "USD", "hold", None)
            .unwrap();
        s.hold_cart(
            "Table 7 — Mary",
            r#"{"lines":[]}"#,
            2,
            850,
            "USD",
            "open_bill",
            Some("Mary"),
        )
        .unwrap();

        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1, "only open bills, not hold carts");
        assert_eq!(open[0].customer_name.as_deref(), Some("Mary"));
    }

    #[test]
    fn open_bill_created_without_customer_name() {
        let conn = fresh();
        let s = store(&conn);

        let id = s
            .hold_cart("Walk-in", "{}", 0, 0, "USD", "open_bill", None)
            .unwrap();

        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1);
        assert!(open[0].customer_name.is_none());
        assert_eq!(open[0].label, "Walk-in");

        // Verify full record.
        let full = s.get_held_cart(&id).unwrap().unwrap();
        assert_eq!(full.bill_type, "open_bill");
        assert!(full.customer_name.is_none());
    }

    // ── Void Sale ────────────────────────────────────────────────

    #[test]
    fn void_sale_changes_status_and_restores_stock() {
        let conn = fresh();
        let s = store(&conn);

        // Create a product with stock.
        let currency: crate::money::Currency = "USD".parse().unwrap();
        let money = crate::Money {
            minor_units: 500,
            currency,
        };
        s.create_product("VOID-SKU", "Voidable", money, None, None, 10, None)
            .unwrap();

        // Create an active sale with the product.
        let cart = make_cart(); // COFFEE x 2 (350) + BAGEL x 1 (450)
        let sale = Sale::from_cart(&cart).unwrap();
        // We need to set the SKU to match our seeded product for stock tracking
        let sale_with_stock = Sale {
            id: sale.id.clone(),
            total: sale.total,
            currency: sale.currency,
            line_count: 1,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: Some("user-1".into()),
            created_at: sale.created_at.clone(),
            updated_at: sale.updated_at.clone(),
            subtotal: sale.total,
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![SaleLine {
                id: uuid::Uuid::now_v7().to_string(),
                sale_id: sale.id.clone(),
                sku: "VOID-SKU".into(),
                qty: 2,
                unit_price: price(500),
                line_total: price(1000),
                line_position: 1,
                tax_amount: price(0),
                tax_rate_id: None,
                serial_number: None,
            }],
        };

        s.create_sale(&sale_with_stock).unwrap();
        // Move to Active so it can be voided.
        s.update_sale_status(&sale_with_stock.id, SaleStatus::Active)
            .unwrap();

        // Void the sale.
        s.void_sale(&sale_with_stock.id, "user-2", "customer request")
            .unwrap();

        // Verify status is voided.
        let loaded = s.get_sale(&sale_with_stock.id).unwrap().unwrap();
        assert_eq!(loaded.status, SaleStatus::Voided);

        // Verify stock was restored (10 + 2 = 12).
        let product_id = s.product_id_by_sku("VOID-SKU").unwrap().unwrap();
        let stock = s.get_stock(&product_id).unwrap();
        assert_eq!(stock, 12, "stock should be restored after void");

        // Verify audit log entry was created.
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.void' AND target_id = ?1",
                rusqlite::params![sale_with_stock.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 1);
    }

    #[test]
    fn void_sale_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.void_sale("nonexistent", "user-1", "test").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn void_sale_only_active_can_be_voided() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        // Sale is Pending, not Active — void should fail with validation error.
        let err = s.void_sale(&sale.id, "user-1", "test").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn void_sale_completed_cannot_be_voided() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        // Move to Active, then Completed.
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let err = s.void_sale(&sale.id, "user-1", "test").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    // ── Export with data ─────────────────────────────────────────

    #[test]
    fn export_daily_summary_with_sales() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        // Export uses date('now') so it should find the sale we just created.
        let rows = s.export_daily_summary().unwrap();
        assert!(!rows.is_empty(), "should find today's sale");
        assert_eq!(rows[0].total_minor, 1150);
    }

    #[test]
    fn create_sale_with_user_and_discount() {
        let conn = fresh();
        let s = store(&conn);
        let mut cart = make_cart();
        cart.set_discount(
            foundation::Percentage::new(10).unwrap(),
            Some("Loyalty".into()),
        );
        let sale = Sale::from_cart(&cart).unwrap();
        // Add user_id to the sale.
        let sale_with_user = Sale {
            user_id: Some("cashier-1".into()),
            customer_id: None,
            version: 1,
            ..sale
        };
        s.create_sale(&sale_with_user).unwrap();

        let loaded = s.get_sale(&sale_with_user.id).unwrap().unwrap();
        assert_eq!(loaded.user_id, Some("cashier-1".into()));
    }

    #[test]
    fn create_sale_discount_persisted() {
        let conn = fresh();
        let s = store(&conn);
        let mut cart = make_cart();
        cart.set_discount(foundation::Percentage::new(15).unwrap(), Some("VIP".into()));
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.discount_percent, 15);
        assert_eq!(loaded.discount_label, Some("VIP".into()));
    }

    // ── Export with data ─────────────────────────────────────────

    #[test]
    fn export_sales_by_hour_with_sales() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let rows = s.export_sales_by_hour().unwrap();
        assert!(!rows.is_empty(), "should find today's hourly aggregation");
        assert_eq!(rows[0].sale_count, 1);
        assert_eq!(rows[0].total_minor, 1150);
    }

    // ── Status transition edge cases ─────────────────────────────

    #[test]
    fn update_sale_status_invalid_stored_status() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        // Manually corrupt the status in the DB.
        conn.execute(
            "UPDATE sales SET status = 'bogus_status' WHERE id = ?1",
            rusqlite::params![sale.id],
        )
        .unwrap();

        let err = s
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    // ── Void edge cases ──────────────────────────────────────────

    #[test]
    fn void_sale_with_unknown_sku() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart(); // COFFEE x 2 (350) + BAGEL x 1 (450)
        let sale = Sale::from_cart(&cart).unwrap();
        // Do NOT create products — product_id_by_sku will return None.
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();

        // Void should succeed and skip stock restoration for unknown SKUs.
        let result = s.void_sale(&sale.id, "user-1", "no product record");
        assert!(
            result.is_ok(),
            "void should succeed even when SKU has no product record"
        );
        let loaded = result.unwrap();
        assert_eq!(loaded.status, SaleStatus::Voided);
    }

    // ── Tax Computation ─────────────────────────────

    fn seed_tax_rate(
        conn: &Connection,
        name: &str,
        rate_bps: i64,
        is_default: bool,
        is_inclusive: bool,
    ) -> String {
        let s = store(conn);
        s.create_tax_rate(name, rate_bps, is_default, is_inclusive)
            .unwrap()
            .id
    }

    fn seed_product_with_category(conn: &Connection, sku: &str, category_id: Option<&str>) {
        let s = store(conn);
        let currency: crate::money::Currency = "USD".parse().unwrap();
        let money = crate::Money {
            minor_units: 1000,
            currency,
        };
        s.create_product(sku, sku, money, category_id, None, 100, None)
            .unwrap();
    }

    fn make_single_line_sale(sku: &str, qty: i64, unit_minor: i64) -> Sale {
        let line_id = uuid::Uuid::now_v7().to_string();
        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Sale {
            id: sale_id.clone(),
            total: price(unit_minor * qty),
            currency: usd(),
            line_count: 1,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(unit_minor * qty),
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![SaleLine {
                id: line_id,
                sale_id,
                sku: sku.into(),
                qty,
                unit_price: price(unit_minor),
                line_total: price(unit_minor * qty),
                line_position: 1,
                tax_amount: price(0),
                tax_rate_id: None,
                serial_number: None,
            }],
        }
    }

    #[test]
    fn compute_tax_no_rates() {
        let conn = fresh();
        let s = store(&conn);
        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        assert_eq!(sale.subtotal.minor_units, 700);
        assert_eq!(sale.tax_total.minor_units, 0);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 0);
        assert!(sale.lines[0].tax_rate_id.is_none());
    }

    #[test]
    fn compute_tax_default_rate_exclusive() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // exclusive: tax = 700 * 1000 / 10000 = 70
        assert_eq!(sale.subtotal.minor_units, 700);
        assert_eq!(sale.tax_total.minor_units, 70);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 70);
        assert!(sale.lines[0].tax_rate_id.is_some());
    }

    #[test]
    fn compute_tax_default_rate_inclusive() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "GST 10%", 1000, true, true);

        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // inclusive: tax = 700 * 1000 / (10000 + 1000) = 700000 / 11000 = 63
        assert_eq!(sale.subtotal.minor_units, 700);
        assert_eq!(sale.tax_total.minor_units, 63);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 63);
    }

    #[test]
    fn compute_tax_product_level_wins() {
        let conn = fresh();
        let s = store(&conn);
        let _default_id = seed_tax_rate(&conn, "Default 5%", 500, true, false);
        let product_id = seed_tax_rate(&conn, "Product 10%", 1000, false, false);
        seed_product_with_category(&conn, "COFFEE", None);
        s.set_product_tax_rates("COFFEE", std::slice::from_ref(&product_id))
            .unwrap();

        let mut sale = make_single_line_sale("COFFEE", 1, 1000);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // product rate (10%) wins over default (5%): tax = 1000 * 1000 / 10000 = 100
        assert_eq!(sale.tax_total.minor_units, 100);
        assert_eq!(
            sale.lines[0].tax_rate_id.as_deref(),
            Some(product_id.as_str())
        );
    }

    #[test]
    fn compute_tax_category_level_wins_over_default() {
        let conn = fresh();
        let s = store(&conn);
        let _default_id = seed_tax_rate(&conn, "Default 5%", 500, true, false);
        let cat_id = seed_tax_rate(&conn, "Category 8%", 800, false, false);
        s.create_category("cat-1", "Beverages", "#fff", "").unwrap();
        s.set_category_tax_rates("cat-1", std::slice::from_ref(&cat_id))
            .unwrap();
        seed_product_with_category(&conn, "COFFEE", Some("cat-1"));

        let mut sale = make_single_line_sale("COFFEE", 1, 1000);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // category rate (8%) wins over default (5%): tax = 1000 * 800 / 10000 = 80
        assert_eq!(sale.tax_total.minor_units, 80);
        assert_eq!(sale.lines[0].tax_rate_id.as_deref(), Some(cat_id.as_str()));
    }

    #[test]
    fn compute_tax_multi_line() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

        let line1 = SaleLine {
            id: uuid::Uuid::now_v7().to_string(),
            sale_id: "sale-1".into(),
            sku: "COFFEE".into(),
            qty: 2,
            unit_price: price(350),
            line_total: price(700),
            line_position: 1,
            tax_amount: price(0),
            tax_rate_id: None,
            serial_number: None,
        };
        let line2 = SaleLine {
            id: uuid::Uuid::now_v7().to_string(),
            sale_id: "sale-1".into(),
            sku: "BAGEL".into(),
            qty: 1,
            unit_price: price(450),
            line_total: price(450),
            line_position: 2,
            tax_amount: price(0),
            tax_rate_id: None,
            serial_number: None,
        };
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut sale = Sale {
            id: "sale-1".into(),
            total: price(1150),
            currency: usd(),
            line_count: 2,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(1150),
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![line1, line2],
        };

        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // line1: 700 * 1000 / 10000 = 70
        // line2: 450 * 1000 / 10000 = 45
        // total tax = 115
        assert_eq!(sale.subtotal.minor_units, 1150);
        assert_eq!(sale.tax_total.minor_units, 115);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 70);
        assert_eq!(sale.lines[1].tax_amount.minor_units, 45);
    }

    #[test]
    fn compute_tax_persisted_after_create() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        s.create_sale(&sale).unwrap();

        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.subtotal.minor_units, 700);
        assert_eq!(loaded.tax_total.minor_units, 70);
        assert_eq!(loaded.lines[0].tax_amount.minor_units, 70);
        assert!(loaded.lines[0].tax_rate_id.is_some());
    }

    #[test]
    fn compute_tax_empty_sale_no_crash() {
        let conn = fresh();
        let s = store(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut sale = Sale {
            id: "empty".into(),
            total: price(0),
            currency: usd(),
            line_count: 0,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![],
        };
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        assert_eq!(sale.subtotal.minor_units, 0);
        assert_eq!(sale.tax_total.minor_units, 0);
    }

    #[test]
    fn void_sale_stock_overflow() {
        let conn = fresh();
        let s = store(&conn);
        let currency: crate::money::Currency = "USD".parse().unwrap();
        let money = crate::Money {
            minor_units: 500,
            currency,
        };
        // Create a product with near-max stock.
        s.create_product(
            "OVERFLOW-SKU",
            "Near Overflow",
            money,
            None,
            None,
            i64::MAX - 1,
            None,
        )
        .unwrap();

        let sale_id = uuid::Uuid::now_v7().to_string();
        let line_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let sale = Sale {
            id: sale_id.clone(),
            total: price(5000),
            currency: usd(),
            line_count: 1,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: Some("user-1".into()),
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(5000),
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![SaleLine {
                id: line_id,
                sale_id: sale_id.clone(),
                sku: "OVERFLOW-SKU".into(),
                qty: 10,
                unit_price: price(500),
                line_total: price(5000),
                line_position: 1,
                tax_amount: price(0),
                tax_rate_id: None,
                serial_number: None,
            }],
        };
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale_id, SaleStatus::Active).unwrap();

        // Stock restore would overflow i64::MAX - 1 + 10.
        let err = s
            .void_sale(&sale_id, "user-1", "trigger overflow")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "qty"));
    }
}
