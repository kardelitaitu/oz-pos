//! Domain events published on the kernel event bus.
//!
//! These events are defined in `oz-core` so that all modules can
//! reference them without circular dependencies. Each event
//! implements `foundation::contracts::DomainEvent` for use with
//! the kernel's `EventBus`.

use foundation::contracts::DomainEvent;

/// Published when a sale is completed at the POS.
///
/// Handlers should use this event to trigger side effects:
/// - **Inventory** → decrement stock for each sold SKU
/// - **CRM** → update customer purchase history
/// - **Audit** → log the completed transaction
/// - **Reporting** → update dashboard metrics
#[derive(Debug, Clone)]
pub struct SaleCompleted {
    /// Unique sale identifier (UUID v4).
    pub sale_id: String,
    /// Line items sold in this transaction.
    pub line_items: Vec<SaleCompletedLine>,
    /// Total sale amount in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Optional customer identifier (set if the sale was linked to a customer).
    pub customer_id: Option<String>,
}

/// A single line item included in a completed sale.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SaleCompletedLine {
    /// Stock-keeping unit code.
    pub sku: String,
    /// Quantity sold.
    pub qty: i64,
    /// Unit price in minor units.
    pub unit_price_minor: i64,
    /// Tax amount for this line in minor units (0 if no tax).
    #[serde(default)]
    pub tax_minor: i64,
    /// Tax rate ID applied (None if no tax).
    #[serde(default)]
    pub tax_rate_id: Option<String>,
}

impl DomainEvent for SaleCompleted {
    fn event_name(&self) -> &'static str {
        "sale.completed"
    }
}

/// Published when a new product is created in the catalog.
///
/// Handlers should use this event to trigger side effects:
/// - **Audit** → log the product creation
/// - **Sync** → queue the new product for cloud sync
#[derive(Debug, Clone)]
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
    pub barcode: Option<String>,
    /// Initial stock quantity.
    pub initial_stock: i64,
}

impl DomainEvent for ProductCreated {
    fn event_name(&self) -> &'static str {
        "product.created"
    }
}

/// Published when a product's stock level is adjusted.
///
/// Handlers should use this event to trigger side effects:
/// - **Audit** → log the stock adjustment
/// - **Reporting** → update inventory dashboard metrics
/// - **Sync** → queue the stock change for cloud sync
#[derive(Debug, Clone)]
pub struct StockAdjusted {
    /// Stock-keeping unit of the adjusted product.
    pub sku: String,
    /// Quantity change (positive = restock, negative = removal).
    pub delta: i64,
    /// New stock quantity after adjustment.
    pub new_qty: i64,
    /// Reason for the adjustment (e.g. "stock-take", "damaged", "return").
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

    #[test]
    fn sale_completed_event_name() {
        let event = SaleCompleted {
            sale_id: "sale-1".into(),
            line_items: vec![SaleCompletedLine {
                sku: "COFFEE".into(),
                qty: 2,
                unit_price_minor: 350,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 700,
            currency: "USD".into(),
            customer_id: None,
        };
        assert_eq!(event.event_name(), "sale.completed");
    }

    #[test]
    fn sale_completed_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<SaleCompleted>();
        assert_sync::<SaleCompleted>();
        assert_send::<SaleCompletedLine>();
        assert_sync::<SaleCompletedLine>();
    }

    #[test]
    fn product_created_event_name() {
        let event = ProductCreated {
            sku: "PROD-1".into(),
            name: "Widget".into(),
            price_minor: 199,
            currency: "USD".into(),
            category_id: None,
            barcode: None,
            initial_stock: 10,
        };
        assert_eq!(event.event_name(), "product.created");
    }

    #[test]
    fn product_created_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<ProductCreated>();
        assert_sync::<ProductCreated>();
    }

    #[test]
    fn stock_adjusted_event_name() {
        let event = StockAdjusted {
            sku: "COFFEE".into(),
            delta: -3,
            new_qty: 47,
            reason: "sale".into(),
        };
        assert_eq!(event.event_name(), "stock.adjusted");
    }

    #[test]
    fn stock_adjusted_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<StockAdjusted>();
        assert_sync::<StockAdjusted>();
    }
}
