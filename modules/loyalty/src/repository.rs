//! Loyalty repository — high-level loyalty operations.
//!
//! This module provides a repository abstraction over the loyalty domain
//! types, wrapping the `oz-core` Store methods. Application code should
//! use this repository rather than calling Store methods directly,
//! keeping the loyalty API surface contained within this module.

use oz_core::db::Store;
use oz_core::error::CoreError;
use oz_core::{LoyaltyAccount, LoyaltyAccountWithDetails, LoyaltyTier, LoyaltyTransaction};

/// Result type for loyalty repository operations.
pub type LoyaltyResult<T> = Result<T, CoreError>;

/// Repository for loyalty program operations.
///
/// Wraps a [`Store`] instance to provide a focused API for loyalty
/// account management, point earn/redeem, tier administration, and
/// point-value conversion.
///
/// # Example
///
/// ```no_run
/// use modules_loyalty::repository::LoyaltyRepository;
/// use oz_core::db::Store;
///
/// # fn example(conn: &rusqlite::Connection) {
/// let store = Store::new(conn);
/// let repo = LoyaltyRepository::new(store);
/// let account = repo.get_or_create_account("cust-1").unwrap();
/// # }
/// ```
pub struct LoyaltyRepository<'a> {
    store: Store<'a>,
}

impl<'a> LoyaltyRepository<'a> {
    /// Create a new loyalty repository from a [`Store`].
    pub fn new(store: Store<'a>) -> Self {
        Self { store }
    }

    /// Get or create a loyalty account for a customer.
    /// If the account already exists, it is returned as-is.
    /// Otherwise a new account is created with the default tier (Bronze).
    pub fn get_or_create_account(&self, customer_id: &str) -> LoyaltyResult<LoyaltyAccount> {
        self.store.get_or_create_loyalty_account(customer_id)
    }

    /// Get full loyalty account details for a customer (with tier info
    /// and recent transactions).
    pub fn get_account(
        &self,
        customer_id: &str,
    ) -> LoyaltyResult<Option<LoyaltyAccountWithDetails>> {
        self.store.get_loyalty_account(customer_id)
    }

    /// List all loyalty accounts with details for management.
    pub fn list_accounts(&self) -> LoyaltyResult<Vec<LoyaltyAccountWithDetails>> {
        self.store.list_loyalty_accounts()
    }

    /// Earn points for a purchase.
    /// `points_earned = (total_minor * tier.points_per_unit / 100) * tier.earn_multiplier`
    pub fn earn_points(
        &self,
        customer_id: &str,
        sale_id: &str,
        total_minor: i64,
    ) -> LoyaltyResult<LoyaltyTransaction> {
        self.store.earn_points(customer_id, sale_id, total_minor)
    }

    /// Redeem points at checkout.
    /// Returns the transaction and the monetary value of redeemed points.
    /// Conversion: 1 point = 1 minor unit.
    pub fn redeem_points(
        &self,
        customer_id: &str,
        points: i64,
        sale_id: &str,
    ) -> LoyaltyResult<(LoyaltyTransaction, i64)> {
        self.store.redeem_points(customer_id, points, sale_id)
    }

    /// List all loyalty tiers ordered by `sort_order`.
    pub fn list_tiers(&self) -> LoyaltyResult<Vec<LoyaltyTier>> {
        self.store.list_tiers()
    }

    /// Update a loyalty tier's configuration.
    pub fn update_tier(
        &self,
        id: &str,
        name: &str,
        min_points: i64,
        points_per_unit: i64,
        earn_multiplier: f64,
        colour: &str,
    ) -> LoyaltyResult<LoyaltyTier> {
        self.store.update_tier(
            id,
            name,
            min_points,
            points_per_unit,
            earn_multiplier,
            colour,
        )
    }

    /// Convert points to monetary value (minor units).
    /// 1 point = 1 minor unit.
    pub fn get_points_value(&self, points: i64) -> i64 {
        self.store.get_points_value(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::{Connection, params};

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn repo(conn: &Connection) -> LoyaltyRepository<'_> {
        LoyaltyRepository::new(Store::new(conn))
    }

    fn seed_customer(conn: &Connection, id: &str, name: &str) {
        conn.execute(
            "INSERT INTO customers (id, name, notes, created_at, updated_at)
             VALUES (?1, ?2, '', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![id, name],
        )
        .unwrap();
    }

    fn seed_sale(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at,
             updated_at, subtotal_minor, tax_total_minor)
             VALUES (?1, 0, 'USD', 0, 'completed', '2025-01-01T00:00:00.000Z',
             '2025-01-01T00:00:00.000Z', 0, 0)",
            params![id],
        )
        .unwrap();
    }

    // ── Account management ───────────────────────────────────────

