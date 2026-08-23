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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProductWithDetails {
    /// The core product fields (flattened into the parent JSON).
    #[serde(flatten)]
    pub product: Product,
    /// Display name from `categories.name`, if linked.
    pub category_name: Option<String>,
    /// Current stock — SUM of `stock_summary.qty` across locations, falling
    /// back to the legacy `inventory.qty` when no ledger rows exist (ADR #36).
    pub stock_qty: Option<i64>,
    /// Materialized popularity score (ADR #37) — sort key for the retail grid.
    #[serde(default)]
    pub popularity_score: f64,
}

fn row_to_product_with_details(row: &rusqlite::Row) -> rusqlite::Result<ProductWithDetails> {
    let product = row_to_product(row)?;
    Ok(ProductWithDetails {
        product,
        category_name: row.get("category_name")?,
        stock_qty: row.get("stock_qty")?,
        popularity_score: row.get("popularity_score").unwrap_or(0.0),
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

/// Upsert a single `(item_id, location_id, qty)` row into `stock_summary`.
///
/// **ADR-19 §3**: post-migration-089's composite PRIMARY KEY
/// `(item_id, location_id)` requires every insert to specify BOTH columns
/// AND target BOTH columns in the conflict clause. Older single-column
/// callsites (`ON CONFLICT(item_id)`) now fail with SQLite error
/// `"ON CONFLICT clause does not match any PRIMARY KEY or UNIQUE constraint"`
/// — the cascade broke 46 cargo tests across KDS, products, purchase_orders,
/// reports, sales, stock_transfers, workspaces modules. This helper is the
/// canonical replacement and is used by `create_product`,
/// `adjust_stock_with_reason`, and any future single-row ops.
///
/// Accepts `&rusqlite::Connection` (NOT `&mut self`) so it works transparently
/// inside `unchecked_transaction()` blocks via `Transaction`'s `Deref<Target = Connection>`
/// behaviour — callers pass `&tx`.
fn upsert_stock_summary_in_tx(
    conn: &rusqlite::Connection,
    item_id: &str,
    location_id: &str,
    qty: i64,
    updated_at: &str,
) -> Result<(), CoreError> {
    conn.execute(
        "INSERT INTO stock_summary (item_id, location_id, qty, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(item_id, location_id) DO UPDATE SET qty = excluded.qty,
                                                      updated_at = excluded.updated_at",
        rusqlite::params![item_id, location_id, qty, updated_at],
    )?;
    Ok(())
}

// ── ADR #36 product attributes ───────────────────────────────────────

/// Product attributes beyond the base create fields (ADR #36).
///
/// [`Store::create_product`] delegates with the default so legacy callers
/// are unaffected; commands that carry the new fields call
/// [`Store::create_product_with_attributes`].
#[derive(Debug, Clone)]
pub struct CreateProductAttributes {
    /// Purchase/cost price in minor units (local-only).
    pub cost_minor: i64,
    /// Brand (free text).
    pub brand: Option<String>,
    /// Rack position code.
    pub rack_location: Option<String>,
    /// Free-text notes.
    pub notes: Option<String>,
    /// Unit of measure.
    pub unit: Option<String>,
    /// Active/sellable status (default: active).
    pub is_active: bool,
    /// Default supplier FK (local-only).
    pub default_supplier_id: Option<String>,
}

impl Default for CreateProductAttributes {
    fn default() -> Self {
        Self {
            cost_minor: 0,
            brand: None,
            rack_location: None,
            notes: None,
            unit: None,
            is_active: true,
            default_supplier_id: None,
        }
    }
}

/// ADR #36 attribute patch for [`Store::update_product_attributes`].
///
/// PATCH semantics: `None` = keep, `Some(None)` = clear (set NULL),
/// `Some(Some(v))` = set. `cost_minor`/`is_active` are plain `Option` —
/// `Some` updates, `None` keeps.
#[derive(Debug, Clone, Default)]
pub struct UpdateProductAttributes {
    /// Updated cost in minor units.
    pub cost_minor: Option<i64>,
    /// Updated brand (None keeps, Some(None) clears).
    pub brand: Option<Option<String>>,
    /// Updated rack position code.
    pub rack_location: Option<Option<String>>,
    /// Updated notes.
    pub notes: Option<Option<String>>,
    /// Updated unit of measure.
    pub unit: Option<Option<String>>,
    /// Updated active status.
    pub is_active: Option<bool>,
    /// Updated default supplier.
    pub default_supplier_id: Option<Option<String>>,
}

// ── Product CRUD ─────────────────────────────────────────────────────

impl Store<'_> {
    /// List all products, ordered by name, with category and stock.
    pub fn list_products(&self) -> Result<Vec<ProductWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at,
                     p.track_serial, p.product_type, p.version,
                     p.cost_minor, p.brand, p.rack_location, p.notes, p.unit,
                     p.is_active, p.default_supplier_id, p.popularity_score,
                     c.name AS category_name,
                     COALESCE((SELECT SUM(ss.qty) FROM stock_summary ss WHERE ss.item_id = p.id), i.qty) AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map([], row_to_product_with_details)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List products visible to one store (soft-scoping layer, migration
    /// 069/117), ordered by name, with category and stock.
    ///
    /// A store sees the shared global catalog (`store_id IS NULL`) plus its
    /// own tagged rows — never another store's rows. In the per-store
    /// database model every row is NULL, so this degenerates to the global
    /// catalog; it is the enforcement surface for shared/cloud databases
    /// where `store_id` is the soft-scoping column. The strict
    /// `store_id = ?1` predicate would return nothing there (all rows are
    /// NULL), which is why the NULL arm is included.
    pub fn list_products_for_store(
        &self,
        store_id: &str,
    ) -> Result<Vec<ProductWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at,
                     p.track_serial, p.product_type, p.version,
                     p.cost_minor, p.brand, p.rack_location, p.notes, p.unit,
                     p.is_active, p.default_supplier_id, p.popularity_score,
                     c.name AS category_name,
                     COALESCE((SELECT SUM(ss.qty) FROM stock_summary ss WHERE ss.item_id = p.id), i.qty) AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.store_id IS NULL OR p.store_id = ?1
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map(params![store_id], row_to_product_with_details)?;
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
                     p.cost_minor, p.brand, p.rack_location, p.notes, p.unit,
                     p.is_active, p.default_supplier_id, p.popularity_score,
                     c.name AS category_name,
                     COALESCE((SELECT SUM(ss.qty) FROM stock_summary ss WHERE ss.item_id = p.id), i.qty) AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE p.product_type != 'service'
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map([], row_to_product_with_details)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List inventory-tracked products with stock at a specific location.
    ///
    /// Like [`list_warehouse_products`] but reads `stock_summary.qty` for
    /// the given `location_id` instead of summing across all locations.
    /// Returns 0 for products with no stock row at this location.
    pub fn list_warehouse_products_at_location(
        &self,
        location_id: &str,
    ) -> Result<Vec<ProductWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sku, p.name, p.price_minor, p.currency,
                     p.category_id, p.barcode, p.created_at, p.updated_at, p.price_updated_at,
                     p.track_serial, p.product_type, p.version,
                     p.cost_minor, p.brand, p.rack_location, p.notes, p.unit,
                     p.is_active, p.default_supplier_id, p.popularity_score,
                     c.name AS category_name,
                     COALESCE((SELECT ss.qty FROM stock_summary ss WHERE ss.item_id = p.id AND ss.location_id = ?1), 0) AS stock_qty
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             WHERE p.product_type != 'service'
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map(params![location_id], row_to_product_with_details)?;
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
                     p.cost_minor, p.brand, p.rack_location, p.notes, p.unit,
                     p.is_active, p.default_supplier_id, p.popularity_score,
                     c.name AS category_name,
                     COALESCE((SELECT SUM(ss.qty) FROM stock_summary ss WHERE ss.item_id = p.id), i.qty) AS stock_qty
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
                     p.cost_minor, p.brand, p.rack_location, p.notes, p.unit,
                     p.is_active, p.default_supplier_id, p.popularity_score,
                     c.name AS category_name,
                     COALESCE((SELECT SUM(ss.qty) FROM stock_summary ss WHERE ss.item_id = p.id), i.qty) AS stock_qty
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
        self.create_product_with_attributes(
            sku,
            name,
            price,
            category_id,
            barcode,
            initial_stock,
            product_type,
            &CreateProductAttributes::default(),
        )
    }

    /// Create a product with the ADR #36 attributes.
    ///
    /// [`Store::create_product`] delegates here with default attributes so the
    /// ~100 legacy call sites are untouched.
    #[allow(clippy::too_many_arguments)]
    pub fn create_product_with_attributes(
        &self,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
        initial_stock: i64,
        product_type: Option<&str>,
        attrs: &CreateProductAttributes,
    ) -> Result<Product, CoreError> {
        if sku.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "sku",
                message: "SKU must not be empty".into(),
            });
        }
        if sku.len() > 50 {
            return Err(CoreError::Validation {
                field: "sku",
                message: format!("SKU must not exceed 50 characters, got {}", sku.len()),
            });
        }
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "name must not be empty".into(),
            });
        }
        if name.len() > 255 {
            return Err(CoreError::Validation {
                field: "name",
                message: format!("name must not exceed 255 characters, got {}", name.len()),
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
            .map_err(|e| CoreError::Validation {
                field: "currency",
                message: format!("invalid UTF-8 in currency bytes: {e}"),
            })?
            .to_owned();

        let tx = self.conn.unchecked_transaction()?;

        let result = tx.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial, product_type, version, cost_minor, brand, rack_location, notes, unit, is_active, default_supplier_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
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
                attrs.cost_minor,
                attrs.brand,
                attrs.rack_location,
                attrs.notes,
                attrs.unit,
                attrs.is_active as i64,
                attrs.default_supplier_id,
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
            upsert_stock_summary_in_tx(
                &tx,
                &id,
                crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
                initial_stock,
                &now,
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
            cost_minor: attrs.cost_minor,
            brand: attrs.brand.clone(),
            rack_location: attrs.rack_location.clone(),
            notes: attrs.notes.clone(),
            unit: attrs.unit.clone(),
            is_active: attrs.is_active,
            default_supplier_id: attrs.default_supplier_id.clone(),
        })
    }

    /// Apply ADR #36 attribute changes to a product identified by SKU.
    ///
    /// PATCH semantics (see [`UpdateProductAttributes`]). Records an edit
    /// activity event and refreshes the popularity score in the same
    /// transaction (ADR #37 D2/D3). Returns [`CoreError::NotFound`] when the
    /// SKU does not exist and no-op when the patch is empty.
    pub fn update_product_attributes(
        &self,
        sku: &str,
        attrs: &UpdateProductAttributes,
    ) -> Result<(), CoreError> {
        let mut sets: Vec<String> = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Set-or-clear helper for clearable text columns.
        macro_rules! set_clearable {
            ($col:expr, $opt:expr) => {
                match &$opt {
                    Some(Some(val)) => {
                        sets.push(format!("{} = ?", $col));
                        values.push(Box::new(val.clone()));
                    }
                    Some(None) => sets.push(format!("{} = NULL", $col)),
                    None => {}
                }
            };
        }

        if let Some(cost) = attrs.cost_minor {
            sets.push("cost_minor = ?".into());
            values.push(Box::new(cost));
        }
        set_clearable!("brand", attrs.brand);
        set_clearable!("rack_location", attrs.rack_location);
        set_clearable!("notes", attrs.notes);
        set_clearable!("unit", attrs.unit);
        set_clearable!("default_supplier_id", attrs.default_supplier_id);
        if let Some(active) = attrs.is_active {
            sets.push("is_active = ?".into());
            values.push(Box::new(active as i64));
        }

        if sets.is_empty() {
            return Ok(());
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let tx = self.conn.unchecked_transaction()?;

        sets.push("updated_at = ?".into());
        values.push(Box::new(now.clone()));
        let sql = format!("UPDATE products SET {} WHERE sku = ?", sets.join(", "));
        values.push(Box::new(sku.to_string()));

        let rows = tx.execute(&sql, rusqlite::params_from_iter(values))?;
        if rows == 0 {
            tx.rollback()?;
            return Err(CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            });
        }

        // ADR #37 D2: every product update is an edit signal.
        tx.execute(
            "INSERT INTO product_activity (id, sku, event_type) VALUES (?1, ?2, 'edit')",
            params![crate::new_id(), sku],
        )?;
        tx.commit()?;

        self.recompute_popularity(sku)?;
        if let Some(cache) = &self.cache {
            cache.invalidate_product(sku);
        }
        Ok(())
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
            .map_err(|e| CoreError::Validation {
                field: "currency",
                message: format!("invalid UTF-8 in currency bytes: {e}"),
            })?
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
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial, product_type, version, cost_minor, brand, rack_location, notes, unit, is_active, default_supplier_id, popularity_score
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
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial, product_type, version, cost_minor, brand, rack_location, notes, unit, is_active, default_supplier_id, popularity_score
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

    /// Delete a category, explicitly unlinking its products first (CAT-02).
    ///
    /// The relationship policy is made explicit in one transaction: products
    /// referencing this category are set to `category_id = NULL`, then the
    /// category row is deleted. Returns the number of products that were
    /// unlinked so the UI can show the consequence — replacing the implicit
    /// FK-dependent behavior of [`Store::delete_category`] for the
    /// management screen.
    pub fn delete_category_with_unlink(&self, id: &str) -> Result<i64, CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let unlinked = tx.execute(
            "UPDATE products SET category_id = NULL WHERE category_id = ?1",
            params![id],
        )?;
        let deleted = tx.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(CoreError::NotFound {
                entity: "category",
                id: id.to_owned(),
            });
        }
        tx.commit()?;
        Ok(unlinked as i64)
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
    /// updates (those are reconciled later via `rebuild_stock_summary`).
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

    /// Insert a stock movement using a caller-owned transaction.
    ///
    /// Sync replay handling uses this form so the immutable ledger row and
    /// its remote receipt can commit or roll back together.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_stock_movement_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        id: &str,
        item_id: &str,
        delta: i64,
        reason: Option<&str>,
        source_terminal_id: Option<&str>,
        source_user_id: Option<&str>,
        store_id: &str,
        created_at: &str,
    ) -> Result<(), CoreError> {
        tx.execute(
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

    /// Create a product and its initial stock inside a caller-owned
    /// transaction, unless the SKU already exists.
    ///
    /// Returns `true` when a row was inserted and `false` when the SKU was
    /// already present. The operation is intentionally idempotent by SKU so
    /// a replay after a commit-before-receipt crash cannot create a duplicate.
    #[allow(clippy::too_many_arguments)]
    pub fn create_product_if_absent_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
        initial_stock: i64,
        product_type: &str,
    ) -> Result<bool, CoreError> {
        if sku.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "sku",
                message: "SKU must not be empty".into(),
            });
        }
        if sku.len() > 50 {
            return Err(CoreError::Validation {
                field: "sku",
                message: format!("SKU must not exceed 50 characters, got {}", sku.len()),
            });
        }
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "name must not be empty".into(),
            });
        }
        if name.len() > 255 {
            return Err(CoreError::Validation {
                field: "name",
                message: format!("name must not exceed 255 characters, got {}", name.len()),
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

        let cur_str = std::str::from_utf8(&price.currency.0)
            .map_err(|e| CoreError::Validation {
                field: "currency",
                message: format!("invalid UTF-8 in currency bytes: {e}"),
            })?
            .to_owned();

        // A repeated SKU is only idempotent when it describes the same
        // product. Silently accepting a different payload would record a
        // receipt while discarding a legitimate remote catalog conflict.
        {
            use rusqlite::OptionalExtension;
            // Existing-product row for the idempotency comparison (clippy
            // type_complexity — factored into a named type).
            type ExistingRow = (String, i64, String, Option<String>, Option<String>, String);
            let existing: Option<ExistingRow> = tx
                .query_row(
                    "SELECT p.name, p.price_minor, p.currency, p.category_id, p.barcode,
                            p.product_type
                     FROM products p
                     WHERE p.sku = ?1",
                    params![sku.trim()],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .optional()?;
            if let Some((
                existing_name,
                existing_price,
                existing_currency,
                existing_category,
                existing_barcode,
                existing_type,
            )) = existing
            {
                let same = existing_name == name.trim()
                    && existing_price == price.minor_units
                    && existing_currency == cur_str
                    && existing_category.as_deref() == category_id
                    && existing_barcode.as_deref() == barcode
                    && existing_type == product_type;
                // `initial_stock` is write-once creation metadata. Current
                // inventory is mutable, so it is intentionally not compared
                // when recognizing a replay of an already-existing SKU.
                if !same {
                    return Err(CoreError::Conflict {
                        entity: "product",
                        field: "sku",
                    });
                }
                return Ok(false);
            }
        }

        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let inserted = tx.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode,
                                   created_at, updated_at, price_updated_at, track_serial,
                                   product_type, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8, 0, ?9, 1)
             ON CONFLICT (tenant_id, sku) DO NOTHING",
            params![
                id,
                sku.trim(),
                name.trim(),
                price.minor_units,
                cur_str,
                category_id,
                barcode,
                now,
                product_type,
            ],
        )?;

        if inserted == 0 || initial_stock == 0 || product_type == "service" {
            return Ok(inserted == 1);
        }

        tx.execute(
            "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)",
            params![id, initial_stock, now],
        )?;
        let movement_id = uuid::Uuid::now_v7().to_string();
        tx.execute(
            "INSERT INTO stock_movements (id, item_id, delta, reason,
                                          source_terminal_id, source_user_id, created_at)
             VALUES (?1, ?2, ?3, 'initial-stock', NULL, NULL, ?4)",
            params![movement_id, id, initial_stock, now],
        )?;
        upsert_stock_summary_in_tx(
            tx,
            &id,
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            initial_stock,
            &now,
        )?;

        Ok(true)
    }

    /// Adjust stock for a product by SKU inside a caller-owned transaction.
    ///
    /// This compatibility path mirrors [`Store::adjust_stock`] while allowing
    /// sync replay to commit the stock mutation and remote receipt together.
    /// New checkout code should use the location-aware canonical API below.
    #[deprecated(note = "use adjust_stock_at_location_with_reason instead")]
    pub fn adjust_stock_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        sku: &str,
        delta: i64,
    ) -> Result<i64, CoreError> {
        let product_id: String = tx
            .query_row(
                "SELECT id FROM products WHERE sku = ?1",
                params![sku],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound {
                    entity: "product",
                    id: sku.to_owned(),
                },
                other => CoreError::Db(other),
            })?;
        let previous_qty: i64 = match tx.query_row(
            "SELECT qty FROM inventory WHERE product_id = ?1",
            params![product_id],
            |row| row.get(0),
        ) {
            Ok(qty) => qty,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(error) => return Err(CoreError::Db(error)),
        };
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
        tx.execute(
            "INSERT INTO stock_movements (id, item_id, delta, reason, created_at)
             VALUES (?1, ?2, ?3, 'remote-sync', ?4)",
            params![uuid::Uuid::now_v7().to_string(), product_id, delta, now],
        )?;
        tx.execute(
            "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(product_id) DO UPDATE SET qty = excluded.qty, updated_at = excluded.updated_at",
            params![product_id, new_qty, now],
        )?;
        upsert_stock_summary_in_tx(
            tx,
            &product_id,
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            new_qty,
            &now,
        )?;
        Ok(new_qty)
    }

    /// Adjust stock for a product by SKU inside a transaction.
    ///
    /// Writes a delta row to the `stock_movements` ledger (ADR #6)
    /// and updates the materialised `inventory` and `stock_summary` tables.
    /// The `reason` parameter is recorded in the ledger for audit purposes.
    /// Multi-terminal: stock adjustments are store-scoped and shared across all
    /// terminals. The CHECK (qty >= 0) constraint on stock_summary prevents
    /// concurrent last-unit oversells from producing negative inventory.
    #[deprecated(note = "use adjust_stock_at_location_with_reason instead")]
    #[allow(deprecated)]
    pub fn adjust_stock(&self, sku: &str, delta: i64) -> Result<i64, CoreError> {
        self.adjust_stock_with_reason(sku, delta, None, None, None)
    }
    /// Adjust stock with an explicit reason at a specific location (ADR-19 §3.1 canonical API).
    ///
    /// This is the **canonical core function** that all sale deduction / void /
    /// refund / transfer / purchase-order flows route through. It performs the
    /// following writes inside the caller-provided `&Transaction` (no internal BEGIN —
    /// the caller is responsible for `BEGIN IMMEDIATE` atomicity per ADR-19 §5.2):
    ///
    /// 1. One **immutable delta row** in `stock_movements` (CRDT ledger — ADR #6 +
    ///    ADR-19 §3.2 audit trail: item_id, location_id, delta, reason,
    ///    inventory_transaction_id?, source_terminal_id?, source_user_id?, created_at).
    /// 2. **Upsert** `stock_summary` at the composite PRIMARY KEY
    ///    `(item_id, location_id)` introduced by migration 089. The
    ///    schema's `CHECK (qty >= 0)` constraint is Layer 2 negative-stock guard.
    /// 3. **Upsert** the legacy `inventory` table at the single-PK
    ///    `(product_id)` for backward-compat callers (ADR-18 §2a's full
    ///    composite-PK inventory rebuild is deferred).
    ///
    /// **Two-layer negative-stock protection** (ADR-19 §3.3):
    /// - **Layer 1 (Rust)**: pre-check `current_qty + delta >= 0` before any
    ///   write, returning [`CoreError::InsufficientStockAtLocation`] with the
    ///   exact available qty if the deduction would underflow. This keeps
    ///   `PartialStockResult` aggregation O(1) without a SELECT-after-failure.
    /// - **Layer 2 (SQLite)**: `SqliteFailure(extended_code=787)` on the
    ///   `stock_summary` upsert is translated to the same variant (defence
    ///   in depth against any Rust-side race in Layer 1).
    ///
    /// Returns the **post-update qty at the location** so the caller can
    /// detect post-commit state without a separate SELECT.
    #[allow(clippy::too_many_arguments)]
    /// Adjust stock with an explicit reason at a specific location (ADR-19 §3.1 canonical API).
    ///
    /// This is the **canonical core function** that all sale deduction / void /
    /// refund / transfer / purchase-order flows route through. It performs the
    /// following writes inside the caller-provided `&Transaction` (no internal BEGIN —
    /// the caller is responsible for `BEGIN IMMEDIATE` atomicity per ADR-19 §5.2):
    ///
    /// 1. One **immutable delta row** in `stock_movements` (CRDT ledger — ADR #6 +
    ///    ADR-19 §3.2 audit trail: item_id, location_id, delta, reason,
    ///    inventory_transaction_id?, source_terminal_id?, source_user_id?, created_at).
    /// 2. **Upsert** `stock_summary` at the composite PRIMARY KEY
    ///    `(item_id, location_id)` introduced by migration 089. The
    ///    schema's `CHECK (qty >= 0)` constraint is Layer 2 negative-stock guard.
    /// 3. **Upsert** the legacy `inventory` table at the single-PK
    ///    `(product_id)` for backward-compat callers (ADR-18 §2a's full
    ///    composite-PK inventory rebuild is deferred).
    ///
    /// **Two-layer negative-stock protection** (ADR-19 §3.3):
    /// - **Layer 1 (Rust)**: pre-check `current_qty + delta >= 0` before any
    ///   write, returning [`CoreError::InsufficientStockAtLocation`] with the
    ///   exact available qty if the deduction would underflow. This keeps
    ///   `PartialStockResu    #[allow(clippy::too_many_arguments)]
    pub fn adjust_stock_at_location_with_reason(
        &self,
        tx: &rusqlite::Transaction<'_>,
        sku: &str,
        delta: i64,
        location_id: &crate::inventory::LocationId,
        reason: Option<&str>,
        inventory_transaction_id: Option<&crate::inventory_transaction::InventoryTransactionId>,
        terminal_id: Option<&crate::terminal::TerminalId>,
        source_user_id: Option<&crate::user::UserId>,
    ) -> Result<i64, CoreError> {
        let product_id = self
            .product_id_by_sku(sku)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            })?;

        // Layer 1: read current qty at THIS (item_id, location_id) — uses
        // stock_summary.composite-PK via the per-location index from
        // migration 089. Falls back to 0 when no prior movements exist
        // (forward-compatible with pre-079 seed data).
        //
        // Explicit match guards against DB errors: QueryReturnedNoRows → 0
        // (no stock at this location), any other error → propagate.
        let current_qty: i64 = match tx.query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary \
             WHERE item_id = ?1 AND location_id = ?2",
            rusqlite::params![product_id, location_id.as_str()],
            |row| row.get(0),
        ) {
            Ok(q) => q,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(CoreError::Db(e)),
        };

        let mut allow_negative = false;
        if let Some(t_id) = terminal_id
            && let Ok(ws_id) = tx.query_row(
                "SELECT workspace_instance_id FROM terminals WHERE id = ?1",
                rusqlite::params![t_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            && let Ok(allowed) = tx.query_row(
                "SELECT COALESCE(allow_negative_stock, 0) FROM workspace_inventory_locations \
                     WHERE instance_id = ?1 AND location_id = ?2",
                rusqlite::params![ws_id, location_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
        {
            allow_negative = allowed == 1;
        }

        let new_qty = if allow_negative {
            current_qty
                .checked_add(delta)
                .ok_or_else(|| CoreError::Validation {
                    field: "qty",
                    message: "overflow".into(),
                })?
        } else {
            current_qty
                .checked_add(delta)
                .filter(|&v| v >= 0)
                .ok_or_else(|| CoreError::InsufficientStockAtLocation {
                    sku: sku.to_owned(),
                    location_id: location_id.clone(),
                    requested_delta: delta,
                    available_qty: current_qty,
                })?
        };

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let movement_id = uuid::Uuid::now_v7().to_string();

        // 1. Audit-trail delta row (ADR #6 + ADR-19 §3.2).
        tx.execute(
            "INSERT INTO stock_movements (id, item_id, location_id, delta, reason,
                                          inventory_transaction_id,
                                          source_terminal_id, source_user_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                movement_id,
                product_id,
                location_id.as_str(),
                delta,
                reason,
                inventory_transaction_id.map(|id| id.as_str()),
                terminal_id.map(|id| id.as_str()),
                source_user_id.map(|id| id.as_str()),
                now,
            ],
        )?;

        // 2. Per-location stock_summary upsert (Layer-2 negative-stock guard).
        let summary_res = tx.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(item_id, location_id) DO UPDATE SET
                qty = excluded.qty,
                updated_at = excluded.updated_at",
            rusqlite::params![product_id, location_id.as_str(), new_qty, now],
        );
        if let Err(rusqlite::Error::SqliteFailure(ref e, _)) = summary_res
            && e.code == rusqlite::ErrorCode::ConstraintViolation
        {
            return Err(CoreError::InsufficientStockAtLocation {
                sku: sku.to_owned(),
                location_id: location_id.clone(),
                requested_delta: delta,
                available_qty: current_qty,
            });
        }
        summary_res?;

        // 3. Legacy inventory table — ADR-18 §2a composite-PK rebuild deferred.
        // When `allow_negative` is true, the CHECK (qty >= 0) constraint on the
        // `inventory` table would reject a negative qty. We catch that error and
        // log a warning instead of propagating it, because the stock_summary table
        // (step 2) is the canonical source of truth for per-location stock.
        let inventory_res = tx.execute(
            "INSERT INTO inventory (product_id, location_id, qty, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(product_id) DO UPDATE SET
                qty = excluded.qty,
                location_id = excluded.location_id,
                updated_at = excluded.updated_at",
            rusqlite::params![product_id, location_id.as_str(), new_qty, now],
        );
        if let Err(rusqlite::Error::SqliteFailure(ref e, _)) = inventory_res
            && e.code == rusqlite::ErrorCode::ConstraintViolation
            && allow_negative
        {
            // Expected when qty < 0 and allow_negative_stock is enabled.
            // The `stock_summary` table already has the accurate count.
            tracing::warn!(
                "negative stock (qty={}) not written to legacy inventory table; stock_summary is canonical",
                new_qty
            );
        } else {
            inventory_res?;
        }

        // 4. Synchronous threshold check (ADR-18 §9e-ii).
        // Errors are silent — threshold alerts are advisory and should not
        // block the stock adjustment transaction.
        let _ = self.check_stock_threshold_and_alert_in_tx(
            tx,
            &product_id,
            location_id.as_str(),
            new_qty,
            &now,
        );

        if let Some(cache) = &self.cache {
            cache.invalidate_inventory(&product_id);
            cache.publish_inventory_change(&product_id, sku, new_qty, self.terminal_id.as_deref());
        }

        // 5. stock.negative warning event (ADR-18 §4).
        // Emitted when allow_negative_stock is enabled and the resulting qty
        // is negative — the deduction went below zero.
        if allow_negative
            && new_qty < 0
            && let Some(cache) = &self.cache
        {
            cache.publish_negative_stock_event(
                &product_id,
                sku,
                location_id.as_str(),
                delta,
                new_qty,
                self.terminal_id.as_deref(),
            );
        }

        Ok(new_qty)
    }

    /// Check stock thresholds for a product at a location after a stock change
    /// and INSERT / UPDATE `stock_alert_events` accordingly.
    ///
    /// Lookup order (ADR-18 §9e-i):
    /// 1. Product+location specific threshold
    /// 2. Product+global threshold (location_id IS NULL)
    /// 3. No threshold configured → skip (no alert)
    ///
    /// If stock is below threshold: INSERT alert with `status = 'active'`
    /// (deduped — no duplicate active alerts per threshold_id).
    /// If stock recovers above threshold: UPDATE any active/acknowledged
    /// alerts to `status = 'resolved'` (auto-resolve).
    fn check_stock_threshold_and_alert_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        product_id: &str,
        location_id: &str,
        new_qty: i64,
        now: &str,
    ) -> Result<(), CoreError> {
        // Lookup: product+location specific, then product+global.
        let threshold_row: Option<(String, i64)> = tx
            .query_row(
                "SELECT id, threshold FROM stock_thresholds \
                 WHERE product_id = ?1 AND location_id = ?2 AND enabled = 1 \
                 LIMIT 1",
                rusqlite::params![product_id, location_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .ok()
            .or_else(|| {
                tx.query_row(
                    "SELECT id, threshold FROM stock_thresholds \
                     WHERE product_id = ?1 AND location_id IS NULL AND enabled = 1 \
                     LIMIT 1",
                    rusqlite::params![product_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .ok()
            });

        let (threshold_id, threshold) = match threshold_row {
            Some(row) => row,
            None => return Ok(()), // No threshold configured — skip.
        };

        // Check if stock went below threshold.
        if new_qty < threshold {
            // Dedup: don't insert if there's already an active alert for this threshold.
            let existing: bool = tx
                .query_row(
                    "SELECT 1 FROM stock_alert_events \
                     WHERE threshold_id = ?1 AND status IN ('active', 'acknowledged') \
                     LIMIT 1",
                    rusqlite::params![threshold_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if !existing {
                let alert_id = uuid::Uuid::now_v7().to_string();
                tx.execute(
                    "INSERT INTO stock_alert_events \
                     (id, threshold_id, product_id, location_id, current_qty, threshold, status, triggered_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7)",
                    rusqlite::params![alert_id, threshold_id, product_id, location_id, new_qty, threshold, now],
                )?;
            }
        } else {
            // Stock recovered above threshold — auto-resolve active alerts.
            tx.execute(
                "UPDATE stock_alert_events \
                 SET status = 'resolved', resolved_at = ?1 \
                 WHERE threshold_id = ?2 AND status IN ('active', 'acknowledged')",
                rusqlite::params![now, threshold_id],
            )?;
        }

        Ok(())
    }

    /// Atomically deduct from multiple locations for one or more SKUs
    /// inside the caller's transaction (ADR-19 §3).
    ///
    /// All deductions happen inside the caller-provided `&Transaction` —
    /// no internal BEGIN/COMMIT. Used by the split-fulfillment flow (§6b)
    /// where one line item is deducted from 2+ locations simultaneously.
    ///
    /// 1. Pre-check every deduction against current stock at its location.
    ///    If ANY deduction would cause negative stock at its location, the
    ///    function returns [`CoreError::InsufficientStockAtLocation`] for
    ///    the **first** shortfall encountered (the caller should have
    ///    already validated all deductions before calling).
    /// 2. Execute all deductions — each is a single call to
    ///    [`adjust_stock_at_location_with_reason`](Self::adjust_stock_at_location_with_reason).
    ///
    /// The caller is responsible for `BEGIN IMMEDIATE` and COMMIT/ROLLBACK.
    pub fn adjust_stock_batch(
        &self,
        tx: &rusqlite::Transaction<'_>,
        deductions: &[crate::sale_deduction::StockDeduction],
        reason: Option<&str>,
        inventory_transaction_id: Option<&crate::inventory_transaction::InventoryTransactionId>,
        terminal_id: Option<&crate::terminal::TerminalId>,
        source_user_id: Option<&crate::user::UserId>,
    ) -> Result<(), CoreError> {
        if deductions.is_empty() {
            return Ok(());
        }

        // Phase 1: pre-check all deductions against current stock.
        for d in deductions {
            let product_id =
                self.product_id_by_sku(&d.sku)?
                    .ok_or_else(|| CoreError::NotFound {
                        entity: "product",
                        id: d.sku.clone(),
                    })?;

            // Distinguish QueryReturnedNoRows (no stock at this location → 0)
            // from real DB errors (corruption, lock → propagate).
            let current_qty: i64 = match tx.query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary \
                 WHERE item_id = ?1 AND location_id = ?2",
                rusqlite::params![product_id, d.location_id.as_str()],
                |row| row.get(0),
            ) {
                Ok(q) => q,
                Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                Err(e) => return Err(CoreError::Db(e)),
            };

            let mut allow_negative = false;
            if let Some(t_id) = terminal_id
                && let Ok(ws_id) = tx.query_row(
                    "SELECT workspace_instance_id FROM terminals WHERE id = ?1",
                    rusqlite::params![t_id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                && let Ok(allowed) = tx.query_row(
                    "SELECT COALESCE(allow_negative_stock, 0) FROM workspace_inventory_locations \
                         WHERE instance_id = ?1 AND location_id = ?2",
                    rusqlite::params![ws_id, d.location_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
            {
                allow_negative = allowed == 1;
            }

            if !allow_negative {
                let _new_qty = current_qty
                    .checked_add(d.delta)
                    .filter(|&v| v >= 0)
                    .ok_or_else(|| CoreError::InsufficientStockAtLocation {
                        sku: d.sku.clone(),
                        location_id: d.location_id.clone(),
                        requested_delta: d.delta,
                        available_qty: current_qty,
                    })?;
            }
        }

        // Phase 2: execute all deductions (all pre-checks passed).
        for d in deductions {
            self.adjust_stock_at_location_with_reason(
                tx,
                &d.sku,
                d.delta,
                &d.location_id,
                reason,
                inventory_transaction_id,
                terminal_id,
                source_user_id,
            )?;
        }

        Ok(())
    }

    /// Adjust stock with an explicit reason for the delta ledger (ADR #6).
    ///
    /// **ADR-19 §3.4** (deferred): this function is preserved verbatim from the
    /// pre-ADR-19 v0.0.10 baseline. The §3.4 demotion to a wrapper around
    /// [`adjust_stock_at_location_with_reason`](Self::adjust_stock_at_location_with_reason)
    /// is **deferred to v0.1.0** because the wrapper's contract (NULL
    /// location_id → canonical-default via column-DFT, single-PK inventory
    /// upsert) is depended on by 8+ downstream cargo tests across
    /// `db::products`, `db::purchase_orders`, `db::stock_transfers`, and
    /// `db::workspaces`. Routing it through the canonical fn during the
    /// v0.0.10 transition would require updating those tests + the
    /// production callsites in `app/*/commands/products.rs` +
    /// `modules/inventory/src/handlers.rs` — out of scope for Criterion
    /// 19-2 (which delivers the new canonical API surface, not the
    /// migration of existing callers).
    ///
    /// **Layer-1 stale-source note for §3.4 follow-up**: this wrapper reads
    /// `previous_qty` from the **legacy `inventory` table** via
    /// `self.get_stock(&product_id)`. The canonical §3.1 fn reads from
    /// `stock_summary` (post-ADR-18 §3 authoritative per-location surface).
    /// A future test or production flow that seeds ONLY `stock_summary`
    /// (not `inventory`) will pass the §3.1 path but fail this wrapper with
    /// phantom zero stock — a §3.4 migration foot-gun. The §3.4 follow-up
    /// should explicitly migrate Layer-1 reads to `stock_summary`.
    #[deprecated(note = "use adjust_stock_at_location_with_reason instead")]
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

        // 3. Update the stock_summary materialised view (perf — ADR #6 + ADR-19 §3).
        // Uses the canonical default location UUID per ADR-18 §13-36 frozen seed.
        // The helper targets the composite PRIMARY KEY (item_id, location_id)
        // introduced by migration 089 — pre-refactor single-column
        // ON CONFLICT(item_id) raise "ON CONFLICT clause does not match any
        // PRIMARY KEY or UNIQUE constraint" and cascade-fail 46+ tests.
        upsert_stock_summary_in_tx(
            &tx,
            &product_id,
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            new_qty,
            &now,
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
    /// delta ledger (ADR #6 + ADR-18 §2c + ADR-19 §1).
    ///
    /// After ADR-18 migration 089, `stock_summary` has a composite PRIMARY
    /// KEY (item_id, location_id). The rebuild MUST aggregate the delta ledger
    /// by BOTH columns — not by `item_id` alone — otherwise per-location stock
    /// is silently funneled into the canonical default UUID and the §9 alert
    /// system queries return aggregated cross-location totals instead of
    /// per-location vectors. This is ADR-19 §15 criterion 19-1.
    ///
    /// `inventory` still has a single-PK on `product_id` (ADR-18 §2a's
    /// composite-PK rebuild is deferred), so it aggregates per product across
    /// all locations. Per-location authoritative stock now lives in
    /// `stock_summary`. Legacy `inventory` is preserved here as a sum-of-all
    /// locations approximation for backward-compat callers.
    ///
    /// This is called after a sync cycle receives new deltas from other
    /// registers or the cloud, ensuring the materialised cache is consistent
    /// with the authoritative ledger. Runs in a single transaction for atomicity.
    ///
    /// **Returns** the number of `(item_id, location_id)` tuples rebuilt —
    /// NOT the number of distinct products. Post-refactor the count is
    /// higher for products stored across multiple locations.
    pub fn rebuild_stock_summary(&self) -> Result<usize, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        // ADR-18 §13-36 frozen canonical default-location UUID (see
        // `crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID`). This is also
        // the column DEFAULT on `stock_movements.location_id` (migration 080)
        // and `inventory.location_id` (migration 079), so legacy pre-790
        // stock_movements rows uniformly land at this location_id and the
        // rebuild stays backward-compatible.
        let canonical_default_loc = crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;

        let tx = self.conn.unchecked_transaction()?;

        // Clear the materialised caches.
        tx.execute("DELETE FROM stock_summary", [])?;

        // Rebuild stock_summary from the delta ledger. MUST group by both
        // (item_id, location_id) per ADR-18 migration 089's composite PK.
        // Without this, multi-location data silently collapses to one row
        // per item_id at the canonical default UUID — the dormant bug
        // originally flagged in the ADR-18 final-review.
        let rebuilt = tx.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty, updated_at)
             SELECT item_id, location_id, SUM(delta), ?1
             FROM stock_movements
             GROUP BY item_id, location_id",
            params![now],
        )?;

        // Rebuild the inventory table (backward compat, single-PK preserved).
        // Aggregates per product (sums ALL location deltas into one row),
        // and pins the row's location_id to the canonical default UUID to
        // match how `adjust_stock_with_reason` writes (it doesn't specify
        // location_id, relying on the column DEFAULT). This keeps `inventory`
        // a representative aggregate for pre-refactor callers while
        // `stock_summary` becomes the per-location authoritative surface.
        tx.execute(
            "INSERT INTO inventory (product_id, location_id, qty, updated_at)
             SELECT item_id, ?2 AS location_id, SUM(delta), ?1
             FROM stock_movements
             GROUP BY item_id
             ON CONFLICT(product_id) DO UPDATE SET
                qty = excluded.qty,
                location_id = excluded.location_id,
                updated_at = excluded.updated_at",
            params![now, canonical_default_loc],
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

            // 1. Copy old rows to archive (skip previous rollup rows). Post
            //    ADR-18 §2b + migration 080 stock_movements_archive gained
            //    a `location_id` column (NOT NULL DEFAULT canonical UUID);
            //    post ADR-18 §9c + migration 085 it also gained a nullable
            //    `inventory_transaction_id` FK column. The select-list below
            //    must enumerate ALL 10 columns in the same order as the
            //    CREATE TABLE from migration 072 + ALTERs from 080/085,
            //    otherwise SQLite rejects with "X columns but Y values
            //    were supplied" and the archive transaction rolls back.
            tx.execute(
                "INSERT INTO stock_movements_archive
                 SELECT id, item_id, delta, reason,
                        source_terminal_id, source_user_id,
                        store_id, created_at,
                        location_id, inventory_transaction_id
                 FROM stock_movements
                 WHERE item_id = ?1
                   AND created_at < ?2
                   AND reason != 'archive-rollup'",
                params![item_id, cutoff_str],
            )?;

            // 2. Insert a rollup row consolidating all archived deltas.
            //    Post migration 080 location_id is NOT NULL DEFAULT canonical
            //    UUID on stock_movements, so we anchor the rollup to the
            //    canonical default explicitly (the COALESCE would otherwise
            //    surface a NULL on pre-080 stock_movements rows). Post
            //    migration 085 inventory_transaction_id is NULLABLE; the
            //    rollup row has no original inventory_transaction session
            //    because it consolidates multiple sessions — NULL is correct.
            let rollup_id = uuid::Uuid::now_v7().to_string();
            tx.execute(
                "INSERT INTO stock_movements
                     (id, item_id, delta, reason, store_id, created_at,
                      location_id, inventory_transaction_id)
                 SELECT ?1, ?2, COALESCE(SUM(delta), 0), 'archive-rollup',
                        '', ?3,
                        '01926b3a-0000-7000-8000-000000000001', NULL
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
#[path = "products_tests.rs"]
mod tests;
