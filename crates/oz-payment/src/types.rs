//! Data types used by the [`PaymentProcessor`](crate::PaymentProcessor) trait.
//!
//! These types model the request/response lifecycle of a payment:
//! authorize → capture → refund.

use foundation::Money;
use serde::{Deserialize, Serialize};

/// The method used to pay.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PaymentMethod {
    /// Physical cash.
    Cash,
    /// Credit / debit card (chip, swipe, or contactless).
    Card,
    /// Mobile QR (QRIS, Alipay, WeChat).
    Qr,
    /// Any other method not covered by the variants above.
    Other(String),
}

impl PaymentMethod {
    /// A human-readable label (e.g. for receipts).
    pub fn label(&self) -> &str {
        match self {
            Self::Cash => "Cash",
            Self::Card => "Card",
            Self::Qr => "QR",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// A request to process a payment.
#[derive(Debug, Clone)]
pub struct PaymentRequest {
    /// The amount to charge.
    pub amount: Money,
    /// Optional reference for card / terminal payments (e.g. invoice ID).
    pub reference: Option<String>,
    /// Optional description shown on the cardholder's statement.
    pub description: Option<String>,
}

/// The outcome of a payment attempt.
#[derive(Debug, Clone)]
pub struct PaymentResult {
    /// Whether the payment was approved.
    pub success: bool,
    /// Processor-assigned transaction ID (present on success).
    pub transaction_id: Option<String>,
    /// Authorization code from the processor (present on success).
    pub auth_code: Option<String>,
    /// The amount that was actually charged (may differ from requested
    /// amount in partial-capture scenarios).
    pub amount_charged: Money,
    /// Human-readable message (e.g. "approved", "declined: insufficient funds").
    pub message: Option<String>,
}

/// Processor-specific receipt / terminal data returned after a successful
/// transaction. May be printed or shown to the customer.
#[derive(Debug, Clone)]
pub struct PaymentReceipt {
    /// Processor-assigned transaction ID.
    pub transaction_id: String,
    /// The payment method used.
    pub method: PaymentMethod,
    /// The amount charged.
    pub amount: Money,
    /// Timestamp of the transaction (ISO-8601).
    pub timestamp: String,
    /// Any raw data the processor returned (e.g. hex-encoded EMV data).
    pub raw_data: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::Currency;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    #[test]
    fn payment_method_label_cash() {
        assert_eq!(PaymentMethod::Cash.label(), "Cash");
    }

    #[test]
    fn payment_method_label_card() {
        assert_eq!(PaymentMethod::Card.label(), "Card");
    }

    #[test]
    fn payment_method_label_qr() {
        assert_eq!(PaymentMethod::Qr.label(), "QR");
    }

    #[test]
    fn payment_method_label_other() {
        assert_eq!(
            PaymentMethod::Other("Gift Card".into()).label(),
            "Gift Card"
        );
    }

    #[test]
    fn payment_request_has_required_fields() {
        let req = PaymentRequest {
            amount: Money::from_major(10, usd()).unwrap(),
            reference: None,
            description: None,
        };
        assert_eq!(req.amount.minor_units, 1000);
    }

    #[test]
    fn payment_result_success_vs_failure() {
        let ok = PaymentResult {
            success: true,
            transaction_id: Some("txn_123".into()),
            auth_code: Some("AUTH01".into()),
            amount_charged: Money::from_major(10, usd()).unwrap(),
            message: Some("approved".into()),
        };
        assert!(ok.success);

        let fail = PaymentResult {
            success: false,
            transaction_id: None,
            auth_code: None,
            amount_charged: Money::from_major(10, usd()).unwrap(),
            message: Some("declined: insufficient funds".into()),
        };
        assert!(!fail.success);
    }

    #[test]
    fn payment_receipt_holds_processor_data() {
        let receipt = PaymentReceipt {
            transaction_id: "txn_456".into(),
            method: PaymentMethod::Card,
            amount: Money::from_major(25, usd()).unwrap(),
            timestamp: "2026-06-30T12:00:00Z".into(),
            raw_data: Some("9F26...".into()),
        };
        assert_eq!(receipt.transaction_id, "txn_456");
        assert_eq!(receipt.method, PaymentMethod::Card);
    }

    #[test]
    fn payment_method_serde_roundtrip() {
        let methods = [
            PaymentMethod::Cash,
            PaymentMethod::Card,
            PaymentMethod::Qr,
            PaymentMethod::Other("Voucher".into()),
        ];
        for m in &methods {
            let json = serde_json::to_string(m).unwrap();
            let back: PaymentMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(*m, back, "roundtrip failed for {m:?}");
        }
    }

    #[test]
    fn payment_result_debug() {
        let r = PaymentResult {
            success: true,
            transaction_id: None,
            auth_code: None,
            amount_charged: Money::zero(usd()),
            message: None,
        };
        assert!(!format!("{r:?}").is_empty());
    }
}
