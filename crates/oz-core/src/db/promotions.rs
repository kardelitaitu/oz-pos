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
        let conn = migrations::fresh_db();
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
    fn list_promotions_empty() {
        let store = setup();
        let list = store.list_promotions().unwrap();
        assert!(list.is_empty());
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

    // ── Additional edge-case tests ─────────────────────────────────

    #[test]
    fn list_promotions_ordered_by_name() {
        let store = setup();
        let c = test_promo("p-c");
        store.create_promotion(&c).unwrap();
        let a = test_promo("p-a");
        store.create_promotion(&a).unwrap();
        let b = test_promo("p-b");
        store.create_promotion(&b).unwrap();

        let list = store.list_promotions().unwrap();
        assert_eq!(list.len(), 3);
        // ORDER BY name ASC: Promo p-a, Promo p-b, Promo p-c
        assert_eq!(list[0].name, "Promo p-a");
        assert_eq!(list[1].name, "Promo p-b");
        assert_eq!(list[2].name, "Promo p-c");
    }

    #[test]
    fn create_promotion_duplicate_id() {
        let store = setup();
        let p = test_promo("dup");
        store.create_promotion(&p).unwrap();
        let result = store.create_promotion(&p);
        assert!(result.is_err());
    }

    #[test]
    fn update_changes_all_fields() {
        let store = setup();
        let mut p = test_promo("all");
        store.create_promotion(&p).unwrap();

        p.name = "All Updated".into();
        p.description = "New desc".into();
        p.promo_type = "fixed".into();
        p.value_minor = 500;
        p.min_qty = Some(2);
        p.trigger_sku = Some("SKU-TRIGGER".into());
        p.reward_sku = Some("SKU-REWARD".into());
        p.reward_qty = Some(1);
        p.min_order_minor = 1000;
        p.category_id = Some("cat-1".into());
        p.active = false;
        p.updated_at = "2025-06-01T00:00:00.000Z".into();
        store.update_promotion(&p).unwrap();

        let found = store.get_promotion("all").unwrap().unwrap();
        assert_eq!(found.name, "All Updated");
        assert_eq!(found.description, "New desc");
        assert_eq!(found.promo_type, "fixed");
        assert_eq!(found.value_minor, 500);
        assert_eq!(found.min_qty, Some(2));
        assert_eq!(found.trigger_sku, Some("SKU-TRIGGER".to_owned()));
        assert_eq!(found.reward_sku, Some("SKU-REWARD".to_owned()));
        assert_eq!(found.reward_qty, Some(1));
        assert_eq!(found.min_order_minor, 1000);
        assert_eq!(found.category_id, Some("cat-1".to_owned()));
        assert!(!found.active);
    }

    #[test]
    fn get_active_promotions_no_time_bounds() {
        let store = setup();
        let p = test_promo("no-time");
        store.create_promotion(&p).unwrap();

        // starts_at = NULL, ends_at = NULL, active = true → should be active
        let active = store.get_active_promotions().unwrap();
        assert!(active.iter().any(|x| x.id == "no-time"));
    }

    #[test]
    fn get_active_promotions_no_active_promos() {
        let store = setup();

        // Create only inactive promos
        let mut p1 = test_promo("i1");
        p1.active = false;
        store.create_promotion(&p1).unwrap();
        let mut p2 = test_promo("i2");
        p2.active = false;
        store.create_promotion(&p2).unwrap();

        let active = store.get_active_promotions().unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn get_active_promotions_future_starts_at() {
        let store = setup();
        let future = chrono::Utc::now() + chrono::Duration::hours(24);

        let mut p = test_promo("future");
        p.starts_at = Some(future.to_rfc3339());
        store.create_promotion(&p).unwrap();

        // starts_at is in the future → not yet active
        let active = store.get_active_promotions().unwrap();
        assert!(!active.iter().any(|x| x.id == "future"));
    }

    #[test]
    fn get_active_promotions_past_ends_at() {
        let store = setup();
        let past = chrono::Utc::now() - chrono::Duration::hours(24);

        let mut p = test_promo("past");
        p.ends_at = Some(past.to_rfc3339());
        store.create_promotion(&p).unwrap();

        // ends_at is in the past → expired
        let active = store.get_active_promotions().unwrap();
        assert!(!active.iter().any(|x| x.id == "past"));
    }

    #[test]
    fn get_promotion_applications_multiple_for_sale() {
        let store = setup();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let p1 = test_promo("mp1");
        store.create_promotion(&p1).unwrap();
        let p2 = test_promo("mp2");
        store.create_promotion(&p2).unwrap();

        store
            .conn
            .execute(
                "INSERT INTO sales (id, total_minor, currency, line_count, created_at, updated_at)
             VALUES ('multi-sale', 2000, 'USD', 2, ?1, ?1)",
                params![now],
            )
            .unwrap();

        let app1 = PromotionApplication {
            id: "app-m1".into(),
            promotion_id: "mp1".into(),
            sale_id: "multi-sale".into(),
            discount_minor: 100,
            description: "10% off".into(),
            created_at: now.clone(),
        };
        let app2 = PromotionApplication {
            id: "app-m2".into(),
            promotion_id: "mp2".into(),
            sale_id: "multi-sale".into(),
            discount_minor: 50,
            description: "$5 off".into(),
            created_at: now.clone(),
        };
        store.record_promotion_application(&app1).unwrap();
        store.record_promotion_application(&app2).unwrap();

        let apps = store
            .get_promotion_applications_for_sale("multi-sale")
            .unwrap();
        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn get_promotion_applications_empty_for_sale() {
        let store = setup();
        let apps = store
            .get_promotion_applications_for_sale("no-apps-sale")
            .unwrap();
        assert!(apps.is_empty());
    }
}
