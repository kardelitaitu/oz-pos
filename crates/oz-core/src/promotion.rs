//! Promotions domain type — buy-X-get-Y, % off, fixed discount.
//!
//! A [`Promotion`] defines a discount rule that can be applied to a sale.
//! The [`PromotionType`] enum discriminates between percentage discounts,
//! fixed-amount discounts, and buy-X-get-Y promotions. Applications are
//! recorded in [`PromotionApplication`] rows for audit and reporting.

use serde::{Deserialize, Serialize};

/// Type of promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromotionType {
    /// Percentage off the product price (value_minor = percent, e.g. 10 = 10%).
    Percentage,
    /// Fixed amount off in minor units.
    FixedAmount,
    /// Buy X, get Y at a discount (value_minor = discount % on the reward item).
    BuyXGetY,
}

impl PromotionType {
    /// Return the string representation used in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Percentage => "percentage",
            Self::FixedAmount => "fixed_amount",
            Self::BuyXGetY => "buy_x_get_y",
        }
    }

    /// Parse a promotion type from its database string representation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "percentage" => Some(Self::Percentage),
            "fixed_amount" => Some(Self::FixedAmount),
            "buy_x_get_y" => Some(Self::BuyXGetY),
            _ => None,
        }
    }
}

/// A promotion rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Promotion {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Human-readable name for display on receipts and the POS UI.
    pub name: String,
    /// Optional detailed description of the promotion terms.
    pub description: String,
    /// Database string for the promotion type.
    pub promo_type: String,
    /// Numeric value whose meaning depends on `promo_type`.
    pub value_minor: i64,
    /// Minimum quantity the customer must buy (buy-X-get-Y only).
    pub min_qty: Option<i64>,
    /// Product SKU that must be in the cart (buy-X-get-Y only).
    pub trigger_sku: Option<String>,
    /// Product SKU that receives the discount (buy-X-get-Y, empty = trigger).
    pub reward_sku: Option<String>,
    /// How many reward items the customer receives (buy-X-get-Y).
    pub reward_qty: Option<i64>,
    /// ISO-8601 start timestamp for time-limited promotions.
    pub starts_at: Option<String>,
    /// ISO-8601 end timestamp for time-limited promotions.
    pub ends_at: Option<String>,
    /// Minimum order subtotal in minor units for the promotion to apply.
    pub min_order_minor: i64,
    /// Optional category ID that this promotion applies to.
    pub category_id: Option<String>,
    /// Whether this promotion is currently active.
    pub active: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

