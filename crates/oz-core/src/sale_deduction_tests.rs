
use super::*;

#[test]
fn complete_sale_result_serde_roundtrip() {
    let result = CompleteSaleResult {
        sale_id: "sale-1".into(),
        status: SaleStatus::Completed,
        receipt_number: "REC-001".into(),
        deduct_tx_id: InventoryTransactionId::from("tx-1"),
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: CompleteSaleResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sale_id, "sale-1");
    assert_eq!(back.status, SaleStatus::Completed);
    assert_eq!(back.receipt_number, "REC-001");
    assert_eq!(back.deduct_tx_id.as_str(), "tx-1");
}

#[test]
fn partial_stock_result_serde_roundtrip() {
    let shortfall = Shortfall {
        sku: "CHO-001".into(),
        product_name: "Choco Bar".into(),
        requested_qty: 20,
        primary_qty_available: 5,
        deficit: 15,
        primary_location_id: LocationId::from("loc-store"),
        alternatives: vec![LocationStock {
            location_id: LocationId::from("loc-wh-a"),
            location_name: "Warehouse A".into(),
            qty_available: 500,
        }],
    };
    let result = PartialStockResult::single(shortfall);
    let json = serde_json::to_string(&result).unwrap();
    let back: PartialStockResult = serde_json::from_str(&json).unwrap();
    assert!(back.requires_resolution);
    assert_eq!(back.shortfalls.len(), 1);
    assert_eq!(back.shortfalls[0].sku, "CHO-001");
    assert_eq!(back.shortfalls[0].deficit, 15);
    assert_eq!(back.shortfalls[0].alternatives.len(), 1);
}

#[test]
fn partial_stock_result_multiple_constructor() {
    let a = Shortfall {
        sku: "A".into(),
        product_name: "Item A".into(),
        requested_qty: 10,
        primary_qty_available: 0,
        deficit: 10,
        primary_location_id: LocationId::from("loc-1"),
        alternatives: vec![],
    };
    let b = Shortfall {
        sku: "B".into(),
        product_name: "Item B".into(),
        requested_qty: 5,
        primary_qty_available: 2,
        deficit: 3,
        primary_location_id: LocationId::from("loc-1"),
        alternatives: vec![],
    };
    let result = PartialStockResult::multiple(vec![a, b]);
    assert_eq!(result.shortfalls.len(), 2);
}

#[test]
fn location_stock_serde_camel_case() {
    let ls = LocationStock {
        location_id: LocationId::from("loc-wh-a"),
        location_name: "Warehouse A".into(),
        qty_available: 250,
    };
    let json = serde_json::to_string(&ls).unwrap();
    assert!(json.contains("locationId"));
    assert!(json.contains("locationName"));
    assert!(json.contains("qtyAvailable"));
}

#[test]
fn resolved_shortfall_serde_roundtrip() {
    let rs = ResolvedShortfall {
        sku: "CHO-001".into(),
        allocations: vec![
            LocationAllocation {
                location_id: LocationId::from("loc-store"),
                qty: 2,
            },
            LocationAllocation {
                location_id: LocationId::from("loc-wh-a"),
                qty: 3,
            },
        ],
    };
    let json = serde_json::to_string(&rs).unwrap();
    let back: ResolvedShortfall = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sku, "CHO-001");
    assert_eq!(back.allocations.len(), 2);
    assert_eq!(back.allocations[0].location_id.as_str(), "loc-store");
    assert_eq!(back.allocations[0].qty, 2);
    assert_eq!(back.allocations[1].location_id.as_str(), "loc-wh-a");
    assert_eq!(back.allocations[1].qty, 3);
    // Verify camelCase naming in JSON
    assert!(json.contains("locationId"));
    assert!(json.contains("\"qty\":"));
}

#[test]
fn location_allocation_serde_roundtrip() {
    let la = LocationAllocation {
        location_id: LocationId::from("loc-wh-b"),
        qty: 10,
    };
    let json = serde_json::to_string(&la).unwrap();
    let back: LocationAllocation = serde_json::from_str(&json).unwrap();
    assert_eq!(back.location_id.as_str(), "loc-wh-b");
    assert_eq!(back.qty, 10);
    assert!(json.contains("locationId"));
}

#[test]
fn stock_deduction_serde_roundtrip() {
    let sd = StockDeduction {
        sku: "CHO-001".into(),
        location_id: LocationId::from("loc-store"),
        delta: -5,
    };
    let json = serde_json::to_string(&sd).unwrap();
    let back: StockDeduction = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sku, "CHO-001");
    assert_eq!(back.location_id.as_str(), "loc-store");
    assert_eq!(back.delta, -5);
    assert!(json.contains("locationId"));
    assert!(json.contains("\"delta\":-5"));
}

#[test]
fn resolved_shortfall_empty_allocations() {
    let rs = ResolvedShortfall {
        sku: "EMPTY-SKU".into(),
        allocations: vec![],
    };
    let json = serde_json::to_string(&rs).unwrap();
    let back: ResolvedShortfall = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sku, "EMPTY-SKU");
    assert!(back.allocations.is_empty());
}

#[test]
fn topology_allocation_fills_locations_in_route_order() {
    let locations = vec![
        LocationStock {
            location_id: LocationId::from("warehouse-a"),
            location_name: "Warehouse A".into(),
            qty_available: 3,
        },
        LocationStock {
            location_id: LocationId::from("warehouse-b"),
            location_name: "Warehouse B".into(),
            qty_available: 10,
        },
    ];

    let allocations = allocate_stock_in_route_order(8, &locations).unwrap();

    assert_eq!(
        allocations,
        vec![
            LocationAllocation {
                location_id: LocationId::from("warehouse-a"),
                qty: 3,
            },
            LocationAllocation {
                location_id: LocationId::from("warehouse-b"),
                qty: 5,
            },
        ]
    );
}

#[test]
fn topology_allocation_rejects_when_routes_cannot_fulfill_quantity() {
    let locations = vec![LocationStock {
        location_id: LocationId::from("warehouse-a"),
        location_name: "Warehouse A".into(),
        qty_available: 3,
    }];

    assert_eq!(allocate_stock_in_route_order(4, &locations), None);
}
