//! Domain events published on the kernel event bus.

use crate::Barcode;
use crate::contracts::DomainEvent;

/// Published when a sale is completed at the POS.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaleCompleted {
    /// Unique sale identifier (UUID v4).
    pub sale_id: String,
    /// The store where the sale occurred.
    pub store_id: Option<String>,
    /// Line items sold in this transaction.
    pub line_items: Vec<SaleCompletedLine>,
    /// Total sale amount in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Optional customer identifier.
    pub customer_id: Option<String>,
}

/// A single line item included in a completed sale.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaleCompletedLine {
    /// Stock-keeping unit code.
    pub sku: String,
    /// Quantity sold.
    pub qty: i64,
    /// Unit price in minor units.
    pub unit_price_minor: i64,
    /// Tax amount for this line in minor units.
    #[serde(default)]
    pub tax_minor: i64,
    /// Tax rate ID applied.
    #[serde(default)]
    pub tax_rate_id: Option<String>,
}

impl DomainEvent for SaleCompleted {
    fn event_name(&self) -> &'static str {
        "sale.completed"
    }
}

/// Published when a new product is created in the catalog.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductCreated {
    /// Stock-keeping unit of the new product.
    pub sku: String,
    /// Display name of the new product.
    pub name: String,
    /// Price in minor units.
    pub price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Optional category id.
    pub category_id: Option<String>,
    /// Optional barcode.
    pub barcode: Option<Barcode>,
    /// Initial stock quantity.
    pub initial_stock: i64,
}

impl DomainEvent for ProductCreated {
    fn event_name(&self) -> &'static str {
        "product.created"
    }
}

/// Published when a product's stock level is adjusted.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StockAdjusted {
    /// Stock-keeping unit of the adjusted product.
    pub sku: String,
    /// Quantity change (positive = restock, negative = removal).
    pub delta: i64,
    /// New stock quantity after adjustment.
    pub new_qty: i64,
    /// Reason for the adjustment.
    pub reason: String,
}

impl DomainEvent for StockAdjusted {
    fn event_name(&self) -> &'static str {
        "stock.adjusted"
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::DomainEvent;

    // ── SaleCompleted ─────────────────────────────────────────────────

    #[test]
    fn sale_completed_event_name() {
        let event = SaleCompleted {
            sale_id: "sale-001".into(),
            store_id: Some("store-1".into()),
            line_items: vec![],
            total_minor: 0,
            currency: "USD".into(),
            customer_id: None,
        };
        assert_eq!(event.event_name(), "sale.completed");
    }

    #[test]
    fn sale_completed_serde_roundtrip() {
        let event = SaleCompleted {
            sale_id: "sale-42".into(),
            store_id: Some("store-1".into()),
            line_items: vec![
                SaleCompletedLine {
                    sku: "COFFEE".into(),
                    qty: 2,
                    unit_price_minor: 350,
                    tax_minor: 35,
                    tax_rate_id: Some("tax-1".into()),
                },
                SaleCompletedLine {
                    sku: "CROISSANT".into(),
                    qty: 1,
                    unit_price_minor: 250,
                    tax_minor: 25,
                    tax_rate_id: Some("tax-1".into()),
                },
            ],
            total_minor: 950,
            currency: "USD".into(),
            customer_id: Some("cust-1".into()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: SaleCompleted = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sale_id, "sale-42");
        assert_eq!(back.line_items.len(), 2);
        assert_eq!(back.line_items[0].sku, "COFFEE");
        assert_eq!(back.line_items[1].sku, "CROISSANT");
        assert_eq!(back.total_minor, 950);
    }

    #[test]
    fn sale_completed_empty_line_items() {
        let event = SaleCompleted {
            sale_id: "sale-empty".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 0,
            currency: "IDR".into(),
            customer_id: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: SaleCompleted = serde_json::from_str(&json).unwrap();
        assert!(back.line_items.is_empty());
        assert!(back.store_id.is_none());
        assert!(back.customer_id.is_none());
    }

    #[test]
    fn sale_completed_line_defaults() {
        // tax_minor defaults to 0, tax_rate_id defaults to None
        let line_json = r#"{"sku":"X","qty":1,"unit_price_minor":100}"#;
        let line: SaleCompletedLine = serde_json::from_str(line_json).unwrap();
        assert_eq!(line.tax_minor, 0);
        assert!(line.tax_rate_id.is_none());
    }

    // ── ProductCreated ────────────────────────────────────────────────

    #[test]
    fn product_created_event_name() {
        let event = ProductCreated {
            sku: "NEW-001".into(),
            name: "Widget".into(),
            price_minor: 1000,
            currency: "USD".into(),
            category_id: Some("cat-1".into()),
            barcode: None,
            initial_stock: 50,
        };
        assert_eq!(event.event_name(), "product.created");
    }

    #[test]
    fn product_created_serde_roundtrip() {
        let event = ProductCreated {
            sku: "SKU-123".into(),
            name: "Gadget".into(),
            price_minor: 2500,
            currency: "IDR".into(),
            category_id: None,
            barcode: Some(Barcode::new("5901234123457").unwrap()),
            initial_stock: 100,
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: ProductCreated = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sku, "SKU-123");
        assert_eq!(back.name, "Gadget");
        assert_eq!(back.price_minor, 2500);
        assert_eq!(back.barcode, Some(Barcode::new("5901234123457").unwrap()));
        assert_eq!(back.initial_stock, 100);
    }

    #[test]
    fn product_created_optional_fields_none() {
        let event = ProductCreated {
            sku: "MIN".into(),
            name: "Min".into(),
            price_minor: 0,
            currency: "USD".into(),
            category_id: None,
            barcode: None,
            initial_stock: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: ProductCreated = serde_json::from_str(&json).unwrap();
        assert!(back.category_id.is_none());
        assert!(back.barcode.is_none());
    }

    // ── StockAdjusted ─────────────────────────────────────────────────

    #[test]
    fn stock_adjusted_event_name() {
        let event = StockAdjusted {
            sku: "WIDGET".into(),
            delta: -5,
            new_qty: 45,
            reason: "Sale #123".into(),
        };
        assert_eq!(event.event_name(), "stock.adjusted");
    }

    #[test]
    fn stock_adjusted_serde_roundtrip() {
        let event = StockAdjusted {
            sku: "WIDGET".into(),
            delta: -10,
            new_qty: 90,
            reason: "Return from customer".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: StockAdjusted = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sku, "WIDGET");
        assert_eq!(back.delta, -10);
        assert_eq!(back.new_qty, 90);
        assert_eq!(back.reason, "Return from customer");
    }

    #[test]
    fn stock_adjusted_positive_delta_restock() {
        let event = StockAdjusted {
            sku: "RESTOCK".into(),
            delta: 50,
            new_qty: 150,
            reason: "PO #7 received".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: StockAdjusted = serde_json::from_str(&json).unwrap();
        assert_eq!(back.delta, 50);
        assert_eq!(back.new_qty, 150);
    }
}
