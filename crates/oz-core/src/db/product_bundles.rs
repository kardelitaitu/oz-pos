//! CRUD for product bundles and bundle-items.
//!
//! A bundle is a single SKU that contains multiple sub-items. All
//! multi-row writes use transactions for atomicity.

use rusqlite::params;

use crate::error::CoreError;
use crate::product_bundle::{BundleItem, BundleWithItems, ProductBundle};

use super::Store;

// ── Row mappers ──────────────────────────────────────────────────────────

fn row_to_bundle(row: &rusqlite::Row) -> rusqlite::Result<ProductBundle> {
    Ok(ProductBundle {
        id: row.get("id")?,
        bundle_sku: row.get("bundle_sku")?,
        name: row.get("name")?,
        description: row.get("description")?,
        bundle_price_minor: row.get("bundle_price_minor")?,
        currency: row.get("currency")?,
        active: row.get::<_, i64>("active")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_bundle_item(row: &rusqlite::Row) -> rusqlite::Result<BundleItem> {
    Ok(BundleItem {
        id: row.get("id")?,
        bundle_id: row.get("bundle_id")?,
        sku: row.get("sku")?,
        qty: row.get("qty")?,
        unit_price_minor: row.get("unit_price_minor")?,
    })
}

// ── CRUD ─────────────────────────────────────────────────────────────────

impl Store<'_> {
    /// List all bundles with their items.
    pub fn list_bundles(&self) -> Result<Vec<BundleWithItems>, CoreError> {
        let bundles = {
            let mut stmt = self.conn.prepare(
                "SELECT id, bundle_sku, name, description, bundle_price_minor,
                        currency, active, created_at, updated_at
                 FROM product_bundles
                 ORDER BY name",
            )?;
            let rows = stmt.query_map([], row_to_bundle)?;
            rows.map(|r| Ok(r?))
                .collect::<Result<Vec<_>, CoreError>>()?
        };

        let items = self.load_all_bundle_items()?;

        Ok(assemble(bundles, items))
    }

    /// Get a single bundle by id.
    pub fn get_bundle(&self, id: &str) -> Result<Option<BundleWithItems>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bundle_sku, name, description, bundle_price_minor,
                    currency, active, created_at, updated_at
             FROM product_bundles
             WHERE id = ?1",
        )?;
        let bundle = match stmt.query_row(params![id], row_to_bundle) {
            Ok(b) => b,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let items = self.load_bundle_items(&bundle.id)?;
        Ok(Some(BundleWithItems { bundle, items }))
    }

    /// Look up a bundle by its SKU (for scanning/lookup).
    pub fn get_bundle_by_sku(&self, sku: &str) -> Result<Option<BundleWithItems>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bundle_sku, name, description, bundle_price_minor,
                    currency, active, created_at, updated_at
             FROM product_bundles
             WHERE bundle_sku = ?1",
        )?;
        let bundle = match stmt.query_row(params![sku], row_to_bundle) {
            Ok(b) => b,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let items = self.load_bundle_items(&bundle.id)?;
        Ok(Some(BundleWithItems { bundle, items }))
    }

    /// Create a new bundle with its items in a transaction.
    pub fn create_bundle(
        &self,
        bundle: &ProductBundle,
        items: &[BundleItem],
    ) -> Result<BundleWithItems, CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO product_bundles (id, bundle_sku, name, description, bundle_price_minor, currency, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                bundle.id,
                bundle.bundle_sku,
                bundle.name,
                bundle.description,
                bundle.bundle_price_minor,
                bundle.currency,
                if bundle.active { 1 } else { 0 },
                bundle.created_at,
                bundle.updated_at,
            ],
        )?;

        for item in items {
            tx.execute(
                "INSERT INTO bundle_items (id, bundle_id, sku, qty, unit_price_minor)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    item.id,
                    item.bundle_id,
                    item.sku,
                    item.qty,
                    item.unit_price_minor
                ],
            )?;
        }

        tx.commit()?;

        Ok(BundleWithItems {
            bundle: bundle.clone(),
            items: items.to_vec(),
        })
    }

    /// Update a bundle and replace its items in a transaction.
    pub fn update_bundle(
        &self,
        bundle: &ProductBundle,
        items: &[BundleItem],
    ) -> Result<BundleWithItems, CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "UPDATE product_bundles
             SET bundle_sku = ?2, name = ?3, description = ?4,
                 bundle_price_minor = ?5, currency = ?6, active = ?7,
                 updated_at = ?8
             WHERE id = ?1",
            params![
                bundle.id,
                bundle.bundle_sku,
                bundle.name,
                bundle.description,
                bundle.bundle_price_minor,
                bundle.currency,
                if bundle.active { 1 } else { 0 },
                bundle.updated_at,
            ],
        )?;

        // Delete old items and re-insert.
        tx.execute(
            "DELETE FROM bundle_items WHERE bundle_id = ?1",
            params![bundle.id],
        )?;
        for item in items {
            tx.execute(
                "INSERT INTO bundle_items (id, bundle_id, sku, qty, unit_price_minor)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    item.id,
                    item.bundle_id,
                    item.sku,
                    item.qty,
                    item.unit_price_minor
                ],
            )?;
        }

        tx.commit()?;

        Ok(BundleWithItems {
            bundle: bundle.clone(),
            items: items.to_vec(),
        })
    }

    /// Delete a bundle and its items.
    pub fn delete_bundle(&self, id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM bundle_items WHERE bundle_id = ?1", params![id])?;
        tx.execute("DELETE FROM product_bundles WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

impl Store<'_> {
    fn load_all_bundle_items(&self) -> Result<Vec<BundleItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bundle_id, sku, qty, unit_price_minor
             FROM bundle_items
             ORDER BY bundle_id, sku",
        )?;
        let rows = stmt.query_map([], row_to_bundle_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    fn load_bundle_items(&self, bundle_id: &str) -> Result<Vec<BundleItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bundle_id, sku, qty, unit_price_minor
             FROM bundle_items
             WHERE bundle_id = ?1
             ORDER BY sku",
        )?;
        let rows = stmt.query_map(params![bundle_id], row_to_bundle_item)?;
        rows.map(|r| Ok(r?)).collect()
    }
}

