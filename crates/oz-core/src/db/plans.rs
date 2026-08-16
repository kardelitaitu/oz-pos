//! Per-tenant sync plans (ADR sync-plan-gating).
//!
//! Cloud sync is a paid feature: a tenant on the [`TenantPlan::Free`] plan
//! can run the POS locally but cannot push/pull to the cloud server. The
//! plan is keyed by `tenant_id` — the same value carried in JWT claims —
//! so every terminal of a store inherits the store's plan. Enforcement is
//! server-side; this table is the source of truth.
//!
//! A missing row means the tenant was never assigned a plan; callers decide
//! how to treat that (dev mode allows, production fails closed to `free`).

use rusqlite::params;

use crate::error::CoreError;

use super::Store;

/// Cloud sync plan for a tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    /// Local-only operation — cloud sync is blocked.
    #[default]
    Free,
    /// Full cloud sync enabled.
    Pro,
}

impl TenantPlan {
    /// Parse a plan string from the DB, defaulting to `free` for unknown
    /// values so a typo in a migration or a future plan name degrades
    /// safely (fail closed).
    pub fn from_db(value: &str) -> Self {
        match value {
            "pro" => TenantPlan::Pro,
            _ => TenantPlan::Free,
        }
    }

    /// Storage representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TenantPlan::Free => "free",
            TenantPlan::Pro => "pro",
        }
    }
}

impl Store<'_> {
    /// Read a tenant's plan. Returns `None` when the tenant has no row yet
    /// (callers decide dev/production semantics for the missing case).
    pub fn get_tenant_plan(&self, tenant_id: &str) -> Result<Option<TenantPlan>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT plan FROM tenant_plans WHERE tenant_id = ?1")?;
        let result = stmt.query_row(params![tenant_id], |row| row.get::<_, String>(0));
        match result {
            Ok(plan) => Ok(Some(TenantPlan::from_db(&plan))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Upsert a tenant's plan. Missing tenants get a row; existing rows are
    /// updated and their `updated_at` refreshed.
    pub fn set_tenant_plan(&self, tenant_id: &str, plan: TenantPlan) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO tenant_plans (tenant_id, plan, updated_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(tenant_id) DO UPDATE SET
                plan = excluded.plan,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![tenant_id, plan.as_db_str()],
        )?;
        Ok(())
    }

    /// List every tenant's plan, ordered by tenant id.
    pub fn list_tenant_plans(&self) -> Result<Vec<(String, TenantPlan)>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT tenant_id, plan FROM tenant_plans ORDER BY tenant_id ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|r| {
            let (tid, plan) = r?;
            Ok((tid, TenantPlan::from_db(&plan)))
        })
        .collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    #[test]
    fn missing_tenant_returns_none() {
        let conn = fresh();
        let plan = store(&conn).get_tenant_plan("tenant-nope").unwrap();
        assert_eq!(plan, None, "a tenant with no row has no assigned plan");
    }

    #[test]
    fn set_then_get_roundtrips() {
        let conn = fresh();
        let s = store(&conn);
        s.set_tenant_plan("tenant-a", TenantPlan::Pro).unwrap();
        assert_eq!(
            s.get_tenant_plan("tenant-a").unwrap(),
            Some(TenantPlan::Pro)
        );
    }

    #[test]
    fn default_is_free_when_unset_plan_value_unknown() {
        assert_eq!(TenantPlan::from_db("free"), TenantPlan::Free);
        assert_eq!(TenantPlan::from_db("pro"), TenantPlan::Pro);
        // Unknown/future values degrade safely to free (fail closed).
        assert_eq!(TenantPlan::from_db("enterprise"), TenantPlan::Free);
    }

    #[test]
    fn upsert_overwrites_existing_plan() {
        let conn = fresh();
        let s = store(&conn);
        s.set_tenant_plan("tenant-a", TenantPlan::Free).unwrap();
        s.set_tenant_plan("tenant-a", TenantPlan::Pro).unwrap();
        assert_eq!(
            s.get_tenant_plan("tenant-a").unwrap(),
            Some(TenantPlan::Pro),
            "upsert must overwrite the previous plan"
        );
    }

    #[test]
    fn list_plans_returns_all_ordered() {
        let conn = fresh();
        let s = store(&conn);
        s.set_tenant_plan("tenant-b", TenantPlan::Pro).unwrap();
        s.set_tenant_plan("tenant-a", TenantPlan::Free).unwrap();
        let plans = s.list_tenant_plans().unwrap();
        assert_eq!(
            plans,
            vec![
                ("tenant-a".to_string(), TenantPlan::Free),
                ("tenant-b".to_string(), TenantPlan::Pro),
            ]
        );
    }
}
