//! Subscription tier definitions, signature verification, quota
//! enforcement, clock rollback detection, and offline grace period
//! for ADR #5 (Subscription Tier & Entitlement Architecture).
//!
//! The `tenant_subscription` table lives in the global database. This
//! module provides the Rust types and logic for reading that table,
//! verifying its cryptographic signature, and enforcing tier limits
//! when creating stores and workspace instances.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Maximum clock skew tolerance before detecting tampering (30 seconds).
///
/// Tightened from the previous 5-minute window (M1 audit finding) to
/// catch clock-rollback bypass attempts sooner. 30s is the smallest
/// value that still absorbs typical RTC drift on consumer hardware
/// without producing false positives on slow or paused devices.
const CLOCK_SKEW_TOLERANCE_SECONDS: i64 = 30;

/// Offline grace period for paid tiers (14 days). After this period
/// without a successful cloud sync, the tier reverts to Free quotas.
const OFFLINE_GRACE_DAYS: i64 = 14;

// ── Instance Status ─────────────────────────────────────────────────

/// Three-state status for workspace instances, replacing the old
/// `is_active` boolean (ADR #4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    /// Normal operating register — fully functional.
    Active,
    /// Suspended automatically by subscription downgrade or offline
    /// grace expiration. Historical data preserved; register cannot
    /// accept new sales until restored.
    QuotaSuspended,
    /// Manually deleted/deactivated by an admin.
    Archived,
}

impl InstanceStatus {
    /// Parse from the database TEXT column.
    pub fn from_db(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "quota_suspended" => Self::QuotaSuspended,
            "archived" => Self::Archived,
            _ => Self::Active, // Default for unknown values
        }
    }

    /// Return the database representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::QuotaSuspended => "quota_suspended",
            Self::Archived => "archived",
        }
    }
}

// ── Subscription Tier ────────────────────────────────────────────────

/// Subscription tiers with their quotas, capabilities, and feature entitlements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionTier {
    /// 90-day Free Trial — 1 store, 1 register, 1 warehouse, offline-only.
    Free,
    /// 1-Time Perpetual License — 1 store, 1 register, 1 warehouse, offline-first.
    OneTime,
    /// Standard SaaS — 1 store, 2 registers, 1 warehouse, QRIS, basic cloud sync.
    Standard,
    /// Pro SaaS — Unlimited stores, unlimited registers, unlimited warehouses, Lua engine, Stripe + QRIS.
    Pro,
    /// Legacy alias for Pro tier.
    Premium,
    /// Enterprise — Unlimited stores/registers/warehouses, regional zones, custom ERP adaptors.
    Enterprise,
}

