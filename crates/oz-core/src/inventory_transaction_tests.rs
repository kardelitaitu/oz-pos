use super::*;

#[test]
fn inventory_transaction_id_constructors() {
    let s = InventoryTransactionId::new();
    assert!(!s.as_str().is_empty());
    let from_str = InventoryTransactionId::from("abc-123");
    assert_eq!(from_str.as_str(), "abc-123");
    let from_string = InventoryTransactionId::from(String::from("xyz-789"));
    assert_eq!(from_string.as_str(), "xyz-789");
}

#[test]
fn inventory_transaction_type_roundtrip() {
    for t in [
        InventoryTransactionType::Sale,
        InventoryTransactionType::Void,
        InventoryTransactionType::Refund,
        InventoryTransactionType::Transfer,
        InventoryTransactionType::PurchaseOrderReceive,
        InventoryTransactionType::StockCount,
        InventoryTransactionType::ManualAdjustment,
    ] {
        let stored = t.as_stored_str();
        let parsed = InventoryTransactionType::from_stored_str(stored);
        assert_eq!(parsed, Some(t));
    }
}

#[test]
fn inventory_transaction_type_unknown_returns_none() {
    assert_eq!(
        InventoryTransactionType::from_stored_str("legacy-future-type"),
        None
    );
}

#[test]
fn inventory_transaction_serde_roundtrip() {
    let tx = InventoryTransaction {
        id: InventoryTransactionId::from("tx-001"),
        transaction_type: InventoryTransactionType::Sale,
        location_id: "loc-001".into(),
        staff_id: "user-001".into(),
        transfer_id: None,
        purchase_order_id: None,
        notes: "Cashier sale #42".into(),
        created_at: "2026-07-20T10:00:00.000Z".into(),
    };
    let json = serde_json::to_string(&tx).unwrap();
    let back: InventoryTransaction = serde_json::from_str(&json).unwrap();
    assert_eq!(back, tx);
}

#[test]
fn inventory_transaction_line_serde_roundtrip() {
    let line = InventoryTransactionLine {
        id: "line-001".into(),
        transaction_id: InventoryTransactionId::from("tx-001"),
        sku: "CHO-001".into(),
        product_name: "Chocolate Bar".into(),
        qty: 2,
        barcode_scanned: Some("5901234123457".into()),
        sort_order: 1,
    };
    let json = serde_json::to_string(&line).unwrap();
    let back: InventoryTransactionLine = serde_json::from_str(&json).unwrap();
    assert_eq!(back, line);
}
