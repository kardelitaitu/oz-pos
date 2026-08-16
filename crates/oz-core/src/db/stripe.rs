//! Stripe customer → tenant mapping (ADR sync-plan-gating follow-up).
//!
//! Cloud sync plans are keyed by `tenant_id`. Stripe subscription events
//! carry a `cus_xxx` customer id; to turn billing state into a plan we
//! must know which tenant that customer belongs to. The mapping is learned
//! on the first event (Checkout Session metadata) and reused by later
//! events (`invoice.paid`, `customer.subscription.updated`, …) that carry
//! only the customer id.

use rusqlite::params;

use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// Record (or refresh) which tenant a Stripe customer belongs to.
    pub fn set_stripe_customer(
        &self,
        stripe_customer_id: &str,
        tenant_id: &str,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO stripe_customers (stripe_customer_id, tenant_id, updated_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(stripe_customer_id) DO UPDATE SET
                tenant_id = excluded.tenant_id,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![stripe_customer_id, tenant_id],
        )?;
        Ok(())
    }

    /// Look up the tenant owning a Stripe customer. `None` when unknown.
    pub fn get_tenant_for_stripe_customer(
        &self,
        stripe_customer_id: &str,
    ) -> Result<Option<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT tenant_id FROM stripe_customers WHERE stripe_customer_id = ?1")?;
        match stmt.query_row(params![stripe_customer_id], |row| row.get::<_, String>(0)) {
            Ok(tenant_id) => Ok(Some(tenant_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
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
    fn set_then_get_roundtrips() {
        let conn = fresh();
        let s = store(&conn);
        s.set_stripe_customer("cus_123", "tenant-a").unwrap();
        assert_eq!(
            s.get_tenant_for_stripe_customer("cus_123").unwrap(),
            Some("tenant-a".to_string())
        );
    }

    #[test]
    fn unknown_customer_returns_none() {
        let conn = fresh();
        let s = store(&conn);
        assert_eq!(s.get_tenant_for_stripe_customer("cus_nope").unwrap(), None);
    }

    #[test]
    fn upsert_moves_customer_to_new_tenant() {
        let conn = fresh();
        let s = store(&conn);
        s.set_stripe_customer("cus_123", "tenant-a").unwrap();
        s.set_stripe_customer("cus_123", "tenant-b").unwrap();
        assert_eq!(
            s.get_tenant_for_stripe_customer("cus_123").unwrap(),
            Some("tenant-b".to_string()),
            "re-mapping must overwrite the previous tenant"
        );
    }
}