/// Record of a promotion being applied to a sale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionApplication {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// FK to `promotions.id`.
    pub promotion_id: String,
    /// FK to `sales.id`.
    pub sale_id: String,
    /// Discount amount applied, in minor units.
    pub discount_minor: i64,
    /// Human-readable description of the discount applied.
    pub description: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_promotion() -> Promotion {
        Promotion {
            id: "promo-1".into(),
            name: "10% Off".into(),
            description: "Get 10% off everything".into(),
            promo_type: "percentage".into(),
            value_minor: 10,
            min_qty: None,
            trigger_sku: None,
            reward_sku: None,
            reward_qty: None,
            starts_at: None,
            ends_at: None,
            min_order_minor: 0,
            category_id: None,
            active: true,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        }
    }

    // ── PromotionType ────────────────────────────────────────────

    #[test]
    fn promotion_type_roundtrip() {
        for (s, expected) in [
            ("percentage", PromotionType::Percentage),
            ("fixed_amount", PromotionType::FixedAmount),
            ("buy_x_get_y", PromotionType::BuyXGetY),
        ] {
            assert_eq!(PromotionType::from_str(s), Some(expected));
            assert_eq!(expected.as_str(), s);
        }
    }

    #[test]
    fn promotion_type_from_str_unknown() {
        assert_eq!(PromotionType::from_str("unknown"), None);
    }

    #[test]
    fn promotion_type_from_str_case_sensitive() {
        assert_eq!(PromotionType::from_str("PERCENTAGE"), None);
        assert_eq!(PromotionType::from_str("Percentage"), None);
    }

    #[test]
    fn promotion_type_debug() {
        assert!(format!("{:?}", PromotionType::Percentage).contains("Percentage"));
        assert!(format!("{:?}", PromotionType::FixedAmount).contains("FixedAmount"));
        assert!(format!("{:?}", PromotionType::BuyXGetY).contains("BuyXGetY"));
    }

    #[test]
    fn promotion_type_serde_json_format() {
        // No #[serde(rename_all)] — uses PascalCase variant names.
        let json = serde_json::to_value(PromotionType::Percentage).unwrap();
        assert_eq!(json, "Percentage");
        let json = serde_json::to_value(PromotionType::FixedAmount).unwrap();
        assert_eq!(json, "FixedAmount");
        let json = serde_json::to_value(PromotionType::BuyXGetY).unwrap();
        assert_eq!(json, "BuyXGetY");
    }

    #[test]
    fn promotion_type_serde_roundtrip_all() {
        for variant in [
            PromotionType::Percentage,
            PromotionType::FixedAmount,
            PromotionType::BuyXGetY,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: PromotionType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let p = sample_promotion();
        let json = serde_json::to_string(&p).unwrap();
        let back: Promotion = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn serde_roundtrip_all_fields() {
        let p = Promotion {
            id: "promo-bogo".into(),
            name: "Buy 1 Get 1 50% Off".into(),
            description: "Buy one coffee, get the second at half price".into(),
            promo_type: "buy_x_get_y".into(),
            value_minor: 50,
            min_qty: Some(2),
            trigger_sku: Some("COFFEE".into()),
            reward_sku: Some("COFFEE".into()),
            reward_qty: Some(1),
            starts_at: Some("2026-01-01T00:00:00.000Z".into()),
            ends_at: Some("2026-12-31T23:59:59.000Z".into()),
            min_order_minor: 500,
            category_id: Some("cat-drinks".into()),
            active: true,
            created_at: "2026-01-01T00:00:00.000Z".into(),
            updated_at: "2026-01-15T12:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: Promotion = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Buy 1 Get 1 50% Off");
        assert_eq!(back.promo_type, "buy_x_get_y");
        assert_eq!(back.min_qty, Some(2));
        assert_eq!(back.trigger_sku, Some("COFFEE".into()));
        assert_eq!(back.reward_sku, Some("COFFEE".into()));
        assert_eq!(back.reward_qty, Some(1));
        assert_eq!(back.min_order_minor, 500);
        assert_eq!(back.category_id, Some("cat-drinks".into()));
    }

    #[test]
    fn serde_json_field_names() {
        let p = sample_promotion();
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["promo_type"], "percentage");
        assert!(json.get("min_qty").unwrap().is_null());
        assert!(json.get("trigger_sku").unwrap().is_null());
        assert_eq!(json["active"], true);
    }

    // ── Active/inactive ──────────────────────────────────────────

    #[test]
    fn promotion_can_be_inactive() {
        let p = Promotion {
            active: false,
            ..sample_promotion()
        };
        assert!(!p.active);
    }

    #[test]
    fn serde_roundtrip_inactive() {
        let p = Promotion {
            active: false,
            ..sample_promotion()
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: Promotion = serde_json::from_str(&json).unwrap();
        assert!(!back.active);
    }

    // ── Time-range fields ────────────────────────────────────────

    #[test]
    fn promotion_with_time_range() {
        let p = Promotion {
            starts_at: Some("2026-06-01T00:00:00.000Z".into()),
            ends_at: Some("2026-06-30T23:59:59.000Z".into()),
            ..sample_promotion()
        };
        assert_eq!(p.starts_at.as_deref(), Some("2026-06-01T00:00:00.000Z"));
        assert_eq!(p.ends_at.as_deref(), Some("2026-06-30T23:59:59.000Z"));
    }

    #[test]
    fn promotion_without_end_date() {
        let p = Promotion {
            starts_at: Some("2026-06-01T00:00:00.000Z".into()),
            ends_at: None,
            ..sample_promotion()
        };
        assert!(p.starts_at.is_some());
        assert!(p.ends_at.is_none());
    }

    // ── Min order ────────────────────────────────────────────────

    #[test]
    fn promotion_with_min_order() {
        let p = Promotion {
            min_order_minor: 10000,
            ..sample_promotion()
        };
        assert_eq!(p.min_order_minor, 10000);
    }

    #[test]
    fn promotion_min_order_defaults_to_zero() {
        let p = Promotion {
            min_order_minor: 0,
            ..sample_promotion()
        };
        assert_eq!(p.min_order_minor, 0);
    }

    #[test]
    fn promotion_min_order_large_value() {
        let p = Promotion {
            min_order_minor: i64::MAX,
            ..sample_promotion()
        };
        assert_eq!(p.min_order_minor, i64::MAX);
    }

    // ── Category-specific ────────────────────────────────────────

    #[test]
    fn promotion_category_specific() {
        let p = Promotion {
            category_id: Some("cat-drinks".into()),
            ..sample_promotion()
        };
        assert_eq!(p.category_id.as_deref(), Some("cat-drinks"));
    }

    #[test]
    fn promotion_no_category_applies_to_all() {
        let p = Promotion {
            category_id: None,
            ..sample_promotion()
        };
        assert!(p.category_id.is_none());
    }

    // ── Value fields ─────────────────────────────────────────────

    #[test]
    fn promotion_value_zero() {
        let p = Promotion {
            value_minor: 0,
            ..sample_promotion()
        };
        assert_eq!(p.value_minor, 0);
    }

    #[test]
    fn promotion_value_large() {
        let p = Promotion {
            value_minor: 100_000,
            ..sample_promotion()
        };
        assert_eq!(p.value_minor, 100_000);
    }

    // ── Clone + equality ─────────────────────────────────────────

    #[test]
    fn promotion_clone_eq() {
        let a = sample_promotion();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn promotion_neq_when_field_differs() {
        let a = sample_promotion();
        let b = Promotion {
            value_minor: 20,
            ..sample_promotion()
        };
        assert_ne!(a, b);
    }

    // ── PromotionApplication ─────────────────────────────────────

    #[test]
    fn application_serde() {
        let a = PromotionApplication {
            id: "app-1".into(),
            promotion_id: "promo-1".into(),
            sale_id: "sale-1".into(),
            discount_minor: 150,
            description: "10% off".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: PromotionApplication = serde_json::from_str(&json).unwrap();
        assert_eq!(back.discount_minor, 150);
    }

    #[test]
    fn application_serde_large_discount() {
        let a = PromotionApplication {
            id: "app-2".into(),
            promotion_id: "promo-2".into(),
            sale_id: "sale-2".into(),
            discount_minor: 999_999_999,
            description: "big savings".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: PromotionApplication = serde_json::from_str(&json).unwrap();
        assert_eq!(back.discount_minor, 999_999_999);
    }

    #[test]
    fn application_serde_zero_discount() {
        let a = PromotionApplication {
            id: "app-3".into(),
            promotion_id: "promo-3".into(),
            sale_id: "sale-3".into(),
            discount_minor: 0,
            description: String::new(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: PromotionApplication = serde_json::from_str(&json).unwrap();
        assert_eq!(back.discount_minor, 0);
        assert!(back.description.is_empty());
    }

    #[test]
    fn application_clone_eq() {
        let a = PromotionApplication {
            id: "app-1".into(),
            promotion_id: "promo-1".into(),
            sale_id: "sale-1".into(),
            discount_minor: 150,
            description: "10% off".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn application_json_field_names() {
        let a = PromotionApplication {
            id: "app-1".into(),
            promotion_id: "promo-1".into(),
            sale_id: "sale-1".into(),
            discount_minor: 150,
            description: "10% off".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["promotion_id"], "promo-1");
        assert_eq!(json["discount_minor"], 150);
    }

    // ── Debug output ────────────────────────────────────────────

    #[test]
    fn promotion_debug_output() {
        let p = sample_promotion();
        let debug = format!("{p:?}");
        assert!(debug.contains("10% Off"));
        assert!(debug.contains("percentage"));
    }

    #[test]
    fn promotion_application_debug_output() {
        let a = PromotionApplication {
            id: "app-1".into(),
            promotion_id: "promo-1".into(),
            sale_id: "sale-1".into(),
            discount_minor: 150,
            description: "10% off".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let debug = format!("{a:?}");
        assert!(debug.contains("app-1"));
        assert!(debug.contains("promo-1"));
    }

    #[test]
    fn promotion_neq_when_active_differs() {
        let a = sample_promotion();
        let b = Promotion {
            active: false,
            ..sample_promotion()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn promotion_neq_when_name_differs() {
        let a = sample_promotion();
        let b = Promotion {
            name: "Different".into(),
            ..sample_promotion()
        };
        assert_ne!(a, b);
    }
}
