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
                    p.category_id, p.barcode, p.created_at, p.updated_at,
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
    pub fn create_product(
        &self,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
        initial_stock: i64,
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

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let cur_str = std::str::from_utf8(&price.currency.0)
            .expect("currency bytes are valid UTF-8")
            .to_owned();

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

        if initial_stock > 0 {
            tx.execute(
                "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, ?3)",
                params![id, initial_stock, now],
            )?;
        }

        tx.commit()?;

        if let Some(cache) = &self.cache {
            cache.invalidate_product(sku.trim());
        }

        Ok(Product {
            id,
            sku: Sku::new(sku.trim()),
            name: name.trim().to_owned(),
            price,
            category_id: category_id.map(|s| s.to_owned()),
            barcode: barcode.and_then(|s| foundation::Barcode::new(s).ok()),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing product identified by SKU.
    pub fn update_product(
        &self,
        sku: &str,
        name: &str,
        price: Money,
        category_id: Option<&str>,
        barcode: Option<&str>,
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

        if let Some(cache) = &self.cache {
            cache.invalidate_product(sku);
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at
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

    /// Adjust stock for a product by SKU inside a transaction.
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

        if let Some(cache) = &self.cache {
            cache.invalidate_inventory(&product_id);
            cache.publish_inventory_change(&product_id, sku, new_qty);
        }

        Ok(new_qty)
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
                variant.barcode,
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
            .create_product("NOSTOCK", "No Stock", price(100), None, None, 0)
            .unwrap();
        let qty = store(&conn).get_stock(&p.id).unwrap();
        assert_eq!(qty, 0);
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
        let err = s
            .create_product("  ", "X", price(1), None, None, 0)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "sku"));
        let err = s
            .create_product("SKU", "", price(1), None, None, 0)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
        let err = s
            .create_product("SKU", "X", price(-1), None, None, 0)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "price"));
        let err = s
            .create_product("SKU", "X", price(1), None, None, -5)
            .unwrap_err();
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
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('pv-parent', 'PARENT-001', 'Parent Product', 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    #[test]
    fn create_and_list_product_variants() {
        let conn = fresh();
        seed_product_variant_parent(&conn);
        let s = store(&conn);

        let v1 = ProductVariant {
            id: uuid::Uuid::new_v4().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "Small".into(),
            sku: "PARENT-001-SMALL".into(),
            price: Some(price(800)),
            barcode: Some("sm-barcode".into()),
            sort_order: 1,
            is_active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };

        let v2 = ProductVariant {
            id: uuid::Uuid::new_v4().to_string(),
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
            id: uuid::Uuid::new_v4().to_string(),
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
            id: uuid::Uuid::new_v4().to_string(),
            parent_sku: "PARENT-001".into(),
            name: "Original".into(),
            sku: "VAR-001".into(),
            price: Some(price(500)),
            barcode: Some("orig".into()),
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
            barcode: Some("new-barcode".into()),
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
            id: uuid::Uuid::new_v4().to_string(),
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
            id: uuid::Uuid::new_v4().to_string(),
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
}
