//! Database facade — typed CRUD for every domain entity.
//!
//! The [`Store`] is a lightweight borrow-wrapper around a
//! `&rusqlite::Connection`. It holds no state of its own; callers
//! create a `Store` on the fly and call methods that map directly to
//! SQL queries. All writes that touch more than one row use
//! `unchecked_transaction` for atomicity.
//!
//! # Example
//!
//! ```ignore
//! use oz_core::db::Store;
//!
//! let conn = Connection::open_in_memory()?;
//! migrations::run(&mut conn)?;
//!
//! let store = Store::new(&conn);
//! let products = store.list_products()?;
//! ```

use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::error::CoreError;
use crate::{Category, Money, Product, Sale, SaleLine, SaleStatus, Settings, Sku};

// ── Store ────────────────────────────────────────────────────────────

/// Typed CRUD facade for the OZ-POS database.
///
/// All methods borrow `&self` and operate on the underlying
/// [`Connection`] directly. The caller is responsible for
/// synchronisation (e.g. `Mutex<Connection>`) and transaction
/// boundaries for multi-statement workflows.
pub struct Store<'a> {
    conn: &'a Connection,
}

impl<'a> Store<'a> {
    /// Wrap an existing connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Return a reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        self.conn
    }
}

// ── Product helpers (internal) ───────────────────────────────────────

