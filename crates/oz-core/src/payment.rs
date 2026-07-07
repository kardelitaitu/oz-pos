//! Payment domain type — individual payment tenders within a sale.
//!
//! A [`Payment`] represents a single tender against a sale. Most sales
//! have one payment (e.g. "cash" for the full amount), but split
//! payments produce multiple payment records (e.g. $10 cash + $5 card).
//! Gateway tracking fields capture external processor references
//! (card-terminal transaction IDs, gateway status, raw responses).
//!
//! # Schema mapping
//!
//! Maps to the `payments` table (migration `022_payments_table.sql`),
//! with gateway fields added by migration `027_payment_gateway_fields.sql`.

use serde::{Deserialize, Serialize};

use crate::money::Money;

/// A single payment tender against a sale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payment {
    /// Internal row id (UUID v4).
    pub id: String,

    /// FK to `sales.id`.
    pub sale_id: String,

    /// Payment method ("cash", "card", "other", etc.).
    pub method: String,

    /// Amount tendered in minor units.
    pub amount: Money,

    /// ISO-8601 timestamp of when this payment was recorded.
    pub created_at: String,

    /// Unique transaction reference returned by the payment gateway.
    pub gateway_reference: Option<String>,

    /// Status returned by the payment gateway (e.g. "approved", "declined").
    pub gateway_status: Option<String>,

    /// Raw JSON response returned by the payment gateway.
    pub gateway_response: Option<String>,
}

