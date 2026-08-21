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
        if promo.name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "promotion name must not be empty".into(),
            });
        }
        if promo.promo_type.trim().is_empty()
            || crate::PromotionType::from_str(promo.promo_type.trim()).is_none()
        {
            return Err(CoreError::Validation {
                field: "promo_type",
                message: "invalid promotion type".into(),
            });
        }
        if promo.value_minor < 0 {
            return Err(CoreError::Validation {
                field: "value_minor",
                message: "value_minor must not be negative".into(),
            });
        }
        if promo.min_order_minor < 0 {
            return Err(CoreError::Validation {
                field: "min_order_minor",
                message: "min_order_minor must not be negative".into(),
            });
        }
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
        if promo.name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "promotion name must not be empty".into(),
            });
        }
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
        if app.discount_minor < 0 {
            return Err(CoreError::Validation {
                field: "discount_minor",
                message: "discount_minor must not be negative".into(),
            });
        }
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
#[path = "promotions_tests.rs"]
mod tests;
