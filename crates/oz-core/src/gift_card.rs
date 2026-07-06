//! Gift card domain types — cards, transactions, balance tracking.
//!
//! Gift cards are prepaid value cards that can be issued to customers,
//! redeemed at checkout, topped up, or frozen. All monetary values are
//! stored as `i64` minor units.

use serde::{Deserialize, Serialize};

/// A gift card with current balance and status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftCard {
    /// Unique identifier.
    pub id: String,
    /// Human-readable card number (scannable barcode).
    pub card_number: String,
    /// PIN for balance checks (optional).
    pub pin: String,
    /// Initial loaded value in minor units.
    pub initial_balance_minor: i64,
    /// Current redeemable value in minor units.
    pub current_balance_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Card status: active, frozen, redeemed, expired.
    pub status: String,
    /// Customer name this card was issued to (optional).
    pub issued_to: String,
    /// ISO-8601 issue date.
    pub issue_date: String,
    /// ISO-8601 expiry date (optional).
    pub expiry_date: Option<String>,
    /// Staff id who created this card.
    pub created_by: Option<String>,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

/// A single gift card transaction — issue, redeem, topup, refund.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftCardTransaction {
    /// Unique identifier.
    pub id: String,
    /// FK to `gift_cards.id`.
    pub gift_card_id: String,
    /// FK to `sales.id`, when tied to a sale.
    pub sale_id: Option<String>,
    /// Transaction type: issue, redeem, topup, refund.
    pub txn_type: String,
    /// Amount delta in minor units (positive for add, negative for deduct).
    pub amount_minor: i64,
    /// Balance after this transaction.
    pub balance_after_minor: i64,
    /// Human-readable notes.
    pub notes: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Gift card with recent transactions (returned by detail API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftCardWithTransactions {
    /// The underlying gift card.
    pub card: GiftCard,
    /// Most recent transactions.
    pub transactions: Vec<GiftCardTransaction>,
}

/// Arguments to issue a new gift card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueGiftCardInput {
    /// Unique card number.
    pub card_number: String,
    /// Optional PIN for balance checks.
    pub pin: Option<String>,
    /// Initial amount in minor units.
    pub initial_amount_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Optional customer name.
    pub issued_to: Option<String>,
    /// Staff id who created this card.
    pub created_by: String,
    /// Optional ISO-8601 expiry date.
    pub expiry_date: Option<String>,
}

/// Filters for listing gift cards.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GiftCardFilter {
    /// Search by card number (partial match).
    pub search: Option<String>,
    /// Filter by status.
    pub status: Option<String>,
    /// Filter by issued_to (partial match).
    pub issued_to: Option<String>,
    /// Minimum remaining balance in minor units.
    pub min_balance: Option<i64>,
}

/// Result of a gift card redemption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemGiftCardResult {
    /// The updated gift card.
    pub card: GiftCard,
    /// The redemption transaction.
    pub transaction: GiftCardTransaction,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gift_card() -> GiftCard {
        GiftCard {
            id: "gc-001".into(),
            card_number: "GC-1234-5678".into(),
            pin: "1234".into(),
            initial_balance_minor: 100000,
            current_balance_minor: 75000,
            currency: "IDR".into(),
            status: "active".into(),
            issued_to: "Alice".into(),
            issue_date: "2026-07-01T00:00:00.000Z".into(),
            expiry_date: Some("2027-07-01T00:00:00.000Z".into()),
            created_by: Some("staff-1".into()),
            updated_at: "2026-07-01T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn gift_card_serde_roundtrip() {
        let card = sample_gift_card();
        let json = serde_json::to_string(&card).unwrap();
        let back: GiftCard = serde_json::from_str(&json).unwrap();
        assert_eq!(card.id, back.id);
        assert_eq!(card.card_number, back.card_number);
        assert_eq!(card.current_balance_minor, back.current_balance_minor);
        assert_eq!(card.status, back.status);
        assert_eq!(card.expiry_date, back.expiry_date);
    }

    #[test]
    fn gift_card_transaction_serde_roundtrip() {
        let txn = GiftCardTransaction {
            id: "txn-1".into(),
            gift_card_id: "gc-001".into(),
            sale_id: Some("sale-1".into()),
            txn_type: "redeem".into(),
            amount_minor: -25000,
            balance_after_minor: 75000,
            notes: "Partial redemption".into(),
            created_at: "2026-07-01T12:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&txn).unwrap();
        let back: GiftCardTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(txn.id, back.id);
        assert_eq!(txn.txn_type, back.txn_type);
        assert_eq!(txn.amount_minor, back.amount_minor);
        assert_eq!(txn.balance_after_minor, back.balance_after_minor);
    }

    #[test]
    fn gift_card_with_transactions_serde_roundtrip() {
        let card = sample_gift_card();
        let txn = GiftCardTransaction {
            id: "txn-1".into(),
            gift_card_id: "gc-001".into(),
            sale_id: None,
            txn_type: "issue".into(),
            amount_minor: 100000,
            balance_after_minor: 100000,
            notes: "Initial issuance".into(),
            created_at: "2026-07-01T00:00:00.000Z".into(),
        };
        let combined = GiftCardWithTransactions {
            card: card.clone(),
            transactions: vec![txn],
        };
        let json = serde_json::to_string(&combined).unwrap();
        let back: GiftCardWithTransactions = serde_json::from_str(&json).unwrap();
        assert_eq!(back.card.id, card.id);
        assert_eq!(back.transactions.len(), 1);
    }

    #[test]
    fn issue_gift_card_input_serde_roundtrip() {
        let input = IssueGiftCardInput {
            card_number: "GC-9999".into(),
            pin: Some("5678".into()),
            initial_amount_minor: 50000,
            currency: "IDR".into(),
            issued_to: Some("Bob".into()),
            created_by: "staff-2".into(),
            expiry_date: Some("2027-12-31T00:00:00.000Z".into()),
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: IssueGiftCardInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.card_number, input.card_number);
        assert_eq!(back.initial_amount_minor, input.initial_amount_minor);
        assert_eq!(back.pin, input.pin);
    }

    #[test]
    fn gift_card_filter_defaults() {
        let filter = GiftCardFilter::default();
        assert!(filter.search.is_none());
        assert!(filter.status.is_none());
        assert!(filter.issued_to.is_none());
        assert!(filter.min_balance.is_none());
    }

    #[test]
    fn redeem_gift_card_result_serde_roundtrip() {
        let card = sample_gift_card();
        let txn = GiftCardTransaction {
            id: "txn-2".into(),
            gift_card_id: "gc-001".into(),
            sale_id: Some("sale-1".into()),
            txn_type: "redeem".into(),
            amount_minor: -25000,
            balance_after_minor: 75000,
            notes: "Redeemed at POS".into(),
            created_at: "2026-07-01T12:00:00.000Z".into(),
        };
        let result = RedeemGiftCardResult {
            card: card.clone(),
            transaction: txn,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: RedeemGiftCardResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.card.id, card.id);
        assert_eq!(back.transaction.txn_type, "redeem");
    }
}
