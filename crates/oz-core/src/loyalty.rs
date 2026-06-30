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
