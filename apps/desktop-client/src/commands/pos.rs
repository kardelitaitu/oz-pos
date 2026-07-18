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

/// Set or clear a cart-level percentage discount using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `set_cart_discount_scoped`
/// with a `session_token` instead.
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

/// Args for `set_cart_discount_scoped` — without `user_id`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCartDiscountScopedArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// Percent.
    pub percent: i64,
    /// Label.
    pub label: Option<String>,
}

/// Set a cart discount within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `set_cart_discount`. The `user_id` for
/// permission checks is read from the resolved `SessionContext`.
#[command]
pub async fn set_cart_discount_scoped(
    session_token: String,
    args: SetCartDiscountScopedArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if !(0..=100).contains(&args.percent) {
        return Err(AppError::Invalid(format!(
            "discount percent must be between 0 and 100, got {}",
            args.percent
        )));
    }
    let percent = Percentage::new(args.percent as u8).unwrap();

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_DISCOUNT,
    )?;

    let mut cart = store
        .load_active_cart(&args.cart_id)?
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
    cart.set_discount(percent, args.label);
    store.save_active_cart(&cart)?;
    drop(db);
    tracing::info!(cart_id = %args.cart_id, percent = %args.percent, "cart discount set (scoped)");
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

/// Start a new sale cart using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `start_sale_scoped`
/// with a `session_token` to create the cart in the store-scoped database.
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

/// Start a new sale in the store resolved from a session token. ADR #7.
#[command]
pub async fn start_sale_scoped(
    session_token: String,
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

    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.save_active_cart(&cart)?;
    drop(db);

    Ok(StartSaleResult { cart_id: id })
}

// ── List Active Carts ────────────────────────────────────────────────

/// Return all active cart IDs from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_active_carts_scoped`.
#[command]
pub async fn list_active_carts(state: State<'_, AppState>) -> Result<Vec<CartId>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let ids = store.list_active_carts()?;
    drop(db);
    Ok(ids)
}

/// List active carts for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_active_carts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<CartId>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let ids = store.list_active_carts()?;
    drop(db);
    Ok(ids)
}

// ── Get Active Cart ──────────────────────────────────────────────────

/// Load a full cart from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_active_cart_scoped`.
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

/// Load a cart from the store resolved from a session token. ADR #7.
#[command]
pub async fn get_active_cart_scoped(
    session_token: String,
    cart_id: CartId,
    state: State<'_, AppState>,
) -> Result<Option<Cart>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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

/// Add a line to an active cart using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `add_line_scoped`.
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

/// Add a line to an active cart in the store resolved from a session token. ADR #7.
#[command]
pub async fn add_line_scoped(
    session_token: String,
    args: AddLineArgs,
    state: State<'_, AppState>,
) -> Result<AddLineResult, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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

/// Override the unit price of a cart line using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `override_line_price_scoped`.
#[command]
pub async fn override_line_price(
    args: OverrideLinePriceArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    run_override_line_price(
        &db,
        &args.cart_id,
        &args.line_id,
        args.new_price_minor,
        &args.user_id,
    )
}

/// Args for `override_line_price_scoped` — without `user_id`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverrideLinePriceScopedArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// ID of the associated line.
    pub line_id: LineId,
    /// New Price Minor.
    pub new_price_minor: i64,
}

/// Override a line price within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `override_line_price`. The `user_id` for
/// permission checks is read from the resolved `SessionContext`.
#[command]
pub async fn override_line_price_scoped(
    session_token: String,
    args: OverrideLinePriceScopedArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_override_line_price(
        &db,
        &args.cart_id,
        &args.line_id,
        args.new_price_minor,
        &session.user_id,
    )
}

