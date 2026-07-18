//! Point-of-Sale pipeline commands: start a cart, add a line,
//! complete the sale, hold/resume carts.
//!
//! These commands are the IPC surface for the POS screen. The actual
//! cart/sale state machine lives in `oz_core`; this file translates
//! between the Tauri argument structs and the domain types.
//!
//! Carts are persisted in the SQLite `active_carts` table so they
//! survive application restarts.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use foundation::Percentage;
use oz_core::db::Store;
use oz_core::events::{SaleCompleted, SaleCompletedLine};
use oz_core::{Cart, CartId, CartLine, LineId, Money, SaleStatus, Sku};

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

// ── Discount ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Setcartdiscountargs.
pub struct SetCartDiscountArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// Discount percentage (0-100). Pass 0 to clear.
    pub percent: i64,
    /// Optional human-readable label (e.g. "Senior 10%").
    pub label: Option<String>,
    /// ID of the user setting the discount (for authz).
    pub user_id: String,
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

    let db = state.db.lock().await;
    let store = Store::new(&db);

    require_permission_for_user(&store, &args.user_id, oz_core::permissions::SALES_DISCOUNT)?;

    let mut cart = store
        .load_active_cart(&args.cart_id)?
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
    cart.set_discount(percent, args.label);
    store.save_active_cart(&cart, None)?;
    drop(db);
    tracing::info!(cart_id = %args.cart_id, percent = %args.percent, "cart discount set");
    Ok(())
}

// ── Start Sale ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Startsaleargs.
pub struct StartSaleArgs {
    /// ISO-4217 currency code for the new cart.
    #[serde(default)]
    pub currency: String,
}

#[derive(Debug, Serialize)]
/// Startsaleresult.
pub struct StartSaleResult {
    /// ID of the associated cart.
    pub cart_id: CartId,
}

#[command]
/// Start sale.
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

    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.save_active_cart(&cart, None)?;
    drop(db);

    Ok(StartSaleResult { cart_id: id })
}

// ── List Active Carts ────────────────────────────────────────────────

/// Return all active cart IDs so the front-end can restore carts
/// after a restart.
#[command]
pub async fn list_active_carts(state: State<'_, AppState>) -> Result<Vec<CartId>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let ids = store.list_active_carts()?;
    drop(db);
    Ok(ids)
}

// ── Get Active Cart ──────────────────────────────────────────────────

/// Load and return the full cart state (lines, discount) by id.
/// The front end uses this to restore a cart after restart or navigation.
#[command]
pub async fn get_active_cart(
    cart_id: CartId,
    state: State<'_, AppState>,
) -> Result<Option<Cart>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let cart = store.load_active_cart(&cart_id)?;
    drop(db);
    Ok(cart)
}

// ── Add Line ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Addlineargs.
pub struct AddLineArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// Stock-keeping unit identifier.
    pub sku: Sku,
    /// Quantity.
    pub qty: i64,
    /// Unit Price Minor.
    pub unit_price_minor: i64,
}

#[derive(Debug, Serialize)]
/// Addlineresult.
pub struct AddLineResult {
    /// ID of the associated line.
    pub line_id: LineId,
    /// Line Total.
    pub line_total: Option<Money>,
}

#[command]
/// Add line.
pub async fn add_line(
    args: AddLineArgs,
    state: State<'_, AppState>,
) -> Result<AddLineResult, AppError> {
    // Load the cart and add the line in a single DB transaction scope.
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let mut cart = store
        .load_active_cart(&args.cart_id)?
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;

    let currency = cart.currency();
    let unit_price = Money {
        minor_units: args.unit_price_minor,
        currency,
    };
    let line = CartLine::new(args.sku.clone(), args.qty, unit_price);
    let line_id = line.id;
    let line_total = line.total();
    cart.add_line(line)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    store.save_active_cart(&cart, None)?;
    drop(db);

    Ok(AddLineResult {
        line_id,
        line_total,
    })
}

// ── Override Line Price ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Overridelinepriceargs.
pub struct OverrideLinePriceArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// ID of the associated line.
    pub line_id: LineId,
    /// The new unit price in minor units (e.g. cents).
    pub new_price_minor: i64,
    /// ID of the manager authorising the override.
    pub user_id: String,
}

/// Override the unit price of a cart line, authorised by a manager PIN.
#[command]
pub async fn override_line_price(
    args: OverrideLinePriceArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let mut cart = store
        .load_active_cart(&args.cart_id)?
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;

    // Permission check: the user authorising the override must have SALES_OVERRIDE_PRICE.
    require_permission_for_user(
        &store,
        &args.user_id,
        oz_core::permissions::SALES_OVERRIDE_PRICE,
    )?;

    let currency = cart.currency();
    let new_price = Money {
        minor_units: args.new_price_minor,
        currency,
    };

    // Find the line and set the override
    let line = cart
        .lines_mut()
        .iter_mut()
        .find(|l| l.id == args.line_id)
        .ok_or_else(|| AppError::Invalid(format!("line not found: {}", args.line_id)))?;

    line.set_overridden_price(new_price)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    store.save_active_cart(&cart, None)?;
    drop(db);

    tracing::info!(cart_id = %args.cart_id, line_id = %args.line_id, new_price_minor = args.new_price_minor, "line price overridden");
    Ok(())
}