impl SubscriptionTier {
    /// Parse from the database TEXT column.
    pub fn from_db(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "free" | "trial" => Self::Free,
            "one_time" | "perpetual" | "one-time" | "onetime" => Self::OneTime,
            "standard" => Self::Standard,
            "pro" => Self::Pro,
            "premium" => Self::Premium,
            "enterprise" => Self::Enterprise,
            _ => Self::Free,
        }
    }

    /// Human-readable tier name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Free => "Free Trial",
            Self::OneTime => "1-Time Perpetual",
            Self::Standard => "Standard",
            Self::Pro => "Pro",
            Self::Premium => "Premium (Pro)",
            Self::Enterprise => "Enterprise",
        }
    }

    /// Maximum number of stores allowed for this tier.
    /// Returns `None` for unlimited (Pro / Enterprise).
    pub fn max_stores(&self) -> Option<i64> {
        match self {
            Self::Free | Self::OneTime | Self::Standard => Some(1),
            Self::Pro | Self::Premium | Self::Enterprise => None,
        }
    }

    /// Maximum POS register instances per store for this tier.
    /// Returns `None` for unlimited (Pro / Enterprise).
    pub fn max_pos_instances(&self) -> Option<i64> {
        match self {
            Self::Free | Self::OneTime => Some(1),
            Self::Standard => Some(2),
            Self::Pro | Self::Premium | Self::Enterprise => None,
        }
    }

    /// Maximum inventory warehouse storage locations allowed for this tier.
    /// Returns `None` for unlimited (Pro / Enterprise).
    pub fn max_warehouses(&self) -> Option<i64> {
        match self {
            Self::Free | Self::OneTime | Self::Standard => Some(1),
            Self::Pro | Self::Premium | Self::Enterprise => None,
        }
    }

    /// Whether this tier supports PostgreSQL background cloud database sync.
    pub fn supports_cloud_sync(&self) -> bool {
        match self {
            Self::Free | Self::OneTime => false,
            Self::Standard | Self::Pro | Self::Premium | Self::Enterprise => true,
        }
    }

    /// Whether this tier supports dynamic QRIS payment processing (Midtrans).
    pub fn supports_qris(&self) -> bool {
        match self {
            Self::Free | Self::OneTime => false,
            Self::Standard | Self::Pro | Self::Premium | Self::Enterprise => true,
        }
    }

    /// Whether this tier supports Stripe credit/debit card processing.
    pub fn supports_stripe(&self) -> bool {
        match self {
            Self::Free | Self::OneTime | Self::Standard => false,
            Self::Pro | Self::Premium | Self::Enterprise => true,
        }
    }

    /// Whether this tier supports embedded Lua VM rule engine for custom promos.
    pub fn supports_lua_engine(&self) -> bool {
        match self {
            Self::Free | Self::OneTime | Self::Standard => false,
            Self::Pro | Self::Premium | Self::Enterprise => true,
        }
    }

    /// Whether this tier supports multi-warehouse stock deduction fallback wires in Node Topology.
    pub fn supports_multi_warehouse_fallback(&self) -> bool {
        match self {
            Self::Free | Self::OneTime | Self::Standard => false,
            Self::Pro | Self::Premium | Self::Enterprise => true,
        }
    }

    /// Whether this tier supports regional zone containers in Node Topology.
    pub fn supports_regional_zones(&self) -> bool {
        matches!(self, Self::Enterprise)
    }

    /// Check whether this tier allows the given workspace type.
    pub fn allows_workspace_type(&self, type_key: &str) -> bool {
        match self {
            Self::Free | Self::OneTime => {
                matches!(type_key, "store-pos" | "restaurant-pos" | "admin")
            }
            Self::Standard => matches!(
                type_key,
                "restaurant-pos" | "store-pos" | "warehouse" | "admin" | "kds"
            ),
            Self::Pro | Self::Premium | Self::Enterprise => true, // All workspace types + custom plugins
        }
    }
}

// ── Subscription Row ──────────────────────────────────────────────────

/// A row from the `tenant_subscription` table.
#[derive(Debug, Clone)]
pub struct TenantSubscription {
    /// The unique identifier of the tenant.
    pub tenant_id: String,
    /// The subscription tier (Free, Pro, Premium, Enterprise).
    pub tier: SubscriptionTier,
    /// The subscription status (e.g. "active", "canceled").
    pub status: String,
    /// The optional expiration timestamp in RFC 3339 format.
    pub expires_at: Option<String>,
    /// The maximum number of stores allowed for this tenant.
    pub max_stores: i64,
    /// The maximum number of POS instances allowed for this tenant.
    pub max_pos_instances: i64,
    /// A JSON string listing the workspace types allowed on this tier.
    pub allowed_types_json: String,
    /// The cryptographic signature verifying the subscription.
    pub signature: String,
    /// The signed subscription payload from the license server (JSON).
    pub signed_payload: String,
    /// The API key for subsequent renew/status calls.
    pub api_key: String,
    /// The timestamp of the last update in RFC 3339 format.
    pub updated_at: String,
}

impl TenantSubscription {
    /// Load the subscription for a tenant from the global database.
    pub fn load(conn: &rusqlite::Connection, tenant_id: &str) -> Result<Option<Self>, CoreError> {
        let mut stmt = conn.prepare(
            "SELECT tenant_id, tier_key, status, expires_at, max_stores,
                    max_pos_instances, allowed_types_json, signature, signed_payload,
                    api_key, updated_at
             FROM tenant_subscription
             WHERE tenant_id = ?1",
        )?;

        let result = stmt.query_row(params![tenant_id], |row| {
            Ok(TenantSubscription {
                tenant_id: row.get(0)?,
                tier: SubscriptionTier::from_db(&row.get::<_, String>(1)?),
                status: row.get(2)?,
                expires_at: row.get(3)?,
                max_stores: row.get(4)?,
                max_pos_instances: row.get(5)?,
                allowed_types_json: row.get(6)?,
                signature: row.get(7)?,
                signed_payload: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                api_key: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                updated_at: row.get(10)?,
            })
        });

        match result {
            Ok(sub) => Ok(Some(sub)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::from(e)),
        }
    }

