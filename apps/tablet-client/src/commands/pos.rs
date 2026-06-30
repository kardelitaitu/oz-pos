//! Point-of-Sale pipeline commands: start a cart, add a line,
//! complete the sale, hold/resume carts.
//!
//! These commands are the IPC surface for the POS screen. The actual
//! cart/sale state machine lives in `oz_core`; this file translates
//! between the Tauri argument structs and the domain types.
//!
//! Carts are held in-memory inside [`AppState::carts`] — they do not
//! survive a restart.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use foundation::Percentage;
use oz_core::db::Store;
use oz_core::events::{SaleCompleted, SaleCompletedLine};
use oz_core::{Cart, CartId, LineId, Money, SaleStatus, Sku};

use crate::error::AppError;
use crate::state::AppState;

// ── Discount ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SetCartDiscountArgs {
    pub cart_id: CartId,
    /// Discount percentage (0-100). Pass 0 to clear.
    pub percent: i64,
    /// Optional human-readable label (e.g. "Senior 10%").
    pub label: Option<String>,
}

/// Set or clear a cart-level percentage discount.
///
/// The discount is applied when the cart total is computed and when
/// the sale is completed. Pass `percent = 0` to clear any existing
/// discount.
#[command]
pub async fn set_cart_discount(
    args: SetCartDiscountArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if !(0..=100).contains(&args.percent) {
        return Err(AppError::Invalid(format!(
            "discount percent must be between 0 and 100, got {}",
            args.percent
        )));
    }
    // SAFETY: args.percent is validated 0..=100 above, so the unwrap is safe.
    let percent = Percentage::new(args.percent as u8).unwrap();
    let mut carts = state.carts.lock().await;
    let cart = carts
        .get_mut(&args.cart_id)
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
    cart.set_discount(percent, args.label);
    tracing::info!(cart_id = %args.cart_id, percent = %args.percent, "cart discount set");
    Ok(())
}

// ── Start Sale ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StartSaleArgs {
    /// ISO-4217 currency code for the new cart.
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

// ── Add Line ─────────────────────────────────────────────────────────

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

// ── Complete Sale ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CompleteSaleArgs {
    pub cart_id: CartId,
    pub payment_method: String,
    pub tendered_minor: Option<i64>,
    pub user_id: String,
    /// Optional customer id to link this sale to a customer
    /// for loyalty tracking and purchase history.
    pub customer_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompleteSaleResult {
    pub sale_id: String,
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

    let line_count = cart.line_count();

    let mut sale = oz_core::Sale::from_cart_with_user(&cart, Some(args.user_id))
        .ok_or_else(|| AppError::Invalid("cart total overflowed i64".into()))?;
    sale.payment_method = Some(args.payment_method);
    sale.tendered_minor = args.tendered_minor;

    let sale_id = sale.id.clone();

    // Scope the DB borrow so Store (which is !Send) is dropped before
    // the next .await point when we lock the kernel for event publishing.
    let updated = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        store.compute_sale_tax(&mut sale)?;
        store.create_sale(&sale)?;
        store.update_sale_status(&sale_id, SaleStatus::Completed)?
    };

    let total = cart.total();
    tracing::info!(%sale_id, ?total, line_count, "sale completed and persisted");

    // Publish the SaleCompleted domain event so that subscribers
    // (InventoryStockHandler, CrmHistoryHandler, AuditLogHandler, etc.)
    // fire their side effects.
    {
        let line_items: Vec<SaleCompletedLine> = sale
            .lines
            .iter()
            .map(|l| SaleCompletedLine {
                sku: l.sku.clone(),
                qty: l.qty,
                unit_price_minor: l.unit_price.minor_units,
                tax_minor: l.tax_amount.minor_units,
                tax_rate_id: l.tax_rate_id.clone(),
            })
            .collect();

        let event = SaleCompleted {
            sale_id: sale_id.clone(),
            line_items,
            total_minor: total.map(|m| m.minor_units).unwrap_or(0),
            currency: String::from_utf8_lossy(&sale.currency.0).into_owned(),
            customer_id: args.customer_id.clone(),
        };

        let kernel = state.kernel.lock().await;
        let bus = kernel.event_bus();
        if let Err(e) = bus.publish(&event) {
            // Logged by the bus; do not fail the command.
            tracing::warn!(%sale_id, error = %e, "event bus publish failed");
        }
    }

    Ok(CompleteSaleResult {
        sale_id: updated.id,
        total,
        line_count,
    })
}

// ── Hold Orders ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct HoldCartArgs {
    pub label: String,
    pub cart_data: String,
    pub item_count: i64,
    pub total_minor: i64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct HoldCartResult {
    pub id: String,
}

/// Park the current sale as a held order.
#[command]
pub async fn hold_cart(
    args: HoldCartArgs,
    state: State<'_, AppState>,
) -> Result<HoldCartResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let id = store.hold_cart(
        &args.label,
        &args.cart_data,
        args.item_count,
        args.total_minor,
        &args.currency,
    )?;
    drop(db);
    tracing::info!(held_cart_id = %id, label = %args.label, "cart held");
    Ok(HoldCartResult { id })
}

/// List all held (parked) orders, most recent first.
#[command]
pub async fn list_held_carts(
    state: State<'_, AppState>,
) -> Result<Vec<oz_core::db::HeldCartRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let carts = store.list_held_carts()?;
    drop(db);
    Ok(carts)
}

/// Resume a held cart by id.
#[command]
pub async fn get_held_cart(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<oz_core::db::HeldCartFull>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let cart = store.get_held_cart(&id)?;
    drop(db);
    Ok(cart)
}

/// Delete a held cart by id.
#[command]
pub async fn delete_held_cart(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_held_cart(&id)?;
    drop(db);
    tracing::info!(held_cart_id = %id, "held cart deleted");
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::CartLine;
    use oz_core::Currency;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    #[test]
    fn start_cart_add_line() {
        let mut cart = oz_core::Cart::new(usd());
        let cart_id = cart.id();

        let line = CartLine::new(Sku::new("COFFEE"), 2, price(350));
        cart.add_line(line).unwrap();

        assert_eq!(cart.line_count(), 1);
        let total = cart.total();
        assert_eq!(total.unwrap().minor_units, 700);
        assert_eq!(total.unwrap().currency, usd());
        assert!(!cart_id.to_string().is_empty());

        let line2 = CartLine::new(Sku::new("BAGEL"), 1, price(450));
        cart.add_line(line2).unwrap();
        assert_eq!(cart.line_count(), 2);
        assert_eq!(cart.total().unwrap().minor_units, 1150);
    }

    #[test]
    fn cart_total_with_fractional_qty() {
        let mut cart = oz_core::Cart::new(usd());
        let line = CartLine::new(Sku::new("TEA"), 3, price(200));
        let line_total = line.total().unwrap();
        cart.add_line(line).unwrap();
        assert_eq!(line_total.minor_units, 600);
        assert_eq!(cart.total().unwrap().minor_units, 600);
    }
}