/// Build a [`Product`] from a `rusqlite::Row`. All 9 `products` columns
/// must be present in the result set.
fn row_to_product(row: &rusqlite::Row) -> rusqlite::Result<Product> {
    let sku_str: String = row.get("sku")?;
    let cur_str: String = row.get("currency")?;
    Ok(Product {
        id: row.get("id")?,
        sku: Sku::new(sku_str),
        name: row.get("name")?,
        price: Money {
            minor_units: row.get("price_minor")?,
            currency: cur_str.parse().expect("valid currency in database"),
        },
        category_id: row.get("category_id")?,
        barcode: row.get("barcode")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

// ── Enriched product type ────────────────────────────────────────────

/// A [`Product`] enriched with category name and stock quantity from
/// LEFT JOINs on `categories` and `inventory`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProductWithDetails {
    /// The core product fields (flattened into the parent JSON).
    #[serde(flatten)]
    pub product: Product,
    /// Display name from `categories.name`, if linked.
    pub category_name: Option<String>,
    /// Current stock from `inventory.qty`, if an inventory row exists.
    pub stock_qty: Option<i64>,
}

fn row_to_product_with_details(row: &rusqlite::Row) -> rusqlite::Result<ProductWithDetails> {
    let product = row_to_product(row)?;
    Ok(ProductWithDetails {
        product,
        category_name: row.get("category_name")?,
        stock_qty: row.get("stock_qty")?,
    })
}

// ── Product CRUD ─────────────────────────────────────────────────────

impl Store<'_> {
    /// List all products, ordered by name, with category and stock.
    pub fn list_products(&self) -> Result<Vec<ProductWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                    p.category_id, p.barcode, p.created_at, p.updated_at,
                    c.name AS category_name,
                    i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map([], row_to_product_with_details)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single product by SKU, including category and stock.
    ///
    /// Returns `None` when no product matches the SKU.
    pub fn get_product(&self, sku: &str) -> Result<Option<ProductWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                    p.category_id, p.barcode, p.created_at, p.updated_at,
                    c.name AS category_name,
                    i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.sku = ?1",
        )?;
        let result = stmt.query_row(params![sku], row_to_product_with_details);
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new product and optionally an inventory row.
    ///
    /// Returns the created [`Product`] (without category/stock enrichment).
    /// If `initial_stock > 0`, an `inventory` row is inserted in the same
    /// operation.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Conflict`] when the SKU or barcode already
    /// exists.
    pub fn create_product(
        &self,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
        initial_stock: i64,
    ) -> Result<Product, CoreError> {
        // --- Validation (mirrors the API handler) ---
        if sku.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "sku",
                message: "SKU must not be empty".into(),
            });
        }
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "name must not be empty".into(),
            });
        }
        if price.minor_units < 0 {
            return Err(CoreError::Validation {
                field: "price",
                message: "price must be ≥ 0".into(),
            });
        }
        if initial_stock < 0 {
            return Err(CoreError::Validation {
                field: "initial_stock",
                message: "initial_stock must be ≥ 0".into(),
            });
        }

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let cur_str = std::str::from_utf8(&price.currency.0)
            .expect("currency bytes are valid UTF-8")
            .to_owned();

        // Use a transaction so product + inventory insert is atomic.
        let tx = self.conn.unchecked_transaction()?;

        let result = tx.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                sku.trim(),
                name.trim(),
                price.minor_units,
                cur_str,
                category_id,
                barcode,
                now,
                now,
            ],
        );

        match result {
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(CoreError::Conflict {
                    entity: "product",
                    field: "sku or barcode",
                });
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        // Insert inventory row if initial stock > 0.
        if initial_stock > 0 {
            tx.execute(
                "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)",
                params![id, initial_stock, now],
            )?;
        }

        tx.commit()?;

        Ok(Product {
            id,
            sku: Sku::new(sku.trim()),
            name: name.trim().to_owned(),
            price,
            category_id: category_id.map(|s| s.to_owned()),
            barcode: barcode.map(|s| s.to_owned()),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing product identified by SKU.
    ///
    /// All fields are required (read the current values first with
    /// [`get_product`], then submit the updated version). Returns the
    /// updated [`Product`] without category/stock enrichment.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the SKU doesn't match.
    /// Returns [`CoreError::Conflict`] when the new SKU or barcode
    /// clashes with another product.
    /// Returns [`CoreError::Validation`] when name or price is invalid.
    pub fn update_product(
        &self,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
    ) -> Result<Product, CoreError> {
        // Validation.
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "name must not be empty".into(),
            });
        }
        if price.minor_units < 0 {
            return Err(CoreError::Validation {
                field: "price",
                message: "price must be ≥ 0".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let cur_str = std::str::from_utf8(&price.currency.0)
            .expect("currency bytes are valid UTF-8")
            .to_owned();

        let rows = self.conn.execute(
            "UPDATE products
             SET name = ?1, price_minor = ?2, currency = ?3,
                 category_id = ?4, barcode = ?5, updated_at = ?6
             WHERE sku = ?7",
            params![
                name.trim(),
                price.minor_units,
                cur_str,
                category_id,
                barcode,
                now,
                sku,
            ],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            });
        }

        // Re-read to get the persisted product.
        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at
             FROM products WHERE sku = ?1",
        )?;
        let product = stmt.query_row(params![sku], row_to_product)?;
        Ok(product)
    }

    /// Delete a product by SKU, including its inventory row (CASCADE
    /// in the schema). Returns [`CoreError::NotFound`] when the SKU
    /// doesn't match any product.
    pub fn delete_product(&self, sku: &str) -> Result<(), CoreError> {
        let rows = self.conn.execute(
            "DELETE FROM products WHERE sku = ?1",
            params![sku],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            });
        }

        Ok(())
    }
}

// ── Category CRUD ─────────────────────────────────────────────────────

