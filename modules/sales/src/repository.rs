//! Sales Repository — database persistence layer for sales, held carts, and refunds.

use foundation::{Currency, Money, SaleStatus};
use rusqlite::{Connection, Transaction, params};

use crate::models::{Sale, SaleLine};

/// Database access repository for sales data.
pub struct SalesRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SalesRepository<'a> {
    /// Create a new `SalesRepository` borrowing a SQLite connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve a sale by ID including its line items.
    pub fn get_sale(&self, id: &str) -> Result<Option<Sale>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, status, total_minor, line_count, currency, payment_method, tendered_minor, user_id, created_at, updated_at, discount_percent, discount_label, subtotal_minor, tax_total_minor, customer_id, version
             FROM sales WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        let row = match rows.next()? {
            Some(r) => r,
            None => return Ok(None),
        };

        let currency_str: String = row.get(4)?;
        let currency: Currency = currency_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid currency code: {}", currency_str))?;

        let status_str: String = row.get(1)?;
        let status: SaleStatus =
            serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or(SaleStatus::Pending);

        let total_minor: i64 = row.get(2)?;
        let total = Money {
            minor_units: total_minor,
            currency,
        };

        let subtotal_minor: i64 = row.get(12).unwrap_or(total_minor);
        let subtotal = Money {
            minor_units: subtotal_minor,
            currency,
        };

        let tax_total_minor: i64 = row.get(13).unwrap_or(0);
        let tax_total = Money {
            minor_units: tax_total_minor,
            currency,
        };

        let mut line_stmt = self.conn.prepare(
            "SELECT id, sale_id, sku, qty, unit_minor, line_minor, line_position, tax_amount_minor, tax_rate_id, serial_number
             FROM sale_lines WHERE sale_id = ?1 ORDER BY line_position ASC",
        )?;

        let line_rows = line_stmt.query_map(params![id], |r| {
            let unit_minor: i64 = r.get(4)?;
            let line_minor: i64 = r.get(5)?;
            let tax_amount_minor: i64 = r.get(7).unwrap_or(0);

            Ok(SaleLine {
                id: r.get(0)?,
                sale_id: r.get(1)?,
                sku: r.get(2)?,
                qty: r.get(3)?,
                unit_price: Money {
                    minor_units: unit_minor,
                    currency,
                },
                line_total: Money {
                    minor_units: line_minor,
                    currency,
                },
                line_position: r.get(6)?,
                tax_amount: Money {
                    minor_units: tax_amount_minor,
                    currency,
                },
                tax_rate_id: r.get(8)?,
                serial_number: r.get(9)?,
            })
        })?;

        let mut lines = Vec::new();
        for line_res in line_rows {
            lines.push(line_res?);
        }

        Ok(Some(Sale {
            id: row.get(0)?,
            status,
            total,
            line_count: row.get(3)?,
            currency,
            payment_method: row.get(5)?,
            tendered_minor: row.get(6)?,
            user_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            lines,
            discount_percent: row.get(10).unwrap_or(0),
            discount_label: row.get(11)?,
            subtotal,
            tax_total,
            customer_id: row.get(14)?,
            version: row.get(15).unwrap_or(1),
        }))
    }

    /// Insert a new sale and its line items inside a transaction.
    pub fn create_sale_tx(&self, tx: &Transaction, sale: &Sale) -> Result<(), anyhow::Error> {
        let status_str = serde_json::to_string(&sale.status)?
            .trim_matches('"')
            .to_string();
        tx.execute(
            "INSERT INTO sales (id, status, total_minor, line_count, currency, payment_method, tendered_minor, user_id, created_at, updated_at, discount_percent, discount_label, subtotal_minor, tax_total_minor, customer_id, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                sale.id,
                status_str,
                sale.total.minor_units,
                sale.line_count,
                sale.currency.to_string(),
                sale.payment_method,
                sale.tendered_minor,
                sale.user_id,
                sale.created_at,
                sale.updated_at,
                sale.discount_percent,
                sale.discount_label,
                sale.subtotal.minor_units,
                sale.tax_total.minor_units,
                sale.customer_id,
                sale.version,
            ],
        )?;

        for line in &sale.lines {
            tx.execute(
                "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, line_position, tax_amount_minor, tax_rate_id, serial_number)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    line.id,
                    line.sale_id,
                    line.sku,
                    line.qty,
                    line.unit_price.minor_units,
                    line.line_total.minor_units,
                    line.line_position,
                    line.tax_amount.minor_units,
                    line.tax_rate_id,
                    line.serial_number,
                ],
            )?;
        }

        Ok(())
    }

    /// Update sale status.
    pub fn update_sale_status(&self, id: &str, status: SaleStatus) -> Result<(), anyhow::Error> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let status_str = serde_json::to_string(&status)?
            .trim_matches('"')
            .to_string();
        self.conn.execute(
            "UPDATE sales SET status = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![status_str, now, id],
        )?;
        Ok(())
    }
}
