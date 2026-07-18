//! Products, categories, inventory, and product variants.
//!
//! Methods are organised under `impl Store<'_>` blocks.

use rusqlite::params;

use crate::error::CoreError;
use crate::money::Currency;
use crate::{Category, Money, Product, ProductVariant, Sku};

use super::{Store, row_to_product};

// ── Enriched product type ────────────────────────────────────────────

/// A [`Product`] enriched with category name and stock quantity from
/// LEFT JOINs on `categories` and `inventory`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// An immutable row in the stock movements delta ledger (ADR #6).
///
/// Each row records a single stock change (+N or -N) with audit
/// metadata. The current stock is computed as `SUM(delta)` across
/// all rows for a given `item_id`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StockMovement {
    /// Unique UUID v7 identifier.
    pub id: String,
    /// Product ID this movement applies to.
    pub item_id: String,
    /// Quantity change: positive = restock, negative = removal.
    pub delta: i64,
    /// Human-readable reason: 'sale', 'restock', 'correction', etc.
    pub reason: Option<String>,
    /// Terminal that performed the operation (for audit/sync).
    pub source_terminal_id: Option<String>,
    /// User who performed the operation (for audit/sync).
    pub source_user_id: Option<String>,
    /// Store where the movement originated (ADR #6 cross-store routing).
    pub store_id: String,
    /// ISO-8601 timestamp of the movement.
    pub created_at: String,
}

// ── Product CRUD ─────────────────────────────────────────────────────