impl Store<'_> {
    /// List all categories, ordered by name.
    pub fn list_categories(&self) -> Result<Vec<Category>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, colour FROM categories ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: row.get("id")?,
                name: row.get("name")?,
                colour: row.get("colour")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Insert a new category.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Conflict`] when the name already exists
    /// (unique constraint).
    pub fn create_category(
        &self,
        id: &str,
        name: &str,
        colour: &str,
    ) -> Result<Category, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "category name must not be empty".into(),
            });
        }

        let result = self.conn.execute(
            "INSERT INTO categories (id, name, colour) VALUES (?1, ?2, ?3)",
            params![id, name.trim(), colour],
        );

        match result {
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(CoreError::Conflict {
                    entity: "category",
                    field: "name",
                });
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        Ok(Category::new(id, name, colour))
    }

    /// Look up a category by id.
    pub fn get_category(&self, id: &str) -> Result<Option<Category>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, colour FROM categories WHERE id = ?1")?;
        let result = stmt.query_row(params![id], |row| {
            Ok(Category {
                id: row.get("id")?,
                name: row.get("name")?,
                colour: row.get("colour")?,
            })
        });
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

// ── Inventory helpers ─────────────────────────────────────────────────

impl Store<'_> {
    /// Read the current stock quantity for a product.
    ///
    /// Returns `0` when no inventory row exists.
    pub fn get_stock(&self, product_id: &str) -> Result<i64, CoreError> {
        let result = self.conn.query_row(
            "SELECT qty FROM inventory WHERE product_id = ?1",
            params![product_id],
            |row| row.get(0),
        );
        match result {
            Ok(q) => Ok(q),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    /// Look up a product id by SKU.
    ///
    /// Returns `None` when no product matches the SKU.
    pub fn product_id_by_sku(&self, sku: &str) -> Result<Option<String>, CoreError> {
        let result = self.conn.query_row(
            "SELECT id FROM products WHERE sku = ?1",
            params![sku],
            |row| row.get(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Adjust stock for a product by SKU inside a transaction.
    ///
    /// Uses `checked_add` + `>= 0` guard (mirrors [`crate::Inventory::adjust_qty`]).
    /// Returns the new quantity on success.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the SKU doesn't match a product.
    /// Returns [`CoreError::Validation`] when the adjustment would cause
    /// negative stock or overflow.
    pub fn adjust_stock(&self, sku: &str, delta: i64) -> Result<i64, CoreError> {
        let product_id = self
            .product_id_by_sku(sku)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            })?;

        let previous_qty = self.get_stock(&product_id)?;

        let new_qty = previous_qty
            .checked_add(delta)
            .filter(|&v| v >= 0)
            .ok_or_else(|| CoreError::Validation {
                field: "delta",
                message: format!(
                    "adjustment would cause negative stock (previous: {previous_qty}, delta: {delta})"
                ),
            })?;

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty,
                                                    updated_at = excluded.updated_at",
            params![product_id, new_qty, now],
        )?;
        tx.commit()?;

        Ok(new_qty)
    }
}

// ── Sale CRUD ────────────────────────────────────────────────────

impl Store<'_> {
    /// Persist a [`Sale`] (header + all line items) inside a single
    /// transaction. The sale should have been created by
    /// [`Sale::from_cart`] so that ids, timestamps, and totals are
    /// already computed.
    pub fn create_sale(&self, sale: &Sale) -> Result<(), CoreError> {
        let cur_str = std::str::from_utf8(&sale.currency.0)
            .expect("currency bytes are valid UTF-8");
        let status_str = sale.status.as_stored_str();

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                sale.id,
                sale.total.minor_units,
                cur_str,
                sale.line_count,
                status_str,
                sale.created_at,
                sale.updated_at,
            ],
        )?;

        for line in &sale.lines {
            let unit_cur = std::str::from_utf8(&line.unit_price.currency.0)
                .expect("currency bytes are valid UTF-8");
            tx.execute(
                "INSERT INTO sale_lines
                    (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    line.id,
                    line.sale_id,
                    line.sku,
                    line.qty,
                    line.unit_price.minor_units,
                    line.line_total.minor_units,
                    unit_cur,
                    line.line_position,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Build a [`SaleLine`] from a rusqlite row that has all
    /// `sale_lines` columns.
    fn row_to_sale_line(row: &rusqlite::Row) -> rusqlite::Result<SaleLine> {
        let unit_cur_str: String = row.get("currency")?;
        Ok(SaleLine {
            id: row.get("id")?,
            sale_id: row.get("sale_id")?,
            sku: row.get("sku")?,
            qty: row.get("qty")?,
            unit_price: Money {
                minor_units: row.get("unit_minor")?,
                currency: unit_cur_str.parse().expect("valid currency in DB"),
            },
            line_total: Money {
                minor_units: row.get("line_minor")?,
                currency: unit_cur_str.parse().expect("valid currency in DB"),
            },
            line_position: row.get("line_position")?,
        })
    }

    /// Look up a single sale by id, including all line items.
    ///
    /// Returns `None` when no sale matches the id.
    pub fn get_sale(&self, id: &str) -> Result<Option<Sale>, CoreError> {
        let mut sale_stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status, created_at, updated_at
             FROM sales WHERE id = ?1",
        )?;

        let sale_result = sale_stmt.query_row(params![id], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let status = SaleStatus::from_stored_str(&status_str)
                .unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency: cur_str.parse().expect("valid currency in DB"),
                },
                line_count: row.get("line_count")?,
                currency: cur_str.parse().expect("valid currency in DB"),
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(), // filled below
            })
        });

        let mut sale = match sale_result {
            Ok(s) => s,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // Load lines.
        let mut line_stmt = self.conn.prepare(
            "SELECT id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position
             FROM sale_lines WHERE sale_id = ?1 ORDER BY line_position",
        )?;
        let line_rows = line_stmt.query_map(params![id], Self::row_to_sale_line)?;
        for line in line_rows {
            sale.lines.push(line?);
        }

        Ok(Some(sale))
    }

    /// Update the status of a sale, validating the state machine
    /// transition. Returns the updated [`Sale`] with all line items.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the sale id doesn't exist.
    /// Returns [`CoreError::Validation`] when the transition is
    /// invalid per the state machine.
    pub fn update_sale_status(
        &self,
        id: &str,
        to: SaleStatus,
    ) -> Result<Sale, CoreError> {
        // Read current status.
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

        let current = SaleStatus::from_stored_str(&current_str).ok_or_else(|| {
            CoreError::Validation {
                field: "status",
                message: format!("invalid stored status: {current_str}"),
            }
        })?;

        if !SaleStatus::can_transition_to(current, to) {
            return Err(CoreError::Validation {
                field: "status",
                message: format!(
                    "cannot transition from {:?} to {:?}",
                    current, to,
                ),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let status_str = to.as_stored_str();
        self.conn.execute(
            "UPDATE sales SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status_str, now, id],
        )?;

        self.get_sale(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: id.to_owned(),
        })
    }
}

// ── Settings delegation ───────────────────────────────────────────────

impl Store<'_> {
    /// Read a single setting. Delegates to [`Settings::get`].
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, CoreError> {
        Settings::get(self.conn, key)
    }

    /// Write a single setting. Delegates to [`Settings::set`].
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), CoreError> {
        Settings::set(self.conn, key, value)
    }

    /// Load the feature flag registry. Delegates to [`Settings::load_features`].
    pub fn load_features(&self) -> Result<crate::FeatureRegistry, CoreError> {
        Settings::load_features(self.conn)
    }

    /// Save the feature flag registry. Delegates to [`Settings::save_features`].
    pub fn save_features(&self, reg: &crate::FeatureRegistry) -> Result<(), CoreError> {
        Settings::save_features(self.conn, reg)
    }

    /// Get the store display name. Delegates to [`Settings::get_store_name`].
    pub fn get_store_name(&self) -> Result<Option<String>, CoreError> {
        Settings::get_store_name(self.conn)
    }

    /// Set the store display name. Delegates to [`Settings::set_store_name`].
    pub fn set_store_name(&self, name: &str) -> Result<(), CoreError> {
        Settings::set_store_name(self.conn, name)
    }

    /// Get the default currency. Delegates to [`Settings::get_default_currency`].
    pub fn get_default_currency(&self) -> Result<Option<String>, CoreError> {
        Settings::get_default_currency(self.conn)
    }

    /// Set the default currency. Delegates to [`Settings::set_default_currency`].
    pub fn set_default_currency(&self, code: &str) -> Result<(), CoreError> {
        Settings::set_default_currency(self.conn, code)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::Currency;
    use crate::Cart;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    fn seed_everything(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO categories (id, name, colour) VALUES
                ('cat-drinks', 'Drinks',  '#06b6d4'),
                ('cat-food',   'Food',    '#f97316');
             INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at) VALUES
                ('prod-1', 'DRINK-001', 'Espresso',   350, 'USD', 'cat-drinks', NULL,           '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('prod-2', 'FOOD-001',  'Bagel',      450, 'USD', 'cat-food',   '5901234123457', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('prod-3', 'DRINK-002', 'Green Tea',  275, 'USD', 'cat-drinks', NULL,           '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty) VALUES
                ('prod-1', 50),
                ('prod-2', 12);",
        )
        .unwrap();
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

    // ── Product queries ──────────────────────────────────────────

    #[test]
    fn list_products_empty_db() {
        let conn = fresh();
        let products = store(&conn).list_products().unwrap();
        assert!(products.is_empty());
    }

    #[test]
    fn list_products_returns_all() {
        let conn = fresh();
        seed_everything(&conn);
        let products = store(&conn).list_products().unwrap();
        assert_eq!(products.len(), 3);
    }

    #[test]
    fn list_products_includes_category_name() {
        let conn = fresh();
        seed_everything(&conn);
        let products = store(&conn).list_products().unwrap();
        let espresso = products.iter().find(|p| p.product.sku.as_str() == "DRINK-001").unwrap();
        assert_eq!(espresso.category_name.as_deref(), Some("Drinks"));
    }

    #[test]
    fn list_products_includes_stock_qty() {
        let conn = fresh();
        seed_everything(&conn);
        let products = store(&conn).list_products().unwrap();
        let espresso = products.iter().find(|p| p.product.sku.as_str() == "DRINK-001").unwrap();
        assert_eq!(espresso.stock_qty, Some(50));
        let tea = products.iter().find(|p| p.product.sku.as_str() == "DRINK-002").unwrap();
        assert_eq!(tea.stock_qty, None); // no inventory row
    }

    #[test]
    fn get_product_by_sku() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn).get_product("DRINK-001").unwrap().unwrap();
        assert_eq!(p.product.sku.as_str(), "DRINK-001");
        assert_eq!(p.product.name, "Espresso");
        assert_eq!(p.product.price.minor_units, 350);
        assert_eq!(p.stock_qty, Some(50));
    }

    #[test]
    fn get_product_unknown_sku() {
        let conn = fresh();
        let p = store(&conn).get_product("NOPE").unwrap();
        assert!(p.is_none());
    }

    // ── Product creation ─────────────────────────────────────────

    #[test]
    fn create_product_minimal() {
        let conn = fresh();
        let p = store(&conn)
            .create_product("NEW-001", "Widget", price(199), None, None, 0)
            .unwrap();
        assert_eq!(p.sku.as_str(), "NEW-001");
        assert_eq!(p.name, "Widget");
        assert_eq!(p.price.minor_units, 199);
        assert!(!p.id.is_empty());
        assert!(p.category_id.is_none());
        assert!(p.barcode.is_none());
    }

    #[test]
    fn create_product_with_all_fields() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .create_product(
                "FULL-001",
                "Full Item",
                price(999),
                Some("cat-drinks"),
                Some("1234567890123"),
                5,
            )
            .unwrap();
        assert_eq!(p.category_id.as_deref(), Some("cat-drinks"));
        assert_eq!(p.barcode.as_deref(), Some("1234567890123"));
        // Stock row should exist.
        let qty = store(&conn).get_stock(&p.id).unwrap();
        assert_eq!(qty, 5);
    }

    #[test]
    fn create_product_without_stock() {
        let conn = fresh();
        let p = store(&conn)
            .create_product("NOSTOCK", "No Stock", price(100), None, None, 0)
            .unwrap();
        let qty = store(&conn).get_stock(&p.id).unwrap();
        assert_eq!(qty, 0, "no inventory row → 0 stock");
    }

    #[test]
    fn create_product_duplicate_sku() {
        let conn = fresh();
        store(&conn)
            .create_product("DUP", "First", price(100), None, None, 0)
            .unwrap();
        let err = store(&conn)
            .create_product("DUP", "Second", price(200), None, None, 0)
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { .. }));
    }

    #[test]
    fn create_product_validation_errors() {
        let conn = fresh();
        let s = store(&conn);

        // Empty SKU
        let err = s.create_product("  ", "X", price(1), None, None, 0).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "sku"));

        // Empty name
        let err = s.create_product("SKU", "", price(1), None, None, 0).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));

        // Negative price
        let err = s.create_product("SKU", "X", price(-1), None, None, 0).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "price"));

        // Negative stock
        let err = s.create_product("SKU", "X", price(1), None, None, -5).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "initial_stock"));
    }

    // ── Product update / delete ─────────────────────────────────

    #[test]
    fn update_product_basic() {
        let conn = fresh();
        seed_everything(&conn);
        let updated = store(&conn)
            .update_product("DRINK-001", "Latte", price(400), None, None)
            .unwrap();
        assert_eq!(updated.name, "Latte");
        assert_eq!(updated.price.minor_units, 400);
        assert_eq!(updated.sku.as_str(), "DRINK-001");
        assert!(updated.updated_at.as_str() > "2025-01-01");
    }

    #[test]
    fn update_product_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_product("NOPE", "X", price(1), None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_product_empty_name() {
        let conn = fresh();
        seed_everything(&conn);
        let err = store(&conn)
            .update_product("DRINK-001", "", price(1), None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn update_product_negative_price() {
        let conn = fresh();
        seed_everything(&conn);
        let err = store(&conn)
            .update_product("DRINK-001", "X", price(-1), None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "price"));
    }

    #[test]
    fn update_product_with_category() {
        let conn = fresh();
        seed_everything(&conn);
        let updated = store(&conn)
            .update_product("DRINK-001", "Latte", price(400), Some("cat-food"), None)
            .unwrap();
        assert_eq!(updated.category_id.as_deref(), Some("cat-food"));
    }

    #[test]
    fn delete_product_removes_row() {
        let conn = fresh();
        seed_everything(&conn);
        store(&conn).delete_product("DRINK-001").unwrap();
        let p = store(&conn).get_product("DRINK-001").unwrap();
        assert!(p.is_none());
    }

    #[test]
    fn delete_product_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_product("NOPE").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    // ── Categories ───────────────────────────────────────────────

    #[test]
    fn list_categories_empty_db() {
        let conn = fresh();
        let cats = store(&conn).list_categories().unwrap();
        assert!(cats.is_empty());
    }

    #[test]
    fn list_categories_seeded() {
        let conn = fresh();
        seed_everything(&conn);
        let cats = store(&conn).list_categories().unwrap();
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0].name, "Drinks");
        assert_eq!(cats[1].name, "Food");
    }

    #[test]
    fn create_category() {
        let conn = fresh();
        let cat = store(&conn)
            .create_category("cat-tools", "Tools", "#10b981")
            .unwrap();
        assert_eq!(cat.id, "cat-tools");
        assert_eq!(cat.name, "Tools");
        assert_eq!(cat.colour, "#10b981");
    }

    #[test]
    fn create_category_duplicate_name() {
        let conn = fresh();
        store(&conn)
            .create_category("cat-1", "Drinks", "#000")
            .unwrap();
        let err = store(&conn)
            .create_category("cat-2", "Drinks", "#fff")
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { .. }));
    }

    #[test]
    fn create_category_empty_name() {
        let conn = fresh();
        let err = store(&conn)
            .create_category("cat-1", "   ", "#000")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn get_category_found() {
        let conn = fresh();
        seed_everything(&conn);
        let cat = store(&conn).get_category("cat-drinks").unwrap().unwrap();
        assert_eq!(cat.name, "Drinks");
    }

    #[test]
    fn get_category_not_found() {
        let conn = fresh();
        let cat = store(&conn).get_category("nope").unwrap();
        assert!(cat.is_none());
    }

    // ── Inventory ────────────────────────────────────────────────

    #[test]
    fn get_stock_existing() {
        let conn = fresh();
        seed_everything(&conn);
        let qty = store(&conn).get_stock("prod-1").unwrap();
        assert_eq!(qty, 50);
    }

    #[test]
    fn get_stock_no_row() {
        let conn = fresh();
        seed_everything(&conn);
        let qty = store(&conn).get_stock("prod-3").unwrap(); // DRINK-002, no inventory
        assert_eq!(qty, 0);
    }

    #[test]
    fn product_id_by_sku_found() {
        let conn = fresh();
        seed_everything(&conn);
        let id = store(&conn).product_id_by_sku("DRINK-001").unwrap();
        assert_eq!(id, Some("prod-1".into()));
    }

    #[test]
    fn product_id_by_sku_not_found() {
        let conn = fresh();
        let id = store(&conn).product_id_by_sku("NOPE").unwrap();
        assert!(id.is_none());
    }

    #[test]
    fn adjust_stock_sell() {
        let conn = fresh();
        seed_everything(&conn);
        let new_qty = store(&conn).adjust_stock("DRINK-001", -10).unwrap();
        assert_eq!(new_qty, 40);
        // Verify persisted.
        let qty = store(&conn).get_stock("prod-1").unwrap();
        assert_eq!(qty, 40);
    }

    #[test]
    fn adjust_stock_restock() {
        let conn = fresh();
        seed_everything(&conn);
        let new_qty = store(&conn).adjust_stock("DRINK-001", 25).unwrap();
        assert_eq!(new_qty, 75);
    }

    #[test]
    fn adjust_stock_oversell_rejected() {
        let conn = fresh();
        seed_everything(&conn);
        let err = store(&conn).adjust_stock("DRINK-001", -100).unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));
        // Stock should be unchanged.
        assert_eq!(store(&conn).get_stock("prod-1").unwrap(), 50);
    }

    #[test]
    fn adjust_stock_unknown_sku() {
        let conn = fresh();
        let err = store(&conn).adjust_stock("NOPE", 10).unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn adjust_stock_no_existing_row() {
        let conn = fresh();
        seed_everything(&conn);
        // DRINK-002 has no inventory row → baseline 0.
        let new_qty = store(&conn).adjust_stock("DRINK-002", 30).unwrap();
        assert_eq!(new_qty, 30);
    }

    // ── Sale CRUD ───────────────────────────────────────────────

    fn make_cart() -> crate::Cart {
        use crate::CartLine;
        let mut cart = crate::Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
            .unwrap();
        cart
    }

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
        // Sale with 0 lines should persist (total = 0, line_count = 0).
        store(&conn).create_sale(&sale).unwrap();
        let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.line_count, 0);
        assert_eq!(loaded.lines.len(), 0);
        assert_eq!(loaded.total.minor_units, 0);
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
        let s = store(&conn).update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        assert_eq!(s.status, SaleStatus::Active);

        // Active -> Completed.
        let s = store(&conn).update_sale_status(&sale.id, SaleStatus::Completed).unwrap();
        assert_eq!(s.status, SaleStatus::Completed);

        // Terminal -> rejected.
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

    // ── Settings delegation ──────────────────────────────────────

    #[test]
    fn store_get_set_setting() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_setting("my.key").unwrap(), None);
        s.set_setting("my.key", "hello").unwrap();
        assert_eq!(s.get_setting("my.key").unwrap(), Some("hello".into()));
    }

    #[test]
    fn store_features_roundtrip() {
        let conn = fresh();
        let s = store(&conn);
        let reg = crate::FeatureRegistry::simple_retail();
        s.save_features(&reg).unwrap();
        let loaded = s.load_features().unwrap();
        assert_eq!(loaded, reg);
    }

    #[test]
    fn store_name_get_set() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_store_name().unwrap(), None);
        s.set_store_name("Acme").unwrap();
        assert_eq!(s.get_store_name().unwrap(), Some("Acme".into()));
    }

    #[test]
    fn store_default_currency_get_set() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_default_currency().unwrap(), None);
        s.set_default_currency("EUR").unwrap();
        assert_eq!(s.get_default_currency().unwrap(), Some("EUR".into()));
    }

    #[test]
    fn store_conn_returns_underlying_connection() {
        let conn = fresh();
        let s = store(&conn);
        // Indirectly verify we have the right connection by inserting
        // through the Store and reading back through the raw conn.
        let p = s.create_product("T1", "Test", price(1), None, None, 0).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM products WHERE sku = 'T1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        drop(p); // silence unused warning
    }

    // ── ProductWithDetails equality ──────────────────────────────

    #[test]
    fn product_with_details_fields() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn).get_product("FOOD-001").unwrap().unwrap();
        assert_eq!(p.product.name, "Bagel");
        assert_eq!(p.category_name.as_deref(), Some("Food"));
        assert_eq!(p.stock_qty, Some(12));
        assert_eq!(p.product.barcode.as_deref(), Some("5901234123457"));
    }
}
