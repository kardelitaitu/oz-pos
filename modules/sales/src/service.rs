//! Sales Service — domain business logic, checkout orchestration, and event dispatching.

use foundation::{Cart, SaleStatus};
use rusqlite::Connection;

use crate::models::Sale;
use crate::repository::SalesRepository;

/// Service encapsulating sales workflows and business operations.
pub struct SalesService;

impl SalesService {
    /// Create a new sale from a cart, persist it, and transition it to Completed state.
    pub fn process_checkout(
        conn: &mut Connection,
        cart: &Cart,
        user_id: Option<String>,
        payment_method: String,
    ) -> Result<Sale, anyhow::Error> {
        let mut sale = Sale::from_cart_with_user(cart, user_id)
            .ok_or_else(|| anyhow::anyhow!("Failed to construct sale from cart — corrupt total"))?;

        sale.payment_method = Some(payment_method);
        sale.transition_to(SaleStatus::Active)?;
        sale.transition_to(SaleStatus::Completed)?;

        let tx = conn.transaction()?;
        {
            let repo = SalesRepository::new(&tx);
            repo.create_sale_tx(&tx, &sale)?;
        }
        tx.commit()?;

        Ok(sale)
    }

    /// Retrieve sale by ID using `SalesRepository`.
    pub fn get_sale(conn: &Connection, id: &str) -> Result<Option<Sale>, anyhow::Error> {
        let repo = SalesRepository::new(conn);
        repo.get_sale(id)
    }

    /// Void an active or completed sale.
    pub fn void_sale(conn: &Connection, id: &str) -> Result<(), anyhow::Error> {
        let repo = SalesRepository::new(conn);
        let sale = repo
            .get_sale(id)?
            .ok_or_else(|| anyhow::anyhow!("Sale not found: {}", id))?;

        if sale.is_terminal() && sale.status == SaleStatus::Voided {
            return Err(anyhow::anyhow!("Sale is already voided"));
        }

        repo.update_sale_status(id, SaleStatus::Voided)?;
        Ok(())
    }
}