    /// Verify the subscription signature using RSA-2048 PKCS1v15.
    ///
    /// During local development / single-store deployments, the bootstrap
    /// signature `BOOTSTRAP_FREE` is accepted. In production, the signature
    /// must be validated against the embedded RSA public key.
    pub fn verify_signature(&self) -> Result<(), CoreError> {
        crate::license_verification::verify_license_signature(&self.signed_payload, &self.signature)
    }

    /// Compute the maximum ledger timestamp across all domain tables
    /// in the given database connection.
    ///
    /// Queries `MAX(created_at)` from the `sales` and `audit_log` tables.
    /// The effective time is the maximum of these values (or `Utc::now()`
    /// if all tables are empty). This prevents users from rolling back
    /// their OS clock to bypass subscription expiration.
    ///
    /// In multi-store mode (Phase 2), this would iterate all store
    /// databases and return the global maximum.
    pub fn compute_max_ledger_timestamp(conn: &rusqlite::Connection) -> Result<String, CoreError> {
        // Get the most recent timestamp from sales.
        let max_sales: Option<String> = conn
            .query_row("SELECT MAX(created_at) FROM sales", [], |row| row.get(0))
            .unwrap_or(None);

        // Get the most recent timestamp from audit_log.
        let max_audit: Option<String> = conn
            .query_row("SELECT MAX(created_at) FROM audit_log", [], |row| {
                row.get(0)
            })
            .unwrap_or(None);

        // Pick the maximum of the two ledger timestamps.
        let ledger_max = match (max_sales, max_audit) {
            (Some(a), Some(b)) => {
                if a > b {
                    a
                } else {
                    b
                }
            }
            (Some(v), None) | (None, Some(v)) => v,
            (None, None) => {
                // No ledger data — use current time.
                return Ok(chrono::Utc::now().to_rfc3339());
            }
        };

        Ok(ledger_max)
    }

    /// Validate that the system clock has not been rolled back.
    ///
    /// Compares the maximum ledger timestamp against `Utc::now()`.
    /// If the ledger has timestamps more than `CLOCK_SKEW_TOLERANCE`
    /// in the future relative to the wall clock, the system detects
    /// clock tampering and returns `CoreError::SystemClockTampered`.
    pub fn validate_clock_rollback(conn: &rusqlite::Connection) -> Result<(), CoreError> {
        let ledger_ts = Self::compute_max_ledger_timestamp(conn)?;
        let ledger_dt = chrono::DateTime::parse_from_rfc3339(&ledger_ts).map_err(|e| {
            CoreError::Internal(format!(
                "failed to parse ledger timestamp '{ledger_ts}': {e}"
            ))
        })?;
        let now_naive = chrono::Utc::now().naive_utc();
        let ledger_naive = ledger_dt.naive_utc();

        // If the ledger timestamp is further in the future than our
        // tolerance window, the clock has been rolled back.
        let delta = ledger_naive.signed_duration_since(now_naive).num_seconds();

        if delta > CLOCK_SKEW_TOLERANCE_SECONDS {
            return Err(CoreError::SystemClockTampered(format!(
                "Ledger timestamp {ledger_ts} is {delta}s ahead of system clock. \
                 Clock rollback detected — register locked until online cloud sync."
            )));
        }

        Ok(())
    }

    /// Check if the subscription is within the offline grace period.
    ///
    /// Free tier has no grace period (always "within grace" since it's free).
    /// Paid tiers (Pro, Premium, Enterprise) get 14 days offline before
    /// quotas revert to Free.
    ///
    /// A canceled subscription is never within grace.
    ///
    /// Returns `true` if the subscription is still valid (not expired or
    /// within grace period).
    pub fn is_within_grace_period(&self) -> bool {
        // Canceled subscriptions are never within grace.
        if self.status == "canceled" {
            return false;
        }

        // Free tier — always within grace.
        if self.tier == SubscriptionTier::Free {
            return true;
        }

        // No expiry — lifetime/perpetual license.
        let expires_at = match &self.expires_at {
            Some(ts) => ts,
            None => return true,
        };

        let expiry = match chrono::DateTime::parse_from_rfc3339(expires_at) {
            Ok(dt) => dt,
            Err(_) => return false, // Unparseable expiry → assume expired
        };

        let now = chrono::Utc::now();
        let grace_deadline = expiry + chrono::Duration::days(OFFLINE_GRACE_DAYS);

        now <= grace_deadline
    }

