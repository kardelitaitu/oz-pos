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
use oz_core::location_resolver;
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

/// Set a cart discount within the session scope. ADR #7 / ADR-19.
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

    let db = state.db.lock().await;
    let store = Store::new(&db);

    let session = state.resolve_session(&session_token)?;
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_DISCOUNT,
    )?;

    let mut cart = store
        .load_active_cart(&args.cart_id)?
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
    cart.set_discount(percent, args.label);
    store.save_active_cart(&cart, None)?;
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
    /// ADR-19 §5.1: the deduction location locked at cart-start time.
    pub deduction_location_id: Option<String>,
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

    Ok(StartSaleResult {
        cart_id: id,
        deduction_location_id: None,
    })
}

/// Start a new sale in the session scope. ADR #7 / ADR-19 §5.1.
///
/// Resolves the primary deduction location from the workspace instance
/// and locks it on the `active_carts` row at cart-start time.
///
/// Requires `SALES_PROCESS` permission from the resolved session (Bug #4).
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

    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);

    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;

    // Resolve the primary deduction location for this workspace instance.
    let deduction_location_id =
        location_resolver::resolve_primary_location(&db, &session.instance_id, None)
            .unwrap_or_else(|_| location_resolver::get_default_location_id());

    store.save_active_cart(&cart, Some(deduction_location_id.as_str()))?;
    drop(db);

    tracing::info!(
        cart_id = %id,
        deduction_location_id = %deduction_location_id,
        "cart created with deduction location lock (scoped)",
    );

    Ok(StartSaleResult {
        cart_id: id,
        deduction_location_id: Some(deduction_location_id.to_string()),
    })
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

/// List active carts in the session scope. ADR #7.
#[command]
pub async fn list_active_carts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<CartId>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
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

/// Load a cart in the session scope. ADR #7.
#[command]
pub async fn get_active_cart_scoped(
    session_token: String,
    cart_id: CartId,
    state: State<'_, AppState>,
) -> Result<Option<Cart>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
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

/// Add a line to an active cart in the session scope.
///
/// ADR #7 / ADR-19 §5.1: rejects the command when the cart has no
/// `deduction_location_id` lock.
///
/// Requires `SALES_PROCESS` permission from the resolved session (Bug #3).
#[command]
pub async fn add_line_scoped(
    session_token: String,
    args: AddLineArgs,
    state: State<'_, AppState>,
) -> Result<AddLineResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    run_add_line_scoped(&db, &session.user_id, &args)
}

/// Shared business logic: permission-check + add line to cart.
/// Extracted so the `SALES_PROCESS` gate is unit-testable without
/// constructing an `AppState` (Bug #3 fix).
fn run_add_line_scoped(
    db: &rusqlite::Connection,
    user_id: &str,
    args: &AddLineArgs,
) -> Result<AddLineResult, AppError> {
    let store = Store::new(db);

    require_permission_for_user(&store, user_id, oz_core::permissions::SALES_PROCESS)?;

    // ADR-19 §5.1: reject add_line when the cart has no deduction location lock.
    store
        .ensure_cart_deduction_location_lock(&args.cart_id)
        .map_err(|_| {
            AppError::Invalid(format!(
                "cart {} has no deduction location lock — create via start_sale_scoped first",
                args.cart_id
            ))
        })?;

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

/// Override a line price within the session scope. ADR #7.
#[command]
pub async fn override_line_price_scoped(
    session_token: String,
    args: OverrideLinePriceScopedArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let mut cart = store
        .load_active_cart(&args.cart_id)?
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;

    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_OVERRIDE_PRICE,
    )?;

    let currency = cart.currency();
    let new_price = Money {
        minor_units: args.new_price_minor,
        currency,
    };

    let line = cart
        .lines_mut()
        .iter_mut()
        .find(|l| l.id == args.line_id)
        .ok_or_else(|| AppError::Invalid(format!("line not found: {}", args.line_id)))?;

    line.set_overridden_price(new_price)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    store.save_active_cart(&cart, None)?;
    drop(db);

    tracing::info!(cart_id = %args.cart_id, line_id = %args.line_id, new_price_minor = args.new_price_minor, "line price overridden (scoped)");
    Ok(())
}