// ── Complete Sale ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialnumberarg.
pub struct SerialNumberArg {
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Serial.
    pub serial: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Completesaleargs.
pub struct CompleteSaleArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// Payment Method.
    pub payment_method: String,
    /// Tendered Minor.
    pub tendered_minor: Option<i64>,
    /// ID of the associated user.
    pub user_id: String,
    /// Optional customer id to link this sale to a customer
    /// for loyalty tracking and purchase history.
    pub customer_id: Option<String>,
    /// Optional customer name (for credit sales).
    pub customer_name: Option<String>,
    /// Optional serial numbers captured at checkout for track_serial products.
    pub serial_numbers: Option<Vec<SerialNumberArg>>,
}

#[derive(Debug, Serialize)]
/// Completesaleresult.
pub struct CompleteSaleResult {
    /// ID of the associated sale.
    pub sale_id: String,
    /// Total amount in minor currency units.
    pub total: Option<Money>,
    /// Line Count.
    pub line_count: usize,
}

#[command]
/// Complete sale.
pub async fn complete_sale(
    args: CompleteSaleArgs,
    state: State<'_, AppState>,
) -> Result<CompleteSaleResult, AppError> {
    // Load and remove the cart from the DB in one scope.
    let cart = {
        let db = state.db.lock().await;
        let store = Store::new(&db);

        require_permission_for_user(&store, &args.user_id, oz_core::permissions::SALES_PROCESS)?;

        let cart = store
            .load_active_cart(&args.cart_id)?
            .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
        store.delete_active_cart(&args.cart_id)?;
        cart
    };

    let line_count = cart.line_count();

    let mut sale = oz_core::Sale::from_cart_with_user(&cart, Some(args.user_id))
        .ok_or_else(|| AppError::Invalid("cart total overflowed i64".into()))?;
    sale.payment_method = Some(args.payment_method);
    sale.tendered_minor = args.tendered_minor;
    sale.customer_id = args.customer_id.clone();

    let sale_id = sale.id.clone();

    // Scope the DB borrow so Store (which is !Send) is dropped before
    // the next .await point when we lock the kernel for event publishing.
    let updated = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        store.compute_sale_tax(&mut sale, &[])?;

        // Match serial numbers from args to sale lines by SKU.
        if let Some(ref serial_numbers) = args.serial_numbers {
            for sn in serial_numbers {
                if let Some(line) = sale.lines.iter_mut().find(|l| l.sku == sn.sku) {
                    line.serial_number = Some(sn.serial.clone());
                }
            }
        }

        store.create_sale(&sale)?;
        // Transition through Active before Completed — the state machine
        // does not allow Pending → Completed directly.
        store.update_sale_status(&sale_id, SaleStatus::Active)?;
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
            store_id: None,
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

// ── Compute Cart Tax ──────────────────────────────────────────────────

/// Compute the total tax for a live cart (front-end preview).
#[command]
pub async fn compute_cart_tax(
    lines: Vec<oz_core::db::CartLineTaxInput>,
    currency: String,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let parsed: oz_core::Currency = currency
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency}")))?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let tax = store.compute_cart_tax(&lines, parsed)?;
    drop(db);
    Ok(tax.minor_units)
}

// ── Hold Orders ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Holdcartargs.
pub struct HoldCartArgs {
    /// Label.
    pub label: String,
    /// Cart Data.
    pub cart_data: String,
    /// Item Count.
    pub item_count: i64,
    /// Total amount in minor currency units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    #[serde(default = "default_bill_type")]
    /// Bill Type.
    pub bill_type: String,
    /// Customer Name.
    pub customer_name: Option<String>,
}

fn default_bill_type() -> String {
    "hold".to_string()
}

#[derive(Debug, Serialize)]
/// Holdcartresult.
pub struct HoldCartResult {
    /// Unique identifier.
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
        &args.bill_type,
        args.customer_name.as_deref(),
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

/// List open bills (bill_type = 'open_bill'), most recent first.
#[command]
pub async fn list_open_bills(
    state: State<'_, AppState>,
) -> Result<Vec<oz_core::db::HeldCartRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let carts = store.list_open_bills()?;
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

    #[test]
    fn start_sale_args_defaults_currency() {
        let json = r#"{}"#;
        let args: StartSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.currency, "");
    }

    #[test]
    fn add_line_args_deserialize() {
        let json = r#"{"cart_id":"550e8400-e29b-41d4-a716-446655440000","sku":"COFFEE","qty":3,"unit_price_minor":350}"#;
        let args: AddLineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku.as_str(), "COFFEE");
        assert_eq!(args.qty, 3);
        assert_eq!(args.unit_price_minor, 350);
    }

    #[test]
    fn set_cart_discount_args_deserialize() {
        let json = r#"{"cart_id":"660e8400-e29b-41d4-a716-446655440001","percent":10,"label":"Senior Discount","user_id":"u1"}"#;
        let args: SetCartDiscountArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.percent, 10);
        assert_eq!(args.label, Some("Senior Discount".into()));
        assert_eq!(args.user_id, "u1");
    }

    #[test]
    fn complete_sale_args_deserialize_minimal() {
        let json = r#"{"cart_id":"770e8400-e29b-41d4-a716-446655440002","payment_method":"cash","user_id":"u2"}"#;
        let args: CompleteSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.payment_method, "cash");
        assert!(args.tendered_minor.is_none());
        assert!(args.customer_id.is_none());
        assert!(args.serial_numbers.is_none());
    }
}
