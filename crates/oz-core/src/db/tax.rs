//! Tax rate CRUD — list, get, create, update, delete, and product/category assignments.

use rusqlite::params;

use crate::error::CoreError;
use crate::tax_rate::TaxRate;

use super::Store;

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

        if is_default {
            self.conn.execute(
                "UPDATE tax_rates SET is_default = 0 WHERE is_default = 1",
                [],
            )?;
        }

        let id = uuid::Uuid::new_v4().to_string();
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

        if is_default {
            self.conn.execute(
                "UPDATE tax_rates SET is_default = 0 WHERE is_default = 1",
                [],
            )?;
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let affected = self.conn.execute(
            "UPDATE tax_rates SET name = ?1, rate_bps = ?2, is_default = ?3, is_inclusive = ?4, updated_at = ?5 WHERE id = ?6",
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
    pub fn delete_tax_rate(&self, id: &str) -> Result<(), CoreError> {
        let affected = self
            .conn
            .execute("DELETE FROM tax_rates WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "tax_rate",
                id: id.to_owned(),
            });
        }
        Ok(())
    }

    /// Assign tax rates to a product.
    pub fn set_product_tax_rates(
        &self,
        sku: &str,
        tax_rate_ids: &[String],
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM product_taxes WHERE product_sku = ?1",
            params![sku],
        )?;
        for id in tax_rate_ids {
            tx.execute(
                "INSERT OR IGNORE INTO product_taxes (product_sku, tax_rate_id) VALUES (?1, ?2)",
                params![sku, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Get all tax rate IDs assigned to a product.
    pub fn get_product_tax_rates(&self, sku: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT tax_rate_id FROM product_taxes WHERE product_sku = ?1 ORDER BY created_at",
        )?;
        let ids = stmt
            .query_map(params![sku], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    /// Assign tax rates to a category.
    pub fn set_category_tax_rates(
        &self,
        category_id: &str,
        tax_rate_ids: &[String],
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM category_taxes WHERE category_id = ?1",
            params![category_id],
        )?;
        for id in tax_rate_ids {
            tx.execute(
                "INSERT OR IGNORE INTO category_taxes (category_id, tax_rate_id) VALUES (?1, ?2)",
                params![category_id, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Get all tax rate IDs assigned to a category.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use foundation::Currency;
    use rusqlite::Connection;
    use std::str::FromStr;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    #[test]
    fn list_tax_rates_empty() {
        let conn = fresh();
        let s = store(&conn);
        let rates = s.list_tax_rates().unwrap();
        assert!(rates.is_empty());
    }

    #[test]
    fn create_and_list_tax_rate() {
        let conn = fresh();
        let s = store(&conn);
        s.create_tax_rate("VAT 10%", 1000, true, false).unwrap();
        let rates = s.list_tax_rates().unwrap();
        assert_eq!(rates.len(), 1);
        assert_eq!(rates[0].name, "VAT 10%");
        assert_eq!(rates[0].rate_bps, 1000);
        assert!(rates[0].is_default);
        assert!(!rates[0].is_inclusive);
    }

    #[test]
    fn create_tax_rate_exclusive() {
        let conn = fresh();
        let s = store(&conn);
        s.create_tax_rate("GST 5%", 500, false, true).unwrap();
        let rates = s.list_tax_rates().unwrap();
        assert_eq!(rates.len(), 1);
        assert!(!rates[0].is_default);
        assert!(rates[0].is_inclusive);
    }

    #[test]
    fn create_tax_rate_empty_name() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.create_tax_rate("", 1000, false, false);
        assert!(matches!(
            result,
            Err(CoreError::Validation { field: "name", .. })
        ));
    }

    #[test]
    fn create_tax_rate_negative_rate() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.create_tax_rate("Bad", -1, false, false);
        assert!(matches!(
            result,
            Err(CoreError::Validation {
                field: "rate_bps",
                ..
            })
        ));
    }

    #[test]
    fn get_tax_rate_found() {
        let conn = fresh();
        let s = store(&conn);
        let created = s.create_tax_rate("VAT 8%", 800, true, false).unwrap();
        let found = s.get_tax_rate(&created.id).unwrap().unwrap();
        assert_eq!(found.name, "VAT 8%");
        assert_eq!(found.rate_bps, 800);
    }

    #[test]
    fn get_tax_rate_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.get_tax_rate("nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_tax_rate_basic() {
        let conn = fresh();
        let s = store(&conn);
        let created = s.create_tax_rate("Old Name", 500, false, false).unwrap();
        let updated = s
            .update_tax_rate(&created.id, "New Name", 600, true, true)
            .unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.rate_bps, 600);
        assert!(updated.is_default);
        assert!(updated.is_inclusive);
    }

    #[test]
    fn update_tax_rate_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.update_tax_rate("bad-id", "X", 100, false, false);
        assert!(matches!(result, Err(CoreError::NotFound { .. })));
    }

    #[test]
    fn update_tax_rate_empty_name() {
        let conn = fresh();
        let s = store(&conn);
        let created = s.create_tax_rate("Test", 100, false, false).unwrap();
        let result = s.update_tax_rate(&created.id, "", 100, false, false);
        assert!(matches!(
            result,
            Err(CoreError::Validation { field: "name", .. })
        ));
    }

    #[test]
    fn update_tax_rate_negative_rate() {
        let conn = fresh();
        let s = store(&conn);
        let created = s.create_tax_rate("Test", 100, false, false).unwrap();
        let result = s.update_tax_rate(&created.id, "Test", -5, false, false);
        assert!(matches!(
            result,
            Err(CoreError::Validation {
                field: "rate_bps",
                ..
            })
        ));
    }

    #[test]
    fn delete_tax_rate_removes() {
        let conn = fresh();
        let s = store(&conn);
        let created = s.create_tax_rate("To Delete", 100, false, false).unwrap();
        s.delete_tax_rate(&created.id).unwrap();
        let found = s.get_tax_rate(&created.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn delete_tax_rate_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.delete_tax_rate("bad-id");
        assert!(matches!(result, Err(CoreError::NotFound { .. })));
    }

    #[test]
    fn default_flag_is_cleared_on_new_default() {
        let conn = fresh();
        let s = store(&conn);
        let first = s.create_tax_rate("First", 500, true, false).unwrap();
        let second = s.create_tax_rate("Second", 1000, true, false).unwrap();

        let r1 = s.get_tax_rate(&first.id).unwrap().unwrap();
        let r2 = s.get_tax_rate(&second.id).unwrap().unwrap();
        assert!(!r1.is_default); // cleared when second was set as default
        assert!(r2.is_default);
    }

    #[test]
    fn set_and_get_product_tax_rates() {
        let conn = fresh();
        let s = store(&conn);
        let currency = Currency::from_str("USD").unwrap();
        let money = crate::Money {
            minor_units: 1000,
            currency,
        };
        s.create_product("SKU-TAX", "Taxed Product", money, None, None, 0)
            .unwrap();

        let rate = s.create_tax_rate("VAT", 1000, true, false).unwrap();
        s.set_product_tax_rates("SKU-TAX", std::slice::from_ref(&rate.id))
            .unwrap();

        let ids = s.get_product_tax_rates("SKU-TAX").unwrap();
        assert_eq!(ids, vec![rate.id]);
    }

    #[test]
    fn set_product_tax_rates_overwrites() {
        let conn = fresh();
        let s = store(&conn);
        let currency = Currency::from_str("USD").unwrap();
        let money = crate::Money {
            minor_units: 1000,
            currency,
        };
        s.create_product("SKU-TAX2", "Item", money, None, None, 0)
            .unwrap();

        let r1 = s.create_tax_rate("R1", 500, false, false).unwrap();
        let r2 = s.create_tax_rate("R2", 1000, false, false).unwrap();

        s.set_product_tax_rates("SKU-TAX2", std::slice::from_ref(&r1.id))
            .unwrap();
        s.set_product_tax_rates("SKU-TAX2", std::slice::from_ref(&r2.id))
            .unwrap();

        let ids = s.get_product_tax_rates("SKU-TAX2").unwrap();
        assert_eq!(ids, vec![r2.id]);
    }

    #[test]
    fn set_and_get_category_tax_rates() {
        let conn = fresh();
        let s = store(&conn);
        s.create_category("cat-tax", "Taxed Cat", "#fff", "")
            .unwrap();

        let rate = s.create_tax_rate("CT", 800, false, false).unwrap();
        s.set_category_tax_rates("cat-tax", std::slice::from_ref(&rate.id))
            .unwrap();

        let ids = s.get_category_tax_rates("cat-tax").unwrap();
        assert_eq!(ids, vec![rate.id]);
    }

    #[test]
    fn get_product_tax_rates_none() {
        let conn = fresh();
        let s = store(&conn);
        let ids = s.get_product_tax_rates("NO-SKU").unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn get_category_tax_rates_none() {
        let conn = fresh();
        let s = store(&conn);
        let ids = s.get_category_tax_rates("no-cat").unwrap();
        assert!(ids.is_empty());
    }
}
