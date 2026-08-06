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
            "SELECT id, sale_id, sku, qty, unit_minor, line_minor, line_position, tax_minor, tax_rate_id, tax_breakdown_json, serial_number, course, modifiers_json
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
                tax_breakdown_json: r.get(9)?,
                serial_number: r.get(10)?,
                course: r.get(11)?,
                modifiers_json: r.get(12)?,
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
                "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, line_position, tax_minor, tax_rate_id, tax_breakdown_json, serial_number, course, modifiers_json, currency)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
                    line.tax_breakdown_json,
                    line.serial_number,
                    line.course,
                    line.modifiers_json,
                    line.unit_price.currency.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::{Cart, CartLine, Sku};
    use rusqlite::Connection;

    fn fresh() -> Connection {
        oz_core::migrations::fresh_db()
    }

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn sample_sale() -> Sale {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("COFFEE"),
            2,
            Money {
                minor_units: 350,
                currency: usd(),
            },
        ))
        .unwrap();
        cart.add_line(CartLine::new(
            Sku::new("CAKE"),
            1,
            Money {
                minor_units: 500,
                currency: usd(),
            },
        ))
        .unwrap();
        Sale::from_cart_with_user(&cart, Some("u-42".to_string())).unwrap()
    }

    #[test]
    fn get_sale_missing_returns_none() {
        let conn = fresh();
        let repo = SalesRepository::new(&conn);
        assert!(repo.get_sale("does-not-exist").unwrap().is_none());
    }

    #[test]
    fn create_sale_then_get_roundtrip() {
        let mut conn = fresh();
        let mut sale = sample_sale();
        sale.payment_method = Some("cash".to_string());
        sale.tendered_minor = Some(1500);

        let tx = conn.transaction().unwrap();
        SalesRepository::new(&tx)
            .create_sale_tx(&tx, &sale)
            .unwrap();
        tx.commit().unwrap();

        let repo = SalesRepository::new(&conn);
        let fetched = repo.get_sale(&sale.id).unwrap().expect("sale must exist");

        assert_eq!(fetched.id, sale.id);
        assert_eq!(fetched.status, sale.status);
        assert_eq!(fetched.total, sale.total);
        assert_eq!(fetched.currency, sale.currency);
        assert_eq!(fetched.line_count, 2);
        assert_eq!(fetched.payment_method.as_deref(), Some("cash"));
        assert_eq!(fetched.tendered_minor, Some(1500));
        assert_eq!(fetched.user_id.as_deref(), Some("u-42"));
        assert_eq!(fetched.version, 1);
        assert_eq!(fetched.lines.len(), 2);
        assert_eq!(fetched.lines[0].sku, "COFFEE");
        assert_eq!(fetched.lines[0].qty, 2);
        assert_eq!(fetched.lines[0].line_position, 1);
        assert_eq!(fetched.lines[0].unit_price.minor_units, 350);
        assert_eq!(fetched.lines[0].line_total.minor_units, 700);
        assert_eq!(fetched.lines[1].sku, "CAKE");
        assert_eq!(fetched.lines[1].line_position, 2);
        assert_eq!(fetched.lines[1].line_total.minor_units, 500);
    }

    #[test]
    fn create_sale_persists_tax_and_breakdown_fields() {
        let mut conn = fresh();
        // tax_rate_id has a FK to tax_rates(id), so seed a matching row.
        conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default) VALUES ('rate-1', 'Sales Tax', 1000, 1)",
            [],
        )
        .unwrap();
        let mut sale = sample_sale();
        sale.lines[0].tax_amount = Money {
            minor_units: 35,
            currency: usd(),
        };
        sale.lines[0].tax_rate_id = Some("rate-1".to_string());
        sale.lines[0].tax_breakdown_json =
            Some("[{\"rate_id\":\"rate-1\",\"rate_bps\":1000}]".to_string());
        sale.lines[0].serial_number = Some("SN-123".to_string());
        sale.lines[0].course = Some("main".to_string());
        sale.lines[0].modifiers_json = Some("[{\"name\":\"Temp\",\"choice\":\"Hot\"}]".to_string());

        let tx = conn.transaction().unwrap();
        SalesRepository::new(&tx)
            .create_sale_tx(&tx, &sale)
            .unwrap();
        tx.commit().unwrap();

        let repo = SalesRepository::new(&conn);
        let fetched = repo.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(fetched.lines[0].tax_amount.minor_units, 35);
        assert_eq!(fetched.lines[0].tax_rate_id.as_deref(), Some("rate-1"));
        assert_eq!(
            fetched.lines[0].tax_breakdown_json.as_deref(),
            Some("[{\"rate_id\":\"rate-1\",\"rate_bps\":1000}]")
        );
        assert_eq!(fetched.lines[0].serial_number.as_deref(), Some("SN-123"));
        assert_eq!(fetched.lines[0].course.as_deref(), Some("main"));
        assert_eq!(
            fetched.lines[0].modifiers_json.as_deref(),
            Some("[{\"name\":\"Temp\",\"choice\":\"Hot\"}]")
        );
    }

    #[test]
    fn get_sale_orders_lines_by_position() {
        let mut conn = fresh();
        let mut sale = sample_sale();
        // Reverse positions in memory to prove the query orders on read.
        sale.lines.reverse();
        let tx = conn.transaction().unwrap();
        SalesRepository::new(&tx)
            .create_sale_tx(&tx, &sale)
            .unwrap();
        tx.commit().unwrap();

        let repo = SalesRepository::new(&conn);
        let fetched = repo.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(fetched.lines[0].sku, "COFFEE");
        assert_eq!(fetched.lines[0].line_position, 1);
        assert_eq!(fetched.lines[1].sku, "CAKE");
        assert_eq!(fetched.lines[1].line_position, 2);
    }

    #[test]
    fn get_sale_rejects_invalid_currency() {
        let mut conn = fresh();
        let sale = sample_sale();
        let tx = conn.transaction().unwrap();
        SalesRepository::new(&tx)
            .create_sale_tx(&tx, &sale)
            .unwrap();
        tx.commit().unwrap();

        // Corrupt the currency code so parsing fails on read.
        conn.execute(
            "UPDATE sales SET currency = 'ZZ' WHERE id = ?1",
            params![sale.id],
        )
        .unwrap();

        let repo = SalesRepository::new(&conn);
        assert!(repo.get_sale(&sale.id).is_err());
    }

    #[test]
    fn update_sale_status_changes_status_and_bumps_version() {
        let mut conn = fresh();
        let sale = sample_sale();
        let tx = conn.transaction().unwrap();
        SalesRepository::new(&tx)
            .create_sale_tx(&tx, &sale)
            .unwrap();
        tx.commit().unwrap();

        let repo = SalesRepository::new(&conn);
        repo.update_sale_status(&sale.id, SaleStatus::Voided)
            .unwrap();

        let fetched = repo.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(fetched.status, SaleStatus::Voided);
        assert_eq!(fetched.version, 2);
    }

    #[test]
    fn update_sale_status_missing_id_is_noop() {
        let conn = fresh();
        let repo = SalesRepository::new(&conn);
        let result = repo.update_sale_status("missing", SaleStatus::Voided);
        assert!(result.is_ok());
    }
}
