//! Inventory Repository — database queries for products, categories, and stock levels.

use crate::models::{Inventory, LocationId, Product, ProductType};
use foundation::{Barcode, Currency, Money, Sku};
use rusqlite::{Connection, Transaction, params};

/// Repository for inventory and product database operations.
pub struct InventoryRepository<'a> {
    conn: &'a Connection,
}

impl<'a> InventoryRepository<'a> {
    /// Create a new `InventoryRepository` borrowing a SQLite connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve a product by ID.
    pub fn get_product(&self, id: &str) -> Result<Option<Product>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial, product_type, version
             FROM products WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        let row = match rows.next()? {
            Some(r) => r,
            None => return Ok(None),
        };

        let currency_str: String = row.get(4)?;
        let currency: Currency = currency_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid currency"))?;
        let price_minor: i64 = row.get(3)?;
        let sku_str: String = row.get(1)?;
        let sku = Sku::try_new(sku_str).ok_or_else(|| anyhow::anyhow!("Invalid SKU"))?;

        let barcode_str: Option<String> = row.get(6)?;
        let barcode = barcode_str.and_then(|b| Barcode::new(b).ok());

        let ptype_str: String = row.get(11).unwrap_or_else(|_| "retail".to_string());
        let product_type = ProductType::parse_str(&ptype_str).unwrap_or_default();

        Ok(Some(Product {
            id: row.get(0)?,
            sku,
            name: row.get(2)?,
            price: Money {
                minor_units: price_minor,
                currency,
            },
            category_id: row.get(5)?,
            barcode,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            price_updated_at: row.get(9)?,
            track_serial: row.get::<_, i64>(10).unwrap_or(0) != 0,
            product_type,
            version: row.get(12).unwrap_or(1),
        }))
    }

    /// Retrieve product stock level for a SKU.
    pub fn get_stock(&self, sku: &Sku) -> Result<Option<Inventory>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT product_id, sku, qty, low_stock_threshold, updated_at, location_id
             FROM inventory WHERE sku = ?1",
        )?;

        let mut rows = stmt.query(params![sku.as_str()])?;
        let row = match rows.next()? {
            Some(r) => r,
            None => return Ok(None),
        };

        let sku_str: String = row.get(1)?;
        let sku = Sku::try_new(sku_str).ok_or_else(|| anyhow::anyhow!("Invalid SKU"))?;
        let loc_str: String = row.get(5).unwrap_or_default();

        Ok(Some(Inventory {
            product_id: row.get(0)?,
            sku,
            qty: row.get(2)?,
            low_stock_threshold: row.get(3)?,
            updated_at: row.get(4)?,
            location_id: LocationId::from(loc_str),
        }))
    }

    /// Adjust stock level for a product inside a transaction.
    pub fn adjust_stock_tx(
        &self,
        tx: &Transaction,
        sku: &Sku,
        delta: i64,
    ) -> Result<(), anyhow::Error> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        tx.execute(
            "UPDATE inventory SET qty = qty + ?1, updated_at = ?2 WHERE sku = ?3",
            params![delta, now, sku.as_str()],
        )?;
        Ok(())
    }
}