/// Argument used to describe a payment split when completing a sale.
///
/// This is the serialisation boundary type used in IPC commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentSplitArg {
    /// Payment method ("cash", "card", "other", etc.).
    pub method: String,

    /// Amount in minor units.
    pub amount_minor: i64,

    /// Optional transaction reference returned by the payment gateway.
    pub gateway_reference: Option<String>,

    /// Optional status returned by the payment gateway.
    pub gateway_status: Option<String>,

    /// Optional raw response returned by the payment gateway.
    pub gateway_response: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;

    // ── Payment serde roundtrip ────────────────────────────────────

    #[test]
    fn payment_serde_roundtrip_basic() {
        let payment = Payment {
            id: "pay-1".into(),
            sale_id: "sale-1".into(),
            method: "cash".into(),
            amount: Money {
                minor_units: 50000,
                currency: "IDR".parse().unwrap(),
            },
            created_at: "2025-07-07T12:00:00.000Z".into(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        let json = serde_json::to_string(&payment).unwrap();
        let back: Payment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "pay-1");
        assert_eq!(back.method, "cash");
        assert_eq!(back.amount.minor_units, 50000);
        assert_eq!(back.amount.currency.to_string(), "IDR");
    }

    #[test]
    fn payment_serde_roundtrip_with_gateway() {
        let payment = Payment {
            id: "pay-2".into(),
            sale_id: "sale-2".into(),
            method: "card".into(),
            amount: Money {
                minor_units: 25000,
                currency: "USD".parse().unwrap(),
            },
            created_at: "2025-07-07T12:00:00.000Z".into(),
            gateway_reference: Some("txn_abc123".into()),
            gateway_status: Some("approved".into()),
            gateway_response: Some(r#"{"id":"txn_abc123"}"#.into()),
        };
        let json = serde_json::to_string(&payment).unwrap();
        let back: Payment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.gateway_reference.as_deref(), Some("txn_abc123"));
        assert_eq!(back.gateway_status.as_deref(), Some("approved"));
        assert!(back.gateway_response.is_some());
    }

    #[test]
    fn payment_split_arg_serde_roundtrip() {
        let split = PaymentSplitArg {
            method: "card".into(),
            amount_minor: 30000,
            gateway_reference: Some("txn_def456".into()),
            gateway_status: Some("approved".into()),
            gateway_response: None,
        };
        let json = serde_json::to_string(&split).unwrap();
        let back: PaymentSplitArg = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, "card");
        assert_eq!(back.amount_minor, 30000);
        assert_eq!(back.gateway_reference.as_deref(), Some("txn_def456"));
        assert_eq!(back.gateway_status.as_deref(), Some("approved"));
        assert!(back.gateway_response.is_none());
    }

    #[test]
    fn payment_split_arg_minimal() {
        let split = PaymentSplitArg {
            method: "cash".into(),
            amount_minor: 50000,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        assert_eq!(split.method, "cash");
        assert_eq!(split.amount_minor, 50000);
        assert!(split.gateway_reference.is_none());
    }

    #[test]
    fn payment_equality() {
        let p1 = Payment {
            id: "p1".into(),
            sale_id: "s1".into(),
            method: "cash".into(),
            amount: Money {
                minor_units: 100,
                currency: "IDR".parse().unwrap(),
            },
            created_at: "2025-01-01T00:00:00.000Z".into(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }

    #[test]
    fn payment_split_arg_equality() {
        let s1 = PaymentSplitArg {
            method: "cash".into(),
            amount_minor: 100,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        let s2 = PaymentSplitArg {
            method: "cash".into(),
            amount_minor: 100,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        assert_eq!(s1, s2);
    }

    #[test]
    fn payment_different_methods_not_equal() {
        let p1 = Payment {
            id: "p1".into(),
            sale_id: "s1".into(),
            method: "cash".into(),
            amount: Money {
                minor_units: 100,
                currency: "IDR".parse().unwrap(),
            },
            created_at: String::new(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        let p2 = Payment {
            id: "p1".into(),
            sale_id: "s1".into(),
            method: "card".into(),
            amount: Money {
                minor_units: 100,
                currency: "IDR".parse().unwrap(),
            },
            created_at: String::new(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        assert_ne!(p1, p2);
    }

    // ── Debug output ────────────────────────────────────────────

    #[test]
    fn payment_debug_output() {
        let payment = Payment {
            id: "pay-1".into(),
            sale_id: "sale-1".into(),
            method: "cash".into(),
            amount: Money {
                minor_units: 50000,
                currency: "IDR".parse().unwrap(),
            },
            created_at: "2025-07-07T12:00:00.000Z".into(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        let debug = format!("{:?}", payment);
        assert!(debug.contains("pay-1"));
        assert!(debug.contains("cash"));
    }

    #[test]
    fn payment_split_arg_debug_output() {
        let split = PaymentSplitArg {
            method: "card".into(),
            amount_minor: 30000,
            gateway_reference: Some("txn_123".into()),
            gateway_status: Some("approved".into()),
            gateway_response: None,
        };
        let debug = format!("{:?}", split);
        assert!(debug.contains("card"));
        assert!(debug.contains("txn_123"));
    }

    // ── Edge cases ──────────────────────────────────────────────

    #[test]
    fn payment_zero_amount() {
        let payment = Payment {
            id: "pay-zero".into(),
            sale_id: "sale-1".into(),
            method: "voucher".into(),
            amount: Money {
                minor_units: 0,
                currency: "IDR".parse().unwrap(),
            },
            created_at: String::new(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        assert_eq!(payment.amount.minor_units, 0);
    }

    #[test]
    fn payment_empty_method() {
        let payment = Payment {
            id: "pay-empty".into(),
            sale_id: "sale-1".into(),
            method: String::new(),
            amount: Money {
                minor_units: 100,
                currency: "IDR".parse().unwrap(),
            },
            created_at: String::new(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        assert_eq!(payment.method, "");
    }

    #[test]
    fn payment_declined_gateway() {
        let payment = Payment {
            id: "pay-declined".into(),
            sale_id: "sale-1".into(),
            method: "card".into(),
            amount: Money {
                minor_units: 50000,
                currency: "IDR".parse().unwrap(),
            },
            created_at: String::new(),
            gateway_reference: Some("txn_fail".into()),
            gateway_status: Some("declined".into()),
            gateway_response: Some(r#"{"error":"insufficient_funds"}"#.into()),
        };
        assert_eq!(payment.gateway_status.as_deref(), Some("declined"));
        assert!(payment.gateway_response.is_some());
    }

    #[test]
    fn payment_split_arg_all_gateway_fields() {
        let split = PaymentSplitArg {
            method: "card".into(),
            amount_minor: 75000,
            gateway_reference: Some("txn_full".into()),
            gateway_status: Some("approved".into()),
            gateway_response: Some(r#"{"id":"txn_full","amount":75000}"#.into()),
        };
        assert_eq!(split.gateway_reference.as_deref(), Some("txn_full"));
        assert_eq!(split.gateway_status.as_deref(), Some("approved"));
        assert!(split.gateway_response.is_some());
    }

    #[test]
    fn payment_amount_large() {
        let payment = Payment {
            id: "pay-large".into(),
            sale_id: "sale-1".into(),
            method: "card".into(),
            amount: Money {
                minor_units: i64::MAX,
                currency: "IDR".parse().unwrap(),
            },
            created_at: String::new(),
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        assert_eq!(payment.amount.minor_units, i64::MAX);
    }

    #[test]
    fn payment_json_field_names() {
        let payment = Payment {
            id: "pay-fields".into(),
            sale_id: "sale-1".into(),
            method: "cash".into(),
            amount: Money {
                minor_units: 50000,
                currency: "IDR".parse().unwrap(),
            },
            created_at: String::new(),
            gateway_reference: Some("ref_1".into()),
            gateway_status: Some("approved".into()),
            gateway_response: None,
        };
        let json = serde_json::to_value(&payment).unwrap();
        assert_eq!(json["method"], "cash");
        assert_eq!(json["sale_id"], "sale-1");
        assert_eq!(json["gateway_reference"], "ref_1");
        assert_eq!(json["gateway_status"], "approved");
        assert!(json.get("gateway_response").unwrap().is_null());
    }

    #[test]
    fn payment_split_arg_json_field_names() {
        let split = PaymentSplitArg {
            method: "card".into(),
            amount_minor: 50000,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        };
        let json = serde_json::to_value(&split).unwrap();
        assert_eq!(json["method"], "card");
        assert_eq!(json["amount_minor"], 50000);
        assert!(json.get("gateway_reference").unwrap().is_null());
    }

    #[test]
    fn payment_clone_preserves_all_fields() {
        let p1 = Payment {
            id: "p1".into(),
            sale_id: "s1".into(),
            method: "card".into(),
            amount: Money {
                minor_units: 25000,
                currency: "USD".parse().unwrap(),
            },
            created_at: "2025-01-01T00:00:00.000Z".into(),
            gateway_reference: Some("txn_abc".into()),
            gateway_status: Some("approved".into()),
            gateway_response: Some(r#"{"ok":true}"#.into()),
        };
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }
}
