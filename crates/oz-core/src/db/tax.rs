//! Tax rate CRUD — list, get, create, update, delete, and product/category assignments.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice B5 part 4)
crate: oz-core | status: SAFE | lint: CLEAN
findings: exemplary — TAX-02 default-flag swap atomic in tx; TAX-03 archive-not-delete with sale-line reference guard + junction cleanup + archived-rate immutability + active-rate validation on assignment; TAX-04 bounded bps with overflow rationale (MAX_TAX_RATE_BPS); PROD-12 batch junction query with documented SQLITE_MAX_VARIABLE_NUMBER bound
next: none | perf: batch query documented
*/

use rusqlite::params;

use crate::error::CoreError;
use crate::tax_rate::TaxRate;

use super::Store;

/// Reference counts for a tax rate (TAX-03) — used by the delete
/// confirmation UI to show dependencies before archiving.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxRateDependencyCounts {
    /// Number of product assignments referencing this rate.
    pub products: i64,
    /// Number of category assignments referencing this rate.
    pub categories: i64,
    /// Number of historical sale lines referencing this rate.
    pub sale_lines: i64,
}

/// Maximum supported tax rate in basis points (1_000_000 bps = 10,000%).
///
/// TAX-04: bounds every accepted rate so the calculation
/// expressions `line_total_minor * rate_bps` and `10_000 + rate_bps`
/// cannot overflow `i64` for realistic amounts, and so an accidental
/// or malicious extreme rate is rejected with a structured error
/// instead of silently corrupting totals.
pub const MAX_TAX_RATE_BPS: i64 = 1_000_000;

