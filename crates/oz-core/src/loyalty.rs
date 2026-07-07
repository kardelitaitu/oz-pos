//! Loyalty program domain types — tiers, accounts, transactions.
//!
//! The loyalty system tracks customer points, tier-based earning
//! multipliers, and point redemption at checkout. Each customer
//! gets one [`LoyaltyAccount`] tied to their [`crate::Customer`]
//! record. Points are earned on completed sales and can be redeemed
//! for a monetary discount (100 points = 100 minor units = $1.00).

use serde::{Deserialize, Serialize};

/// A loyalty tier defining the earning rate and multiplier for a group of customers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyTier {
    /// Unique identifier for the tier.
    pub id: String,
    /// Display name (e.g. "Bronze", "Silver").
    pub name: String,
    /// Minimum lifetime points required to reach this tier.
    pub min_points: i64,
    /// Base points earned per minor unit of spend.
    pub points_per_unit: i64,
    /// Multiplier applied on top of base earnings (e.g. 1.5 = 1.5x).
    pub earn_multiplier: f64,
    /// Hex colour for UI badge.
    pub colour: String,
    /// Display ordering (lower = higher priority).
    pub sort_order: i64,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// A customer's loyalty account — points balance and current tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyAccount {
    /// Internal row id (UUID v4).
    pub id: String,
    /// FK to `customers.id`.
    pub customer_id: String,
    /// Current redeemable points balance.
    pub points: i64,
    /// Total points earned over the lifetime of the account.
    pub lifetime_points: i64,
    /// FK to `loyalty_tiers.id` (current tier).
    pub tier_id: Option<String>,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// A single points transaction — earn, redeem, adjust, or expire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyTransaction {
    /// Internal row id (UUID v4).
    pub id: String,
    /// FK to `loyalty_accounts.id`.
    pub account_id: String,
    /// FK to `sales.id`, when tied to a sale.
    pub sale_id: Option<String>,
    /// Points delta (positive for earn, negative for redeem).
    pub points: i64,
    /// Transaction type: "earn", "redeem", "adjust", "expire".
    pub txn_type: String,
    /// Human-readable description of the transaction.
    pub description: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Account with tier info and recent transactions (returned by API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyAccountWithDetails {
    /// The underlying loyalty account.
    pub account: LoyaltyAccount,
    /// Current tier details (if any tier is assigned).
    pub tier: Option<LoyaltyTier>,
    /// Most recent 5–20 transactions.
    pub recent_transactions: Vec<LoyaltyTransaction>,
    /// The next tier above the current one (if any).
    pub next_tier: Option<LoyaltyTier>,
    /// Points required to reach the next tier.
    pub points_to_next_tier: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tier() -> LoyaltyTier {
        LoyaltyTier {
            id: "tier-bronze".into(),
            name: "Bronze".into(),
            min_points: 0,
            points_per_unit: 10,
            earn_multiplier: 1.0,
            colour: "#cd7f32".into(),
            sort_order: 1,
            created_at: "2025-01-01T00:00:00.000Z".into(),
        }
    }

    fn sample_account() -> LoyaltyAccount {
        LoyaltyAccount {
            id: "acct-1".into(),
            customer_id: "cust-1".into(),
            points: 150,
            lifetime_points: 500,
            tier_id: Some("tier-bronze".into()),
            updated_at: "2025-01-02T00:00:00.000Z".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        }
    }

    fn sample_txn() -> LoyaltyTransaction {
        LoyaltyTransaction {
            id: "txn-1".into(),
            account_id: "acct-1".into(),
            sale_id: Some("sale-1".into()),
            points: 100,
            txn_type: "earn".into(),
            description: "Earned from purchase".into(),
            created_at: "2025-01-02T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn loyalty_tier_serde_roundtrip() {
        let tier = sample_tier();
        let json = serde_json::to_string(&tier).unwrap();
        let back: LoyaltyTier = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, tier.id);
        assert_eq!(back.name, tier.name);
        assert_eq!(back.points_per_unit, tier.points_per_unit);
        assert_eq!(back.earn_multiplier, tier.earn_multiplier);
        assert_eq!(back.colour, tier.colour);
    }

    #[test]
    fn loyalty_account_serde_roundtrip() {
        let acct = sample_account();
        let json = serde_json::to_string(&acct).unwrap();
        let back: LoyaltyAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, acct.id);
        assert_eq!(back.customer_id, acct.customer_id);
        assert_eq!(back.points, acct.points);
        assert_eq!(back.lifetime_points, acct.lifetime_points);
        assert_eq!(back.tier_id, acct.tier_id);
    }

    #[test]
    fn loyalty_transaction_serde_roundtrip() {
        let txn = sample_txn();
        let json = serde_json::to_string(&txn).unwrap();
        let back: LoyaltyTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, txn.id);
        assert_eq!(back.account_id, txn.account_id);
        assert_eq!(back.sale_id, txn.sale_id);
        assert_eq!(back.txn_type, txn.txn_type);
    }

    #[test]
    fn loyalty_account_with_details_serde_roundtrip() {
        let details = LoyaltyAccountWithDetails {
            account: sample_account(),
            tier: Some(sample_tier()),
            recent_transactions: vec![sample_txn()],
            next_tier: Some(sample_tier()),
            points_to_next_tier: 500,
        };
        let json = serde_json::to_string(&details).unwrap();
        let back: LoyaltyAccountWithDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(back.account.id, details.account.id);
        assert_eq!(back.tier.as_ref().unwrap().name, "Bronze");
        assert_eq!(back.recent_transactions.len(), 1);
        assert_eq!(back.points_to_next_tier, 500);
    }

    #[test]
    fn loyalty_transaction_redeem_negative_points() {
        let txn = LoyaltyTransaction {
            id: "txn-2".into(),
            account_id: "acct-1".into(),
            sale_id: None,
            points: -50,
            txn_type: "redeem".into(),
            description: "Redeemed points".into(),
            created_at: "2025-01-02T00:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&txn).unwrap();
        let back: LoyaltyTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.points, -50);
        assert_eq!(back.txn_type, "redeem");
    }

    #[test]
    fn loyalty_account_without_tier() {
        let acct = LoyaltyAccount {
            tier_id: None,
            ..sample_account()
        };
        assert!(acct.tier_id.is_none());
        let json = serde_json::to_string(&acct).unwrap();
        let back: LoyaltyAccount = serde_json::from_str(&json).unwrap();
        assert!(back.tier_id.is_none());
    }
}