// ── Get Cart Deduction Location ───────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Info about the deduction location locked on an active cart. ADR-19 §17.
pub struct DeductionLocationInfo {
    /// The location UUID.
    pub location_id: String,
    /// Human-readable location name.
    pub location_name: String,
    /// ISO-8601 timestamp of the last manager override, or `None`.
    pub overridden_at: Option<String>,
}

/// Return the deduction location info for an active cart.
#[command]
pub async fn get_cart_deduction_location(
    cart_id: CartId,
    state: State<'_, AppState>,
) -> Result<Option<DeductionLocationInfo>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_active_cart_deduction_location_info(&cart_id)?;
    drop(db);
    Ok(
        result.map(|(loc_id, loc_name, overridden_at)| DeductionLocationInfo {
            location_id: loc_id,
            location_name: loc_name,
            overridden_at,
        }),
    )
}

// ── Override Deduction Location ───────────────────────────────────────

/// Override the deduction location lock on an active cart.
///
/// **Deprecated for session-scoped auth (ADR #7):** Use
/// `override_cart_deduction_location_scoped` which reads the user ID from
/// the resolved session.
///
/// Requires `SALES_OVERRIDE_PRICE` permission (Bug #2 fix).
#[command]
pub async fn override_cart_deduction_location(
    cart_id: CartId,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    run_override_cart_deduction_location(&db, &user_id, &cart_id)
}

/// Shared business logic: permission-check + override.
/// Extracted so both the deprecated and scoped commands share the same
/// `SALES_OVERRIDE_PRICE` gate, and so the gate is unit-testable without
/// constructing an `AppState`.
fn run_override_cart_deduction_location(
    db: &rusqlite::Connection,
    user_id: &str,
    cart_id: &CartId,
) -> Result<(), AppError> {
    let store = Store::new(db);

    require_permission_for_user(&store, user_id, oz_core::permissions::SALES_OVERRIDE_PRICE)?;

    store
        .override_active_cart_deduction_location(cart_id)
        .map_err(|e| AppError::Internal(format!("failed to override deduction location: {e}")))?;

    tracing::info!(cart_id = %cart_id, user_id = %user_id, "deduction location override recorded");
    Ok(())
}

/// Override the deduction location lock on an active cart (scoped).
///
/// ADR-19 §17: Records the manager override timestamp on the cart.
/// Requires `SALES_OVERRIDE_PRICE` permission from the resolved session.
#[command]
pub async fn override_cart_deduction_location_scoped(
    session_token: String,
    cart_id: CartId,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    run_override_cart_deduction_location(&db, &session.user_id, &cart_id)
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

/// Args for `complete_sale_scoped` — without `user_id`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSaleScopedArgs {
    /// ID of the associated cart.
    pub cart_id: CartId,
    /// Payment Method.
    pub payment_method: String,
    /// Tendered Minor.
    pub tendered_minor: Option<i64>,
    /// Customer ID.
    pub customer_id: Option<String>,
    /// Payment Splits.
    pub payment_splits: Option<Vec<PaymentSplitArg>>,
    /// Customer Name.
    pub customer_name: Option<String>,
    /// Serial Numbers.
    pub serial_numbers: Option<Vec<SerialNumberArg>>,
}