impl Store<'_> {
    /// Validate the shared tax-rate input invariants (name + bounded rate).
    ///
    /// TAX-04: rejects negative rates and rates above
    /// [`MAX_TAX_RATE_BPS`], giving callers a structured validation
    /// error instead of allowing overflow-prone extremes into the store.
    fn validate_tax_rate_input(name: &str, rate_bps: i64) -> Result<(), CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "tax rate name must not be empty".into(),
            });
        }
        if !(0..=MAX_TAX_RATE_BPS).contains(&rate_bps) {
            return Err(CoreError::Validation {
                field: "rate_bps",
                message: format!("rate must be between 0 and {MAX_TAX_RATE_BPS} bps"),
            });
        }
        Ok(())
    }

    /// List all active tax rates, ordered by name.
    ///
    /// TAX-03: archived (soft-deleted, `is_active = 0`) rates are hidden
    /// from listing so they can no longer be assigned or selected.
    pub fn list_tax_rates(&self) -> Result<Vec<TaxRate>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at
             FROM tax_rates WHERE is_active = 1 ORDER BY name",
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

    /// Look up a single active tax rate by id.
    ///
    /// TAX-03: archived rates return `None` so they are invisible to
    /// rate resolution and management screens alike.
    pub fn get_tax_rate(&self, id: &str) -> Result<Option<TaxRate>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at
             FROM tax_rates WHERE id = ?1 AND is_active = 1",
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

    /// Return the active tax rate marked as the store-wide default.
    ///
    /// Returns `None` when no default rate is configured or the
    /// default rate has been archived.
    pub fn get_default_tax_rate(&self) -> Result<Option<TaxRate>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at
             FROM tax_rates WHERE is_default = 1 AND is_active = 1",
        )?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => Ok(Some(TaxRate {
                id: row.get("id")?,
                name: row.get("name")?,
                rate_bps: row.get("rate_bps")?,
                is_default: row.get("is_default")?,
                is_inclusive: row.get("is_inclusive")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })),
            None => Ok(None),
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
        Self::validate_tax_rate_input(name, rate_bps)?;

        // TAX-02: clear the previous default and insert the new rate in one
        // transaction so a failed write cannot leave the store without a
        // default rate, and concurrent writers cannot race the flag swap.
        let tx = self.conn.unchecked_transaction()?;
        if is_default {
            tx.execute(
                "UPDATE tax_rates SET is_default = 0 WHERE is_default = 1",
                [],
            )?;
        }

        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        tx.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, name.trim(), rate_bps, is_default, is_inclusive, now, now],
        )?;
        tx.commit()?;

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
        Self::validate_tax_rate_input(name, rate_bps)?;

        // TAX-02: clear the previous default and apply the update in one
        // transaction so a failure cannot leave the store default-less or
        // with a stale default flag.
        let tx = self.conn.unchecked_transaction()?;
        if is_default {
            tx.execute(
                "UPDATE tax_rates SET is_default = 0 WHERE is_default = 1",
                [],
            )?;
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        // TAX-03: archived (is_active = 0) rates are immutable — updating them
        // would resurrect a rate the operator explicitly hid, so the write is
        // rejected with NotFound (same as a missing id).
        let affected = tx.execute(
            "UPDATE tax_rates SET name = ?1, rate_bps = ?2, is_default = ?3, is_inclusive = ?4, updated_at = ?5 WHERE id = ?6 AND is_active = 1",
            params![name.trim(), rate_bps, is_default, is_inclusive, now, id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "tax_rate",
                id: id.to_owned(),
            });
        }
        tx.commit()?;

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

    /// Count every reference to a tax rate (TAX-03).
    ///
    /// Returns how many products, categories, and historical sale lines
    /// currently reference the rate. Used by the delete confirmation UI
    /// to show dependencies before archiving, and by [`Self::delete_tax_rate`]
    /// to enforce the "block archiving rates referenced by sales" policy.
    pub fn tax_rate_dependency_counts(
        &self,
        id: &str,
    ) -> Result<TaxRateDependencyCounts, CoreError> {
        let products: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM product_taxes WHERE tax_rate_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let categories: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM category_taxes WHERE tax_rate_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let sale_lines: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sale_lines WHERE tax_rate_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(TaxRateDependencyCounts {
            products,
            categories,
            sale_lines,
        })
    }

    /// Archive (soft-delete) a tax rate by id.
    ///
    /// TAX-03: instead of a hard `DELETE`, sets `is_active = 0` so
    /// historical `sale_lines.tax_rate_id` references keep a resolvable
    /// (though hidden) rate row for audit/reconciliation. Junction rows
    /// in `product_taxes`/`category_taxes` are removed in the same
    /// transaction, mirroring the old cascade behaviour.
    ///
    /// Archiving a rate still referenced by historical sales is blocked
    /// with a [`CoreError::Validation`] — receipts and audit trails must
    /// keep their rate linkage.
    pub fn delete_tax_rate(&self, id: &str) -> Result<(), CoreError> {
        // TAX-03: never archive a rate referenced by historical sales.
        let counts = self.tax_rate_dependency_counts(id)?;
        if counts.sale_lines > 0 {
            return Err(CoreError::Validation {
                field: "tax_rate",
                message: format!(
                    "cannot archive tax rate {id}: referenced by {} historical sale line(s)",
                    counts.sale_lines
                ),
            });
        }

        let tx = self.conn.unchecked_transaction()?;
        let affected = tx.execute(
            "UPDATE tax_rates SET is_active = 0 WHERE id = ?1 AND is_active = 1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "tax_rate",
                id: id.to_owned(),
            });
        }
        // Junction rows are configuration, not history — drop them so no
        // product/category points at an archived rate.
        tx.execute(
            "DELETE FROM product_taxes WHERE tax_rate_id = ?1",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM category_taxes WHERE tax_rate_id = ?1",
            params![id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Validate that every id in `tax_rate_ids` resolves to an active
    /// (`is_active = 1`) tax rate.
    ///
    /// TAX-03: archived rates are immutable and hidden — an assignment
    /// must not silently point a product/category at one. Unknown ids are
    /// rejected with the same structured `NotFound` so a stale/malformed
    /// payload cannot wedge a junction row against a missing rate.
    fn ensure_active_tax_rate_ids(&self, tax_rate_ids: &[String]) -> Result<(), CoreError> {
        for id in tax_rate_ids {
            if self.get_tax_rate(id)?.is_none() {
                return Err(CoreError::NotFound {
                    entity: "tax_rate",
                    id: id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Assign tax rates to a product.
    ///
    /// TAX-03: every id must resolve to an active rate — archived or
    /// unknown ids are rejected up front so the junction can never point
    /// at a hidden/immutable rate (defense-in-depth on top of the UI only
    /// listing active rates).
    pub fn set_product_tax_rates(
        &self,
        sku: &str,
        tax_rate_ids: &[String],
    ) -> Result<(), CoreError> {
        self.ensure_active_tax_rate_ids(tax_rate_ids)?;
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

    /// Get tax rate IDs for many products in one query (PROD-12).
    ///
    /// Returns a map of `product_sku -> [tax_rate_id, ...]` ordered by
    /// `created_at`. This replaces the per-product `get_product_tax_rates`
    /// loop in list endpoints, removing the N+1 database pattern for
    /// catalog loads. Products with no assignments are absent from the map.
    ///
    /// Bounds: the `IN (...)` clause binds one parameter per SKU, so very
    /// large catalogs are capped by SQLite's `SQLITE_MAX_VARIABLE_NUMBER`
    /// (999 in common builds). Callers with larger catalogs should chunk
    /// the SKU list; the current list endpoint stays well under this limit.
    pub fn get_product_tax_rates_batch(
        &self,
        skus: &[String],
    ) -> Result<std::collections::HashMap<String, Vec<String>>, CoreError> {
        use std::collections::HashMap;

        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        if skus.is_empty() {
            return Ok(map);
        }
        let placeholders: Vec<String> = (1..=skus.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "SELECT product_sku, tax_rate_id FROM product_taxes \
             WHERE product_sku IN ({}) ORDER BY created_at",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(skus.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (sku, rate_id) = row?;
            map.entry(sku).or_default().push(rate_id);
        }
        Ok(map)
    }

    /// Assign tax rates to a category.
    ///
    /// TAX-03: every id must resolve to an active rate — archived or
    /// unknown ids are rejected up front (see
    /// `Self::ensure_active_tax_rate_ids`).
    pub fn set_category_tax_rates(
        &self,
        category_id: &str,
        tax_rate_ids: &[String],
    ) -> Result<(), CoreError> {
        self.ensure_active_tax_rate_ids(tax_rate_ids)?;
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
#[path = "tax_tests.rs"]
mod tests;
