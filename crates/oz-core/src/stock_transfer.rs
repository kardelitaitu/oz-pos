//! Stock transfers between terminals/stores.
//!
//! A [`StockTransfer`] moves inventory from one location or terminal to
//! another. Each transfer carries one or more [`StockTransferLine`] items
//! and progresses through a status state machine:
//! draft → pending → in_transit → received / cancelled.

use serde::{Deserialize, Serialize};

/// A single stock transfer between locations/terminals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTransfer {
    pub id: String,
    pub transfer_number: String,
    pub status: String,
    pub source_location: Option<String>,
    pub destination_location: Option<String>,
    pub source_terminal_id: Option<String>,
    pub destination_terminal_id: Option<String>,
    pub notes: String,
    pub created_by: String,
    pub received_by: Option<String>,
    pub created_at: String,
    pub sent_at: Option<String>,
    pub received_at: Option<String>,
    pub updated_at: String,
}

/// A line item in a stock transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTransferLine {
    pub id: String,
    pub transfer_id: String,
    pub sku: String,
    pub product_name: String,
    pub qty: i64,
    pub received_qty: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_transfer() -> StockTransfer {
        StockTransfer {
            id: "st-001".into(),
            transfer_number: "TRF-20260701-001".into(),
            status: "draft".into(),
            source_location: Some("Warehouse A".into()),
            destination_location: Some("Store B".into()),
            source_terminal_id: Some("term-1".into()),
            destination_terminal_id: Some("term-2".into()),
            notes: "Urgent restock".into(),
            created_by: "staff-1".into(),
            received_by: None,
            created_at: "2026-07-01T00:00:00.000Z".into(),
            sent_at: None,
            received_at: None,
            updated_at: "2026-07-01T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn stock_transfer_serde_roundtrip() {
        let transfer = sample_transfer();
        let json = serde_json::to_string(&transfer).unwrap();
        let back: StockTransfer = serde_json::from_str(&json).unwrap();
        assert_eq!(transfer.id, back.id);
        assert_eq!(transfer.transfer_number, back.transfer_number);
        assert_eq!(transfer.status, back.status);
        assert_eq!(transfer.source_location, back.source_location);
        assert_eq!(transfer.destination_location, back.destination_location);
    }

    #[test]
    fn stock_transfer_sent_and_received() {
        let mut transfer = sample_transfer();
        transfer.status = "in_transit".into();
        transfer.sent_at = Some("2026-07-01T10:00:00.000Z".into());
        transfer.received_at = Some("2026-07-01T14:00:00.000Z".into());
        transfer.received_by = Some("staff-2".into());

        let json = serde_json::to_string(&transfer).unwrap();
        let back: StockTransfer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, "in_transit");
        assert_eq!(back.sent_at, Some("2026-07-01T10:00:00.000Z".into()));
        assert_eq!(back.received_at, Some("2026-07-01T14:00:00.000Z".into()));
        assert_eq!(back.received_by, Some("staff-2".into()));
    }

    #[test]
    fn stock_transfer_line_serde_roundtrip() {
        let line = StockTransferLine {
            id: "stl-1".into(),
            transfer_id: "st-001".into(),
            sku: "SKU-123".into(),
            product_name: "Widget".into(),
            qty: 10,
            received_qty: 0,
        };
        let json = serde_json::to_string(&line).unwrap();
        let back: StockTransferLine = serde_json::from_str(&json).unwrap();
        assert_eq!(line.id, back.id);
        assert_eq!(line.sku, back.sku);
        assert_eq!(line.qty, back.qty);
        assert_eq!(line.received_qty, back.received_qty);
    }

    #[test]
    fn stock_transfer_line_partial_receive() {
        let line = StockTransferLine {
            id: "stl-2".into(),
            transfer_id: "st-001".into(),
            sku: "SKU-456".into(),
            product_name: "Gadget".into(),
            qty: 20,
            received_qty: 15,
        };
        assert_eq!(line.qty, 20);
        assert_eq!(line.received_qty, 15);
        assert!(line.received_qty < line.qty);
    }
}
