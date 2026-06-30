//! Promotion CRUD — list, get, create, update, delete, and application recording.

use rusqlite::params;

use crate::error::CoreError;
use crate::{Promotion, PromotionApplication};

use super::Store;

impl Store<'_> {
    /// List all promotions, ordered by name.
    pub fn list_promotions(&self) -> Result<Vec<Promotion>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, promo_type, value_minor,
                    min_qty, trigger_sku, reward_sku, reward_qty,
                    starts_at, ends_at, min_order_minor, category_id,
                    active, created_at, updated_at
             FROM promotions
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], row_to_promotion)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single promotion by id.
    pub fn get_promotion(&self, id: &str) -> Result<Option<Promotion>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, promo_type, value_minor,
                    min_qty, trigger_sku, reward_sku, reward_qty,
                    starts_at, ends_at, min_order_minor, category_id,
                    active, created_at, updated_at
             FROM promotions
             WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], row_to_promotion);
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new promotion.
    pub fn create_promotion(&self, promo: &Promotion) -> Result<Promotion, CoreError> {
        self.conn.execute(
            "INSERT INTO promotions (id, name, description, promo_type, value_minor,
                                     min_qty, trigger_sku, reward_sku, reward_qty,
                                     starts_at, ends_at, min_order_minor, category_id,
                                     active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                promo.id,
                promo.name,
                promo.description,
                promo.promo_type,
                promo.value_minor,
                promo.min_qty,
                promo.trigger_sku,
                promo.reward_sku,
                promo.reward_qty,
                promo.starts_at,
                promo.ends_at,
                promo.min_order_minor,
                promo.category_id,
                promo.active as i64,
                promo.created_at,
                promo.updated_at,
            ],
        )?;
        Ok(promo.clone())
    }

    /// Update an existing promotion by id.
    pub fn update_promotion(&self, promo: &Promotion) -> Result<Promotion, CoreError> {
        let rows = self.conn.execute(
            "UPDATE promotions
             SET name = ?1, description = ?2, promo_type = ?3, value_minor = ?4,
                 min_qty = ?5, trigger_sku = ?6, reward_sku = ?7, reward_qty = ?8,
                 starts_at = ?9, ends_at = ?10, min_order_minor = ?11, category_id = ?12,
                 active = ?13, updated_at = ?14
             WHERE id = ?15",
            params![
                promo.name,
                promo.description,
                promo.promo_type,
                promo.value_minor,
                promo.min_qty,
                promo.trigger_sku,
                promo.reward_sku,
                promo.reward_qty,
                promo.starts_at,
                promo.ends_at,
                promo.min_order_minor,
                promo.category_id,
                promo.active as i64,
                promo.updated_at,
                promo.id,
            ],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "promotion",
                id: promo.id.clone(),
            });
        }
        Ok(promo.clone())
    }

    /// Delete a promotion by id.
    pub fn delete_promotion(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM promotions WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "promotion",
                id: id.to_owned(),
            });
        }
        Ok(())
    }

    /// List all currently active promotions.
    ///
    /// A promotion is active when `active = 1` AND (`starts_at` IS NULL OR
    /// `starts_at` <= current time) AND (`ends_at` IS NULL OR `ends_at` >
    /// current time).
    pub fn get_active_promotions(&self) -> Result<Vec<Promotion>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, promo_type, value_minor,
                    min_qty, trigger_sku, reward_sku, reward_qty,
                    starts_at, ends_at, min_order_minor, category_id,
                    active, created_at, updated_at
             FROM promotions
             WHERE active = 1
               AND (ends_at IS NULL OR ends_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
               AND (starts_at IS NULL OR starts_at <= strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], row_to_promotion)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Record a promotion application against a sale.
    pub fn record_promotion_application(
        &self,
        app: &PromotionApplication,
    ) -> Result<PromotionApplication, CoreError> {
        self.conn.execute(
            "INSERT INTO promotion_applications (id, promotion_id, sale_id, discount_minor, description, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                app.id,
                app.promotion_id,
                app.sale_id,
                app.discount_minor,
                app.description,
                app.created_at,
            ],
        )?;
        Ok(app.clone())
    }

    /// List all promotion applications for a given sale.
    pub fn get_promotion_applications_for_sale(
        &self,
        sale_id: &str,
    ) -> Result<Vec<PromotionApplication>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, promotion_id, sale_id, discount_minor, description, created_at
             FROM promotion_applications
             WHERE sale_id = ?1
             ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![sale_id], row_to_promotion_application)?;
        rows.map(|r| Ok(r?)).collect()
    }
}

