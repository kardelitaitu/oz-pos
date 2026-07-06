//! Store-profile domain type — each location has its own identity,
//! settings, and feature flags.
//!
//! Maps to the `store_profiles` table (migration `025_store_profiles.sql`).

use serde::{Deserialize, Serialize};

/// A store location / outlet in a multi-store deployment.
///
/// Every deployment has exactly one **primary** store, created on first
/// startup (`id = "default"`). Additional stores can be added for
/// multi-location operators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreProfile {
    /// Row id (`"default"` for the primary store, UUID for others).
    pub id: String,

    /// Display name (e.g. "Downtown Flagship").
    pub name: String,

    /// Street address (printed on receipts).
    pub address: String,

    /// Tax / VAT registration number.
    pub tax_id: String,

    /// ISO-4217 currency code (e.g. "USD", "IDR").
    pub currency: String,

    /// IANA timezone (e.g. "America/New_York", "Asia/Jakarta").
    pub timezone: String,

    /// Whether this is the primary store (exactly one per deployment).
    pub is_primary: bool,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_profile() -> StoreProfile {
        StoreProfile {
            id: "default".into(),
            name: "Main Store".into(),
            address: "123 Main St".into(),
            tax_id: "TAX123".into(),
            currency: "USD".into(),
            timezone: "UTC".into(),
            is_primary: true,
            created_at: "2026-06-30T12:00:00Z".into(),
            updated_at: "2026-06-30T12:00:00Z".into(),
        }
    }

    #[test]
    fn store_profile_has_required_fields() {
        let p = default_profile();
        assert_eq!(p.name, "Main Store");
        assert!(p.is_primary);
    }

    #[test]
    fn store_profile_long_name() {
        let long_name = "A".repeat(255);
        let p = StoreProfile {
            name: long_name.clone(),
            ..default_profile()
        };
        assert_eq!(p.name, long_name);
    }

    #[test]
    fn store_profile_empty_address() {
        let p = StoreProfile {
            address: String::new(),
            ..default_profile()
        };
        assert!(p.address.is_empty());
    }

    #[test]
    fn store_profile_empty_tax_id() {
        let p = StoreProfile {
            tax_id: String::new(),
            ..default_profile()
        };
        assert!(p.tax_id.is_empty());
    }

    #[test]
    fn store_profile_different_currencies() {
        for currency in ["USD", "EUR", "IDR", "JPY", "GBP"] {
            let p = StoreProfile {
                currency: currency.to_owned(),
                ..default_profile()
            };
            assert_eq!(p.currency, currency);
        }
    }

    #[test]
    fn store_profile_different_timezones() {
        for tz in ["UTC", "America/New_York", "Asia/Jakarta", "Europe/London"] {
            let p = StoreProfile {
                timezone: tz.to_owned(),
                ..default_profile()
            };
            assert_eq!(p.timezone, tz);
        }
    }

    #[test]
    fn store_profile_serde_roundtrip() {
        let p = default_profile();
        let json = serde_json::to_string(&p).unwrap();
        let back: StoreProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn store_profile_serde_roundtrip_non_primary() {
        let p = StoreProfile {
            id: "branch-2".into(),
            name: "Branch 2".into(),
            address: String::new(),
            tax_id: String::new(),
            currency: "IDR".into(),
            timezone: "Asia/Jakarta".into(),
            is_primary: false,
            created_at: "2026-06-30T12:00:00Z".into(),
            updated_at: "2026-06-30T12:00:00Z".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: StoreProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn store_profile_serde_json_field_names() {
        let p = default_profile();
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["name"], "Main Store");
        assert_eq!(json["is_primary"], true);
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["timezone"], "UTC");
    }

    #[test]
    fn store_profile_debug() {
        let p = default_profile();
        assert!(!format!("{p:?}").is_empty());
    }

    #[test]
    fn store_profile_clone_eq() {
        let a = default_profile();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn store_profile_non_primary() {
        let p = StoreProfile {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Branch 2".into(),
            address: String::new(),
            tax_id: String::new(),
            currency: "IDR".into(),
            timezone: "Asia/Jakarta".into(),
            is_primary: false,
            created_at: "2026-06-30T12:00:00Z".into(),
            updated_at: "2026-06-30T12:00:00Z".into(),
        };
        assert!(!p.is_primary);
        assert_eq!(p.currency, "IDR");
    }

    #[test]
    fn store_profile_equality_depends_on_all_fields() {
        let a = default_profile();
        let b = StoreProfile {
            name: "Different Name".into(),
            ..default_profile()
        };
        assert_ne!(a, b);
    }
}