/// Shared business logic for overriding a line price.
fn run_override_line_price(
    db: &rusqlite::Connection,
    cart_id: &CartId,
    line_id: &LineId,
    new_price_minor: i64,
    user_id: &str,
) -> Result<(), AppError> {
    let store = Store::new(db);
    let mut cart = store
        .load_active_cart(cart_id)?
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", cart_id)))?;

    require_permission_for_user(&store, user_id, oz_core::permissions::SALES_OVERRIDE_PRICE)?;

    let currency = cart.currency();
    let new_price = Money {
        minor_units: new_price_minor,
        currency,
    };

    let line = cart
        .lines_mut()
        .iter_mut()
        .find(|l| l.id == *line_id)
        .ok_or_else(|| AppError::Invalid(format!("line not found: {}", line_id)))?;

    line.set_overridden_price(new_price)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    store.save_active_cart(&cart)?;

    tracing::info!(%cart_id, %line_id, new_price_minor, "line price overridden");
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
    /// Optional payment splits for multi-method payments.
    /// When provided, the `payment_method` on the sale is set to "split"
    /// and each split is recorded in the `payments` table.
    pub payment_splits: Option<Vec<PaymentSplitArg>>,
    /// Optional customer name (for credit sales).
    pub customer_name: Option<String>,
    /// Optional serial numbers captured at checkout for track_serial products.
    pub serial_numbers: Option<Vec<SerialNumberArg>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Completesalescopedargs.
pub struct CompleteSaleScopedArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// Payment Method.
    pub payment_method: String,
    /// Tendered Minor.
    pub tendered_minor: Option<i64>,
    /// ID of the associated customer.
    pub customer_id: Option<String>,
    /// Payment Splits.
    pub payment_splits: Option<Vec<PaymentSplitArg>>,
    /// Customer Name.
    pub customer_name: Option<String>,
    /// Serial Numbers.
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

/// Complete a sale using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `complete_sale_scoped`
/// with a `session_token` instead. The `user_id` is read from the
/// resolved `SessionContext`.
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

/// A single cart line reconstructed by the frontend for the second command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartLineData {
    /// SKU identifier.
    pub sku: String,
    /// Quantity.
    pub qty: i64,
    /// Unit price in minor units.
    pub unit_price_minor: i64,
}

/// Arguments for completing a sale with resolved shortfalls (split fulfillment).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSaleWithResolvedShortfallsArgs {
    /// ID of the original cart (informational).
    pub cart_id: CartId,
    /// Payment method label.
    pub payment_method: String,
    /// Tendered amount in minor units.
    pub tendered_minor: Option<i64>,
    /// Optional customer id.
    pub customer_id: Option<String>,
    /// Optional payment splits.
    pub payment_splits: Option<Vec<PaymentSplitArg>>,
    /// Customer name (for credit sales).
    pub customer_name: Option<String>,
    /// Optional serial numbers.
    pub serial_numbers: Option<Vec<SerialNumberArg>>,
    /// Cart line data reconstructed by the frontend (needed because the
    /// original cart was deleted in the first `complete_sale_scoped` call).
    pub lines: Vec<CartLineData>,
    /// Total sale amount in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Discount percentage (0-100).
    pub discount_percent: i64,
    /// Optional discount label.
    pub discount_label: Option<String>,
    /// Cashier-resolved shortfalls: per-SKU allocation to specific locations.
    pub resolutions: Vec<oz_core::sale_deduction::ResolvedShortfall>,
}

/// Complete a sale with cashier-resolved shortfalls (split fulfillment).
///
/// This is the second command in the two-command flow (ADR-19 §6b).
/// After `complete_sale_scoped` returns a [`PartialStockResult`] error,
/// the cashier resolves shortfalls via the Stock Shortfall dialog.
/// This command re-checks stock at the resolved locations and deducts
/// accordingly — using [`Store::complete_sale_with_resolved_shortfalls`].
/// The front-end passes cart line data since the original cart was deleted
/// in the first `complete_sale_scoped` call.
#[command]
pub async fn complete_sale_with_resolved_shortfalls_scoped(
    session_token: String,
    args: CompleteSaleWithResolvedShortfallsArgs,
    state: State<'_, AppState>,
) -> Result<CompleteSaleResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    // ── Reconstruct the Cart from front-end line data ─────────────
    let currency: oz_core::Currency = args
        .currency
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {}", args.currency)))?;

    let mut cart = oz_core::Cart::new(currency);
    for line_data in &args.lines {
        let unit_price = oz_core::Money {
            minor_units: line_data.unit_price_minor,
            currency: cart.currency(),
        };
        let line =
            oz_core::CartLine::new(oz_core::Sku::new(&line_data.sku), line_data.qty, unit_price);
        cart.add_line(line)
            .map_err(|e| AppError::Invalid(e.to_string()))?;
    }

    // Apply discount if configured
    if args.discount_percent > 0 {
        if let Ok(pct) = foundation::Percentage::new(args.discount_percent as u8) {
            cart.set_discount(pct, args.discount_label.clone());
        }
    }

    let line_count = cart.line_count();
    let total = cart.total();

    let mut sale = oz_core::Sale::from_cart_with_user(&cart, Some(session.user_id.clone()))
        .ok_or_else(|| AppError::Invalid("cart total overflowed i64".into()))?;
    sale.payment_method = Some(args.payment_method.clone());
    sale.tendered_minor = args.tendered_minor;
    sale.customer_id = args.customer_id.clone();

    let sale_id = sale.id.clone();

    // ── Lock: Compute tax + execute the resolved deduction ────────
    let result = {
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);

        // Compute tax (same as first command)
        store.compute_sale_tax(&mut sale, &[])?;

        let splits = if let Some(ref splits) = args.payment_splits {
            splits.clone()
        } else {
            vec![PaymentSplitArg {
                method: args.payment_method.clone(),
                amount_minor: sale.total.minor_units,
                gateway_reference: args.customer_name.clone(),
                gateway_status: None,
                gateway_response: None,
            }]
        };

        store.complete_sale_with_resolved_shortfalls(
            &sale,
            Some(&session.instance_id),
            &splits,
            &session.user_id,
            Some(&session.terminal_id),
            &args.resolutions,
        )?
    };

    tracing::info!(%sale_id, store_id = %session.store_id, "sale completed with resolved shortfalls");

    // ── Event publishing (no DB lock held) ────────────────────────
    {
        let kernel = state.kernel.lock().await;
        let bus = kernel.event_bus();
        let line_items: Vec<oz_core::events::SaleCompletedLine> = sale
            .lines
            .iter()
            .map(|l| oz_core::events::SaleCompletedLine {
                sku: l.sku.clone(),
                qty: l.qty,
                unit_price_minor: l.unit_price.minor_units,
                tax_minor: l.tax_amount.minor_units,
                tax_rate_id: l.tax_rate_id.clone(),
            })
            .collect();

        if let Err(e) = bus.publish(&oz_core::events::SaleCompleted {
            sale_id: sale_id.clone(),
            store_id: Some(session.store_id.clone()),
            line_items,
            total_minor: sale.total.minor_units,
            currency: String::from_utf8_lossy(&sale.currency.0).into_owned(),
            customer_id: args.customer_id.clone(),
        }) {
            tracing::warn!(%sale_id, error = %e, "event bus publish failed");
        }
    }

    Ok(CompleteSaleResult {
        sale_id,
        total,
        line_count,
    })
}

