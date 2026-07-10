//! Subscription tier definitions, signature verification, and quota
//! enforcement for ADR #5 (Subscription Tier & Entitlement Architecture).
//!
//! The `tenant_subscription` table lives in the global database. This
//! module provides the Rust types and logic for reading that table,
//! verifying its cryptographic signature, and enforcing tier limits
//! when creating stores and workspace instances.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

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

/// Subscription tiers with their quotas and allowed workspace types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionTier {
    /// 1 store, 1 POS register, basic types only.
    Free,
    /// Up to 2 stores, 3 registers/store, inventory support.
    Pro,
    /// Up to 5 stores, 10 registers/store, KDS + analytics.
    Premium,
    /// Unlimited stores, unlimited registers, all types + plugins.
    Enterprise,
}

impl SubscriptionTier {
    /// Parse from the database TEXT column.
    pub fn from_db(s: &str) -> Self {
        match s {
            "free" => Self::Free,
            "pro" => Self::Pro,
            "premium" => Self::Premium,
            "enterprise" => Self::Enterprise,
            _ => Self::Free,
        }
    }

    /// Human-readable tier name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Pro => "Pro",
            Self::Premium => "Premium",
            Self::Enterprise => "Enterprise",
        }
    }

    /// Maximum number of stores allowed for this tier.
    /// Returns `None` for unlimited (Enterprise).
    pub fn max_stores(&self) -> Option<i64> {
        match self {
            Self::Free => Some(1),
            Self::Pro => Some(2),
            Self::Premium => Some(5),
            Self::Enterprise => None,
        }
    }

    /// Maximum POS register instances per store for this tier.
    /// Returns `None` for unlimited (Enterprise).
    pub fn max_pos_instances(&self) -> Option<i64> {
        match self {
            Self::Free => Some(1),
            Self::Pro => Some(3),
            Self::Premium => Some(10),
            Self::Enterprise => None,
        }
    }

    /// Check whether this tier allows the given workspace type.
    pub fn allows_workspace_type(&self, type_key: &str) -> bool {
        match self {
            Self::Free => matches!(type_key, "store-pos" | "restaurant-pos" | "admin"),
            Self::Pro => matches!(
                type_key,
                "restaurant-pos" | "store-pos" | "inventory" | "admin"
            ),
            Self::Premium => matches!(
                type_key,
                "restaurant-pos" | "store-pos" | "inventory" | "admin" | "kds" | "analytics-pro"
            ),
            Self::Enterprise => true, // All types + custom plugins
        }
    }
}

// ── Subscription Row ──────────────────────────────────────────────────

/// A row from the `tenant_subscription` table.
#[derive(Debug, Clone)]
pub struct TenantSubscription {
    pub tenant_id: String,
    pub tier: SubscriptionTier,
    pub status: String,
    pub expires_at: Option<String>,
    pub max_stores: i64,
    pub max_pos_instances: i64,
    pub allowed_types_json: String,
    pub signature: String,
    pub updated_at: String,
}

impl TenantSubscription {
    /// Load the subscription for a tenant from the global database.
    pub fn load(conn: &rusqlite::Connection, tenant_id: &str) -> Result<Option<Self>, CoreError> {
        let mut stmt = conn.prepare(
            "SELECT tenant_id, tier_key, status, expires_at, max_stores,
                    max_pos_instances, allowed_types_json, signature, updated_at
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
                updated_at: row.get(8)?,
            })
        });

        match result {
            Ok(sub) => Ok(Some(sub)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::from(e)),
        }
    }

    /// Verify the subscription signature.
    ///
    /// During local development / single-store deployments, the bootstrap
    /// signature `BOOTSTRAP_FREE` is accepted. In production, the signature
    /// must be validated against `oz-pos-updater.key.pub`.
    pub fn verify_signature(&self, _public_key_pem: &str) -> Result<(), CoreError> {
        // BOOTSTRAP_FREE is the sentinel value seeded by the migration
        // for single-store deployments without a cloud-server.
        if self.signature == "BOOTSTRAP_FREE" {
            return Ok(());
        }

        // TODO(ADR #5): Implement real RSA/HMAC signature verification
        // against oz-pos-updater.key.pub when apps/cloud-server is
        // available for subscription signing.
        //
        // For now, any non-BOOTSTRAP signature is rejected as invalid.
        Err(CoreError::InvalidSubscriptionSignature(
            "Subscription signature verification is not yet implemented. Connect to cloud-server to validate your subscription.".into(),
        ))
    }
}

// ── Quota Enforcement ─────────────────────────────────────────────────

/// Error type for quota-related failures, used by the subscription
/// module to provide actionable upgrade messaging.
#[derive(Debug)]
pub enum QuotaError {
    /// The tenant has reached their per-store register limit.
    RegisterLimit {
        tier: String,
        limit: i64,
        current: i64,
    },
    /// The tenant has reached their store count limit.
    StoreLimit {
        tier: String,
        limit: i64,
        current: i64,
    },
    /// The workspace type is not available on this tier.
    TypeNotAllowed { tier: String, type_key: String },
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
        assert_eq!(SubscriptionTier::Pro.max_stores(), Some(2));
        assert_eq!(SubscriptionTier::Premium.max_stores(), Some(5));
        assert_eq!(SubscriptionTier::Enterprise.max_stores(), None);
    }

    #[test]
    fn tier_max_pos_instances() {
        assert_eq!(SubscriptionTier::Free.max_pos_instances(), Some(1));
        assert_eq!(SubscriptionTier::Pro.max_pos_instances(), Some(3));
        assert_eq!(SubscriptionTier::Premium.max_pos_instances(), Some(10));
        assert_eq!(SubscriptionTier::Enterprise.max_pos_instances(), None);
    }

    #[test]
    fn tier_allows_workspace_type() {
        // Free tier
        assert!(SubscriptionTier::Free.allows_workspace_type("store-pos"));
        assert!(SubscriptionTier::Free.allows_workspace_type("admin"));
        assert!(!SubscriptionTier::Free.allows_workspace_type("kds"));
        assert!(!SubscriptionTier::Free.allows_workspace_type("inventory"));

        // Pro tier
        assert!(SubscriptionTier::Pro.allows_workspace_type("inventory"));
        assert!(!SubscriptionTier::Pro.allows_workspace_type("kds"));

        // Premium tier
        assert!(SubscriptionTier::Premium.allows_workspace_type("kds"));
        assert!(SubscriptionTier::Premium.allows_workspace_type("analytics-pro"));

        // Enterprise
        assert!(SubscriptionTier::Enterprise.allows_workspace_type("anything"));
    }

    #[test]
    fn tier_name() {
        assert_eq!(SubscriptionTier::Free.name(), "Free");
        assert_eq!(SubscriptionTier::Pro.name(), "Pro");
        assert_eq!(SubscriptionTier::Premium.name(), "Premium");
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
            updated_at: String::new(),
        };
        assert!(sub.verify_signature("").is_ok());
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
            updated_at: String::new(),
        };
        assert!(sub.verify_signature("").is_err());
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
}
