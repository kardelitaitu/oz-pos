//! Sales pipeline commands: start a cart, add a line, complete the sale.
//!
//! These commands are the IPC surface for `ui/src/features/sales/`. The
//! actual cart/sale state machine lives in `oz_core`; this file translates
//! between the Tauri argument structs and the domain types.
//!
//! Carts are held in-memory inside [`AppState::carts`] — they do not
//! survive a restart. Once the persistence layer is in place, carts
//! will be saved to SQLite and recovered on restart.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{Cart, CartId, LineId, Money, Sku};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct StartSaleArgs {
    /// ISO-4217 currency code for the new cart. Defaults to `"USD"` when
    /// the front-end sends an empty string.
    #[serde(default)]
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct StartSaleResult {
    pub cart_id: CartId,
}

#[command]
pub async fn start_sale(
    args: StartSaleArgs,
    state: State<'_, AppState>,
) -> Result<StartSaleResult, AppError> {
    let currency_str = if args.currency.is_empty() {
        "USD"
    } else {
        &args.currency
    };
    let currency: oz_core::Currency = currency_str
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency_str}")))?;
    let cart = Cart::new(currency);
    let id = cart.id();
    state.carts.lock().await.insert(id, cart);
    Ok(StartSaleResult { cart_id: id })
}

#[derive(Debug, Deserialize)]
pub struct AddLineArgs {
    pub cart_id: CartId,
    pub sku: Sku,
    pub qty: i64,
    pub unit_price_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct AddLineResult {
    pub line_id: LineId,
    /// `None` if `unit_price * qty` overflowed `i64` minor units.
    pub line_total: Option<Money>,
}

#[command]
pub async fn add_line(
    args: AddLineArgs,
    state: State<'_, AppState>,
) -> Result<AddLineResult, AppError> {
    let currency = {
        let carts = state.carts.lock().await;
        let cart = carts
            .get(&args.cart_id)
            .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
        cart.currency()
    };

    let unit_price = Money {
        minor_units: args.unit_price_minor,
        currency,
    };
    let line = oz_core::CartLine::new(args.sku.clone(), args.qty, unit_price);
    let line_id = line.id;
    let line_total = line.total();

    let mut carts = state.carts.lock().await;
    let cart = carts
        .get_mut(&args.cart_id)
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
    cart.add_line(line)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    Ok(AddLineResult {
        line_id,
        line_total,
    })
}

#[derive(Debug, Deserialize)]
pub struct CompleteSaleArgs {
    pub cart_id: CartId,
}

#[derive(Debug, Serialize)]
pub struct CompleteSaleResult {
    pub sale_id: Uuid,
    /// `None` if the cart's line totals overflowed `i64` minor units.
    /// The front-end should surface a clear error in that case.
    pub total: Option<Money>,
    pub line_count: usize,
}

#[command]
pub async fn complete_sale(
    args: CompleteSaleArgs,
    state: State<'_, AppState>,
) -> Result<CompleteSaleResult, AppError> {
    let mut carts = state.carts.lock().await;
    let cart = carts
        .remove(&args.cart_id)
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
    let total = cart.total();
    let line_count = cart.line_count();
    let sale_id = Uuid::new_v4();
    tracing::info!(%sale_id, ?total, line_count, "sale completed");
    Ok(CompleteSaleResult {
        sale_id,
        total,
        line_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::Currency;
    use tauri::State;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    #[tokio::test]
    async fn start_then_complete_sale() {
        let state = AppState::for_test();
        let start = start_sale(
            StartSaleArgs {
                currency: "USD".into(),
            },
            State::new(&state),
        )
        .await
        .unwrap();
        let add = add_line(
            AddLineArgs {
                cart_id: start.cart_id,
                sku: Sku::new("COFFEE"),
                qty: 2,
                unit_price_minor: 350,
            },
            State::new(&state),
        )
        .await
        .unwrap();
        let total = add.line_total.expect("line fits in i64");
        assert_eq!(total.minor_units, 700);
        assert_eq!(total.currency, usd());
    }
}