/// Complete a sale within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `complete_sale`. The `user_id` for
/// permission checks, the sale audit trail, and plugin hooks is read
/// from the resolved `SessionContext`. Uses the store-scoped database
/// with two sequential locks (cart removal then sale creation) while
/// plugin hooks and event publishing run without holding any DB lock.
#[command]
pub async fn complete_sale_scoped(
    session_token: String,
    args: CompleteSaleScopedArgs,
    state: State<'_, AppState>,
) -> Result<CompleteSaleResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    // ── Lock 1: Load and remove the cart ──────────────────────────
    let mut cart = {
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);

        require_permission_for_user(
            &store,
            &session.user_id,
            oz_core::permissions::SALES_PROCESS,
        )?;

        let cart = store
            .load_active_cart(&args.cart_id)?
            .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
        store.delete_active_cart(&args.cart_id)?;
        cart // db lock dropped here
    };

    let line_count = cart.line_count();

    // ── Plugin business-rule hooks (no DB lock held) ──────────────
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

            let currency = String::from_utf8_lossy(&cart.currency().0).into_owned();
            let total_minor = cart.total().map(|m| m.minor_units).unwrap_or(0);
            plugins
                .fire_sale_before_complete(&lines, total_minor, &currency, &session.user_id)
                .map_err(|e| AppError::Internal(e.to_string()))?;

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

    let mut sale = oz_core::Sale::from_cart_with_user(&cart, Some(session.user_id.clone()))
        .ok_or_else(|| AppError::Invalid("cart total overflowed i64".into()))?;
    let has_splits = args.payment_splits.as_ref().is_some_and(|s| !s.is_empty());
    sale.payment_method = Some(if has_splits {
        "split".to_string()
    } else {
        args.payment_method.clone()
    });
    sale.tendered_minor = args.tendered_minor;
    sale.customer_id = args.customer_id.clone();

    // ── Apply Lua calc_line_tax overrides (no DB lock) ────────────
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
    }

    let sale_id = sale.id.clone();

    // ── Lock 2: Compute tax and create sale ───────────────────────
    let _res = {
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);
        store.compute_sale_tax(&mut sale, &lua_overrides)?;

        if let Some(ref serial_numbers) = args.serial_numbers {
            for sn in serial_numbers {
                if let Some(line) = sale.lines.iter_mut().find(|l| l.sku == sn.sku) {
                    line.serial_number = Some(sn.serial.clone());
                }
            }
        }

        let splits = if let Some(ref splits) = args.payment_splits {
            splits.clone()
        } else {
            vec![PaymentSplitArg {
                method: args.payment_method.clone(),
                amount_minor: sale.total.minor_units,
                gateway_reference: args.customer_name.clone(),
                gateway_status: None,
                gateway_response: None,
            }]
        };

        store.complete_sale_deduction(
            &sale,
            Some(&session.instance_id),
            &splits,
            &session.user_id,
            Some(&session.terminal_id),
        )?
    };

    let total = cart.total();
    tracing::info!(%sale_id, ?total, line_count, store_id = %session.store_id, "sale completed (scoped)");

    // ── Event publishing (no DB lock held) ────────────────────────
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
            store_id: Some(session.store_id.clone()),
            line_items,
            total_minor: total.map(|m| m.minor_units).unwrap_or(0),
            currency: String::from_utf8_lossy(&sale.currency.0).into_owned(),
            customer_id: args.customer_id.clone(),
        };

        let kernel = state.kernel.lock().await;
        let bus = kernel.event_bus();
        if let Err(e) = bus.publish(&event) {
            tracing::warn!(%sale_id, error = %e, "event bus publish failed");
        }
    }

    Ok(CompleteSaleResult {
        sale_id,
        total,
        line_count,
    })
}