fn row_to_promotion(row: &rusqlite::Row) -> rusqlite::Result<Promotion> {
    Ok(Promotion {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        promo_type: row.get("promo_type")?,
        value_minor: row.get("value_minor")?,
        min_qty: row.get("min_qty")?,
        trigger_sku: row.get("trigger_sku")?,
        reward_sku: row.get("reward_sku")?,
        reward_qty: row.get("reward_qty")?,
        starts_at: row.get("starts_at")?,
        ends_at: row.get("ends_at")?,
        min_order_minor: row.get("min_order_minor")?,
        category_id: row.get("category_id")?,
        active: row.get::<_, i64>("active")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_promotion_application(row: &rusqlite::Row) -> rusqlite::Result<PromotionApplication> {
    Ok(PromotionApplication {
        id: row.get("id")?,
        promotion_id: row.get("promotion_id")?,
        sale_id: row.get("sale_id")?,
        discount_minor: row.get("discount_minor")?,
        description: row.get("description")?,
        created_at: row.get("created_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Store;
    use crate::migrations;

    fn setup() -> Store<'static> {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        migrations::run(&mut conn).unwrap();
        // Leak the connection to get a 'static ref for tests.
        let conn = Box::leak(Box::new(conn));
        Store::new(conn)
    }

    fn test_promo(id: &str) -> Promotion {
        Promotion {
            id: id.to_owned(),
            name: format!("Promo {id}"),
            description: "Test".into(),
            promo_type: "percentage".into(),
            value_minor: 10,
            min_qty: None,
            trigger_sku: None,
            reward_sku: None,
            reward_qty: None,
            starts_at: None,
            ends_at: None,
            min_order_minor: 0,
            category_id: None,
            active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn create_and_list() {
        let store = setup();
        let p = test_promo("p1");
        store.create_promotion(&p).unwrap();
        let list = store.list_promotions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Promo p1");
    }

    #[test]
    fn get_by_id() {
        let store = setup();
        let p = test_promo("p2");
        store.create_promotion(&p).unwrap();
        let found = store.get_promotion("p2").unwrap().unwrap();
        assert_eq!(found.name, "Promo p2");
        assert!(store.get_promotion("nonexistent").unwrap().is_none());
    }

    #[test]
    fn update() {
        let store = setup();
        let mut p = test_promo("p3");
        store.create_promotion(&p).unwrap();
        p.name = "Updated".into();
        p.updated_at = "2025-06-01T00:00:00.000Z".into();
        store.update_promotion(&p).unwrap();
        let found = store.get_promotion("p3").unwrap().unwrap();
        assert_eq!(found.name, "Updated");
    }

    #[test]
    fn update_not_found() {
        let store = setup();
        let p = test_promo("nonexistent");
        let err = store.update_promotion(&p).unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn delete() {
        let store = setup();
        let p = test_promo("p4");
        store.create_promotion(&p).unwrap();
        store.delete_promotion("p4").unwrap();
        assert!(store.get_promotion("p4").unwrap().is_none());
    }

    #[test]
    fn delete_not_found() {
        let store = setup();
        let err = store.delete_promotion("nonexistent").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn get_active_promotions() {
        let store = setup();
        let now = chrono::Utc::now();
        let past = now - chrono::Duration::hours(2);
        let future = now + chrono::Duration::hours(2);

        // Active — no time bounds.
        let p1 = test_promo("p1");
        store.create_promotion(&p1).unwrap();

        // Active — within window.
        let mut p2 = test_promo("p2");
        p2.starts_at = Some(past.to_rfc3339());
        p2.ends_at = Some(future.to_rfc3339());
        store.create_promotion(&p2).unwrap();

        // Inactive — active = 0.
        let mut p3 = test_promo("p3");
        p3.active = false;
        store.create_promotion(&p3).unwrap();

        // Expired.
        let far_past = now - chrono::Duration::hours(48);
        let mut p4 = test_promo("p4");
        p4.starts_at = Some(far_past.to_rfc3339());
        p4.ends_at = Some((far_past + chrono::Duration::hours(1)).to_rfc3339());
        store.create_promotion(&p4).unwrap();

        let active = store.get_active_promotions().unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn record_and_get_applications() {
        let store = setup();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Create a promotion first (FK constraint).
        let promo = test_promo("promo-1");
        store.create_promotion(&promo).unwrap();

        // Create a sale (FK constraint).
        store
            .conn
            .execute(
                "INSERT INTO sales (id, total_minor, currency, line_count, created_at, updated_at)
             VALUES ('sale-1', 1000, 'USD', 1, ?1, ?1)",
                params![now],
            )
            .unwrap();

        let app = PromotionApplication {
            id: "app-1".into(),
            promotion_id: "promo-1".into(),
            sale_id: "sale-1".into(),
            discount_minor: 100,
            description: "10% off".into(),
            created_at: now.clone(),
        };
        store.record_promotion_application(&app).unwrap();

        let apps = store.get_promotion_applications_for_sale("sale-1").unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].discount_minor, 100);
    }
}
