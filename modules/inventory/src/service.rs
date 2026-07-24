//! Inventory Service — product catalog and stock adjustment orchestration.

use crate::models::{Inventory, Product};
use crate::repository::InventoryRepository;
use foundation::Sku;
use rusqlite::Connection;

/// Service encapsulating product and inventory domain operations.
pub struct InventoryService;

impl InventoryService {
    /// Retrieve product by ID.
    pub fn get_product(conn: &Connection, id: &str) -> Result<Option<Product>, anyhow::Error> {
        let repo = InventoryRepository::new(conn);
        repo.get_product(id)
    }

    /// Retrieve inventory stock level for a SKU.
    pub fn get_stock(conn: &Connection, sku: &Sku) -> Result<Option<Inventory>, anyhow::Error> {
        let repo = InventoryRepository::new(conn);
        repo.get_stock(sku)
    }

    /// Adjust stock level for a product.
    pub fn adjust_stock(conn: &mut Connection, sku: &Sku, delta: i64) -> Result<(), anyhow::Error> {
        let tx = conn.transaction()?;
        {
            let repo = InventoryRepository::new(&tx);
            repo.adjust_stock_tx(&tx, sku, delta)?;
        }
        tx.commit()?;
        Ok(())
    }
}