    #[test]
    fn get_or_create_creates_account() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        let account = repo(&conn).get_or_create_account("cust-1").unwrap();
        assert_eq!(account.customer_id, "cust-1");
        assert_eq!(account.points, 0);
        assert_eq!(account.tier_id.as_deref(), Some("tier-bronze"));
    }

    #[test]
    fn get_or_create_returns_existing() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        let a1 = repo(&conn).get_or_create_account("cust-1").unwrap();
        let a2 = repo(&conn).get_or_create_account("cust-1").unwrap();
        assert_eq!(a1.id, a2.id);
    }

    #[test]
    fn get_account_returns_details() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        repo(&conn).get_or_create_account("cust-1").unwrap();
        repo(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();

        let details = repo(&conn).get_account("cust-1").unwrap().unwrap();
        assert_eq!(details.account.customer_id, "cust-1");
        assert!(details.tier.is_some());
        assert_eq!(details.recent_transactions.len(), 1);
    }

    #[test]
    fn get_account_nonexistent_returns_none() {
        let conn = fresh();
        let result = repo(&conn).get_account("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_accounts_empty() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        let accounts = repo(&conn).list_accounts().unwrap();
        assert!(accounts.is_empty());
    }

    #[test]
    fn list_accounts_ordered_by_points() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_customer(&conn, "cust-2", "Bob");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");

        let r = repo(&conn);
        r.get_or_create_account("cust-1").unwrap();
        r.get_or_create_account("cust-2").unwrap();
        r.earn_points("cust-1", "sale-1", 1000).unwrap();
        r.earn_points("cust-2", "sale-2", 5000).unwrap();

        let accounts = r.list_accounts().unwrap();
        assert_eq!(accounts.len(), 2);
        // Higher lifetime_points first
        assert_eq!(accounts[0].account.customer_id, "cust-2");
        assert_eq!(accounts[1].account.customer_id, "cust-1");
    }

    // ── Points operations ────────────────────────────────────────

    #[test]
    fn earn_points_creates_transaction() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        repo(&conn).get_or_create_account("cust-1").unwrap();

        let txn = repo(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();
        assert_eq!(txn.txn_type, "earn");
        assert_eq!(txn.points, 100); // 1000 * 10 / 100 * 1.0

        let details = repo(&conn).get_account("cust-1").unwrap().unwrap();
        assert_eq!(details.account.points, 100);
    }

    #[test]
    fn earn_points_zero_total_rejected() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        repo(&conn).get_or_create_account("cust-1").unwrap();

        let err = repo(&conn).earn_points("cust-1", "sale-1", 0).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "total_minor"));
    }

    #[test]
    fn redeem_points_deducts_correctly() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");
        repo(&conn).get_or_create_account("cust-1").unwrap();
        repo(&conn).earn_points("cust-1", "sale-1", 5000).unwrap();

        let (txn, discount) = repo(&conn).redeem_points("cust-1", 200, "sale-2").unwrap();
        assert_eq!(txn.points, -200);
        assert_eq!(discount, 200);

        let details = repo(&conn).get_account("cust-1").unwrap().unwrap();
        // 500 points earned - 200 redeemed = 300
        assert_eq!(details.account.points, 300);
    }

    #[test]
    fn redeem_points_insufficient_balance() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        repo(&conn).get_or_create_account("cust-1").unwrap();

        let err = repo(&conn)
            .redeem_points("cust-1", 100, "sale-1")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "points"));
    }

    #[test]
    fn redeem_points_no_account() {
        let conn = fresh();
        seed_sale(&conn, "sale-1");
        let err = repo(&conn)
            .redeem_points("nonexistent", 100, "sale-1")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "loyalty_account"));
    }

    // ── Tier management ──────────────────────────────────────────

    #[test]
    fn list_tiers_returns_seeded() {
        let conn = fresh();
        let tiers = repo(&conn).list_tiers().unwrap();
        assert_eq!(tiers.len(), 4);
        assert_eq!(tiers[0].name, "Bronze");
        assert_eq!(tiers[1].name, "Silver");
        assert_eq!(tiers[2].name, "Gold");
        assert_eq!(tiers[3].name, "Platinum");
    }

    #[test]
    fn update_tier_modifies_fields() {
        let conn = fresh();
        let updated = repo(&conn)
            .update_tier("tier-bronze", "Bronze v2", 0, 15, 1.5, "#ff6600")
            .unwrap();
        assert_eq!(updated.name, "Bronze v2");
        assert_eq!(updated.points_per_unit, 15);
        assert_eq!(updated.earn_multiplier, 1.5);
        assert_eq!(updated.colour, "#ff6600");
    }

    #[test]
    fn update_tier_not_found() {
        let conn = fresh();
        let err = repo(&conn)
            .update_tier("nonexistent", "No Tier", 0, 10, 1.0, "#000")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "loyalty_tier"));
    }

    // ── Points value conversion ──────────────────────────────────

    #[test]
    fn get_points_value_converts() {
        let conn = fresh();
        let r = repo(&conn);
        assert_eq!(r.get_points_value(100), 100);
        assert_eq!(r.get_points_value(50), 50);
        assert_eq!(r.get_points_value(0), 0);
    }

    // ── Tier auto-upgrade ────────────────────────────────────────

    #[test]
    fn earn_points_auto_upgrades_tier() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        repo(&conn).get_or_create_account("cust-1").unwrap();

        // 2000 * 10 / 100 * 1.0 = 200 points — reaches Silver (min_points=200)
        repo(&conn).earn_points("cust-1", "sale-1", 2000).unwrap();

        let details = repo(&conn).get_account("cust-1").unwrap().unwrap();
        assert_eq!(details.account.tier_id.as_deref(), Some("tier-silver"));
        assert_eq!(details.tier.as_ref().unwrap().name, "Silver");
    }

    #[test]
    fn customer_not_found_returns_not_found_error() {
        let conn = fresh();
        let err = repo(&conn)
            .get_or_create_account("nonexistent")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "customer"));
    }
}