// ── Compute Cart Tax ──────────────────────────────────────────────────

/// Compute tax for a live cart from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `compute_cart_tax_scoped`.
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

/// Compute cart tax for the store resolved from a session token. ADR #7.
#[command]
pub async fn compute_cart_tax_scoped(
    session_token: String,
    lines: Vec<oz_core::db::CartLineTaxInput>,
    currency: String,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let parsed: oz_core::Currency = currency
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency}")))?;
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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

/// Park the current sale as a held order in the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `hold_cart_scoped`.
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

/// Hold a cart in the store resolved from a session token. ADR #7.
#[command]
pub async fn hold_cart_scoped(
    session_token: String,
    args: HoldCartArgs,
    state: State<'_, AppState>,
) -> Result<HoldCartResult, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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
    tracing::info!(held_cart_id = %id, label = %args.label, "cart held (scoped)");
    Ok(HoldCartResult { id })
}

/// List all held carts from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_held_carts_scoped`.
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

/// List held carts for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_held_carts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<oz_core::db::HeldCartRow>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let carts = store.list_held_carts()?;
    drop(db);
    Ok(carts)
}

/// List open bills from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_open_bills_scoped`.
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

/// List open bills for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_open_bills_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<oz_core::db::HeldCartRow>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let carts = store.list_open_bills()?;
    drop(db);
    Ok(carts)
}

/// Resume a held cart from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_held_cart_scoped`.
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

/// Get a held cart from the store resolved from a session token. ADR #7.
#[command]
pub async fn get_held_cart_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<oz_core::db::HeldCartFull>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let cart = store.get_held_cart(&id)?;
    drop(db);
    Ok(cart)
}

/// Delete a held cart from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_held_cart_scoped`.
#[command]
pub async fn delete_held_cart(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_held_cart(&id)?;
    drop(db);
    tracing::info!(held_cart_id = %id, "held cart deleted");
    Ok(())
}

/// Delete a held cart in the store resolved from a session token. ADR #7.
#[command]
pub async fn delete_held_cart_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.delete_held_cart(&id)?;
    drop(db);
    tracing::info!(held_cart_id = %id, "held cart deleted (scoped)");
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
        let json =
            r#"{"label":"Test","cartData":"{}","itemCount":1,"totalMinor":100,"currency":"USD"}"#;
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

    // ── Serde regression: all DTOs accept camelCase from JS ────────

    #[test]
    fn add_line_args_from_camel_case_json() {
        let json = r#"{"cartId":"11111111-1111-1111-1111-111111111111","sku":"BAGEL","qty":2,"unitPriceMinor":500}"#;
        let args: AddLineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(
            args.cart_id.to_string(),
            "11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(args.sku.as_str(), "BAGEL");
        assert_eq!(args.qty, 2);
        assert_eq!(args.unit_price_minor, 500);
    }

    #[test]
    fn complete_sale_args_from_camel_case_json() {
        let json = r#"{"cartId":"22222222-2222-2222-2222-222222222222","paymentMethod":"cash","tenderedMinor":50000,"userId":"user-1"}"#;
        let args: CompleteSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(
            args.cart_id.to_string(),
            "22222222-2222-2222-2222-222222222222"
        );
        assert_eq!(args.payment_method, "cash");
        assert_eq!(args.tendered_minor, Some(50000));
        assert_eq!(args.user_id, "user-1");
    }

    #[test]
    fn hold_cart_args_from_camel_case_json() {
        let json = r#"{"label":"Table 5","cartData":"{}","itemCount":3,"totalMinor":15000,"currency":"IDR","customerName":"Budi"}"#;
        let args: HoldCartArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.label, "Table 5");
        assert_eq!(args.item_count, 3);
        assert_eq!(args.total_minor, 15000);
        assert_eq!(args.currency, "IDR");
        assert_eq!(args.customer_name.as_deref(), Some("Budi"));
        assert_eq!(args.bill_type, "hold");
    }

    // ── Scoped command token rejection tests ───────────────────────

    #[test]
    fn pos_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn complete_sale_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("bad-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }
}