    /// Determine the effective subscription tier after applying
    /// offline grace rules.
    ///
    /// - If the subscription has not expired (or is within the 14-day
    ///   grace period), returns the actual tier.
    /// - If the grace period has elapsed and the register is still
    ///   offline, returns `Free` (downgraded).
    pub fn effective_tier(&self) -> SubscriptionTier {
        if self.is_within_grace_period() {
            self.tier.clone()
        } else {
            tracing::warn!(
                tier = %self.tier.name(),
                expires_at = ?self.expires_at,
                "subscription grace period expired — reverting to Free tier"
            );
            SubscriptionTier::Free
        }
    }
}

// ── Quota Enforcement ─────────────────────────────────────────────────

/// Error type for quota-related failures, used by the subscription
/// module to provide actionable upgrade messaging.
#[derive(Debug)]
pub enum QuotaError {
    /// The tenant has reached their per-store register limit.
    RegisterLimit {
        /// The subscription tier name.
        tier: String,
        /// The maximum number allowed.
        limit: i64,
        /// The current usage count.
        current: i64,
    },
    /// The tenant has reached their store count limit.
    StoreLimit {
        /// The subscription tier name.
        tier: String,
        /// The maximum number allowed.
        limit: i64,
        /// The current usage count.
        current: i64,
    },
    /// The workspace type is not available on this tier.
    TypeNotAllowed {
        /// The subscription tier name.
        tier: String,
        /// The workspace type key that was rejected.
        type_key: String,
    },
}

impl std::fmt::Display for QuotaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegisterLimit {
                tier,
                limit,
                current,
            } => {
                write!(
                    f,
                    "Your {tier} tier allows maximum {limit} registers per store. \
                     This store already has {current}. Upgrade to add more."
                )
            }
            Self::StoreLimit {
                tier,
                limit,
                current,
            } => {
                write!(
                    f,
                    "Your {tier} tier allows maximum {limit} stores. \
                     You currently have {current}. Upgrade to add more."
                )
            }
            Self::TypeNotAllowed { tier, type_key } => {
                write!(
                    f,
                    "The '{type_key}' workspace type requires a higher tier. \
                     Your current tier is {tier}."
                )
            }
        }
    }
}

