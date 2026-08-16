//! Sales Service — domain business logic, checkout orchestration, and event dispatching.

use crate::error::SalesError;
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
    ) -> Result<Sale, SalesError> {
        let mut sale = Sale::from_cart_with_user(cart, user_id).ok_or_else(|| {
            SalesError::validation("cart", "failed to construct sale from cart — corrupt total")
        })?;

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
    pub fn get_sale(conn: &Connection, id: &str) -> Result<Option<Sale>, SalesError> {
        let repo = SalesRepository::new(conn);
        repo.get_sale(id)
    }

    /// Void an active or completed sale.
    pub fn void_sale(conn: &Connection, id: &str) -> Result<(), SalesError> {
        let repo = SalesRepository::new(conn);
        let sale = repo.get_sale(id)?.ok_or_else(|| SalesError::NotFound {
            entity: "sale",
            id: id.to_string(),
        })?;

        if sale.is_terminal() && sale.status == SaleStatus::Voided {
            return Err(SalesError::validation("status", "sale is already voided"));
        }

        repo.update_sale_status(id, SaleStatus::Voided)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::{Cart, CartLine, Sku};

    fn usd() -> foundation::Currency {
        "USD".parse().unwrap()
    }

    fn cart_with_line() -> Cart {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(
            Sku::new("COFFEE"),
            2,
            foundation::Money {
                minor_units: 350,
                currency: usd(),
            },
        ))
        .unwrap();
        cart
    }

    fn fresh_conn() -> rusqlite::Connection {
        oz_core::migrations::fresh_db()
    }

    #[test]
    fn process_checkout_persists_completed_sale() {
        let mut conn = fresh_conn();
        let sale = SalesService::process_checkout(
            &mut conn,
            &cart_with_line(),
            Some("u-1".to_string()),
            "cash".to_string(),
        )
        .unwrap();

        assert_eq!(sale.status, SaleStatus::Completed);
        assert_eq!(sale.payment_method.as_deref(), Some("cash"));
        assert_eq!(sale.total.minor_units, 700);

        // Read back from the DB through the service.
        let fetched = SalesService::get_sale(&conn, &sale.id).unwrap().unwrap();
        assert_eq!(fetched.id, sale.id);
        assert_eq!(fetched.status, SaleStatus::Completed);
        assert_eq!(fetched.total.minor_units, 700);
        assert_eq!(fetched.line_count, 1);
        assert_eq!(fetched.lines.len(), 1);
        assert_eq!(fetched.lines[0].sku, "COFFEE");
    }

    #[test]
    fn process_checkout_without_user_id() {
        let mut conn = fresh_conn();
        let sale =
            SalesService::process_checkout(&mut conn, &cart_with_line(), None, "card".to_string())
                .unwrap();
        assert_eq!(sale.payment_method.as_deref(), Some("card"));
        assert!(sale.user_id.is_none());
    }

    #[test]
    fn get_sale_via_service_missing_returns_none() {
        let conn = fresh_conn();
        assert!(SalesService::get_sale(&conn, "missing").unwrap().is_none());
    }

    #[test]
    fn void_sale_marks_sale_voided() {
        let mut conn = fresh_conn();
        let sale =
            SalesService::process_checkout(&mut conn, &cart_with_line(), None, "cash".to_string())
                .unwrap();

        SalesService::void_sale(&conn, &sale.id).unwrap();

        let fetched = SalesService::get_sale(&conn, &sale.id).unwrap().unwrap();
        assert_eq!(fetched.status, SaleStatus::Voided);
        assert!(fetched.is_terminal());
    }

    #[test]
    fn void_sale_not_found_errors() {
        let conn = fresh_conn();
        let err = SalesService::void_sale(&conn, "missing").unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn void_sale_already_voided_errors() {
        let mut conn = fresh_conn();
        let sale =
            SalesService::process_checkout(&mut conn, &cart_with_line(), None, "cash".to_string())
                .unwrap();

        SalesService::void_sale(&conn, &sale.id).unwrap();
        let err = SalesService::void_sale(&conn, &sale.id).unwrap_err();
        assert!(err.to_string().contains("already voided"));
    }
}
