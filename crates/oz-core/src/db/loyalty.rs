//! Loyalty program CRUD — points, tiers, redemption.

use rusqlite::params;

use crate::error::CoreError;
use crate::loyalty::{LoyaltyAccount, LoyaltyAccountWithDetails, LoyaltyTier, LoyaltyTransaction};

use super::Store;

/// Fixed conversion: 100 points = 100 minor units ($1.00).
const POINTS_TO_MINOR_RATIO: i64 = 1;

impl Store<'_> {
    /// Get or create a loyalty account for a customer.
    /// If the account already exists, it is returned as-is.
    /// Otherwise a new account is created with the default tier (Bronze).
    pub fn get_or_create_loyalty_account(
        &self,
        customer_id: &str,
    ) -> Result<LoyaltyAccount, CoreError> {
        // Check if customer exists.
        let customer_exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM customers WHERE id = ?1",
                params![customer_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !customer_exists {
            return Err(CoreError::NotFound {
                entity: "customer",
                id: customer_id.to_owned(),
            });
        }

        // Try to get existing account.
        if let Some(account) = self.get_loyalty_account_raw(customer_id)? {
            return Ok(account);
        }

        // Create new account with Bronze tier.
        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "INSERT INTO loyalty_accounts (id, customer_id, tier_id, updated_at, created_at)
             VALUES (?1, ?2, 'tier-bronze', ?3, ?4)",
            params![id, customer_id, now, now],
        )?;

        Ok(LoyaltyAccount {
            id,
            customer_id: customer_id.to_owned(),
            points: 0,
            lifetime_points: 0,
            tier_id: Some("tier-bronze".into()),
            updated_at: now.clone(),
            created_at: now,
        })
    }

    fn get_loyalty_account_raw(
        &self,
        customer_id: &str,
    ) -> Result<Option<LoyaltyAccount>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, customer_id, points, lifetime_points, tier_id, updated_at, created_at
             FROM loyalty_accounts WHERE customer_id = ?1",
        )?;
        let result = stmt.query_row(params![customer_id], |row| {
            Ok(LoyaltyAccount {
                id: row.get("id")?,
                customer_id: row.get("customer_id")?,
                points: row.get("points")?,
                lifetime_points: row.get("lifetime_points")?,
                tier_id: row.get("tier_id")?,
                updated_at: row.get("updated_at")?,
                created_at: row.get("created_at")?,
            })
        });
        match result {
            Ok(a) => Ok(Some(a)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get full loyalty account details for a customer (with tier info and recent transactions).
    pub fn get_loyalty_account(
        &self,
        customer_id: &str,
    ) -> Result<Option<LoyaltyAccountWithDetails>, CoreError> {
        let account = match self.get_loyalty_account_raw(customer_id)? {
            Some(a) => a,
            None => return Ok(None),
        };

        let tier = if let Some(ref tid) = account.tier_id {
            self.get_loyalty_tier(tid)?
        } else {
            None
        };

        let tiers = self.list_tiers()?;
        let next_tier = tiers
            .iter()
            .filter(|t| t.min_points > account.lifetime_points)
            .min_by_key(|t| t.min_points)
            .cloned();

        let points_to_next_tier = next_tier
            .as_ref()
            .map(|t| t.min_points - account.lifetime_points)
            .unwrap_or(0);

        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, sale_id, points, txn_type, description, created_at
             FROM loyalty_transactions WHERE account_id = ?1
             ORDER BY created_at DESC LIMIT 20",
        )?;
        let recent_transactions: Vec<LoyaltyTransaction> = stmt
            .query_map(params![account.id], |row| {
                Ok(LoyaltyTransaction {
                    id: row.get("id")?,
                    account_id: row.get("account_id")?,
                    sale_id: row.get("sale_id")?,
                    points: row.get("points")?,
                    txn_type: row.get("txn_type")?,
                    description: row.get("description")?,
                    created_at: row.get("created_at")?,
                })
            })?
            .map(|r| Ok(r?))
            .collect::<Result<Vec<_>, CoreError>>()?;

        Ok(Some(LoyaltyAccountWithDetails {
            account,
            tier,
            recent_transactions,
            next_tier,
            points_to_next_tier,
        }))
    }

    /// List all loyalty accounts with details for management.
    pub fn list_loyalty_accounts(&self) -> Result<Vec<LoyaltyAccountWithDetails>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, customer_id, points, lifetime_points, tier_id, updated_at, created_at
             FROM loyalty_accounts ORDER BY lifetime_points DESC",
        )?;
        let accounts: Vec<LoyaltyAccount> = stmt
            .query_map([], |row| {
                Ok(LoyaltyAccount {
                    id: row.get("id")?,
                    customer_id: row.get("customer_id")?,
                    points: row.get("points")?,
                    lifetime_points: row.get("lifetime_points")?,
                    tier_id: row.get("tier_id")?,
                    updated_at: row.get("updated_at")?,
                    created_at: row.get("created_at")?,
                })
            })?
            .map(|r| Ok(r?))
            .collect::<Result<Vec<_>, CoreError>>()?;

        let tiers = self.list_tiers()?;
        let mut all_tiers = tiers;
        all_tiers.sort_by_key(|t| t.min_points);

        let mut results = Vec::new();
        for account in accounts {
            let tier = account
                .tier_id
                .as_ref()
                .and_then(|tid| all_tiers.iter().find(|t| t.id == *tid))
                .cloned();

            let next_tier = all_tiers
                .iter()
                .filter(|t| t.min_points > account.lifetime_points)
                .min_by_key(|t| t.min_points)
                .cloned();

            let points_to_next_tier = next_tier
                .as_ref()
                .map(|t| t.min_points - account.lifetime_points)
                .unwrap_or(0);

            let mut txn_stmt = self.conn.prepare(
                "SELECT id, account_id, sale_id, points, txn_type, description, created_at
                 FROM loyalty_transactions WHERE account_id = ?1
                 ORDER BY created_at DESC LIMIT 5",
            )?;
            let recent_transactions: Vec<LoyaltyTransaction> = txn_stmt
                .query_map(params![account.id], |row| {
                    Ok(LoyaltyTransaction {
                        id: row.get("id")?,
                        account_id: row.get("account_id")?,
                        sale_id: row.get("sale_id")?,
                        points: row.get("points")?,
                        txn_type: row.get("txn_type")?,
                        description: row.get("description")?,
                        created_at: row.get("created_at")?,
                    })
                })?
                .map(|r| Ok(r?))
                .collect::<Result<Vec<_>, CoreError>>()?;

            results.push(LoyaltyAccountWithDetails {
                account,
                tier,
                recent_transactions,
                next_tier,
                points_to_next_tier,
            });
        }

        Ok(results)
    }

    /// Earn points for a purchase.
    /// points_earned = (total_minor * tier.points_per_unit / 100) * tier.earn_multiplier
    pub fn earn_points(
        &self,
        customer_id: &str,
        sale_id: &str,
        total_minor: i64,
    ) -> Result<LoyaltyTransaction, CoreError> {
        let account = self.get_or_create_loyalty_account(customer_id)?;

        // Get tier multiplier.
        let tier = account
            .tier_id
            .as_ref()
            .and_then(|tid| self.get_loyalty_tier(tid).ok()?)
            .unwrap_or(LoyaltyTier {
                id: "tier-bronze".into(),
                name: "Bronze".into(),
                min_points: 0,
                points_per_unit: 10,
                earn_multiplier: 1.0,
                colour: "#cd7f32".into(),
                sort_order: 1,
                created_at: String::new(),
            });

        // Multiply first (still in i64) then convert to f64 for the /100 division
        // to preserve fractional cents. Integer division truncates, which would
        // cause precision loss for sub-dollar amounts.
        let base = total_minor.saturating_mul(tier.points_per_unit);
        let points = ((base as f64) / 100.0 * tier.earn_multiplier).round() as i64;

        if points <= 0 {
            return Err(CoreError::Validation {
                field: "total_minor",
                message: "purchase total too small to earn points".into(),
            });
        }

        let txn_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tx = self.conn.unchecked_transaction()?;

        // Insert transaction.
        tx.execute(
            "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
             VALUES (?1, ?2, ?3, ?4, 'earn', ?5, ?6)",
            params![
                txn_id,
                account.id,
                sale_id,
                points,
                format!("Earned {} points from purchase", points),
                now,
            ],
        )?;

        // Update account.
        tx.execute(
            "UPDATE loyalty_accounts SET points = points + ?1, lifetime_points = lifetime_points + ?1,
             tier_id = (SELECT id FROM loyalty_tiers WHERE min_points <= lifetime_points + ?1
                        ORDER BY min_points DESC LIMIT 1),
             updated_at = ?2 WHERE id = ?3",
            params![points, now, account.id],
        )?;

        tx.commit()?;

        Ok(LoyaltyTransaction {
            id: txn_id,
            account_id: account.id,
            sale_id: Some(sale_id.to_owned()),
            points,
            txn_type: "earn".into(),
            description: format!("Earned {} points from purchase", points),
            created_at: now,
        })
    }

    /// Redeem points at checkout.
    /// Returns the transaction and the monetary value of redeemed points.
    /// Conversion: 100 points = 100 minor units ($1.00).
    pub fn redeem_points(
        &self,
        customer_id: &str,
        points: i64,
        sale_id: &str,
    ) -> Result<(LoyaltyTransaction, i64), CoreError> {
        let account = match self.get_loyalty_account_raw(customer_id)? {
            Some(a) => a,
            None => {
                return Err(CoreError::NotFound {
                    entity: "loyalty_account",
                    id: customer_id.to_owned(),
                });
            }
        };

        if points <= 0 {
            return Err(CoreError::Validation {
                field: "points",
                message: "points must be positive".into(),
            });
        }

        if account.points < points {
            return Err(CoreError::Validation {
                field: "points",
                message: format!(
                    "insufficient points: have {}, requested {}",
                    account.points, points
                ),
            });
        }

        let discount_minor = points * POINTS_TO_MINOR_RATIO;

        let txn_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO loyalty_transactions (id, account_id, sale_id, points, txn_type, description, created_at)
             VALUES (?1, ?2, ?3, ?4, 'redeem', ?5, ?6)",
            params![
                txn_id,
                account.id,
                sale_id,
                -points,
                format!("Redeemed {} points for {} discount", points, discount_minor),
                now,
            ],
        )?;

        tx.execute(
            "UPDATE loyalty_accounts SET points = points - ?1, updated_at = ?2 WHERE id = ?3",
            params![points, now, account.id],
        )?;

        tx.commit()?;

        Ok((
            LoyaltyTransaction {
                id: txn_id,
                account_id: account.id,
                sale_id: Some(sale_id.to_owned()),
                points: -points,
                txn_type: "redeem".into(),
                description: format!("Redeemed {} points for {} discount", points, discount_minor),
                created_at: now,
            },
            discount_minor,
        ))
    }

    /// List all loyalty tiers.
    pub fn list_tiers(&self) -> Result<Vec<LoyaltyTier>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, min_points, points_per_unit, earn_multiplier, colour, sort_order, created_at
             FROM loyalty_tiers ORDER BY sort_order",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LoyaltyTier {
                id: row.get("id")?,
                name: row.get("name")?,
                min_points: row.get("min_points")?,
                points_per_unit: row.get("points_per_unit")?,
                earn_multiplier: row.get("earn_multiplier")?,
                colour: row.get("colour")?,
                sort_order: row.get("sort_order")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    fn get_loyalty_tier(&self, id: &str) -> Result<Option<LoyaltyTier>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, min_points, points_per_unit, earn_multiplier, colour, sort_order, created_at
             FROM loyalty_tiers WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(LoyaltyTier {
                id: row.get("id")?,
                name: row.get("name")?,
                min_points: row.get("min_points")?,
                points_per_unit: row.get("points_per_unit")?,
                earn_multiplier: row.get("earn_multiplier")?,
                colour: row.get("colour")?,
                sort_order: row.get("sort_order")?,
                created_at: row.get("created_at")?,
            })
        });
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update a loyalty tier.
    pub fn update_tier(
        &self,
        id: &str,
        name: &str,
        min_points: i64,
        points_per_unit: i64,
        earn_multiplier: f64,
        colour: &str,
    ) -> Result<LoyaltyTier, CoreError> {
        let rows = self.conn.execute(
            "UPDATE loyalty_tiers SET name = ?1, min_points = ?2, points_per_unit = ?3,
             earn_multiplier = ?4, colour = ?5 WHERE id = ?6",
            params![
                name,
                min_points,
                points_per_unit,
                earn_multiplier,
                colour,
                id
            ],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "loyalty_tier",
                id: id.to_owned(),
            });
        }

        self.get_loyalty_tier(id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "loyalty_tier",
                id: id.to_owned(),
            })
    }

    /// Convert points to monetary value (minor units).
    pub fn get_points_value(&self, points: i64) -> i64 {
        points * POINTS_TO_MINOR_RATIO
    }
}

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
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at, subtotal_minor, tax_total_minor)
             VALUES (?1, 0, 'USD', 0, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 0, 0)",
            params![id],
        )
        .unwrap();
    }

    #[test]
    fn get_or_create_creates_account() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        let account = store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();
        assert_eq!(account.customer_id, "cust-1");
        assert_eq!(account.points, 0);
        assert_eq!(account.lifetime_points, 0);
        assert_eq!(account.tier_id.as_deref(), Some("tier-bronze"));
    }

    #[test]
    fn get_or_create_returns_existing() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        let a1 = store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();
        let a2 = store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();
        assert_eq!(a1.id, a2.id);
    }

    #[test]
    fn earn_points_creates_transaction() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        let txn = store(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();
        assert_eq!(txn.txn_type, "earn");
        assert_eq!(txn.points, 100); // 1000 * 10 / 100 * 1.0

        let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
        assert_eq!(details.account.points, 100);
        assert_eq!(details.account.lifetime_points, 100);
    }

    #[test]
    fn earn_points_with_silver_multiplier() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        // Create account first, then bump to Silver (1.25x multiplier).
        store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();
        conn.execute(
            "UPDATE loyalty_accounts SET tier_id = 'tier-silver' WHERE customer_id = 'cust-1'",
            [],
        )
        .unwrap();

        let txn = store(&conn).earn_points("cust-1", "sale-1", 1000).unwrap();
        // 1000 * 10 / 100 * 1.25 = 125
        assert_eq!(txn.points, 125);
    }

    #[test]
    fn redeem_points_deducts_and_returns_value() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");
        store(&conn).earn_points("cust-1", "sale-1", 5000).unwrap();

        let (txn, discount) = store(&conn).redeem_points("cust-1", 200, "sale-2").unwrap();
        assert_eq!(txn.points, -200);
        assert_eq!(discount, 200); // 200 points = 200 minor units

        let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
        // 5000 * 10 / 100 * 1.0 = 500 earned - 200 redeemed = 300
        assert_eq!(details.account.points, 300);
    }

    #[test]
    fn redeem_points_insufficient() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();
        let err = store(&conn)
            .redeem_points("cust-1", 100, "sale-1")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "points"));
    }

    #[test]
    fn list_tiers_returns_seeded() {
        let conn = fresh();
        let tiers = store(&conn).list_tiers().unwrap();
        assert_eq!(tiers.len(), 4);
        assert_eq!(tiers[0].name, "Bronze");
        assert_eq!(tiers[1].name, "Silver");
        assert_eq!(tiers[2].name, "Gold");
        assert_eq!(tiers[3].name, "Platinum");
    }

    #[test]
    fn update_tier_modifies_fields() {
        let conn = fresh();
        let updated = store(&conn)
            .update_tier("tier-bronze", "Bronze Updated", 0, 15, 1.5, "#ff0000")
            .unwrap();
        assert_eq!(updated.name, "Bronze Updated");
        assert_eq!(updated.points_per_unit, 15);
        assert_eq!(updated.earn_multiplier, 1.5);
    }

    #[test]
    fn get_points_value_converts_correctly() {
        let conn = fresh();
        assert_eq!(store(&conn).get_points_value(100), 100);
        assert_eq!(store(&conn).get_points_value(50), 50);
        assert_eq!(store(&conn).get_points_value(0), 0);
    }

    #[test]
    fn customer_not_found_returns_error() {
        let conn = fresh();
        let err = store(&conn)
            .get_or_create_loyalty_account("nonexistent")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "customer"));
    }

    // ── Additional edge-case tests ─────────────────────────────────

    #[test]
    fn list_loyalty_accounts_ordered_by_points() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_customer(&conn, "cust-2", "Bob");
        seed_customer(&conn, "cust-3", "Charlie");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");

        let s = store(&conn);
        // Create accounts
        s.get_or_create_loyalty_account("cust-1").unwrap();
        s.get_or_create_loyalty_account("cust-2").unwrap();
        s.get_or_create_loyalty_account("cust-3").unwrap();

        // Earn different point amounts
        s.earn_points("cust-1", "sale-1", 5000).unwrap(); // 500 points
        s.earn_points("cust-2", "sale-2", 10000).unwrap(); // 1000 points

        let accounts = s.list_loyalty_accounts().unwrap();
        assert_eq!(accounts.len(), 3);
        // ORDER BY lifetime_points DESC: cust-2 (1000) first, cust-1 (500) second, cust-3 (0) third
        assert_eq!(accounts[0].account.customer_id, "cust-2");
        assert_eq!(accounts[1].account.customer_id, "cust-1");
        assert_eq!(accounts[2].account.customer_id, "cust-3");
    }

    #[test]
    fn list_loyalty_accounts_empty() {
        let conn = fresh();
        // Seed a customer but don't create any loyalty accounts
        seed_customer(&conn, "cust-1", "Alice");
        let accounts = store(&conn).list_loyalty_accounts().unwrap();
        assert!(accounts.is_empty());
    }

    #[test]
    fn earn_points_validation_zero_total() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();

        // total_minor = 0 → 0 points → Validation error
        let err = store(&conn).earn_points("cust-1", "sale-1", 0).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "total_minor"));
    }

    #[test]
    fn earn_points_small_total_rounds_to_one() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();

        // total_minor = 9 → base = 90 → 90.0 / 100.0 * 1.0 = 0.9 → rounds to 1
        // With the fix for integer truncation, fractional cents are preserved
        let txn = store(&conn).earn_points("cust-1", "sale-1", 9).unwrap();
        assert_eq!(
            txn.points, 1,
            "9 cents with 10 pts/unit should round 0.9 → 1"
        );
    }

    #[test]
    fn earn_points_tiny_total_rounds_to_zero() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();

        // total_minor = 4 → base = 40 → 40.0 / 100.0 * 1.0 = 0.4 → rounds to 0 → err
        let err = store(&conn).earn_points("cust-1", "sale-1", 4).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "total_minor"));
    }

    #[test]
    fn earn_points_no_integer_truncation_for_sub_dollar_amounts() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        store(&conn)
            .get_or_create_loyalty_account("cust-1")
            .unwrap();

        // total_minor = 155 ($1.55), points_per_unit = 10, earn_multiplier = 1.0
        // Correct math: 155 * 10 / 100 = 15.5 → round → 16
        // Integer-division bug: 155 * 10 / 100 = 15 (truncated) → 15
        let txn = store(&conn).earn_points("cust-1", "sale-1", 155).unwrap();
        assert_eq!(
            txn.points, 16,
            "$1.55 with 10 pts/unit should earn 16 pts (rounded from 15.5), \
             not {} from integer truncation",
            txn.points
        );

        // Verify account balance matches
        let details = store(&conn).get_loyalty_account("cust-1").unwrap().unwrap();
        assert_eq!(details.account.points, 16);
        assert_eq!(details.account.lifetime_points, 16);
    }

    #[test]
    fn earn_points_multiple_sub_dollar_amounts_stack_correctly() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");
        seed_sale(&conn, "sale-3");
        let s = store(&conn);
        s.get_or_create_loyalty_account("cust-1").unwrap();

        // Three sub-dollar purchases that should round correctly
        // $1.55 → 16 pts, $2.49 → 25 pts, $0.99 → 10 pts
        s.earn_points("cust-1", "sale-1", 155).unwrap();
        s.earn_points("cust-1", "sale-2", 249).unwrap();
        s.earn_points("cust-1", "sale-3", 99).unwrap();

        // Expected: 16 + 25 + 10 = 51 (with correct float division)
        // Bug: 15 + 24 + 9 = 48 (with integer truncation before f64 cast)
        let details = s.get_loyalty_account("cust-1").unwrap().unwrap();
        assert_eq!(
            details.account.points, 51,
            "accumulated points from 155+249+99 should be 51, got {}",
            details.account.points
        );
        assert_eq!(
            details.account.lifetime_points, 51,
            "lifetime_points should also be 51, got {}",
            details.account.lifetime_points
        );
    }

    #[test]
    fn redeem_points_zero_returns_validation_error() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");
        store(&conn).earn_points("cust-1", "sale-1", 5000).unwrap();

        let err = store(&conn)
            .redeem_points("cust-1", 0, "sale-2")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "points"));
    }

    #[test]
    fn redeem_points_negative_returns_validation_error() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");
        store(&conn).earn_points("cust-1", "sale-1", 5000).unwrap();

        let err = store(&conn)
            .redeem_points("cust-1", -50, "sale-2")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "points"));
    }

    #[test]
    fn redeem_points_no_account_returns_not_found() {
        let conn = fresh();
        seed_sale(&conn, "sale-1");
        let err = store(&conn)
            .redeem_points("nonexistent", 100, "sale-1")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "loyalty_account"));
    }

    #[test]
    fn update_tier_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_tier("nonexistent", "No Tier", 0, 10, 1.0, "#000")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "loyalty_tier"));
    }

    #[test]
    fn earn_points_updates_tier_automatically() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");

        let s = store(&conn);
        s.get_or_create_loyalty_account("cust-1").unwrap();

        // Earn enough points to reach Silver tier (min_points = 200 for Silver)
        // 2000 * 10 / 100 * 1.0 = 200 points → should auto-upgrade to Silver
        s.earn_points("cust-1", "sale-1", 2000).unwrap();

        let details = s.get_loyalty_account("cust-1").unwrap().unwrap();
        assert_eq!(details.account.tier_id.as_deref(), Some("tier-silver"));
        assert!(details.tier.is_some());
        assert_eq!(details.tier.as_ref().unwrap().name, "Silver");
    }

    #[test]
    fn earn_points_multiple_stacked() {
        let conn = fresh();
        seed_customer(&conn, "cust-1", "Alice");
        seed_sale(&conn, "sale-1");
        seed_sale(&conn, "sale-2");
        seed_sale(&conn, "sale-3");

        let s = store(&conn);
        s.get_or_create_loyalty_account("cust-1").unwrap();

        // sale-1: Bronze 1.0x → 1000*10/100=100 points. Upgrades to Silver.
        // sale-2: Silver 1.25x → 2000*10/100*1.25=250 points. Total: 350.
        // sale-3: Silver 1.25x → 3000*10/100*1.25=375 points. Total: 725.
        s.earn_points("cust-1", "sale-1", 1000).unwrap();
        s.earn_points("cust-1", "sale-2", 2000).unwrap();
        s.earn_points("cust-1", "sale-3", 3000).unwrap();

        let details = s.get_loyalty_account("cust-1").unwrap().unwrap();
        assert_eq!(details.account.points, 725);
        assert_eq!(details.account.lifetime_points, 725);
        // Should have 3 recent transactions
        assert_eq!(details.recent_transactions.len(), 3);
    }
}