/// Complete a sale within the session scope. ADR #7 / ADR-19 §6.
///
/// Uses the `complete_sale_deduction` path which checks stock at the
/// resolved deduction location, performs per-location deduction, and
/// writes `deduction_locations` JSON on the sale row.
/// Returns `PartialStockResult` as an error when stock is insufficient.
#[command]
pub async fn complete_sale_scoped(
    session_token: String,
    args: CompleteSaleScopedArgs,
    state: State<'_, AppState>,
) -> Result<CompleteSaleResult, AppError> {
    let session = state.resolve_session(&session_token)?;

    // ── Lock 1: Load and remove the cart ──────────────────────────
    let cart = {
        let db = state.db.lock().await;
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
        cart
    };

    let line_count = cart.line_count();

    let mut sale = oz_core::Sale::from_cart_with_user(&cart, Some(session.user_id.clone()))
        .ok_or_else(|| AppError::Invalid("cart total overflowed i64".into()))?;
    sale.payment_method = Some(args.payment_method.clone());
    sale.tendered_minor = args.tendered_minor;
    sale.customer_id = args.customer_id.clone();

    let sale_id = sale.id.clone();

    // ── Lock 2: Compute tax and execute deduction ─────────────────
    let _res = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        store.compute_sale_tax(&mut sale, &[])?;

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
                idempotency_key: None,
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

/// Compute tax within the session scope. ADR #7.
#[command]
pub async fn compute_cart_tax_scoped(
    session_token: String,
    lines: Vec<oz_core::db::CartLineTaxInput>,
    currency: String,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let session = state.resolve_session(&session_token)?;
    let parsed: oz_core::Currency = currency
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency}")))?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
    let tax = store.compute_cart_tax(&lines, parsed)?;
    drop(db);
    Ok(tax.minor_units)
}