impl From<QuotaError> for CoreError {
    fn from(e: QuotaError) -> Self {
        CoreError::SubscriptionLimitExceeded(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── InstanceStatus ────────────────────────────────────

    #[test]
    fn instance_status_from_db() {
        assert_eq!(InstanceStatus::from_db("active"), InstanceStatus::Active);
        assert_eq!(
            InstanceStatus::from_db("quota_suspended"),
            InstanceStatus::QuotaSuspended
        );
        assert_eq!(
            InstanceStatus::from_db("archived"),
            InstanceStatus::Archived
        );
        assert_eq!(InstanceStatus::from_db("unknown"), InstanceStatus::Active);
    }

    #[test]
    fn instance_status_as_str() {
        assert_eq!(InstanceStatus::Active.as_str(), "active");
        assert_eq!(InstanceStatus::QuotaSuspended.as_str(), "quota_suspended");
        assert_eq!(InstanceStatus::Archived.as_str(), "archived");
    }

    #[test]
    fn instance_status_serialize() {
        let json = serde_json::to_value(InstanceStatus::Active).unwrap();
        assert_eq!(json, "active");

        let json = serde_json::to_value(InstanceStatus::QuotaSuspended).unwrap();
        assert_eq!(json, "quota_suspended");
    }

    // ── SubscriptionTier ──────────────────────────────────

    #[test]
    fn tier_from_db() {
        assert_eq!(SubscriptionTier::from_db("free"), SubscriptionTier::Free);
        assert_eq!(SubscriptionTier::from_db("pro"), SubscriptionTier::Pro);
        assert_eq!(
            SubscriptionTier::from_db("premium"),
            SubscriptionTier::Premium
        );
        assert_eq!(
            SubscriptionTier::from_db("enterprise"),
            SubscriptionTier::Enterprise
        );
        assert_eq!(SubscriptionTier::from_db("invalid"), SubscriptionTier::Free);
    }

    #[test]
    fn tier_max_stores() {
        assert_eq!(SubscriptionTier::Free.max_stores(), Some(1));
        assert_eq!(SubscriptionTier::OneTime.max_stores(), Some(1));
        assert_eq!(SubscriptionTier::Standard.max_stores(), Some(1));
        assert_eq!(SubscriptionTier::Pro.max_stores(), None);
        assert_eq!(SubscriptionTier::Premium.max_stores(), None);
        assert_eq!(SubscriptionTier::Enterprise.max_stores(), None);
    }

    #[test]
    fn tier_max_pos_instances() {
        assert_eq!(SubscriptionTier::Free.max_pos_instances(), Some(1));
        assert_eq!(SubscriptionTier::OneTime.max_pos_instances(), Some(1));
        assert_eq!(SubscriptionTier::Standard.max_pos_instances(), Some(2));
        assert_eq!(SubscriptionTier::Pro.max_pos_instances(), None);
        assert_eq!(SubscriptionTier::Premium.max_pos_instances(), None);
        assert_eq!(SubscriptionTier::Enterprise.max_pos_instances(), None);
    }

    #[test]
    fn tier_allows_workspace_type() {
        // Free tier & OneTime
        assert!(SubscriptionTier::Free.allows_workspace_type("store-pos"));
        assert!(SubscriptionTier::Free.allows_workspace_type("admin"));
        assert!(!SubscriptionTier::Free.allows_workspace_type("kds"));
        assert!(!SubscriptionTier::Free.allows_workspace_type("warehouse"));

        // Standard tier
        assert!(SubscriptionTier::Standard.allows_workspace_type("warehouse"));
        assert!(SubscriptionTier::Standard.allows_workspace_type("kds"));

        // Pro & Enterprise tier allow all
        assert!(SubscriptionTier::Pro.allows_workspace_type("kds"));
        assert!(SubscriptionTier::Pro.allows_workspace_type("analytics-pro"));
        assert!(SubscriptionTier::Pro.allows_workspace_type("warehouse"));
        assert!(SubscriptionTier::Enterprise.allows_workspace_type("anything"));
    }

    #[test]
    fn tier_name() {
        assert_eq!(SubscriptionTier::Free.name(), "Free Trial");
        assert_eq!(SubscriptionTier::OneTime.name(), "1-Time Perpetual");
        assert_eq!(SubscriptionTier::Standard.name(), "Standard");
        assert_eq!(SubscriptionTier::Pro.name(), "Pro");
        assert_eq!(SubscriptionTier::Enterprise.name(), "Enterprise");
    }

    #[test]
    fn tier_serialize() {
        let json = serde_json::to_value(SubscriptionTier::Free).unwrap();
        assert_eq!(json, "free");
    }

    // ── Signature Verification ────────────────────────────

    #[test]
    fn verify_bootstrap_signature_passes() {
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Free,
            status: "active".into(),
            expires_at: None,
            max_stores: 1,
            max_pos_instances: 1,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(sub.verify_signature().is_ok());
    }

    #[test]
    fn verify_non_bootstrap_signature_rejected() {
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Free,
            status: "active".into(),
            expires_at: None,
            max_stores: 1,
            max_pos_instances: 1,
            allowed_types_json: "[]".into(),
            signature: "TAMPERED_SIGNATURE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(sub.verify_signature().is_err());
    }

    // ── QuotaError Display ────────────────────────────────

    #[test]
    fn quota_error_register_limit() {
        let err = QuotaError::RegisterLimit {
            tier: "Free".into(),
            limit: 1,
            current: 1,
        };
        let msg = err.to_string();
        assert!(msg.contains("Free"));
        assert!(msg.contains("1"));
    }

    #[test]
    fn quota_error_store_limit() {
        let err = QuotaError::StoreLimit {
            tier: "Pro".into(),
            limit: 2,
            current: 2,
        };
        let msg = err.to_string();
        assert!(msg.contains("Pro"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn quota_error_type_not_allowed() {
        let err = QuotaError::TypeNotAllowed {
            tier: "Free".into(),
            type_key: "kds".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("kds"));
        assert!(msg.contains("Free"));
    }

    // ── Clock Rollback Detection ──────────────────────────

    #[test]
    fn clock_rollback_detects_future_timestamps() {
        use crate::migrations;
        let conn = migrations::fresh_db();

        // Insert a sale with a timestamp far in the future.
        conn.execute(
            "INSERT INTO sales (id, status, total_minor, currency, line_count, created_at, updated_at)
             VALUES ('sale-1', 'completed', 1000, 'USD', 1, '2099-01-01T00:00:00.000Z', '2099-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let result = TenantSubscription::validate_clock_rollback(&conn);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("system clock tampered"));
        assert!(err.contains("2099"));
    }

    #[test]
    fn clock_rollback_passes_with_recent_timestamps() {
        use crate::migrations;
        let conn = migrations::fresh_db();

        // Insert a sale with a recent timestamp.
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO sales (id, status, total_minor, currency, line_count, created_at, updated_at)
             VALUES ('sale-1', 'completed', 1000, 'USD', 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();

        let result = TenantSubscription::validate_clock_rollback(&conn);
        assert!(result.is_ok(), "expected OK, got: {result:?}");
    }

    #[test]
    fn clock_rollback_passes_with_empty_tables() {
        use crate::migrations;
        let conn = migrations::fresh_db();
        // No sales or audit_logs — should default to Utc::now().
        let result = TenantSubscription::validate_clock_rollback(&conn);
        assert!(result.is_ok());
    }

    #[test]
    fn compute_max_ledger_timestamp_prefers_recent_over_older() {
        use crate::migrations;
        let conn = migrations::fresh_db();

        conn.execute(
            "INSERT INTO sales (id, status, total_minor, currency, line_count, created_at, updated_at)
             VALUES ('s1', 'completed', 1000, 'USD', 1, '2025-06-01T00:00:00.000Z', '2025-06-01T00:00:00.000Z')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO audit_log (id, action, user_id, created_at)
             VALUES ('a1', 'login', 'user-1', '2025-07-01T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let ts = TenantSubscription::compute_max_ledger_timestamp(&conn).unwrap();
        // Should pick the audit_log timestamp (2025-07-01) over sales (2025-06-01).
        assert!(ts.contains("2025-07-01"), "expected July, got: {ts}");
    }

    // ── Offline Grace Period ──────────────────────────────

    #[test]
    fn free_tier_always_within_grace() {
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Free,
            status: "active".into(),
            expires_at: Some("2020-01-01T00:00:00.000Z".into()),
            max_stores: 1,
            max_pos_instances: 1,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(sub.is_within_grace_period());
        assert_eq!(sub.effective_tier(), SubscriptionTier::Free);
    }

    #[test]
    fn paid_tier_with_no_expiry_within_grace() {
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Pro,
            status: "active".into(),
            expires_at: None, // lifetime
            max_stores: 2,
            max_pos_instances: 3,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(sub.is_within_grace_period());
        assert_eq!(sub.effective_tier(), SubscriptionTier::Pro);
    }

    #[test]
    fn paid_tier_within_14_day_grace() {
        // Expiry is 7 days ago — still within 14-day grace.
        let recent = chrono::Utc::now() - chrono::Duration::days(7);
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Premium,
            status: "active".into(),
            expires_at: Some(recent.to_rfc3339()),
            max_stores: 5,
            max_pos_instances: 10,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(sub.is_within_grace_period());
        assert_eq!(sub.effective_tier(), SubscriptionTier::Premium);
    }

    #[test]
    fn paid_tier_outside_grace_downgrades_to_free() {
        // Expiry is 30 days ago — outside 14-day grace.
        let old = chrono::Utc::now() - chrono::Duration::days(30);
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Premium,
            status: "active".into(),
            expires_at: Some(old.to_rfc3339()),
            max_stores: 5,
            max_pos_instances: 10,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(!sub.is_within_grace_period());
        assert_eq!(sub.effective_tier(), SubscriptionTier::Free);
    }

    #[test]
    fn enterprise_lifetime_never_downgrades() {
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Enterprise,
            status: "active".into(),
            expires_at: None,
            max_stores: 0,
            max_pos_instances: 0,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(sub.is_within_grace_period());
        assert_eq!(sub.effective_tier(), SubscriptionTier::Enterprise);
    }

    // ── constants ────────────────────────────────────────

    #[test]
    fn canceled_subscription_not_within_grace() {
        let sub = TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Pro,
            status: "canceled".into(),
            expires_at: None, // lifetime but canceled
            max_stores: 2,
            max_pos_instances: 3,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        };
        assert!(!sub.is_within_grace_period());
        assert_eq!(sub.effective_tier(), SubscriptionTier::Free);
    }

    #[test]
    fn clock_skew_constants_are_reasonable() {
        assert_eq!(CLOCK_SKEW_TOLERANCE_SECONDS, 30);
        assert_eq!(OFFLINE_GRACE_DAYS, 14);
    }
}
