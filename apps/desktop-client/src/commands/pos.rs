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
use oz_core::{Cart, CartId, CartLine, LineId, Money, PaymentSplitArg, SaleStatus, Sku};

use crate::commands::authz::require_permission_for_user;
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
    store.save_active_cart(&cart)?;
    drop(db);
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

    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.save_active_cart(&cart)?;
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
    store.save_active_cart(&cart)?;
    drop(db);

    Ok(AddLineResult {
        line_id,
        line_total,
    })
}

// ── Override Line Price ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OverrideLinePriceArgs {
    pub cart_id: CartId,
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

    line.set_overridden_price(new_price);

    store.save_active_cart(&cart)?;
    drop(db);

    tracing::info!(cart_id = %args.cart_id, line_id = %args.line_id, new_price_minor = args.new_price_minor, "line price overridden");
    Ok(())
}

// ── Complete Sale ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SerialNumberArg {
    pub sku: String,
    pub serial: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteSaleArgs {
    pub cart_id: CartId,
    pub payment_method: String,
    pub tendered_minor: Option<i64>,
    pub user_id: String,
    /// Optional customer id to link this sale to a customer
    /// for loyalty tracking and purchase history.
    pub customer_id: Option<String>,
    /// Optional payment splits for multi-method payments.
    /// When provided, the `payment_method` on the sale is set to "split"
    /// and each split is recorded in the `payments` table.
    pub payment_splits: Option<Vec<PaymentSplitArg>>,
    /// Optional customer name (for credit sales).
    pub customer_name: Option<String>,
    /// Optional serial numbers captured at checkout for track_serial products.
    pub serial_numbers: Option<Vec<SerialNumberArg>>,
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
    // Load and remove the cart from the DB in one scope.
    let mut cart = {
        let db = state.db.lock().await;
        let store = Store::new(&db);

        require_permission_for_user(&store, &args.user_id, oz_core::permissions::SALES_PROCESS)?;

        let cart = store
            .load_active_cart(&args.cart_id)?
            .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
        store.delete_active_cart(&args.cart_id)?;
        // `db` / `store` dropped here, cart is now owned.
        cart
    };

    let line_count = cart.line_count();

    // ── Plugin business-rule hooks ────────────────────────────────
    {
        let plugins = state.plugins.lock().await;
        if let Some(ref plugins) = *plugins {
            let lines: Vec<oz_lua::CartLineData> = cart
                .lines()
                .iter()
                .map(|cl| oz_lua::CartLineData {
                    sku: cl.sku.as_str().to_owned(),
                    qty: cl.qty,
                    unit_price_minor: cl.unit_price.minor_units,
                    currency: String::from_utf8_lossy(&cl.unit_price.currency.0).into_owned(),
                })
                .collect();

            // Validate the order via Lua — abort on errors.
            let errors = plugins
                .validate_order(
                    &lines,
                    cart.total().map(|m| m.minor_units).unwrap_or(0),
                    &String::from_utf8_lossy(&cart.currency().0),
                )
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if !errors.is_empty() {
                return Err(AppError::Invalid(format!(
                    "order validation failed: {}",
                    errors.join("; ")
                )));
            }

            // Apply a dynamic discount from Lua.
            if let Some(discount) = plugins
                .apply_discount(&lines)
                .map_err(|e| AppError::Internal(e.to_string()))?
            {
                let label = discount.label.clone().unwrap_or_else(|| "Lua Rule".into());
                if !(0..=100).contains(&discount.percent) {
                    return Err(AppError::Invalid(format!(
                        "Lua returned invalid discount percent: {}",
                        discount.percent
                    )));
                }
                // SAFETY: discount.percent is validated 0..=100 above.
                let lua_pct = Percentage::new(discount.percent as u8).unwrap();
                cart.set_discount(lua_pct, Some(label));
            }

            // Fire sale.before_complete event (registered via oz.register_hook).
            let currency = String::from_utf8_lossy(&cart.currency().0).into_owned();
            let total_minor = cart.total().map(|m| m.minor_units).unwrap_or(0);
            plugins
                .fire_sale_before_complete(&lines, total_minor, &currency, &args.user_id)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // Drain any pending discounts registered by hooks via oz.apply_discount().
            if let Some(pd) = plugins.drain_pending_discounts().into_iter().next() {
                if !(0..=100).contains(&pd.percent) {
                    return Err(AppError::Invalid(format!(
                        "Plugin returned invalid discount percent: {}",
                        pd.percent
                    )));
                }
                let pct = Percentage::new(pd.percent as u8).unwrap();
                cart.set_discount(pct, Some(pd.target));
            }
        }
    }

    let mut sale = oz_core::Sale::from_cart_with_user(&cart, Some(args.user_id))
        .ok_or_else(|| AppError::Invalid("cart total overflowed i64".into()))?;
    let has_splits = args.payment_splits.as_ref().is_some_and(|s| !s.is_empty());
    let payment_method = if has_splits {
        "split".to_string()
    } else {
        args.payment_method.clone()
    };
    sale.payment_method = Some(payment_method);
    sale.tendered_minor = args.tendered_minor;
    sale.customer_id = args.customer_id.clone();

    // ── Apply Lua calc_line_tax overrides before DB tax computation ───
    let mut lua_overrides: Vec<(String, i64, bool)> = Vec::new();
    {
        let plugins = state.plugins.lock().await;
        if let Some(ref plugins) = *plugins {
            for cl in cart.lines() {
                let currency_str = String::from_utf8_lossy(&cl.unit_price.currency.0).into_owned();
                if let Some(override_) = plugins
                    .calc_line_tax(
                        cl.sku.as_str(),
                        cl.qty,
                        cl.unit_price.minor_units,
                        &currency_str,
                    )
                    .map_err(|e| AppError::Internal(e.to_string()))?
                {
                    lua_overrides.push((
                        cl.sku.as_str().to_owned(),
                        override_.rate_bps,
                        override_.is_inclusive,
                    ));
                }
            }
        }
        // plugins lock released here
    }

    let sale_id = sale.id.clone();

    // Scope the DB borrow so Store (which is !Send) is dropped before
    // the next .await point when we lock the kernel for event publishing.
    let updated = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        store.compute_sale_tax(&mut sale, &lua_overrides)?;

        // Match serial numbers from args to sale lines by SKU.
        if let Some(ref serial_numbers) = args.serial_numbers {
            for sn in serial_numbers {
                if let Some(line) = sale.lines.iter_mut().find(|l| l.sku == sn.sku) {
                    line.serial_number = Some(sn.serial.clone());
                }
            }
        }

        store.create_sale(&sale)?;

        // Create payment records for each split (or a single payment record
        // for backward compatibility).
        if let Some(ref splits) = args.payment_splits {
            if !splits.is_empty() {
                store.create_payments(&sale_id, splits, &sale.currency, &sale.created_at)?;
            }
        } else {
            let payment_method = args.payment_method.as_str();
            let single_split = vec![PaymentSplitArg {
                method: payment_method.into(),
                amount_minor: sale.total.minor_units,
                gateway_reference: args.customer_name.clone(),
                gateway_status: None,
                gateway_response: None,
            }];
            store.create_payments(&sale_id, &single_split, &sale.currency, &sale.created_at)?;
        }

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
pub struct HoldCartArgs {
    pub label: String,
    pub cart_data: String,
    pub item_count: i64,
    pub total_minor: i64,
    pub currency: String,
    #[serde(default = "default_bill_type")]
    pub bill_type: String,
    pub customer_name: Option<String>,
}

fn default_bill_type() -> String {
    "hold".to_string()
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

    // ── DTO struct tests ─────────────────────────────────────────────

    #[test]
    fn set_cart_discount_args_debug() {
        let args = SetCartDiscountArgs {
            cart_id: CartId::new(),
            percent: 10,
            label: Some("Senior".into()),
            user_id: "user-1".into(),
        };
        let debug = format!("{args:?}");
        assert!(debug.contains("Senior"));
        assert!(debug.contains("10"));
    }

    #[test]
    fn start_sale_args_default_currency() {
        let json = r#"{}"#;
        let args: StartSaleArgs = serde_json::from_str(json).unwrap();
        assert!(args.currency.is_empty());
    }

    #[test]
    fn start_sale_result_debug() {
        let cart_id = CartId::new();
        let result = StartSaleResult { cart_id };
        let debug = format!("{result:?}");
        assert!(debug.contains("StartSaleResult"));
    }

    #[test]
    fn add_line_args_fields() {
        let args = AddLineArgs {
            cart_id: CartId::new(),
            sku: Sku::new("COFFEE"),
            qty: 3,
            unit_price_minor: 350,
        };
        assert_eq!(args.qty, 3);
        assert_eq!(args.unit_price_minor, 350);
        assert_eq!(args.sku.as_str(), "COFFEE");
    }

    #[test]
    fn serial_number_arg_fields() {
        let arg = SerialNumberArg {
            sku: "LAPTOP".into(),
            serial: "SN12345".into(),
        };
        assert_eq!(arg.sku, "LAPTOP");
        assert_eq!(arg.serial, "SN12345");
    }

    #[test]
    fn hold_cart_args_default_bill_type() {
        let json = r#"{"label":"Test","cart_data":"{}","item_count":1,"total_minor":100,"currency":"USD"}"#;
        let args: HoldCartArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.bill_type, "hold");
    }

    #[test]
    fn complete_sale_result_debug() {
        let result = CompleteSaleResult {
            sale_id: "sale-1".into(),
            total: Some(price(1000)),
            line_count: 2,
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("sale-1"));
        assert!(debug.contains("1000"));
    }
}