// ── Complete Sale With Resolved Shortfalls ───────────────────────────

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
    /// Cart line data reconstructed by the frontend.
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
/// After `complete_sale_scoped` returns a `PartialStockResult` error,
/// the cashier resolves shortfalls via the Stock Shortfall dialog.
/// This command re-checks stock at the resolved locations and deducts
/// accordingly.
#[command]
pub async fn complete_sale_with_resolved_shortfalls_scoped(
    session_token: String,
    args: CompleteSaleWithResolvedShortfallsArgs,
    state: State<'_, AppState>,
) -> Result<CompleteSaleResult, AppError> {
    let session = state.resolve_session(&session_token)?;

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
    if args.discount_percent > 0
        && let Some(pct) = foundation::Percentage::new(args.discount_percent as u8)
    {
        cart.set_discount(pct, args.discount_label.clone());
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
    let _result = {
        let db = state.db.lock().await;
        let store = Store::new(&db);

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
                idempotency_key: None,
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
    /// ADR-19 §6.3: deduction location UUID locked at cart-start time.
    /// When restoring a held cart, the caller should pass the same
    /// `deduction_location_id` that was stored when the cart was held.
    pub deduction_location_id: Option<String>,
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
        args.deduction_location_id.as_deref(),
    )?;
    drop(db);
    tracing::info!(held_cart_id = %id, label = %args.label, "cart held");
    Ok(HoldCartResult { id })
}

/// Park the current sale as a held order (scoped).
#[command]
pub async fn hold_cart_scoped(
    session_token: String,
    args: HoldCartArgs,
    state: State<'_, AppState>,
) -> Result<HoldCartResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
    let id = store.hold_cart(
        &args.label,
        &args.cart_data,
        args.item_count,
        args.total_minor,
        &args.currency,
        &args.bill_type,
        args.customer_name.as_deref(),
        args.deduction_location_id.as_deref(),
    )?;
    drop(db);
    tracing::info!(held_cart_id = %id, label = %args.label, "cart held (scoped)");
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

/// List held carts in the session scope. ADR #7.
#[command]
pub async fn list_held_carts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<oz_core::db::HeldCartRow>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
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

/// List open bills in the session scope. ADR #7.
#[command]
pub async fn list_open_bills_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<oz_core::db::HeldCartRow>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
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

/// Resume a held cart in the session scope. ADR #7.
#[command]
pub async fn get_held_cart_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<oz_core::db::HeldCartFull>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
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

/// Delete a held cart in the session scope. ADR #7.
#[command]
pub async fn delete_held_cart_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::SALES_PROCESS,
    )?;
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
    use oz_core::migrations;
    use rusqlite::Connection;

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
        let json = r#"{"cartId":"550e8400-e29b-41d4-a716-446655440000","sku":"COFFEE","qty":3,"unitPriceMinor":350}"#;
        let args: AddLineArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku.as_str(), "COFFEE");
        assert_eq!(args.qty, 3);
        assert_eq!(args.unit_price_minor, 350);
    }

    #[test]
    fn set_cart_discount_args_deserialize() {
        let json = r#"{"cartId":"660e8400-e29b-41d4-a716-446655440001","percent":10,"label":"Senior Discount","userId":"u1"}"#;
        let args: SetCartDiscountArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.percent, 10);
        assert_eq!(args.label, Some("Senior Discount".into()));
        assert_eq!(args.user_id, "u1");
    }

    #[test]
    fn complete_sale_args_deserialize_minimal() {
        let json = r#"{"cartId":"770e8400-e29b-41d4-a716-446655440002","paymentMethod":"cash","userId":"u2"}"#;
        let args: CompleteSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.payment_method, "cash");
        assert!(args.tendered_minor.is_none());
        assert!(args.customer_id.is_none());
        assert!(args.serial_numbers.is_none());
    }

    // ── Bug #2: override_cart_deduction_location permission check ───

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    /// Seed a user with ONLY sales:process permission (no SALES_OVERRIDE_PRICE).
    fn seed_cashier_without_override_permission(conn: &Connection, user_id: &str) {
        conn.execute_batch(&format!(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-cashier', 'Cashier', 'Cashier', '[\"sales:process\"]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, display_name, role_id, pin_hash, is_active, created_at, updated_at) VALUES
                ('{user_id}', '{user_id}', 'Cashier', 'role-cashier', 'hashed', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        )).unwrap();
    }

    /// Seed a user with SALES_OVERRIDE_PRICE permission.
    fn seed_manager_with_override_permission(conn: &Connection, user_id: &str) {
        conn.execute_batch(&format!(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-manager', 'Manager', 'Manager', '[\"sales:override_price\"]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, display_name, role_id, pin_hash, is_active, created_at, updated_at) VALUES
                ('{user_id}', '{user_id}', 'Manager', 'role-manager', 'hashed', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        )).unwrap();
    }
    /// Insert an active cart row and return its `CartId`.
    /// Also seeds a minimal inventory_location row so the FK on
    /// `deduction_location_id` is satisfied.
    fn seed_active_cart(conn: &Connection) -> CartId {
        // Satisfy the FK from active_carts.deduction_location_id → inventory_locations(id).
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, created_at, updated_at)
             VALUES ('loc-warehouse-1', 'Warehouse', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let cart = oz_core::Cart::new("USD".parse::<Currency>().unwrap());
        let cart_id = cart.id();
        let cart_data = serde_json::to_string(&cart).unwrap();
        conn.execute(
            "INSERT INTO active_carts (id, cart_data, deduction_location_id, updated_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            rusqlite::params![cart_id.to_string(), cart_data, "loc-warehouse-1"],
        )
        .unwrap();
        cart_id
    }

    #[test]
    fn override_cart_deduction_location_rejects_user_without_sales_override_price() {
        // Bug #2: the non-scoped command had NO permission check, so any
        // caller could override a deduction location — a silent privilege
        // bypass. After the fix, a user without SALES_OVERRIDE_PRICE must
        // be rejected before the DB write executes.
        let conn = fresh_conn();
        seed_cashier_without_override_permission(&conn, "user-cashier");
        let cart_id = seed_active_cart(&conn);

        let result = run_override_cart_deduction_location(&conn, "user-cashier", &cart_id);

        assert!(
            result.is_err(),
            "Bug #2: override lacked permission check — \
             cashier without SALES_OVERRIDE_PRICE must be rejected"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.to_lowercase().contains("permission") || err.to_lowercase().contains("denied"),
            "error must mention permission/denied, got: {err}"
        );
    }

    #[test]
    fn override_cart_deduction_location_allows_user_with_sales_override_price() {
        // Happy-path regression: a manager with SALES_OVERRIDE_PRICE should
        // succeed — the permission check must not reject authorised users.
        let conn = fresh_conn();
        seed_manager_with_override_permission(&conn, "user-mgr");
        let cart_id = seed_active_cart(&conn);

        let result = run_override_cart_deduction_location(&conn, "user-mgr", &cart_id);

        assert!(
            result.is_ok(),
            "manager with SALES_OVERRIDE_PRICE must be allowed, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn override_cart_deduction_location_fails_for_nonexistent_cart() {
        // Edge case: permission check passes but the cart doesn't exist.
        let conn = fresh_conn();
        seed_manager_with_override_permission(&conn, "user-mgr");

        // Create a CartId that won't exist in the DB.
        let cart_id = oz_core::Cart::new("USD".parse::<Currency>().unwrap()).id();
        let result = run_override_cart_deduction_location(&conn, "user-mgr", &cart_id);

        assert!(
            result.is_err(),
            "nonexistent cart must fail after permission check"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found") || err.contains("active_cart"),
            "error must mention not-found, got: {err}"
        );
    }

    // ── Bug #3: add_line_scoped session authorization ──────────────

    /// Seed a user with NO sales permissions at all.
    fn seed_user_without_sales_process(conn: &Connection, user_id: &str) {
        conn.execute_batch(&format!(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-no-sales', 'No Sales', 'No sales permissions', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, display_name, role_id, pin_hash, is_active, created_at, updated_at) VALUES
                ('{user_id}', '{user_id}', 'No Sales', 'role-no-sales', 'hashed', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        )).unwrap();
    }

    #[test]
    fn add_line_scoped_rejects_user_without_sales_process() {
        // Bug #3: add_line_scoped resolved the session but stored it as
        // _session (unused). A user without SALES_PROCESS could add lines
        // to any cart — a silent authorization gap. After the fix, the
        // permission check must reject unprivileged users.
        let conn = fresh_conn();
        seed_user_without_sales_process(&conn, "user-no-sales");
        let cart_id = seed_active_cart(&conn);

        let args = AddLineArgs {
            cart_id,
            sku: Sku::new("COFFEE"),
            qty: 1,
            unit_price_minor: 350,
        };
        let result = run_add_line_scoped(&conn, "user-no-sales", &args);

        assert!(
            result.is_err(),
            "Bug #3: add_line_scoped lacked SALES_PROCESS check — \
             user without sales:process must be rejected"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.to_lowercase().contains("permission") || err.to_lowercase().contains("denied"),
            "error must mention permission/denied, got: {err}"
        );
    }

    #[test]
    fn add_line_scoped_allows_user_with_sales_process() {
        // Happy-path regression: a cashier with SALES_PROCESS must be
        // able to add lines to a cart with a deduction_location lock.
        let conn = fresh_conn();
        // seed_cashier_without_override_permission gives the user sales:process
        seed_cashier_without_override_permission(&conn, "user-cashier");
        let cart_id = seed_active_cart(&conn);

        let args = AddLineArgs {
            cart_id,
            sku: Sku::new("LATTE"),
            qty: 2,
            unit_price_minor: 450,
        };
        let result = run_add_line_scoped(&conn, "user-cashier", &args);

        assert!(
            result.is_ok(),
            "cashier with SALES_PROCESS must be allowed to add lines, got: {:?}",
            result.err()
        );
        let r = result.unwrap();
        assert_eq!(r.line_total.unwrap().minor_units, 900);
    }
}