fn assemble(bundles: Vec<ProductBundle>, items: Vec<BundleItem>) -> Vec<BundleWithItems> {
    let mut grouped: std::collections::HashMap<String, Vec<BundleItem>> =
        std::collections::HashMap::new();
    for item in items {
        grouped
            .entry(item.bundle_id.clone())
            .or_default()
            .push(item);
    }
    bundles
        .into_iter()
        .map(|b| BundleWithItems {
            bundle: b.clone(),
            items: grouped.remove(&b.id).unwrap_or_default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn fresh_store() -> Store<'static> {
        let conn = migrations::fresh_db();

        // Seed products so FK constraints are satisfied.
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
             VALUES ('p1', 'ITEM-A', 'Item A', 100, 'USD', 'now', 'now'),
                    ('p2', 'ITEM-B', 'Item B', 200, 'USD', 'now', 'now'),
                    ('p3', 'ITEM-C', 'Item C', 150, 'USD', 'now', 'now'),
                    ('p4', 'BUNDLE1', 'Bundle One', 0, 'USD', 'now', 'now'),
                    ('p5', 'B-Gift Box', 'Gift Box Bundle', 0, 'USD', 'now', 'now'),
                    ('p6', 'B-Hamper', 'Hamper Bundle', 0, 'USD', 'now', 'now'),
                    ('p7', 'B-Sampler', 'Sampler Bundle', 0, 'USD', 'now', 'now'),
                    ('p8', 'B-Edit Me', 'Edit Me Bundle', 0, 'USD', 'now', 'now'),
                    ('p9', 'B-Delete Me', 'Delete Me Bundle', 0, 'USD', 'now', 'now')",
        )
        .unwrap();

        // We need a static reference for Store — use leak to satisfy lifetime.
        let conn = Box::leak(Box::new(conn));
        Store::new(conn)
    }

    fn make_bundle(name: &str) -> ProductBundle {
        ProductBundle {
            id: uuid::Uuid::new_v4().to_string(),
            bundle_sku: format!("B-{name}"),
            name: name.into(),
            description: String::new(),
            bundle_price_minor: Some(500),
            currency: "USD".into(),
            active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        }
    }

    fn make_item(bundle_id: &str, sku: &str, qty: i64) -> BundleItem {
        BundleItem {
            id: uuid::Uuid::new_v4().to_string(),
            bundle_id: bundle_id.into(),
            sku: sku.into(),
            qty,
            unit_price_minor: None,
        }
    }

    #[test]
    fn list_bundles_empty() {
        let store = fresh_store();
        let bundles = store.list_bundles().unwrap();
        // Our bundle product "BUNDLE1" is in products but not in product_bundles, so empty.
        assert!(bundles.is_empty());
    }

    #[test]
    fn create_and_list_bundles() {
        let store = fresh_store();
        let bundle = make_bundle("Gift Box");
        let items = vec![
            make_item(&bundle.id, "ITEM-A", 1),
            make_item(&bundle.id, "ITEM-B", 2),
        ];

        store.create_bundle(&bundle, &items).unwrap();

        let all = store.list_bundles().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].bundle.name, "Gift Box");
        assert_eq!(all[0].items.len(), 2);
    }

    #[test]
    fn get_bundle_by_id() {
        let store = fresh_store();
        let bundle = make_bundle("Hamper");
        let items = vec![make_item(&bundle.id, "ITEM-C", 3)];
        store.create_bundle(&bundle, &items).unwrap();

        let found = store.get_bundle(&bundle.id).unwrap().unwrap();
        assert_eq!(found.bundle.bundle_sku, bundle.bundle_sku);
        assert_eq!(found.items.len(), 1);
    }

    #[test]
    fn get_bundle_by_sku() {
        let store = fresh_store();
        let bundle = make_bundle("Sampler");
        let items = vec![make_item(&bundle.id, "ITEM-A", 1)];
        store.create_bundle(&bundle, &items).unwrap();

        let found = store
            .get_bundle_by_sku(&bundle.bundle_sku)
            .unwrap()
            .unwrap();
        assert_eq!(found.bundle.name, "Sampler");
    }

    #[test]
    fn get_missing_bundle_returns_none() {
        let store = fresh_store();
        assert!(store.get_bundle("nonexistent").unwrap().is_none());
        assert!(store.get_bundle_by_sku("NONEXISTENT").unwrap().is_none());
    }

    #[test]
    fn update_bundle_replaces_items() {
        let store = fresh_store();
        let mut bundle = make_bundle("Edit Me");
        let items = vec![make_item(&bundle.id, "ITEM-A", 1)];
        store.create_bundle(&bundle, &items).unwrap();

        bundle.name = "Edited".into();
        let new_items = vec![
            make_item(&bundle.id, "ITEM-B", 1),
            make_item(&bundle.id, "ITEM-C", 2),
        ];
        store.update_bundle(&bundle, &new_items).unwrap();

        let found = store.get_bundle(&bundle.id).unwrap().unwrap();
        assert_eq!(found.bundle.name, "Edited");
        assert_eq!(found.items.len(), 2);
    }

    #[test]
    fn delete_bundle_removes_items() {
        let store = fresh_store();
        let bundle = make_bundle("Delete Me");
        let items = vec![make_item(&bundle.id, "ITEM-A", 1)];
        store.create_bundle(&bundle, &items).unwrap();

        store.delete_bundle(&bundle.id).unwrap();
        assert!(store.get_bundle(&bundle.id).unwrap().is_none());

        // Items should also be gone.
        let all_items = store.load_all_bundle_items().unwrap();
        assert!(all_items.is_empty());
    }

    #[test]
    fn delete_nonexistent_bundle_is_noop() {
        let store = fresh_store();
        // Deleting a nonexistent bundle should not error.
        store.delete_bundle("no-such-bundle").unwrap();
    }
}
