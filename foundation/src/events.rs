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
