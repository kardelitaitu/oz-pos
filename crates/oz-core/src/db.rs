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
use crate::offline::{OfflineQueueItem, OfflineQueueStatus};
use crate::refund::{Refund, RefundLine};
use crate::{Category, Customer, Money, Product, ProductVariant, Role, Sale, SaleLine, SaleStatus, Settings, Sku, Terminal, User};
use crate::tax_rate::TaxRate;

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

    /// Look up a single product by barcode, including category and stock.
    ///
    /// Returns `None` when no product matches the barcode, or when
    /// the barcode is empty.
    pub fn lookup_product_with_details_by_barcode(
        &self,
        barcode: &str,
    ) -> Result<Option<ProductWithDetails>, CoreError> {
        if barcode.trim().is_empty() {
            return Ok(None);
        }
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                    p.category_id, p.barcode, p.created_at, p.updated_at,
                    c.name AS category_name,
                    i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.barcode = ?1",
        )?;
        let result = stmt.query_row(params![barcode.trim()], row_to_product_with_details);
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

    /// Look up a product by barcode.
    ///
    /// Returns `None` when no product has the given barcode, or if
    /// the barcode is empty.
    pub fn get_product_by_barcode(&self, barcode: &str) -> Result<Option<Product>, CoreError> {
        if barcode.trim().is_empty() {
            return Ok(None);
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at
             FROM products WHERE barcode = ?1",
        )?;
        let result = stmt.query_row(params![barcode.trim()], row_to_product);
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
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

    /// Delete a category by id. Returns [`CoreError::NotFound`] when
    /// the id doesn't match any category.
    pub fn delete_category(&self, id: &str) -> Result<(), CoreError> {
        let rows = self.conn.execute(
            "DELETE FROM categories WHERE id = ?1",
            params![id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "category",
                id: id.to_owned(),
            });
        }
        Ok(())
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
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor,
                                discount_percent, discount_label, user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                sale.id,
                sale.total.minor_units,
                cur_str,
                sale.line_count,
                status_str,
                sale.payment_method,
                sale.tendered_minor,
                sale.discount_percent,
                sale.discount_label,
                sale.user_id,
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

    /// List all sales ordered by creation date (most recent first),
    /// without line items (each sale has an empty `lines` vec).
    pub fn list_sales(&self) -> Result<Vec<Sale>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at
             FROM sales
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
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
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row.get::<_, Option<i64>>("discount_percent").unwrap_or(Some(0)).unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(),
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single sale by id, including all line items.
    ///
    /// Returns `None` when no sale matches the id.
    pub fn get_sale(&self, id: &str) -> Result<Option<Sale>, CoreError> {
        let mut sale_stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at
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
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row.get::<_, Option<i64>>("discount_percent").unwrap_or(Some(0)).unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
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

// ── Backup / Export ────────────────────────────────────────────────────

impl Store<'_> {
    /// Create a snapshot of the database to a file at `output_path`.
    ///
    /// Uses SQLite's online backup API so the source connection can
    /// remain in use during the copy.
    pub fn backup(&self, output_path: &str) -> Result<(), CoreError> {
        // VACUUM INTO creates a clean, optimized database snapshot.
        let escaped = output_path.replace('\'', "''");
        let sql = format!("VACUUM INTO '{escaped}'");
        self.conn.execute_batch(&sql)?;
        Ok(())
    }
}

/// Row returned by [`Store::export_daily_summary`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailySummaryRow {
    /// The sale id.
    pub sale_id: String,
    /// Total minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Number of line items.
    pub line_count: i64,
    /// Current sale status.
    pub status: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Row returned by [`Store::export_sales_by_hour`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct SalesByHourRow {
    /// Hour of day (0-23).
    pub hour: i64,
    /// Total minor units sold in that hour.
    pub total_minor: i64,
    /// Number of sales in that hour.
    pub sale_count: i64,
}

/// Summary row for a held (parked) cart.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeldCartRow {
    /// Internal row id (UUID v4).
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Number of line items.
    pub item_count: i64,
    /// Cart total in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Full held cart data including the JSON cart_data blob.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeldCartFull {
    /// Internal row id (UUID v4).
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// JSON-serialized cart data (lines, discount, currency).
    pub cart_data: String,
    /// Number of line items.
    pub item_count: i64,
    /// Cart total in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

impl Store<'_> {
    /// Query all sales for today, ordered chronologically.
    pub fn export_daily_summary(&self) -> Result<Vec<DailySummaryRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status, created_at
             FROM sales
             WHERE date(created_at) = date('now')
             ORDER BY created_at",
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
                    SUM(total_minor) AS total_minor,
                    COUNT(*) AS sale_count
             FROM sales
             WHERE date(created_at) = date('now')
             GROUP BY hour
             ORDER BY hour",
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

// ── Customer CRUD ─────────────────────────────────────────────────

impl Store<'_> {
    /// List all customers, ordered by name.
    pub fn list_customers(&self) -> Result<Vec<Customer>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency,
                    notes, created_at, updated_at
             FROM customers ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Customer {
                id: row.get("id")?,
                name: row.get("name")?,
                email: row.get("email")?,
                phone: row.get("phone")?,
                loyalty_points: row.get("loyalty_points")?,
                total_spent_minor: row.get("total_spent_minor")?,
                currency: row.get("currency")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single customer by id.
    ///
    /// Returns `None` when no customer matches.
    pub fn get_customer(&self, id: &str) -> Result<Option<Customer>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency,
                    notes, created_at, updated_at
             FROM customers WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(Customer {
                id: row.get("id")?,
                name: row.get("name")?,
                email: row.get("email")?,
                phone: row.get("phone")?,
                loyalty_points: row.get("loyalty_points")?,
                total_spent_minor: row.get("total_spent_minor")?,
                currency: row.get("currency")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new customer.
    ///
    /// Generates a UUID for the customer id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Validation`] when the name is empty.
    pub fn create_customer(
        &self,
        name: &str,
        email: Option<&str>,
        phone: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Customer, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "customer name must not be empty".into(),
            });
        }

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "INSERT INTO customers (id, name, email, phone, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, name.trim(), email, phone, notes.unwrap_or_default(), now, now],
        )?;

        Ok(Customer {
            id,
            name: name.trim().to_owned(),
            email: email.map(|s| s.to_owned()),
            phone: phone.map(|s| s.to_owned()),
            loyalty_points: 0,
            total_spent_minor: 0,
            currency: "USD".into(),
            notes: notes.unwrap_or_default().to_owned(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing customer.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the id doesn't match.
    /// Returns [`CoreError::Validation`] when the name is empty.
    pub fn update_customer(
        &self,
        id: &str,
        name: &str,
        email: Option<&str>,
        phone: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Customer, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "customer name must not be empty".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let rows = self.conn.execute(
            "UPDATE customers
             SET name = ?1, email = ?2, phone = ?3, notes = ?4, updated_at = ?5
             WHERE id = ?6",
            params![name.trim(), email, phone, notes.unwrap_or_default(), now, id],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "customer",
                id: id.to_owned(),
            });
        }

        self.get_customer(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "customer",
            id: id.to_owned(),
        })
    }

    /// Delete a customer by id.
    ///
    /// Returns [`CoreError::NotFound`] when the id doesn't match.
    pub fn delete_customer(&self, id: &str) -> Result<(), CoreError> {
        let rows = self.conn.execute(
            "DELETE FROM customers WHERE id = ?1",
            params![id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "customer",
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}

// ── Role CRUD ───────────────────────────────────────────────────

impl Store<'_> {
    /// List all roles, ordered by name.
    pub fn list_roles(&self) -> Result<Vec<Role>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, permissions, created_at, updated_at
             FROM roles ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Role {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
                permissions: row.get("permissions")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single role by id.
    ///
    /// Returns `None` when no role matches.
    pub fn get_role(&self, id: &str) -> Result<Option<Role>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, permissions, created_at, updated_at
             FROM roles WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(Role {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
                permissions: row.get("permissions")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new role.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Conflict`] when the name already exists.
    pub fn create_role(
        &self,
        id: &str,
        name: &str,
        description: &str,
        permissions: &str,
    ) -> Result<Role, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let result = self.conn.execute(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name.trim(), description, permissions, now, now],
        );

        match result {
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(CoreError::Conflict {
                    entity: "role",
                    field: "name",
                });
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        Ok(Role {
            id: id.to_owned(),
            name: name.trim().to_owned(),
            description: description.to_owned(),
            permissions: permissions.to_owned(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

// ── User CRUD ───────────────────────────────────────────────────

impl Store<'_> {
    /// List all users, ordered by display_name.
    pub fn list_users(&self) -> Result<Vec<User>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, pin_hash, display_name, role_id, is_active,
                    created_at, updated_at
             FROM users ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get("id")?,
                username: row.get("username")?,
                pin_hash: row.get("pin_hash")?,
                display_name: row.get("display_name")?,
                role_id: row.get("role_id")?,
                is_active: row.get("is_active")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single user by id.
    ///
    /// Returns `None` when no user matches.
    pub fn get_user(&self, id: &str) -> Result<Option<User>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, pin_hash, display_name, role_id, is_active,
                    created_at, updated_at
             FROM users WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(User {
                id: row.get("id")?,
                username: row.get("username")?,
                pin_hash: row.get("pin_hash")?,
                display_name: row.get("display_name")?,
                role_id: row.get("role_id")?,
                is_active: row.get("is_active")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(u) => Ok(Some(u)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Look up a user by username.
    ///
    /// Returns `None` when no user matches.
    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, pin_hash, display_name, role_id, is_active,
                    created_at, updated_at
             FROM users WHERE username = ?1",
        )?;
        let result = stmt.query_row(params![username], |row| {
            Ok(User {
                id: row.get("id")?,
                username: row.get("username")?,
                pin_hash: row.get("pin_hash")?,
                display_name: row.get("display_name")?,
                role_id: row.get("role_id")?,
                is_active: row.get("is_active")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(u) => Ok(Some(u)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new user.
    ///
    /// Generates a UUID for the user id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Validation`] when the username or display_name is empty.
    /// Returns [`CoreError::Conflict`] when the username already exists.
    pub fn create_user(
        &self,
        username: &str,
        pin_hash: &str,
        display_name: &str,
        role_id: &str,
    ) -> Result<User, CoreError> {
        if username.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "username",
                message: "username must not be empty".into(),
            });
        }
        if display_name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "display_name",
                message: "display name must not be empty".into(),
            });
        }

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let result = self.conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, username.trim(), pin_hash, display_name.trim(), role_id, now, now],
        );

        match result {
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(CoreError::Conflict {
                    entity: "user",
                    field: "username",
                });
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        Ok(User {
            id,
            username: username.trim().to_owned(),
            pin_hash: pin_hash.to_owned(),
            display_name: display_name.trim().to_owned(),
            role_id: role_id.to_owned(),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing user.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the id doesn't match.
    /// Returns [`CoreError::Validation`] when display_name is empty.
    pub fn update_user(
        &self,
        id: &str,
        username: &str,
        display_name: &str,
        role_id: &str,
        is_active: bool,
    ) -> Result<User, CoreError> {
        if display_name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "display_name",
                message: "display name must not be empty".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let rows = self.conn.execute(
            "UPDATE users
             SET username = ?1, display_name = ?2, role_id = ?3,
                 is_active = ?4, updated_at = ?5
             WHERE id = ?6",
            params![username.trim(), display_name.trim(), role_id, is_active, now, id],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "user",
                id: id.to_owned(),
            });
        }

        self.get_user(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "user",
            id: id.to_owned(),
        })
    }

    /// Delete a user by id.
    ///
    /// Returns [`CoreError::NotFound`] when the id doesn't match.
    pub fn delete_user(&self, id: &str) -> Result<(), CoreError> {
        let rows = self.conn.execute(
            "DELETE FROM users WHERE id = ?1",
            params![id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "user",
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}

// ── Tax Rate CRUD ───────────────────────────────────────────────────

impl Store<'_> {
    /// List all tax rates, ordered by name.
    pub fn list_tax_rates(&self) -> Result<Vec<TaxRate>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at
             FROM tax_rates ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TaxRate {
                id: row.get("id")?,
                name: row.get("name")?,
                rate_bps: row.get("rate_bps")?,
                is_default: row.get("is_default")?,
                is_inclusive: row.get("is_inclusive")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single tax rate by id.
    ///
    /// Returns `None` when no rate matches.
    pub fn get_tax_rate(&self, id: &str) -> Result<Option<TaxRate>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at
             FROM tax_rates WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(TaxRate {
                id: row.get("id")?,
                name: row.get("name")?,
                rate_bps: row.get("rate_bps")?,
                is_default: row.get("is_default")?,
                is_inclusive: row.get("is_inclusive")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new tax rate.
    ///
    /// Generates a UUID for the rate id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Validation`] when the name is empty.
    pub fn create_tax_rate(
        &self,
        name: &str,
        rate_bps: i64,
        is_default: bool,
        is_inclusive: bool,
    ) -> Result<TaxRate, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "tax rate name must not be empty".into(),
            });
        }
        if rate_bps < 0 {
            return Err(CoreError::Validation {
                field: "rate_bps",
                message: "rate must be non-negative".into(),
            });
        }

        // If this is the default, clear any existing default first.
        if is_default {
            self.conn.execute("UPDATE tax_rates SET is_default = 0 WHERE is_default = 1", [])?;
        }

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, name.trim(), rate_bps, is_default, is_inclusive, now, now],
        )?;

        Ok(TaxRate {
            id,
            name: name.trim().to_owned(),
            rate_bps,
            is_default,
            is_inclusive,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing tax rate.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the id doesn't match.
    /// Returns [`CoreError::Validation`] when the name is empty or rate is negative.
    pub fn update_tax_rate(
        &self,
        id: &str,
        name: &str,
        rate_bps: i64,
        is_default: bool,
        is_inclusive: bool,
    ) -> Result<TaxRate, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "tax rate name must not be empty".into(),
            });
        }
        if rate_bps < 0 {
            return Err(CoreError::Validation {
                field: "rate_bps",
                message: "rate must be non-negative".into(),
            });
        }

        // If this is the new default, clear any existing default first.
        if is_default {
            self.conn.execute("UPDATE tax_rates SET is_default = 0 WHERE is_default = 1", [])?;
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let affected = self.conn.execute(
            "UPDATE tax_rates
             SET name = ?1, rate_bps = ?2, is_default = ?3, is_inclusive = ?4, updated_at = ?5
             WHERE id = ?6",
            params![name.trim(), rate_bps, is_default, is_inclusive, now, id],
        )?;

        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "tax_rate",
                id: id.to_owned(),
            });
        }

        Ok(TaxRate {
            id: id.to_owned(),
            name: name.trim().to_owned(),
            rate_bps,
            is_default,
            is_inclusive,
            created_at: String::new(),
            updated_at: now,
        })
    }

    /// Delete a tax rate by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the id doesn't match.
    pub fn delete_tax_rate(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "DELETE FROM tax_rates WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "tax_rate",
                id: id.to_owned(),
            });
        }
        Ok(())
    }

    /// Assign tax rates to a product (replaces any existing assignments).
    ///
    /// Uses a transaction to atomically delete old rows and insert new ones.
    /// Unknown `tax_rate_id`s are silently ignored (`INSERT OR IGNORE`).
    pub fn set_product_tax_rates(&self, sku: &str, tax_rate_ids: &[String]) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM product_taxes WHERE product_sku = ?1", params![sku])?;
        for id in tax_rate_ids {
            tx.execute(
                "INSERT OR IGNORE INTO product_taxes (product_sku, tax_rate_id) VALUES (?1, ?2)",
                params![sku, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Get all tax rate IDs assigned to a product, ordered by creation time.
    pub fn get_product_tax_rates(&self, sku: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT tax_rate_id FROM product_taxes WHERE product_sku = ?1 ORDER BY created_at",
        )?;
        let ids = stmt
            .query_map(params![sku], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    /// Assign tax rates to a category (replaces any existing assignments).
    pub fn set_category_tax_rates(&self, category_id: &str, tax_rate_ids: &[String]) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM category_taxes WHERE category_id = ?1", params![category_id])?;
        for id in tax_rate_ids {
            tx.execute(
                "INSERT OR IGNORE INTO category_taxes (category_id, tax_rate_id) VALUES (?1, ?2)",
                params![category_id, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Get all tax rate IDs assigned to a category, ordered by creation time.
    pub fn get_category_tax_rates(&self, category_id: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT tax_rate_id FROM category_taxes WHERE category_id = ?1 ORDER BY created_at",
        )?;
        let ids = stmt
            .query_map(params![category_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

}

// ── Audit log ─────────────────────────────────────────────────────────

impl Store<'_> {
    /// Insert a new audit log entry (append-only).
    ///
    /// The audit_log table has no UPDATE/DELETE methods — once written,
    /// entries are immutable. This satisfies PCI-DSS 10.3.1.
    pub fn log_audit(&self, entry: &crate::AuditEntry) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO audit_log (id, user_id, action, target_type, target_id, details, outcome, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.id,
                entry.user_id,
                entry.action,
                entry.target_type,
                entry.target_id,
                entry.details,
                entry.outcome,
                entry.created_at,
            ],
        )?;
        Ok(())
    }

    /// List audit log entries in reverse chronological order.
    pub fn list_audit_entries(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::AuditEntry>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, action, target_type, target_id, details, outcome, created_at
             FROM audit_log
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], |row| {
            Ok(crate::AuditEntry {
                id: row.get("id")?,
                user_id: row.get("user_id")?,
                action: row.get("action")?,
                target_type: row.get("target_type")?,
                target_id: row.get("target_id")?,
                details: row.get("details")?,
                outcome: row.get("outcome")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Void a sale and restore stock for all line items.
    ///
    /// This is an atomic operation inside a single transaction:
    /// 1. Transitions sale status to Voided
    /// 2. Restores stock for each line item
    /// 3. Writes an audit log entry
    ///
    /// Returns the updated sale with line items.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] if the sale doesn't exist.
    /// Returns [`CoreError::Validation`] if the sale can't be voided
    /// (e.g. already voided or completed).
    pub fn void_sale(&self, sale_id: &str, user_id: &str, reason: &str) -> Result<Sale, CoreError> {
        use crate::AuditEntry;

        // Load the sale with lines.
        let sale = self.get_sale(sale_id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "sale",
                id: sale_id.to_owned(),
            })?;

        // Validate the transition.
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

        // Atomic transaction: status update + stock restore + audit.
        let tx = self.conn.unchecked_transaction()?;

        // 1. Update status to Voided.
        tx.execute(
            "UPDATE sales SET status = 'voided', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, sale_id],
        )?;

        // 2. Restore stock for each line item.
        for line in &sale.lines {
            // Get product id by SKU.
            if let Some(product_id) = self.product_id_by_sku(&line.sku)? {
                let current_qty = self.get_stock(&product_id)?;
                let new_qty = current_qty.checked_add(line.qty).ok_or_else(|| {
                    CoreError::Validation {
                        field: "qty",
                        message: "stock overflow during void".into(),
                    }
                })?;
                tx.execute(
                    "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty,
                                                            updated_at = excluded.updated_at",
                    rusqlite::params![product_id, new_qty, now],
                )?;
            }
        }

        // 3. Audit log entry (uses serde_json for safe JSON escaping).
        let details = serde_json::json!({
            "reason": reason,
            "total_minor": sale.total.minor_units,
        }).to_string();
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

        // Reload and return the updated sale.
        self.get_sale(sale_id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: sale_id.to_owned(),
        })
    }

    /// Persist a cart as a held (parked) order.
    ///
    /// Serializes the cart's lines and metadata to JSON and stores it
    /// in the `held_carts` table. The cart can be resumed later via
    /// [`Store::get_held_cart`].
    pub fn hold_cart(
        &self,
        label: &str,
        cart_data: &str,
        item_count: i64,
        total_minor: i64,
        currency: &str,
    ) -> Result<String, CoreError> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO held_carts (id, label, cart_data, item_count, total_minor, currency)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, label.trim(), cart_data, item_count, total_minor, currency],
        )?;
        Ok(id)
    }

    /// List all held (parked) orders, most recent first.
    pub fn list_held_carts(&self) -> Result<Vec<HeldCartRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, item_count, total_minor, currency, created_at
             FROM held_carts
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HeldCartRow {
                id: row.get("id")?,
                label: row.get("label")?,
                item_count: row.get("item_count")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a held cart by id, including its JSON cart_data.
    ///
    /// Returns `None` when the id doesn't match.
    pub fn get_held_cart(&self, id: &str) -> Result<Option<HeldCartFull>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, cart_data, item_count, total_minor, currency, created_at
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
            })
        });
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a held cart by id.
    ///
    /// Returns [`CoreError::NotFound`] when the id doesn't match any held cart.
    pub fn delete_held_cart(&self, id: &str) -> Result<(), CoreError> {
        let rows = self.conn.execute(
            "DELETE FROM held_carts WHERE id = ?1",
            params![id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "held_cart",
                id: id.to_owned(),
            });
        }
        Ok(())
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

    /// Prune stale feature rows. Delegates to [`Settings::prune_stale_features`].
    pub fn prune_stale_features(&self, reg: &crate::FeatureRegistry) -> Result<usize, CoreError> {
        Settings::prune_stale_features(self.conn, reg)
    }

    /// Get the store display name. Delegates to [`Settings::get_store_name`].
    pub fn get_store_name(&self) -> Result<Option<String>, CoreError> {
        Settings::get_store_name(self.conn)
    }

    /// Set the store display name. Delegates to [`Settings::set_store_name`].
    pub fn set_store_name(&self, name: &str) -> Result<(), CoreError> {
        Settings::set_store_name(self.conn, name)
    }

    /// Get the store address. Delegates to [`Settings::get_store_address`].
    pub fn get_store_address(&self) -> Result<Option<String>, CoreError> {
        Settings::get_store_address(self.conn)
    }

    /// Set the store address. Delegates to [`Settings::set_store_address`].
    pub fn set_store_address(&self, addr: &str) -> Result<(), CoreError> {
        Settings::set_store_address(self.conn, addr)
    }

    /// Get the store tax / VAT number. Delegates to [`Settings::get_store_tax_id`].
    pub fn get_store_tax_id(&self) -> Result<Option<String>, CoreError> {
        Settings::get_store_tax_id(self.conn)
    }

    /// Set the store tax / VAT number. Delegates to [`Settings::set_store_tax_id`].
    pub fn set_store_tax_id(&self, id: &str) -> Result<(), CoreError> {
        Settings::set_store_tax_id(self.conn, id)
    }

    /// Get the default currency. Delegates to [`Settings::get_default_currency`].
    pub fn get_default_currency(&self) -> Result<Option<String>, CoreError> {
        Settings::get_default_currency(self.conn)
    }

    /// Set the default currency. Delegates to [`Settings::set_default_currency`].
    pub fn set_default_currency(&self, code: &str) -> Result<(), CoreError> {
        Settings::set_default_currency(self.conn, code)
    }

    /// List all currencies from the ISO-4217 table.
    pub fn list_currencies(&self) -> Result<Vec<(String, String, u32, String)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT code, name, minor_exponent, symbol FROM currencies ORDER BY code"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,  // code
                row.get::<_, String>(1)?,  // name
                row.get::<_, u32>(2)?,     // minor_exponent
                row.get::<_, String>(3)?,  // symbol
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// List all exchange rates, ordered by from_currency, to_currency.
    pub fn list_exchange_rates(&self) -> Result<Vec<crate::exchange_rate::ExchangeRateRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate, source, effective_date, created_at
             FROM exchange_rates ORDER BY from_currency, to_currency"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::exchange_rate::ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate: row.get(3)?,
                source: row.get(4)?,
                effective_date: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Create a new exchange rate entry.
    pub fn create_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        rate: f64,
        source: &str,
        effective_date: &str,
    ) -> Result<crate::exchange_rate::ExchangeRateRow, CoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO exchange_rates (id, from_currency, to_currency, rate, source, effective_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, from_currency, to_currency, rate, source, effective_date],
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT id, from_currency, to_currency, rate, source, effective_date, created_at
             FROM exchange_rates WHERE id = ?1"
        )?;
        let row = stmt.query_row(rusqlite::params![id], |row| {
            Ok(crate::exchange_rate::ExchangeRateRow {
                id: row.get(0)?,
                from_currency: row.get(1)?,
                to_currency: row.get(2)?,
                rate: row.get(3)?,
                source: row.get(4)?,
                effective_date: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(row)
    }

    /// Delete an exchange rate by ID.
    pub fn delete_exchange_rate(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "DELETE FROM exchange_rates WHERE id = ?1",
            rusqlite::params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "exchange_rate",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    // ── Product Variants ─────────────────────────────────────────

    /// List all variants for a given parent SKU, ordered by sort_order.
    pub fn list_product_variants(&self, parent_sku: &str) -> Result<Vec<ProductVariant>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, parent_sku, name, sku, price_minor, currency, barcode,
                    sort_order, is_active, created_at, updated_at
             FROM product_variants
             WHERE parent_sku = ?1
             ORDER BY sort_order ASC, name ASC"
        )?;
        let rows = stmt.query_map(params![parent_sku], Self::row_to_product_variant)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get a single variant by its own SKU.
    pub fn get_product_variant(&self, sku: &str) -> Result<Option<ProductVariant>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, parent_sku, name, sku, price_minor, currency, barcode,
                    sort_order, is_active, created_at, updated_at
             FROM product_variants WHERE sku = ?1"
        )?;
        let result = stmt.query_row(params![sku], Self::row_to_product_variant);
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create a new product variant.
    pub fn create_product_variant(&self, variant: &ProductVariant) -> Result<(), CoreError> {
        let (price_minor, currency_str) = match &variant.price {
            Some(m) => (Some(m.minor_units), Some(std::str::from_utf8(&m.currency.0).unwrap_or("USD").to_owned())),
            None => (None, None),
        };

        self.conn.execute(
            "INSERT INTO product_variants (id, parent_sku, name, sku, price_minor, currency, barcode,
                                           sort_order, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                variant.id, variant.parent_sku, variant.name, variant.sku,
                price_minor, currency_str, variant.barcode,
                variant.sort_order, variant.is_active as i64,
                variant.created_at, variant.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Update an existing product variant (matched by SKU).
    pub fn update_product_variant(&self, variant: &ProductVariant) -> Result<(), CoreError> {
        let (price_minor, currency_str) = match &variant.price {
            Some(m) => (Some(m.minor_units), Some(std::str::from_utf8(&m.currency.0).unwrap_or("USD").to_owned())),
            None => (None, None),
        };

        let affected = self.conn.execute(
            "UPDATE product_variants SET name = ?1, price_minor = ?2, currency = ?3,
                                          barcode = ?4, sort_order = ?5, is_active = ?6,
                                          updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE sku = ?7",
            params![variant.name, price_minor, currency_str, variant.barcode,
                    variant.sort_order, variant.is_active as i64, variant.sku],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound { entity: "product_variant", id: variant.sku.clone() });
        }
        Ok(())
    }

    /// Delete a product variant by its own SKU.
    pub fn delete_product_variant(&self, sku: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "DELETE FROM product_variants WHERE sku = ?1",
            params![sku],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound { entity: "product_variant", id: sku.to_owned() });
        }
        Ok(())
    }

    fn row_to_product_variant(row: &rusqlite::Row) -> rusqlite::Result<ProductVariant> {
        let price_minor: Option<i64> = row.get("price_minor")?;
        let currency_str: Option<String> = row.get("currency")?;
        let price = match (price_minor, currency_str) {
            (Some(minor), Some(cur)) => {
                let c: Result<crate::Currency, _> = cur.parse();
                c.ok().map(|currency| Money { minor_units: minor, currency })
            }
            _ => None,
        };

        Ok(ProductVariant {
            id: row.get("id")?,
            parent_sku: row.get("parent_sku")?,
            name: row.get("name")?,
            sku: row.get("sku")?,
            price,
            barcode: row.get("barcode")?,
            sort_order: row.get("sort_order")?,
            is_active: row.get::<_, i64>("is_active")? != 0,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

// ── Terminal Management ────────────────────────────────────────────────

impl Store<'_> {
    /// List all registered terminals.
    pub fn list_terminals(&self) -> Result<Vec<Terminal>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, device_id, terminal_secret, is_active,
                    last_seen_at, metadata, created_at, updated_at
             FROM terminals
             ORDER BY name ASC"
        )?;
        let rows = stmt.query_map([], Self::row_to_terminal)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get a terminal by id.
    pub fn get_terminal(&self, id: &str) -> Result<Option<Terminal>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, device_id, terminal_secret, is_active,
                    last_seen_at, metadata, created_at, updated_at
             FROM terminals WHERE id = ?1"
        )?;
        let result = stmt.query_row(params![id], Self::row_to_terminal);
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a terminal by device_id.
    pub fn get_terminal_by_device_id(&self, device_id: &str) -> Result<Option<Terminal>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, device_id, terminal_secret, is_active,
                    last_seen_at, metadata, created_at, updated_at
             FROM terminals WHERE device_id = ?1"
        )?;
        let result = stmt.query_row(params![device_id], Self::row_to_terminal);
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Register a new terminal.
    pub fn create_terminal(&self, terminal: &Terminal) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO terminals (id, name, device_id, terminal_secret, is_active,
                                    last_seen_at, metadata, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                terminal.id, terminal.name, terminal.device_id,
                terminal.terminal_secret, terminal.is_active as i64,
                terminal.last_seen_at, terminal.metadata,
                terminal.created_at, terminal.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Update an existing terminal.
    pub fn update_terminal(&self, terminal: &Terminal) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE terminals SET name = ?1, device_id = ?2, terminal_secret = ?3,
                                   is_active = ?4, last_seen_at = ?5, metadata = ?6,
                                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?7",
            params![
                terminal.name, terminal.device_id, terminal.terminal_secret,
                terminal.is_active as i64, terminal.last_seen_at, terminal.metadata,
                terminal.id,
            ],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound { entity: "terminal", id: terminal.id.clone() });
        }
        Ok(())
    }

    /// Update a terminal's last_seen_at timestamp.
    pub fn ping_terminal(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE terminals SET last_seen_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound { entity: "terminal", id: id.to_owned() });
        }
        Ok(())
    }

    /// Delete a terminal by id.
    pub fn delete_terminal(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "DELETE FROM terminals WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound { entity: "terminal", id: id.to_owned() });
        }
        Ok(())
    }

    fn row_to_terminal(row: &rusqlite::Row) -> rusqlite::Result<Terminal> {
        Ok(Terminal {
            id: row.get("id")?,
            name: row.get("name")?,
            device_id: row.get("device_id")?,
            terminal_secret: row.get("terminal_secret")?,
            is_active: row.get::<_, i64>("is_active")? != 0,
            last_seen_at: row.get("last_seen_at")?,
            metadata: row.get("metadata")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

// ── Offline Queue ─────────────────────────────────────────────────────

impl Store<'_> {
    /// Enqueue a transaction for later sync.
    pub fn enqueue_offline(&self, action: &str, payload: &str) -> Result<OfflineQueueItem, CoreError> {
        let item = OfflineQueueItem::new(action, payload);
        self.conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                item.id, item.action, item.payload,
                item.status.as_stored_str(), item.retry_count,
                item.last_error, item.created_at, item.synced_at,
            ],
        )?;
        Ok(item)
    }

    /// List all pending (unsynced) offline queue items, oldest first.
    pub fn list_pending_offline(&self) -> Result<Vec<OfflineQueueItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at
             FROM offline_queue
             WHERE status = 'pending'
             ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map([], Self::row_to_offline_queue_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List all offline queue items.
    pub fn list_all_offline(&self) -> Result<Vec<OfflineQueueItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at
             FROM offline_queue
             ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], Self::row_to_offline_queue_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Mark an offline queue item as synced.
    pub fn mark_offline_synced(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE offline_queue SET status = 'synced', synced_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound { entity: "offline_queue", id: id.to_owned() });
        }
        Ok(())
    }

    /// Mark an offline queue item as failed with an error message.
    pub fn mark_offline_failed(&self, id: &str, error: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE offline_queue SET status = 'failed', last_error = ?1, retry_count = retry_count + 1
             WHERE id = ?2",
            params![error, id],
        )?;
        Ok(())
    }

    /// Get the count of pending offline items.
    pub fn pending_offline_count(&self) -> Result<i64, CoreError> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending'",
            [],
            |row| row.get(0),
        ).map_err(Into::into)
    }

    /// Delete a processed offline queue item.
    pub fn delete_offline_item(&self, id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM offline_queue WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    fn row_to_offline_queue_item(row: &rusqlite::Row) -> rusqlite::Result<OfflineQueueItem> {
        let status_str: String = row.get("status")?;
        Ok(OfflineQueueItem {
            id: row.get("id")?,
            action: row.get("action")?,
            payload: row.get("payload")?,
            status: OfflineQueueStatus::from_stored_str(&status_str).unwrap_or(OfflineQueueStatus::Pending),
            retry_count: row.get("retry_count")?,
            last_error: row.get("last_error")?,
            created_at: row.get("created_at")?,
            synced_at: row.get("synced_at")?,
        })
    }
}