impl Store<'_> {
    /// List all products, ordered by name, with category and stock.
    pub fn list_products(&self) -> Result<Vec<ProductWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at,
                     p.track_serial, p.product_type, p.version,
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

    /// List inventory-tracked products only, ordered by name, with category
    /// and stock. Excludes service-type products (e.g. "car wash") that have
    /// no physical stock. Used by the warehouse/inventory workspace.
    pub fn list_warehouse_products(&self) -> Result<Vec<ProductWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at,
                     p.track_serial, p.product_type, p.version,
                     c.name AS category_name,
                     i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.product_type != 'service'
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map([], row_to_product_with_details)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single product by SKU, including category and stock.
    ///
    /// Checks the cache first; on cache miss, queries the database and
    /// populates the cache.
    pub fn get_product(&self, sku: &str) -> Result<Option<ProductWithDetails>, CoreError> {
        if let Some(cache) = &self.cache
            && let Some(product) = cache.get_product(sku)
        {
            return Ok(Some(product));
        }

        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at,
                     p.track_serial, p.product_type, p.version,
                     c.name AS category_name,
                     i.qty AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.sku = ?1",
        )?;
        let result = stmt.query_row(params![sku], row_to_product_with_details);
        let product = match result {
            Ok(p) => Some(p),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(e.into()),
        };

        if let (Some(cache), Some(p)) = (&self.cache, &product) {
            cache.set_product(sku, p);
        }

        Ok(product)
    }

    /// Look up a single product by barcode, including category and stock.
    pub fn lookup_product_with_details_by_barcode(
        &self,
        barcode: &str,
    ) -> Result<Option<ProductWithDetails>, CoreError> {
        if barcode.trim().is_empty() {
            return Ok(None);
        }
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at,
                     p.track_serial, p.product_type, p.version,
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
    /// `product_type` defaults to `"retail"` when `None`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_product(
        &self,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
        initial_stock: i64,
        product_type: Option<&str>,
    ) -> Result<Product, CoreError> {
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

        let product_type = product_type.unwrap_or("retail");
        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let cur_str = std::str::from_utf8(&price.currency.0)
            .expect("currency bytes are valid UTF-8")
            .to_owned();

        let tx = self.conn.unchecked_transaction()?;

        let result = tx.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial, product_type, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1)",
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
                now,
                0i32,
                product_type,
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

        // Service products never get inventory rows — they have unlimited stock.
        if initial_stock > 0 && product_type != "service" {
            tx.execute(
                "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)",
                params![id, initial_stock, now],
            )?;
            // ADR #6: Record initial stock in the delta ledger.
            let movement_id = uuid::Uuid::now_v7().to_string();
            tx.execute(
                "INSERT INTO stock_movements (id, item_id, delta, reason,
                                              source_terminal_id, source_user_id, created_at)
                 VALUES (?1, ?2, ?3, 'initial-stock', NULL, NULL, ?4)",
                params![movement_id, id, initial_stock, now],
            )?;
            tx.execute(
                "INSERT INTO stock_summary (item_id, qty, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(item_id) DO UPDATE SET qty = excluded.qty,
                                                     updated_at = excluded.updated_at",
                params![id, initial_stock, now],
            )?;
        }

        tx.commit()?;

        if let Some(cache) = &self.cache {
            cache.invalidate_product(sku.trim());
        }

        let parsed_pt = crate::ProductType::parse_str(product_type).unwrap_or_default();
        Ok(Product {
            id,
            sku: Sku::new(sku.trim()),
            name: name.trim().to_owned(),
            price,
            category_id: category_id.map(|s| s.to_owned()),
            barcode: barcode.and_then(|s| foundation::Barcode::new(s).ok()),
            created_at: now.clone(),
            updated_at: now.clone(),
            price_updated_at: now,
            track_serial: false,
            product_type: parsed_pt,
            version: 1,
        })
    }

    /// Update an existing product identified by SKU.
    ///
    /// Uses optimistic concurrency (ADR #6): when `expected_version` is
    /// `Some`, includes `version` in the WHERE clause and increments it
    /// on success. Returns [`CoreError::Conflict`] if another process
    /// modified the product concurrently. When `None`, the update is
    /// performed unconditionally (backward-compat for callers that do
    /// not track versions).
    #[allow(clippy::too_many_arguments)]
    pub fn update_product(
        &self,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
        product_type: Option<&str>,
        expected_version: Option<i64>,
    ) -> Result<Product, CoreError> {
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

        let rows = if let Some(ver) = expected_version {
            self.conn.execute(
                "UPDATE products
                 SET name = ?1, price_minor = ?2, currency = ?3,
                     category_id = ?4, barcode = ?5, updated_at = ?6,
                     product_type = COALESCE(?7, product_type),
                     price_updated_at = CASE WHEN price_minor <> ?2 OR currency <> ?3 THEN ?6 ELSE price_updated_at END,
                     version = version + 1
                 WHERE sku = ?8 AND version = ?9",
                params![
                    name.trim(),
                    price.minor_units,
                    cur_str,
                    category_id,
                    barcode,
                    now,
                    product_type,
                    sku,
                    ver,
                ],
            )?
        } else {
            self.conn.execute(
                "UPDATE products
                 SET name = ?1, price_minor = ?2, currency = ?3,
                     category_id = ?4, barcode = ?5, updated_at = ?6,
                     product_type = COALESCE(?7, product_type),
                     price_updated_at = CASE WHEN price_minor <> ?2 OR currency <> ?3 THEN ?6 ELSE price_updated_at END,
                     version = version + 1
                 WHERE sku = ?8",
                params![
                    name.trim(),
                    price.minor_units,
                    cur_str,
                    category_id,
                    barcode,
                    now,
                    product_type,
                    sku,
                ],
            )?
        };

        if rows == 0 {
            if expected_version.is_some() {
                // Determine if it's a version conflict or a not-found.
                let exists: bool = self.conn.query_row(
                    "SELECT COUNT(*) > 0 FROM products WHERE sku = ?1",
                    params![sku],
                    |r| r.get(0),
                )?;
                if exists {
                    return Err(CoreError::Conflict {
                        entity: "product",
                        field: "version",
                    });
                }
            }
            return Err(CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            });
        }

        if let Some(cache) = &self.cache {
            cache.invalidate_product(sku);
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial, product_type, version
             FROM products WHERE sku = ?1",
        )?;
        let product = stmt.query_row(params![sku], row_to_product)?;
        Ok(product)
    }

    /// Look up a product by barcode (without enrichment).
    pub fn get_product_by_barcode(&self, barcode: &str) -> Result<Option<Product>, CoreError> {
        if barcode.trim().is_empty() {
            return Ok(None);
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial, product_type, version
             FROM products WHERE barcode = ?1",
        )?;
        let result = stmt.query_row(params![barcode.trim()], row_to_product);
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set the `track_serial` flag for a product identified by SKU.
    pub fn set_product_track_serial(&self, sku: &str, track_serial: bool) -> Result<(), CoreError> {
        let rows = self.conn.execute(
            "UPDATE products SET track_serial = ?1 WHERE sku = ?2",
            params![track_serial as i64, sku],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            });
        }
        if let Some(cache) = &self.cache {
            cache.invalidate_product(sku);
        }
        Ok(())
    }

    /// Delete a product by SKU.
    pub fn delete_product(&self, sku: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM products WHERE sku = ?1", params![sku])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            });
        }

        if let Some(cache) = &self.cache {
            cache.invalidate_product(sku);
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
            .prepare("SELECT id, name, colour, icon FROM categories ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: row.get("id")?,
                name: row.get("name")?,
                colour: row.get("colour")?,
                icon: row.get("icon")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Insert a new category.
    pub fn create_category(
        &self,
        id: &str,
        name: &str,
        colour: &str,
        icon: &str,
    ) -> Result<Category, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "category name must not be empty".into(),
            });
        }

        let result = self.conn.execute(
            "INSERT INTO categories (id, name, colour, icon) VALUES (?1, ?2, ?3, ?4)",
            params![id, name.trim(), colour, icon],
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

        Ok(Category::new(id, name, colour, icon))
    }

    /// Update an existing category's name, colour, and icon.
    ///
    /// Returns [`CoreError::NotFound`] if no category with `id` exists.
    pub fn update_category(
        &self,
        id: &str,
        name: &str,
        colour: &str,
        icon: &str,
    ) -> Result<Category, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "category name must not be empty".into(),
            });
        }

        let rows = self.conn.execute(
            "UPDATE categories SET name = ?1, colour = ?2, icon = ?3 WHERE id = ?4",
            params![name.trim(), colour, icon, id],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "category",
                id: id.to_owned(),
            });
        }

        Ok(Category::new(id, name, colour, icon))
    }

    /// Delete a category by id.
    pub fn delete_category(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM categories WHERE id = ?1", params![id])?;
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
            .prepare("SELECT id, name, colour, icon FROM categories WHERE id = ?1")?;
        let result = stmt.query_row(params![id], |row| {
            Ok(Category {
                id: row.get("id")?,
                name: row.get("name")?,
                colour: row.get("colour")?,
                icon: row.get("icon")?,
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
    /// Checks the cache first; on cache miss, queries the database and
    /// populates the cache.
    pub fn get_stock(&self, product_id: &str) -> Result<i64, CoreError> {
        if let Some(cache) = &self.cache
            && let Some(qty) = cache.get_inventory(product_id)
        {
            return Ok(qty);
        }

        let result = self.conn.query_row(
            "SELECT qty FROM inventory WHERE product_id = ?1",
            params![product_id],
            |row| row.get(0),
        );
        let qty = match result {
            Ok(q) => q,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(e.into()),
        };

        if let Some(cache) = &self.cache {
            cache.set_inventory(product_id, qty);
        }

        Ok(qty)
    }

    /// Look up a product id by SKU.
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

    /// Look up a product SKU by product ID.
    pub fn product_sku_by_id(&self, product_id: &str) -> Result<Option<String>, CoreError> {
        let result = self.conn.query_row(
            "SELECT sku FROM products WHERE id = ?1",
            params![product_id],
            |row| row.get(0),
        );
        match result {
            Ok(sku) => Ok(Some(sku)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Look up a product's `product_type` by product ID.
    pub fn product_type_by_id(&self, product_id: &str) -> Result<Option<String>, CoreError> {
        let result = self.conn.query_row(
            "SELECT product_type FROM products WHERE id = ?1",
            params![product_id],
            |row| row.get(0),
        );
        match result {
            Ok(pt) => Ok(Some(pt)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a stock movement delta row directly into the ledger.
    ///
    /// This is the low-level insert used by the sync layer to apply
    /// incoming remote deltas without triggering inventory or stock_summary
    /// updates (those are reconciled later via [`rebuild_stock_summary`]).
    ///
    /// The `store_id` identifies which store the delta originated from
    /// for cross-store routing (ADR #6).
    #[allow(clippy::too_many_arguments)]
    pub fn insert_stock_movement(
        &self,
        id: &str,
        item_id: &str,
        delta: i64,
        reason: Option<&str>,
        source_terminal_id: Option<&str>,
        source_user_id: Option<&str>,
        store_id: &str,
        created_at: &str,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO stock_movements (id, item_id, delta, reason,
                                          source_terminal_id, source_user_id,
                                          store_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                item_id,
                delta,
                reason,
                source_terminal_id,
                source_user_id,
                store_id,
                created_at,
            ],
        )?;
        Ok(())
    }

    /// Adjust stock for a product by SKU inside a transaction.
    ///
    /// Writes a delta row to the `stock_movements` ledger (ADR #6)
    /// and updates the materialised `inventory` and `stock_summary` tables.
    /// The `reason` parameter is recorded in the ledger for audit purposes.
    pub fn adjust_stock(&self, sku: &str, delta: i64) -> Result<i64, CoreError> {
        self.adjust_stock_with_reason(sku, delta, None, None, None)
    }

    /// Adjust stock with an explicit reason for the delta ledger (ADR #6).
    ///
    /// Records the reason (e.g. "sale", "restock", "correction") in the
    /// `stock_movements` row for audit and sync purposes.
    ///
    /// `source_terminal_id` and `source_user_id` are audit columns
    /// populated from the caller's `SessionContext` (ADR #6). Pass
    /// `None` for test/deprecated callers.
    pub fn adjust_stock_with_reason(
        &self,
        sku: &str,
        delta: i64,
        reason: Option<&str>,
        source_terminal_id: Option<&str>,
        source_user_id: Option<&str>,
    ) -> Result<i64, CoreError> {
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
        let movement_id = uuid::Uuid::now_v7().to_string();

        let tx = self.conn.unchecked_transaction()?;

        // 1. Write the immutable delta row (CRDT ledger — ADR #6).
        tx.execute(
            "INSERT INTO stock_movements (id, item_id, delta, reason,
                                          source_terminal_id, source_user_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                movement_id,
                product_id,
                delta,
                reason,
                source_terminal_id,
                source_user_id,
                now
            ],
        )?;

        // 2. Update the materialised inventory table (backward compat).
        tx.execute(
            "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty,
                                                     updated_at = excluded.updated_at",
            params![product_id, new_qty, now],
        )?;

        // 3. Update the stock_summary materialised view (perf — ADR #6).
        tx.execute(
            "INSERT INTO stock_summary (item_id, qty, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(item_id) DO UPDATE SET qty = excluded.qty,
                                                 updated_at = excluded.updated_at",
            params![product_id, new_qty, now],
        )?;

        tx.commit()?;

        if let Some(cache) = &self.cache {
            cache.invalidate_inventory(&product_id);
            cache.publish_inventory_change(&product_id, sku, new_qty, self.terminal_id.as_deref());
        }

        Ok(new_qty)
    }

    /// Compute the current stock quantity from the delta ledger (ADR #6).
    ///
    /// Returns `SUM(delta)` from `stock_movements` for the given product.
    /// Falls back to `inventory.qty` if the ledger table has no rows yet
    /// (backward compatibility with pre-migration databases).
    pub fn get_stock_from_ledger(&self, product_id: &str) -> Result<i64, CoreError> {
        let result = self.conn.query_row(
            "SELECT SUM(delta) FROM stock_movements WHERE item_id = ?1",
            params![product_id],
            |row| row.get::<_, Option<i64>>(0),
        );

        match result {
            Ok(Some(sum)) => Ok(sum),
            Ok(None) => {
                // No deltas yet — fall back to inventory table.
                self.get_stock(product_id)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Rebuild the materialised `stock_summary` and `inventory` tables from the
    /// delta ledger (ADR #6).
    ///
    /// This is called after a sync cycle receives new deltas from other
    /// registers or the cloud, ensuring the materialised cache is consistent
    /// with the authoritative ledger. Runs in a single transaction for atomicity.
    ///
    /// Returns the number of products whose stock was rebuilt.
    pub fn rebuild_stock_summary(&self) -> Result<usize, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tx = self.conn.unchecked_transaction()?;

        // Clear the materialised caches.
        tx.execute("DELETE FROM stock_summary", [])?;

        // Rebuild stock_summary from the delta ledger.
        let rebuilt = tx.execute(
            "INSERT INTO stock_summary (item_id, qty, updated_at)
             SELECT item_id, SUM(delta), ?1
             FROM stock_movements
             GROUP BY item_id",
            params![now],
        )?;

        // Rebuild the inventory table (backward compat) to match.
        // Upsert: set qty for products with deltas, leave others untouched.
        tx.execute(
            "INSERT INTO inventory (product_id, qty, updated_at)
             SELECT item_id, SUM(delta), ?1
             FROM stock_movements
             GROUP BY item_id
             ON CONFLICT(product_id) DO UPDATE SET
                qty = excluded.qty,
                updated_at = excluded.updated_at",
            params![now],
        )?;

        // Zero out inventory for products whose ledger SUM is 0 or negative
        // (e.g., all stock was sold). The INSERT … ON CONFLICT above only
        // handles items present in stock_movements; items with net-zero deltas
        // need explicit zeroing.
        tx.execute(
            "UPDATE inventory SET qty = 0, updated_at = ?1
             WHERE product_id IN (
                SELECT item_id FROM stock_movements
                GROUP BY item_id
                HAVING SUM(delta) <= 0
             )",
            params![now],
        )?;

        tx.commit()?;

        Ok(rebuilt)
    }

    /// List all stock movement rows for a product, ordered by time (ADR #6).
    ///
    /// Returns the complete immutable delta ledger for audit and sync.
    pub fn list_stock_movements(
        &self,
        product_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StockMovement>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_id, delta, reason, source_terminal_id, source_user_id,
                    store_id, created_at
             FROM stock_movements
             WHERE item_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![product_id, limit, offset], |row| {
            Ok(StockMovement {
                id: row.get(0)?,
                item_id: row.get(1)?,
                delta: row.get(2)?,
                reason: row.get(3)?,
                source_terminal_id: row.get(4)?,
                source_user_id: row.get(5)?,
                store_id: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Archive stock movements older than `older_than_days` days.
    ///
    /// Uses archive-rollup consolidation (ADR #6 Q4 / P-1 Ledger Retention):
    ///
    /// 1. Copies old rows to `stock_movements_archive` for audit compliance.
    /// 2. Inserts a single rollup row per product — `SUM(delta)` of all
    ///    archived rows, with `reason: 'archive-rollup'`.
    /// 3. Deletes old rows from the live table.
    ///
    /// Rollup rows are excluded from future archiving via `WHERE reason != 'archive-rollup'`.
    /// Each item_id group is processed in its own transaction so concurrent
    /// `adjust_stock` calls are never blocked for long.
    ///
    /// Capped at `max_groups` item_id groups per call to bound runtime
    /// (subsequent calls pick up remaining groups — idempotent).
    ///
    /// Returns the number of item groups that were archived.
    pub fn archive_stock_movements(
        &self,
        older_than_days: i64,
        max_groups: usize,
    ) -> Result<usize, CoreError> {
        // Compute the cutoff timestamp (now minus older_than_days).
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days);
        let cutoff_str = cutoff.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        // Find item_ids that have archivable rows (excluding rollup rows).
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT item_id
             FROM stock_movements
             WHERE created_at < ?1
               AND reason != 'archive-rollup'
             LIMIT ?2",
        )?;
        let item_ids: Vec<String> = stmt
            .query_map(params![cutoff_str, max_groups as i64], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        if item_ids.is_empty() {
            return Ok(0);
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut groups_archived = 0usize;

        for item_id in &item_ids {
            let tx = self.conn.unchecked_transaction()?;

            // 1. Copy old rows to archive (skip previous rollup rows).
            tx.execute(
                "INSERT INTO stock_movements_archive
                 SELECT id, item_id, delta, reason,
                        source_terminal_id, source_user_id,
                        store_id, created_at
                 FROM stock_movements
                 WHERE item_id = ?1
                   AND created_at < ?2
                   AND reason != 'archive-rollup'",
                params![item_id, cutoff_str],
            )?;

            // 2. Insert a rollup row consolidating all archived deltas.
            let rollup_id = uuid::Uuid::now_v7().to_string();
            tx.execute(
                "INSERT INTO stock_movements (id, item_id, delta, reason, store_id, created_at)
                 SELECT ?1, ?2, COALESCE(SUM(delta), 0), 'archive-rollup', '', ?3
                 FROM stock_movements
                 WHERE item_id = ?2
                   AND created_at < ?4
                   AND reason != 'archive-rollup'",
                params![rollup_id, item_id, now, cutoff_str],
            )?;

            // 3. Delete old rows from the live table.
            tx.execute(
                "DELETE FROM stock_movements
                 WHERE item_id = ?1
                   AND created_at < ?2
                   AND reason != 'archive-rollup'",
                params![item_id, cutoff_str],
            )?;

            tx.commit()?;
            groups_archived += 1;
        }

        // Run incremental vacuum once after all groups to reclaim disk space.
        self.conn
            .execute_batch("PRAGMA incremental_vacuum(50)")
            .map_err(|e| CoreError::Internal(format!("incremental_vacuum failed: {e}")))?;

        Ok(groups_archived)
    }
}

// ── Product Variants ─────────────────────────────────────────

impl Store<'_> {
    /// List all variants for a given parent SKU, ordered by sort_order.
    pub fn list_product_variants(
        &self,
        parent_sku: &str,
    ) -> Result<Vec<ProductVariant>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, parent_sku, name, sku, price_minor, currency, barcode,
                    sort_order, is_active, created_at, updated_at
             FROM product_variants
             WHERE parent_sku = ?1
             ORDER BY sort_order ASC, name ASC",
        )?;
        let rows = stmt.query_map(params![parent_sku], Self::row_to_product_variant)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get a single variant by its own SKU.
    pub fn get_product_variant(&self, sku: &str) -> Result<Option<ProductVariant>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, parent_sku, name, sku, price_minor, currency, barcode,
                    sort_order, is_active, created_at, updated_at
             FROM product_variants WHERE sku = ?1",
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
            Some(m) => (
                Some(m.minor_units),
                Some(
                    std::str::from_utf8(&m.currency.0)
                        .unwrap_or("USD")
                        .to_owned(),
                ),
            ),
            None => (None, None),
        };

        self.conn.execute(
            "INSERT INTO product_variants (id, parent_sku, name, sku, price_minor, currency, barcode,
                                           sort_order, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                variant.id, variant.parent_sku, variant.name, variant.sku,
                price_minor, currency_str, variant.barcode.as_ref().map(|b| b.as_str()),
                variant.sort_order, variant.is_active as i64,
                variant.created_at, variant.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Update an existing product variant (matched by SKU).
    pub fn update_product_variant(&self, variant: &ProductVariant) -> Result<(), CoreError> {
        let (price_minor, currency_str) = match &variant.price {
            Some(m) => (
                Some(m.minor_units),
                Some(
                    std::str::from_utf8(&m.currency.0)
                        .unwrap_or("USD")
                        .to_owned(),
                ),
            ),
            None => (None, None),
        };

        let affected = self.conn.execute(
            "UPDATE product_variants SET name = ?1, price_minor = ?2, currency = ?3,
                                          barcode = ?4, sort_order = ?5, is_active = ?6,
                                          updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE sku = ?7",
            params![
                variant.name,
                price_minor,
                currency_str,
                variant.barcode.as_ref().map(|b| b.as_str()),
                variant.sort_order,
                variant.is_active as i64,
                variant.sku
            ],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "product_variant",
                id: variant.sku.clone(),
            });
        }
        Ok(())
    }

    /// Delete a product variant by its own SKU.
    pub fn delete_product_variant(&self, sku: &str) -> Result<(), CoreError> {
        let affected = self
            .conn
            .execute("DELETE FROM product_variants WHERE sku = ?1", params![sku])?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "product_variant",
                id: sku.to_owned(),
            });
        }
        Ok(())
    }

    fn row_to_product_variant(row: &rusqlite::Row) -> rusqlite::Result<ProductVariant> {
        let price_minor: Option<i64> = row.get("price_minor")?;
        let currency_str: Option<String> = row.get("currency")?;
        let price = match (price_minor, currency_str) {
            (Some(minor), Some(cur)) => {
                let c: Result<Currency, _> = cur.parse();
                c.ok().map(|currency| Money {
                    minor_units: minor,
                    currency,
                })
            }
            _ => None,
        };

        let barcode_raw: Option<String> = row.get("barcode")?;
        Ok(ProductVariant {
            id: row.get("id")?,
            parent_sku: row.get("parent_sku")?,
            name: row.get("name")?,
            sku: row.get("sku")?,
            price,
            barcode: barcode_raw.and_then(|s| foundation::Barcode::new(&s).ok()),
            sort_order: row.get("sort_order")?,
            is_active: row.get::<_, i64>("is_active")? != 0,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Money;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
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
        let espresso = products
            .iter()
            .find(|p| p.product.sku.as_str() == "DRINK-001")
            .unwrap();
        assert_eq!(espresso.category_name.as_deref(), Some("Drinks"));
    }

    #[test]
    fn list_products_includes_stock_qty() {
        let conn = fresh();
        seed_everything(&conn);
        let products = store(&conn).list_products().unwrap();
        let espresso = products
            .iter()
            .find(|p| p.product.sku.as_str() == "DRINK-001")
            .unwrap();
        assert_eq!(espresso.stock_qty, Some(50));
        let tea = products
            .iter()
            .find(|p| p.product.sku.as_str() == "DRINK-002")
            .unwrap();
        assert_eq!(tea.stock_qty, None);
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
            .create_product("NEW-001", "Widget", price(199), None, None, 0, None)
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
                None,
            )
            .unwrap();
        assert_eq!(p.category_id.as_deref(), Some("cat-drinks"));
        assert_eq!(
            p.barcode.as_ref().map(|b| b.as_str()),
            Some("1234567890123")
        );
        let qty = store(&conn).get_stock(&p.id).unwrap();
        assert_eq!(qty, 5);
    }

    #[test]
    fn create_product_without_stock() {
        let conn = fresh();
        let p = store(&conn)
            .create_product("NOSTOCK", "No Stock", price(100), None, None, 0, None)
            .unwrap();
        let qty = store(&conn).get_stock(&p.id).unwrap();
        assert_eq!(qty, 0);
    }

    #[test]
    fn create_product_duplicate_sku() {
        let conn = fresh();
        store(&conn)
            .create_product("DUP", "First", price(100), None, None, 0, None)
            .unwrap();
        let err = store(&conn)
            .create_product("DUP", "Second", price(200), None, None, 0, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { .. }));
    }

    #[test]
    fn create_product_validation_errors() {
        let conn = fresh();
        let s = store(&conn);
        let err = s
            .create_product("  ", "X", price(1), None, None, 0, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "sku"));
        let err = s
            .create_product("SKU", "", price(1), None, None, 0, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
        let err = s
            .create_product("SKU", "X", price(-1), None, None, 0, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "price"));
        let err = s
            .create_product("SKU", "X", price(1), None, None, -5, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "initial_stock"));
    }

    #[test]
    fn create_service_product_never_gets_inventory_row() {
        let conn = fresh();
        // Even with initial_stock > 0, service products skip inventory.
        let p = store(&conn)
            .create_product(
                "CARWASH",
                "Car Wash",
                price(5000),
                None,
                None,
                10,
                Some("service"),
            )
            .unwrap();
        assert_eq!(p.product_type, crate::ProductType::Service);
        // get_stock returns 0 when no inventory row exists.
        let qty = store(&conn).get_stock(&p.id).unwrap();
        assert_eq!(qty, 0);
        // list_products returns stock_qty = None via LEFT JOIN.
        let pwd = store(&conn).get_product("CARWASH").unwrap().unwrap();
        assert_eq!(
            pwd.stock_qty, None,
            "service product should have null stock_qty"
        );
    }

    // ── Product update / delete ─────────────────────────────────

    #[test]
    fn update_product_basic() {
        let conn = fresh();
        seed_everything(&conn);
        let updated = store(&conn)
            .update_product("DRINK-001", "Latte", price(400), None, None, None, Some(1))
            .unwrap();
        assert_eq!(updated.name, "Latte");
        assert_eq!(updated.price.minor_units, 400);
        assert_eq!(updated.sku.as_str(), "DRINK-001");
    }

    #[test]
    fn update_product_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_product("NOPE", "X", price(1), None, None, None, Some(1))
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_product_empty_name() {
        let conn = fresh();
        seed_everything(&conn);
        let err = store(&conn)
            .update_product("DRINK-001", "", price(1), None, None, None, Some(1))
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn update_product_negative_price() {
        let conn = fresh();
        seed_everything(&conn);
        let err = store(&conn)
            .update_product("DRINK-001", "X", price(-1), None, None, None, Some(1))
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "price"));
    }

    #[test]
    fn update_product_with_category() {
        let conn = fresh();
        seed_everything(&conn);
        let updated = store(&conn)
            .update_product(
                "DRINK-001",
                "Latte",
                price(400),
                Some("cat-food"),
                None,
                None,
                Some(1),
            )
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
            .create_category("cat-tools", "Tools", "#10b981", "dots-1")
            .unwrap();
        assert_eq!(cat.id, "cat-tools");
        assert_eq!(cat.name, "Tools");
        assert_eq!(cat.colour, "#10b981");
        assert_eq!(cat.icon, "dots-1");
    }

    #[test]
    fn create_category_duplicate_name() {
        let conn = fresh();
        store(&conn)
            .create_category("cat-1", "Drinks", "#000", "")
            .unwrap();
        let err = store(&conn)
            .create_category("cat-2", "Drinks", "#fff", "")
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { .. }));
    }

    #[test]
    fn create_category_empty_name() {
        let conn = fresh();
        let err = store(&conn)
            .create_category("cat-1", "   ", "#000", "")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn delete_category_removes_row() {
        let conn = fresh();
        store(&conn)
            .create_category("cat-orphan", "Orphan", "#000", "")
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

    // ── Inventory ────────────────────────────────────────────────

    #[test]
    fn adjust_stock_add() {
        let conn = fresh();
        seed_everything(&conn);
        let new_qty = store(&conn).adjust_stock("DRINK-001", 5).unwrap();
        assert_eq!(new_qty, 55);
    }

    #[test]
    fn adjust_stock_remove() {
        let conn = fresh();
        seed_everything(&conn);
        let new_qty = store(&conn).adjust_stock("DRINK-001", -10).unwrap();
        assert_eq!(new_qty, 40);
    }

    #[test]
    fn adjust_stock_negative_error() {
        let conn = fresh();
        seed_everything(&conn);
        let err = store(&conn).adjust_stock("DRINK-001", -100).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "delta"));
    }

    #[test]
    fn adjust_stock_unknown_sku() {
        let conn = fresh();
        let err = store(&conn).adjust_stock("NO-SKU", 5).unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    // ── Barcode lookup ───────────────────────────────────────────

    #[test]
    fn lookup_product_by_barcode_found() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .lookup_product_with_details_by_barcode("5901234123457")
            .unwrap()
            .unwrap();
        assert_eq!(p.product.sku.as_str(), "FOOD-001");
        assert_eq!(p.product.name, "Bagel");
        assert_eq!(p.stock_qty, Some(12));
    }

    #[test]
    fn lookup_product_by_barcode_not_found() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .lookup_product_with_details_by_barcode("0000000000000")
            .unwrap();
        assert!(p.is_none());
    }

    #[test]
    fn lookup_product_by_barcode_empty_string() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .lookup_product_with_details_by_barcode("")
            .unwrap();
        assert!(p.is_none(), "empty barcode should return None");
    }

    #[test]
    fn lookup_product_by_barcode_whitespace() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .lookup_product_with_details_by_barcode("   ")
            .unwrap();
        assert!(p.is_none(), "whitespace-only barcode should return None");
    }

    #[test]
    fn get_product_by_barcode_found() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .get_product_by_barcode("5901234123457")
            .unwrap()
            .unwrap();
        assert_eq!(p.sku.as_str(), "FOOD-001");
    }

    #[test]
    fn get_product_by_barcode_not_found() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .get_product_by_barcode("0000000000000")
            .unwrap();
        assert!(p.is_none());
    }

    #[test]
    fn get_product_by_barcode_empty() {
        let conn = fresh();
        let p = store(&conn).get_product_by_barcode("").unwrap();
        assert!(p.is_none());
    }

    #[test]
    fn get_product_by_barcode_trims_input() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn)
            .get_product_by_barcode("  5901234123457  ")
            .unwrap()
            .unwrap();
        assert_eq!(p.sku.as_str(), "FOOD-001");
    }

    #[test]
    fn product_has_no_barcode_by_default() {
        let conn = fresh();
        seed_everything(&conn);
        let p = store(&conn).get_product("DRINK-001").unwrap().unwrap();
        assert!(p.product.barcode.is_none());
    }

    // ── get_stock / product_id_by_sku ────────────────────────────

    #[test]
    fn get_stock_for_existing_product() {
        let conn = fresh();
        seed_everything(&conn);
        let id = store(&conn)
            .product_id_by_sku("DRINK-001")
            .unwrap()
            .unwrap();
        let qty = store(&conn).get_stock(&id).unwrap();
        assert_eq!(qty, 50);
    }

    #[test]
    fn get_stock_for_unstocked_product() {
        let conn = fresh();
        seed_everything(&conn);
        let id = store(&conn)
            .product_id_by_sku("DRINK-002")
            .unwrap()
            .unwrap();
        let qty = store(&conn).get_stock(&id).unwrap();
        assert_eq!(qty, 0, "unstocked product should return 0");
    }

    #[test]
    fn product_id_by_sku_unknown() {
        let conn = fresh();
        let id = store(&conn).product_id_by_sku("NO-SKU").unwrap();
        assert!(id.is_none());
    }

    // ── Product Variant CRUD ─────────────────────────────────────

    fn seed_product_variant_parent(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at, price_updated_at) VALUES
                ('pv-parent', 'PARENT-001', 'Parent Product', 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    #[test]
    fn create_and_list_product_variants() {
        let conn = fresh();
        seed_product_variant_parent(&conn);
        let s = store(&conn);

        let v1 = ProductVariant {
            id: uuid::Uuid::now_v7().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "Small".into(),
            sku: "PARENT-001-SMALL".into(),
            price: Some(price(800)),
            barcode: Some(foundation::Barcode::new("sm-barcode").unwrap()),
            sort_order: 1,
            is_active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };

        let v2 = ProductVariant {
            id: uuid::Uuid::now_v7().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "Large".into(),
            sku: "PARENT-001-LARGE".into(),
            price: Some(price(1200)),
            barcode: None,
            sort_order: 2,
            is_active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };

        s.create_product_variant(&v1).unwrap();
        s.create_product_variant(&v2).unwrap();

        let variants = s.list_product_variants("PARENT-001").unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].sku, "PARENT-001-SMALL");
        assert_eq!(variants[1].sku, "PARENT-001-LARGE");

        // Verify price and barcode on first variant.
        assert_eq!(variants[0].price.unwrap().minor_units, 800);
        assert_eq!(
            variants[0].barcode.as_ref().map(|b| b.as_str()),
            Some("sm-barcode")
        );
        assert!(variants[0].is_active);
    }

    #[test]
    fn list_product_variants_empty() {
        let conn = fresh();
        seed_product_variant_parent(&conn);
        let variants = store(&conn).list_product_variants("PARENT-001").unwrap();
        assert!(variants.is_empty());
    }

    #[test]
    fn get_product_variant_found() {
        let conn = fresh();
        seed_product_variant_parent(&conn);
        let s = store(&conn);
        let v = ProductVariant {
            id: uuid::Uuid::now_v7().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "Medium".into(),
            sku: "PARENT-001-MED".into(),
            price: None,
            barcode: None,
            sort_order: 1,
            is_active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };
        s.create_product_variant(&v).unwrap();

        let found = s.get_product_variant("PARENT-001-MED").unwrap().unwrap();
        assert_eq!(found.name, "Medium");
        assert!(found.price.is_none());
    }

    #[test]
    fn get_product_variant_not_found() {
        let conn = fresh();
        let found = store(&conn).get_product_variant("NO-VARIANT").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn update_product_variant() {
        let conn = fresh();
        seed_product_variant_parent(&conn);
        let s = store(&conn);
        let v = ProductVariant {
            id: uuid::Uuid::now_v7().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "Original".into(),
            sku: "VAR-001".into(),
            price: Some(price(500)),
            barcode: Some(foundation::Barcode::new("orig").unwrap()),
            sort_order: 1,
            is_active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };
        s.create_product_variant(&v).unwrap();

        let updated_v = ProductVariant {
            name: "Updated".into(),
            sku: "VAR-001".into(),
            price: Some(price(600)),
            barcode: Some(foundation::Barcode::new("new-barcode").unwrap()),
            sort_order: 2,
            is_active: false,
            ..v
        };
        s.update_product_variant(&updated_v).unwrap();

        let found = s.get_product_variant("VAR-001").unwrap().unwrap();
        assert_eq!(found.name, "Updated");
        assert_eq!(found.price.unwrap().minor_units, 600);
        assert!(!found.is_active);
    }

    #[test]
    fn update_product_variant_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let v = ProductVariant {
            id: "vid".into(),
            parent_sku: "P".into(),
            name: "X".into(),
            sku: "NO-SKU".into(),
            price: None,
            barcode: None,
            sort_order: 0,
            is_active: true,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let err = s.update_product_variant(&v).unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "product_variant"));
    }

    #[test]
    fn delete_product_variant_removes() {
        let conn = fresh();
        seed_product_variant_parent(&conn);
        let s = store(&conn);
        let v = ProductVariant {
            id: uuid::Uuid::now_v7().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "Delete Me".into(),
            sku: "VAR-TO-DEL".into(),
            price: None,
            barcode: None,
            sort_order: 0,
            is_active: true,
            created_at: String::new(),
            updated_at: String::new(),
        };
        s.create_product_variant(&v).unwrap();
        s.delete_product_variant("VAR-TO-DEL").unwrap();
        let found = s.get_product_variant("VAR-TO-DEL").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn delete_product_variant_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_product_variant("NO-SKU").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "product_variant"));
    }

    #[test]
    fn variant_price_as_none() {
        let conn = fresh();
        seed_product_variant_parent(&conn);
        let s = store(&conn);
        let v = ProductVariant {
            id: uuid::Uuid::now_v7().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "No Price".into(),
            sku: "VAR-NO-PRICE".into(),
            price: None,
            barcode: None,
            sort_order: 0,
            is_active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };
        s.create_product_variant(&v).unwrap();
        let found = s.get_product_variant("VAR-NO-PRICE").unwrap().unwrap();
        assert!(found.price.is_none());
    }

    // ── Stock Movements Delta Ledger (ADR #6) ───────────────────

    #[test]
    fn stock_movements_table_exists() {
        let conn = fresh();
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stock_movements'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            exists, 1,
            "stock_movements table should exist after migration"
        );
    }

    #[test]
    fn stock_summary_table_exists() {
        let conn = fresh();
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stock_summary'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            exists, 1,
            "stock_summary table should exist after migration"
        );
    }

    #[test]
    fn adjust_stock_writes_to_ledger() {
        let conn = fresh();
        seed_everything(&conn);

        store(&conn)
            .adjust_stock_with_reason(
                "DRINK-001",
                -3,
                Some("sale"),
                Some("term-1"),
                Some("user-1"),
            )
            .unwrap();

        // Verify ledger row was written.
        let movements = store(&conn).list_stock_movements("prod-1", 10, 0).unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].delta, -3);
        assert_eq!(movements[0].reason.as_deref(), Some("sale"));
        assert_eq!(movements[0].item_id, "prod-1");
    }

    #[test]
    fn adjust_stock_without_reason_writes_to_ledger() {
        let conn = fresh();
        seed_everything(&conn);

        store(&conn).adjust_stock("DRINK-001", 5).unwrap();

        let movements = store(&conn).list_stock_movements("prod-1", 10, 0).unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].delta, 5);
        assert!(movements[0].reason.is_none());
    }

    #[test]
    fn get_stock_from_ledger_computes_sum() {
        let conn = fresh();
        seed_everything(&conn);

        // The migration backfill runs against empty inventory (before seed_everything),
        // so the ledger starts with no rows. get_stock_from_ledger falls back to
        // inventory.qty = 50.
        let initial = store(&conn).get_stock_from_ledger("prod-1").unwrap();
        assert_eq!(initial, 50, "fallback to inventory returns 50");

        // Adjustment writes a delta row. SUM(delta) = 10 (just the adjustment).
        store(&conn).adjust_stock("DRINK-001", 10).unwrap();
        let after = store(&conn).get_stock_from_ledger("prod-1").unwrap();
        assert_eq!(after, 10, "SUM(delta) should be 10 (only adjustment row)");

        // Multiple adjustments accumulate.
        store(&conn).adjust_stock("DRINK-001", -5).unwrap();
        store(&conn).adjust_stock("DRINK-001", 20).unwrap();
        let after2 = store(&conn).get_stock_from_ledger("prod-1").unwrap();
        assert_eq!(after2, 25, "SUM of deltas: 10 + (-5) + 20 = 25");
    }

    #[test]
    fn get_stock_from_ledger_zero_deltas() {
        let conn = fresh();
        // fresh DB has no products, so ledger should have no rows.
        // Fallback to inventory table returns 0.
        let qty = store(&conn).get_stock_from_ledger("nonexistent").unwrap();
        assert_eq!(qty, 0);
    }
    #[test]
    fn list_stock_movements_paginated() {
        let conn = fresh();
        seed_everything(&conn);

        // Write 5 movements (migration backfill ran against empty inventory,
        // so only these 5 adjust_stock calls create rows).
        for _i in 0..5 {
            store(&conn)
                .adjust_stock_with_reason(
                    "DRINK-001",
                    1,
                    Some("restock"),
                    Some("term-1"),
                    Some("user-1"),
                )
                .unwrap();
        }

        // With limit 3, should return 3 most recent.
        let page1 = store(&conn).list_stock_movements("prod-1", 3, 0).unwrap();
        assert_eq!(page1.len(), 3);

        // With offset 3, should return remaining 2.
        let page2 = store(&conn).list_stock_movements("prod-1", 10, 3).unwrap();
        assert_eq!(page2.len(), 2);
    }

    #[test]
    fn adjust_stock_writes_source_audit_fields() {
        let conn = fresh();
        seed_everything(&conn);

        store(&conn)
            .adjust_stock_with_reason(
                "DRINK-001",
                -5,
                Some("sale"),
                Some("term-kitchen"),
                Some("user-alice"),
            )
            .unwrap();

        let movements = store(&conn).list_stock_movements("prod-1", 1, 0).unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(
            movements[0].source_terminal_id.as_deref(),
            Some("term-kitchen")
        );
        assert_eq!(movements[0].source_user_id.as_deref(), Some("user-alice"));
        assert_eq!(movements[0].delta, -5);
        assert_eq!(movements[0].reason.as_deref(), Some("sale"));
    }

    #[test]
    fn adjust_stock_without_source_audit_stores_nulls() {
        let conn = fresh();
        seed_everything(&conn);

        // adjust_stock (the backward-compat wrapper) passes None for audit fields.
        store(&conn).adjust_stock("DRINK-001", 10).unwrap();

        let movements = store(&conn).list_stock_movements("prod-1", 1, 0).unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].source_terminal_id, None);
        assert_eq!(movements[0].source_user_id, None);
    }

    #[test]
    fn rebuild_stock_summary_from_ledger() {
        let conn = fresh();
        seed_everything(&conn);

        // Insert deltas that bypass the normal adjust_stock path
        // (simulating external sync deltas).
        conn.execute_batch(
            "INSERT INTO stock_movements (id, item_id, delta, reason, created_at) VALUES
                ('sm-1', 'prod-1', 50, 'migration-seed', '2025-01-01T00:00:00.000Z'),
                ('sm-2', 'prod-1', -10, 'sale', '2025-01-02T00:00:00.000Z'),
                ('sm-3', 'prod-2', 100, 'restock', '2025-01-01T00:00:00.000Z'),
                ('sm-4', 'prod-2', -25, 'sale', '2025-01-02T00:00:00.000Z');",
        )
        .unwrap();

        let count = store(&conn).rebuild_stock_summary().unwrap();
        assert_eq!(count, 2, "should rebuild 2 product stock levels");

        // Verify stock_summary was rebuilt.
        let qty1: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary WHERE item_id = 'prod-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(qty1, 40, "prod-1: 50 + (-10) = 40");

        let qty2: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary WHERE item_id = 'prod-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(qty2, 75, "prod-2: 100 + (-25) = 75");

        // Verify inventory was synced.
        let inv1 = store(&conn).get_stock("prod-1").unwrap();
        assert_eq!(inv1, 40);
        let inv2 = store(&conn).get_stock("prod-2").unwrap();
        assert_eq!(inv2, 75);
    }

    #[test]
    fn rebuild_stock_summary_empty_ledger() {
        let conn = fresh();

        // Rebuild on a fresh DB with no movements.
        let count = store(&conn).rebuild_stock_summary().unwrap();
        assert_eq!(count, 0, "no rows to rebuild");

        // stock_summary should be empty.
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock_summary", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }

    // ── Archive Stock Movements (ADR #6 Q4) ─────────────────────

    #[test]
    fn archive_movements_table_exists() {
        let conn = fresh();
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stock_movements_archive'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            exists, 1,
            "stock_movements_archive table should exist after migration 072"
        );
    }

    #[test]
    fn archive_movements_empty_db_returns_zero() {
        let conn = fresh();
        let count = store(&conn).archive_stock_movements(90, 50).unwrap();
        assert_eq!(count, 0, "no rows to archive in empty DB");
    }

    #[test]
    fn archive_movements_no_old_rows_returns_zero() {
        let conn = fresh();
        seed_everything(&conn);
        // Write a recent movement.
        store(&conn).adjust_stock("DRINK-001", 5).unwrap();

        // All rows are recent — nothing to archive.
        let count = store(&conn).archive_stock_movements(90, 50).unwrap();
        assert_eq!(count, 0);

        // Live table still has the adjustment row.
        let movements = store(&conn).list_stock_movements("prod-1", 10, 0).unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].delta, 5);
    }

    #[test]
    fn archive_movements_creates_rollup_row() {
        let conn = fresh();
        seed_everything(&conn);
        let s = store(&conn);

        // Insert old rows by manually setting created_at.
        conn.execute_batch(
            "INSERT INTO stock_movements (id, item_id, delta, reason, store_id, created_at) VALUES
                ('sm-old-1', 'prod-1', 30, 'restock', '', '2020-01-01T00:00:00Z'),
                ('sm-old-2', 'prod-1', -5, 'sale',    '', '2020-02-01T00:00:00Z'),
                ('sm-old-3', 'prod-1', 10, 'restock', '', '2020-03-01T00:00:00Z');",
        )
        .unwrap();

        // Archive with 30-day window (all rows are "old").
        let count = s.archive_stock_movements(30, 50).unwrap();
        assert_eq!(count, 1, "one item group archived");

        // Live table should have one rollup row.
        let movements = s.list_stock_movements("prod-1", 10, 0).unwrap();
        assert_eq!(movements.len(), 1, "one rollup row in live table");
        assert_eq!(movements[0].reason.as_deref(), Some("archive-rollup"));
        assert_eq!(
            movements[0].delta, 35,
            "SUM(old deltas) = 30 + (-5) + 10 = 35"
        );

        // Archive table should have the 3 old rows.
        let archived: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stock_movements_archive WHERE item_id = 'prod-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(archived, 3, "three old rows archived");

        // SUM(delta) from live table should equal SUM(delta) of original rows.
        let from_ledger = s.get_stock_from_ledger("prod-1").unwrap();
        assert_eq!(from_ledger, 35, "SUM(delta) preserved via rollup");
    }

    #[test]
    fn archive_movements_preserves_recent_rows() {
        let conn = fresh();
        seed_everything(&conn);
        let s = store(&conn);

        // Mix of old and new rows.
        conn.execute_batch(
            "INSERT INTO stock_movements (id, item_id, delta, reason, store_id, created_at) VALUES
                ('sm-old-1', 'prod-1', 50, 'restock', '', '2020-01-01T00:00:00Z'),
                ('sm-old-2', 'prod-1', -10, 'sale',    '', '2020-02-01T00:00:00Z');",
        )
        .unwrap();
        // New row via normal API (gets current timestamp).
        s.adjust_stock("DRINK-001", 5).unwrap();

        let count = s.archive_stock_movements(30, 50).unwrap();
        assert_eq!(count, 1, "one item group archived");

        let movements = s.list_stock_movements("prod-1", 10, 0).unwrap();
        // Should have: 1 recent adjustment + 1 rollup = 2 rows.
        assert_eq!(movements.len(), 2);

        let rollup = movements
            .iter()
            .find(|m| m.reason.as_deref() == Some("archive-rollup"))
            .unwrap();
        assert_eq!(rollup.delta, 40, "SUM of archived deltas = 50 + (-10) = 40");

        let recent = movements
            .iter()
            .find(|m| m.reason.as_deref() != Some("archive-rollup"))
            .unwrap();
        assert_eq!(recent.delta, 5, "recent delta untouched");

        // SUM from ledger = rollup + recent = 40 + 5 = 45.
        let from_ledger = s.get_stock_from_ledger("prod-1").unwrap();
        assert_eq!(from_ledger, 45);
    }

    #[test]
    fn archive_movements_idempotent() {
        let conn = fresh();
        seed_everything(&conn);
        let s = store(&conn);

        conn.execute_batch(
            "INSERT INTO stock_movements (id, item_id, delta, reason, store_id, created_at) VALUES
                ('sm-old-1', 'prod-1', 20, 'restock', '', '2020-01-01T00:00:00Z');",
        )
        .unwrap();

        // First archive creates the rollup.
        let first = s.archive_stock_movements(30, 50).unwrap();
        assert_eq!(first, 1);

        // Second archive should be a no-op (rollup excluded from archiving).
        let second = s.archive_stock_movements(30, 50).unwrap();
        assert_eq!(second, 0, "no new groups to archive");

        let movements = s.list_stock_movements("prod-1", 10, 0).unwrap();
        assert_eq!(movements.len(), 1, "still one rollup row");
        assert_eq!(movements[0].delta, 20);
    }

    #[test]
    fn archive_movements_respects_max_groups() {
        let conn = fresh();
        seed_everything(&conn);
        let s = store(&conn);

        // Insert old rows for two products.
        conn.execute_batch(
            "INSERT INTO stock_movements (id, item_id, delta, reason, store_id, created_at) VALUES
                ('sm-old-a', 'prod-1', 10, 'restock', '', '2020-01-01T00:00:00Z'),
                ('sm-old-b', 'prod-2', 20, 'restock', '', '2020-01-01T00:00:00Z');",
        )
        .unwrap();

        // Cap at 1 group — should only archive prod-1.
        let count = s.archive_stock_movements(30, 1).unwrap();
        assert_eq!(count, 1, "only one group archived (capped)");

        // Second call picks up remaining group.
        let count2 = s.archive_stock_movements(30, 50).unwrap();
        assert_eq!(count2, 1, "second group archived");
    }

    #[test]
    fn archive_movements_does_not_archive_rollup_rows() {
        let conn = fresh();
        seed_everything(&conn);
        let s = store(&conn);

        // Insert old rows.
        conn.execute_batch(
            "INSERT INTO stock_movements (id, item_id, delta, reason, store_id, created_at) VALUES
                ('sm-old-1', 'prod-1', 50, 'restock', '', '2020-01-01T00:00:00Z');",
        )
        .unwrap();

        // Archive once.
        s.archive_stock_movements(30, 50).unwrap();

        // Verify the rollup row is not in the archive table.
        let rollup_in_archive: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stock_movements_archive WHERE reason = 'archive-rollup'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rollup_in_archive, 0, "rollup rows are never archived");

        // The original old row IS in the archive.
        let old_in_archive: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stock_movements_archive WHERE id = 'sm-old-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old_in_archive, 1, "old row preserved in archive");
    }

    #[test]
    fn archive_movements_zero_sum_creates_rollup_with_zero() {
        let conn = fresh();
        seed_everything(&conn);
        let s = store(&conn);

        // Rows that cancel out: 50 + (-30) + (-20) = 0.
        conn.execute_batch(
            "INSERT INTO stock_movements (id, item_id, delta, reason, store_id, created_at) VALUES
                ('sm-zero-1', 'prod-1', 50,  'restock', '', '2020-01-01T00:00:00Z'),
                ('sm-zero-2', 'prod-1', -30, 'sale',    '', '2020-02-01T00:00:00Z'),
                ('sm-zero-3', 'prod-1', -20, 'sale',    '', '2020-03-01T00:00:00Z');",
        )
        .unwrap();

        s.archive_stock_movements(30, 50).unwrap();

        let movements = s.list_stock_movements("prod-1", 10, 0).unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(
            movements[0].delta, 0,
            "rollup delta = 0 for net-zero deltas"
        );

        let from_ledger = s.get_stock_from_ledger("prod-1").unwrap();
        assert_eq!(from_ledger, 0);
    }

    #[test]
    fn stock_summary_tracks_latest_quantity() {
        let conn = fresh();
        seed_everything(&conn);

        // Migration backfill ran against empty inventory, so stock_summary starts empty.
        // After the first adjust_stock call, the summary row is created.
        store(&conn).adjust_stock("DRINK-001", 20).unwrap();
        let qty: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary WHERE item_id = 'prod-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // new_qty = previous_qty (50 from inventory) + 20 = 70
        assert_eq!(
            qty, 70,
            "stock_summary should reflect current total after adjustment"
        );

        // Second adjustment updates the summary.
        store(&conn).adjust_stock("DRINK-001", -10).unwrap();
        let qty2: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary WHERE item_id = 'prod-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(qty2, 60);
    }
}