// ── Refunds ───────────────────────────────────────────────────

impl Store<'_> {
    /// Process a refund — persist refund + lines inside a transaction.
    /// Does NOT modify the original sale's status or restore stock.
    /// Stock restoration is handled separately via inventory adjustment.
    pub fn create_refund(&self, refund: &Refund) -> Result<(), CoreError> {
        let cur_str = std::str::from_utf8(&refund.total.currency.0)
            .expect("currency bytes are valid UTF-8");

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO refunds (id, sale_id, total_minor, currency, reason, note, processed_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                refund.id, refund.sale_id,
                refund.total.minor_units, cur_str,
                refund.reason, refund.note,
                refund.processed_by, refund.created_at,
            ],
        )?;

        for line in &refund.lines {
            let line_cur = std::str::from_utf8(&line.unit_price.currency.0)
                .expect("currency bytes are valid UTF-8");
            tx.execute(
                "INSERT INTO refund_lines (id, refund_id, sale_line_id, sku, qty, unit_minor, line_minor, currency, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    line.id, line.refund_id, line.sale_line_id,
                    line.sku, line.qty,
                    line.unit_price.minor_units, line.line_total.minor_units,
                    line_cur, line.created_at,
                ],
            )?;
        }

        // Write audit log.
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
             FROM refunds WHERE sale_id = ?1 ORDER BY created_at ASC"
        )?;
        let refunds: Vec<Refund> = stmt.query_map(params![sale_id], |row| {
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
        })?.collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        // Load lines for each refund.
        let mut line_stmt = self.conn.prepare(
            "SELECT id, refund_id, sale_line_id, sku, qty, unit_minor, line_minor, currency, created_at
             FROM refund_lines WHERE refund_id = ?1 ORDER BY created_at ASC"
        )?;
        let mut result: Vec<Refund> = Vec::new();
        for mut r in refunds {
            let lines: Vec<RefundLine> = line_stmt.query_map(params![r.id], Self::row_to_refund_line)?
                .collect::<Result<Vec<_>, _>>()?;
            r.lines = lines;
            result.push(r);
        }

        Ok(result)
    }

    /// Get total refunded amount for a sale (sum of all refunds).
    pub fn total_refunded_for_sale(&self, sale_id: &str) -> Result<Money, CoreError> {
        let row = self.conn.query_row(
            "SELECT COALESCE(SUM(total_minor), 0) AS total, currency
             FROM refunds WHERE sale_id = ?1
             GROUP BY currency",
            params![sale_id],
            |row| {
                let total: i64 = row.get("total")?;
                let cur_str: String = row.get("currency")?;
                Ok((total, cur_str))
            },
        );
        match row {
            Ok((total, cur_str)) => {
                let currency: crate::Currency = cur_str.parse().expect("valid currency in DB");
                Ok(Money { minor_units: total, currency })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(CoreError::NotFound { entity: "refund", id: sale_id.to_owned() })
            }
            Err(e) => Err(e.into()),
        }
    }

    fn row_to_refund_line(row: &rusqlite::Row) -> rusqlite::Result<RefundLine> {
        let cur_str: String = row.get("currency")?;
        Ok(RefundLine {
            id: row.get("id")?,
            refund_id: row.get("refund_id")?,
            sale_line_id: row.get("sale_line_id")?,
            sku: row.get("sku")?,
            qty: row.get("qty")?,
            unit_price: Money {
                minor_units: row.get("unit_minor")?,
                currency: cur_str.parse().expect("valid currency in DB"),
            },
            line_total: Money {
                minor_units: row.get("line_minor")?,
                currency: cur_str.parse().expect("valid currency in DB"),
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
    fn delete_category_removes_row() {
        let conn = fresh();
        store(&conn)
            .create_category("cat-orphan", "Orphan", "#000")
            .unwrap();
        store(&conn).delete_category("cat-orphan").unwrap();
        let cat = store(&conn).get_category("cat-orphan").unwrap();
        assert!(cat.is_none());
    }

    #[test]
    fn delete_category_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_category("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
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

        // Also create a second sale with a slightly different cart.
        let mut cart2 = Cart::new(usd());
        use crate::CartLine;
        cart2.add_line(CartLine::new(Sku::new("TEA"), 1, price(200))).unwrap();
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

    // ── Backup / Export ───────────────────────────────────────────

    #[test]
    fn backup_creates_snapshot_file() {
        let conn = fresh();
        seed_everything(&conn);
        let s = store(&conn);

        let tmp = std::env::temp_dir().join("oz-test-backup.db");
        let _ = std::fs::remove_file(&tmp); // clean slate

        s.backup(tmp.to_str().unwrap()).unwrap();

        // Open the backup and verify data exists.
        let backup_conn = Connection::open(&tmp).unwrap();
        let count: i64 = backup_conn
            .query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
        let cat_count: i64 = backup_conn
            .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cat_count, 2);

        let _ = std::fs::remove_file(&tmp);
    }

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

    // ── Customer CRUD ────────────────────────────────────────────

    fn seed_customers(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO customers (id, name, email, phone, notes, created_at, updated_at) VALUES
                ('cust-1', 'Alice',  'alice@example.com',  '+1-555-0101', 'Regular',   '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('cust-2', 'Bob',    NULL,                 '+1-555-0102', '',          '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('cust-3', 'Carol',  'carol@example.com',  NULL,          'VIP',       '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();
    }

    #[test]
    fn list_customers_empty_db() {
        let conn = fresh();
        let customers = store(&conn).list_customers().unwrap();
        assert!(customers.is_empty());
    }

    #[test]
    fn list_customers_returns_all() {
        let conn = fresh();
        seed_customers(&conn);
        let customers = store(&conn).list_customers().unwrap();
        assert_eq!(customers.len(), 3);
        assert_eq!(customers[0].name, "Alice");
        assert_eq!(customers[1].name, "Bob");
        assert_eq!(customers[2].name, "Carol");
    }

    #[test]
    fn get_customer_found() {
        let conn = fresh();
        seed_customers(&conn);
        let c = store(&conn).get_customer("cust-1").unwrap().unwrap();
        assert_eq!(c.name, "Alice");
        assert_eq!(c.email.as_deref(), Some("alice@example.com"));
        assert_eq!(c.phone.as_deref(), Some("+1-555-0101"));
        assert_eq!(c.notes, "Regular");
    }

    #[test]
    fn get_customer_not_found() {
        let conn = fresh();
        let c = store(&conn).get_customer("nope").unwrap();
        assert!(c.is_none());
    }

    #[test]
    fn get_customer_nullable_fields() {
        let conn = fresh();
        seed_customers(&conn);
        let c = store(&conn).get_customer("cust-2").unwrap().unwrap();
        assert_eq!(c.name, "Bob");
        assert!(c.email.is_none());
        assert_eq!(c.phone.as_deref(), Some("+1-555-0102"));
    }

    #[test]
    fn create_customer_minimal() {
        let conn = fresh();
        let c = store(&conn)
            .create_customer("Diana", None, None, None)
            .unwrap();
        assert_eq!(c.name, "Diana");
        assert!(c.email.is_none());
        assert!(c.phone.is_none());
        assert_eq!(c.notes, "");
        assert!(!c.id.is_empty());
    }

    #[test]
    fn create_customer_with_all_fields() {
        let conn = fresh();
        let c = store(&conn)
            .create_customer("Diana", Some("diana@test.com"), Some("555-0100"), Some("Preferred"))
            .unwrap();
        assert_eq!(c.name, "Diana");
        assert_eq!(c.email.as_deref(), Some("diana@test.com"));
        assert_eq!(c.phone.as_deref(), Some("555-0100"));
        assert_eq!(c.notes, "Preferred");
        assert_eq!(c.loyalty_points, 0);
        assert_eq!(c.total_spent_minor, 0);
    }

    #[test]
    fn create_customer_empty_name() {
        let conn = fresh();
        let err = store(&conn)
            .create_customer("   ", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn update_customer_basic() {
        let conn = fresh();
        seed_customers(&conn);
        let updated = store(&conn)
            .update_customer("cust-1", "Alice Updated", Some("alice@new.com"), None, Some("Changed"))
            .unwrap();
        assert_eq!(updated.name, "Alice Updated");
        assert_eq!(updated.email.as_deref(), Some("alice@new.com"));
        assert_eq!(updated.notes, "Changed");
        assert!(updated.updated_at.as_str() > "2025-01-01");
    }

    #[test]
    fn update_customer_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_customer("nope", "X", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_customer_empty_name() {
        let conn = fresh();
        seed_customers(&conn);
        let err = store(&conn)
            .update_customer("cust-1", "", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn delete_customer_removes_row() {
        let conn = fresh();
        seed_customers(&conn);
        store(&conn).delete_customer("cust-1").unwrap();
        let c = store(&conn).get_customer("cust-1").unwrap();
        assert!(c.is_none());
    }

    #[test]
    fn delete_customer_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_customer("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    // ── Role CRUD ────────────────────────────────────────────────

    fn seed_roles(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-owner',   'owner',   'Full access',           '[\"*\"]',                                 '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('role-cashier', 'cashier', 'Process sales',         '[\"sales:process\"]',                     '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('role-manager', 'manager', 'Manage products + sales','[\"products:crud\",\"sales:void\"]',  '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();
    }

    #[test]
    fn list_roles_empty_db() {
        let conn = fresh();
        let roles = store(&conn).list_roles().unwrap();
        assert!(roles.is_empty());
    }

    #[test]
    fn list_roles_seeded() {
        let conn = fresh();
        seed_roles(&conn);
        let roles = store(&conn).list_roles().unwrap();
        assert_eq!(roles.len(), 3);
        // Ordered by name: cashier, manager, owner.
        assert_eq!(roles[0].name, "cashier");
        assert_eq!(roles[1].name, "manager");
        assert_eq!(roles[2].name, "owner");
    }

    #[test]
    fn get_role_found() {
        let conn = fresh();
        seed_roles(&conn);
        let r = store(&conn).get_role("role-owner").unwrap().unwrap();
        assert_eq!(r.name, "owner");
        assert_eq!(r.permissions, "[\"*\"]");
    }

    #[test]
    fn get_role_not_found() {
        let conn = fresh();
        let r = store(&conn).get_role("nope").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn create_role_basic() {
        let conn = fresh();
        let r = store(&conn)
            .create_role("role-viewer", "viewer", "Read-only access", "[\"sales:view\"]")
            .unwrap();
        assert_eq!(r.name, "viewer");
        assert_eq!(r.description, "Read-only access");
        assert_eq!(r.permissions, "[\"sales:view\"]");
    }

    #[test]
    fn create_role_duplicate_name() {
        let conn = fresh();
        seed_roles(&conn);
        let err = store(&conn)
            .create_role("role-dup", "owner", "Dup", "[]")
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { entity, .. } if entity == "role"));
    }

    // ── User CRUD ────────────────────────────────────────────────

    fn seed_users(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-owner',   'owner',   'Full access',    '[\"*\"]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('role-cashier', 'cashier', 'Process sales',  '[\"sales:process\"]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-1', 'alice',   'hash_alice',   'Alice',   'role-cashier', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('user-2', 'bob',     'hash_bob',     'Bob',     'role-owner',   1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('user-3', 'carol',   'hash_carol',   'Carol',   'role-cashier', 0, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();
    }

    #[test]
    fn list_users_empty_db() {
        let conn = fresh();
        let users = store(&conn).list_users().unwrap();
        assert!(users.is_empty());
    }

    #[test]
    fn list_users_returns_all() {
        let conn = fresh();
        seed_users(&conn);
        let users = store(&conn).list_users().unwrap();
        assert_eq!(users.len(), 3);
        // Ordered by display_name: Alice, Bob, Carol.
        assert_eq!(users[0].username, "alice");
        assert_eq!(users[1].username, "bob");
        assert_eq!(users[2].username, "carol");
    }

    #[test]
    fn get_user_found() {
        let conn = fresh();
        seed_users(&conn);
        let u = store(&conn).get_user("user-1").unwrap().unwrap();
        assert_eq!(u.username, "alice");
        assert_eq!(u.display_name, "Alice");
        assert_eq!(u.role_id, "role-cashier");
        assert!(u.is_active);
    }

    #[test]
    fn get_user_not_found() {
        let conn = fresh();
        let u = store(&conn).get_user("nope").unwrap();
        assert!(u.is_none());
    }

    #[test]
    fn get_user_by_username_found() {
        let conn = fresh();
        seed_users(&conn);
        let u = store(&conn).get_user_by_username("bob").unwrap().unwrap();
        assert_eq!(u.id, "user-2");
        assert_eq!(u.display_name, "Bob");
    }

    #[test]
    fn get_user_by_username_not_found() {
        let conn = fresh();
        let u = store(&conn).get_user_by_username("nobody").unwrap();
        assert!(u.is_none());
    }

    #[test]
    fn get_user_inactive_user() {
        let conn = fresh();
        seed_users(&conn);
        let u = store(&conn).get_user("user-3").unwrap().unwrap();
        assert_eq!(u.username, "carol");
        assert!(!u.is_active);
    }

    #[test]
    fn create_user_minimal() {
        let conn = fresh();
        seed_roles(&conn);
        let u = store(&conn)
            .create_user("diana", "hash_diana", "Diana", "role-cashier")
            .unwrap();
        assert_eq!(u.username, "diana");
        assert_eq!(u.display_name, "Diana");
        assert_eq!(u.role_id, "role-cashier");
        assert!(u.is_active);
        assert!(!u.id.is_empty());
    }

    #[test]
    fn create_user_empty_username() {
        let conn = fresh();
        seed_roles(&conn);
        let err = store(&conn)
            .create_user("", "hash", "Diana", "role-cashier")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "username"));
    }

    #[test]
    fn create_user_empty_display_name() {
        let conn = fresh();
        seed_roles(&conn);
        let err = store(&conn)
            .create_user("diana", "hash", "   ", "role-cashier")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "display_name"));
    }

    #[test]
    fn create_user_duplicate_username() {
        let conn = fresh();
        seed_users(&conn);
        let err = store(&conn)
            .create_user("alice", "hash2", "Alice 2", "role-owner")
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { .. }));
    }

    #[test]
    fn update_user_basic() {
        let conn = fresh();
        seed_users(&conn);
        let updated = store(&conn)
            .update_user("user-1", "alice_new", "Alice Updated", "role-owner", true)
            .unwrap();
        assert_eq!(updated.username, "alice_new");
        assert_eq!(updated.display_name, "Alice Updated");
        assert_eq!(updated.role_id, "role-owner");
        assert!(updated.is_active);
        assert!(updated.updated_at.as_str() > "2025-01-01");
    }

    #[test]
    fn update_user_deactivate() {
        let conn = fresh();
        seed_users(&conn);
        let updated = store(&conn)
            .update_user("user-1", "alice", "Alice", "role-cashier", false)
            .unwrap();
        assert!(!updated.is_active);
    }

    #[test]
    fn update_user_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_user("nope", "u", "U", "role-owner", true)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_user_empty_display_name() {
        let conn = fresh();
        seed_users(&conn);
        let err = store(&conn)
            .update_user("user-1", "alice", "", "role-cashier", true)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "display_name"));
    }

    #[test]
    fn delete_user_removes_row() {
        let conn = fresh();
        seed_users(&conn);
        store(&conn).delete_user("user-3").unwrap();
        let u = store(&conn).get_user("user-3").unwrap();
        assert!(u.is_none());
    }

    #[test]
    fn delete_user_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_user("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
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

    // ── Refund tests ─────────────────────────────────────────────

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

        let line = RefundLine::new(
            "ref-sl-1", "COFFEE", 2,
            price(350), price(700),
        );
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

        let line = RefundLine::new(
            "sl-x", "COFFEE", 1, price(350), price(350),
        );
        let refund = Refund::new(
            "nonexistent",
            price(350),
            "test", "", "user-1",
            vec![line],
        );

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
        let r1 = Refund::new("ref-sale-1", price(350), "partial", "", "user-1", vec![line1]);
        s.create_refund(&r1).unwrap();

        // Second refund: 1 item.
        let line2 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
        let r2 = Refund::new("ref-sale-1", price(350), "partial", "", "user-1", vec![line2]);
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
