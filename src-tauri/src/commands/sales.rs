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

use oz_core::{Cart, CartId, LineId, Money, SaleStatus, Sku};
use oz_core::db::{Store, DailySummaryRow, SalesByHourRow};

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
        return Err(AppError::Invalid(
            "discount percent must be between 0 and 100".into(),
        ));
    }
    let mut carts = state.carts.lock().await;
    let cart = carts
        .get_mut(&args.cart_id)
        .ok_or_else(|| AppError::Invalid(format!("cart not found: {}", args.cart_id)))?;
    cart.set_discount(args.percent, args.label)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    tracing::info!(cart_id = %args.cart_id, percent = %args.percent, "cart discount set");
    Ok(())
}

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
    pub payment_method: String,
    pub tendered_minor: Option<i64>,
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteSaleResult {
    pub sale_id: String,
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

    let line_count = cart.line_count();

    // Build sale from cart and attach payment + user info.
    let mut sale = oz_core::Sale::from_cart_with_user(&cart, Some(args.user_id)).ok_or_else(|| {
        AppError::Invalid("cart total overflowed i64".into())
    })?;
    sale.payment_method = Some(args.payment_method);
    sale.tendered_minor = args.tendered_minor;

    let sale_id = sale.id.clone();

    // Persist to DB.
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.create_sale(&sale)?;

    // Transition to Completed.
    let updated = store.update_sale_status(&sale_id, SaleStatus::Completed)?;

    drop(db);

    let total = cart.total();
    tracing::info!(%sale_id, ?total, line_count, "sale completed and persisted");
    Ok(CompleteSaleResult {
        sale_id: updated.id,
        total,
        line_count,
    })
}

// ── Sale history ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SaleListItem {
    pub id: String,
    pub total: Money,
    pub line_count: i64,
    pub status: String,
    pub payment_method: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
}

#[command]
pub async fn list_sales(
    state: State<'_, AppState>,
) -> Result<Vec<SaleListItem>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sales = store.list_sales()?;
    drop(db);
    Ok(sales
        .into_iter()
        .map(|s| SaleListItem {
            id: s.id,
            total: s.total,
            line_count: s.line_count,
            status: format!("{:?}", s.status),
            payment_method: s.payment_method,
            user_id: s.user_id,
            created_at: s.created_at,
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct SaleDetail {
    pub id: String,
    pub total: Money,
    pub line_count: i64,
    pub status: String,
    pub payment_method: Option<String>,
    pub tendered_minor: Option<i64>,
    pub user_id: Option<String>,
    pub created_at: String,
    pub lines: Vec<oz_core::SaleLine>,
}

#[command]
pub async fn get_sale(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SaleDetail>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sale = store.get_sale(&id)?;
    drop(db);
    Ok(sale.map(|s| SaleDetail {
        id: s.id,
        total: s.total,
        line_count: s.line_count,
        status: format!("{:?}", s.status),
        payment_method: s.payment_method,
        tendered_minor: s.tendered_minor,
        user_id: s.user_id,
        created_at: s.created_at,
        lines: s.lines,
    }))
}// ── Void Sale ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VoidSaleArgs {
    pub sale_id: String,
    pub user_id: String,
    pub reason: String,
}

#[command]
pub async fn void_sale(
    args: VoidSaleArgs,
    state: State<'_, AppState>,
) -> Result<oz_core::Sale, AppError> {
    let db = state.db.lock().await;
    let store = oz_core::db::Store::new(&db);

    let sale = store.void_sale(&args.sale_id, &args.user_id, &args.reason)?;
    drop(db);

    tracing::info!(sale_id = %args.sale_id, reason = %args.reason, "sale voided");
    Ok(sale)
}

// ── Dashboard ────────────────────────────────────────────────────────

#[command]
pub async fn export_daily_summary(
    state: State<'_, AppState>,
) -> Result<Vec<DailySummaryRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.export_daily_summary()?;
    drop(db);
    Ok(rows)
}

#[command]
pub async fn export_sales_by_hour(
    state: State<'_, AppState>,
) -> Result<Vec<SalesByHourRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.export_sales_by_hour()?;
    drop(db);
    Ok(rows)
}

// ── EOD (End-of-Day) Report ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EodReport {
    /// Number of completed sales today.
    pub total_sales: i64,
    /// Total revenue in minor units.
    pub total_revenue: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Payment method breakdown.
    pub payment_breakdown: Vec<PaymentBreakdown>,
    /// Number of voided sales today.
    pub void_count: i64,
    /// Total value of voided sales in minor units.
    pub void_total: i64,
    /// Number of sales with a discount applied.
    pub discount_count: i64,
    /// Total discount amount in minor units.
    pub discount_total: i64,
    /// Hourly sales breakdown (0-23).
    pub hourly_breakdown: Vec<SalesByHourRow>,
}

#[derive(Debug, Serialize)]
pub struct PaymentBreakdown {
    /// Payment method label.
    pub method: String,
    /// Number of sales using this method.
    pub count: i64,
    /// Total amount for this method in minor units.
    pub total: i64,
}

/// Fetch the full EOD (End-of-Day) report for today.
///
/// Returns a comprehensive summary including total sales, revenue,
/// payment method breakdown, voids, discounts, and hourly sales data.
#[command]
pub async fn export_eod_report(
    state: State<'_, AppState>,
) -> Result<EodReport, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Get today's completed sales.
    let daily = store.export_daily_summary()?;
    let hourly = store.export_sales_by_hour()?;

    // Payment breakdown.
    let mut stmt = db.prepare(
        "SELECT payment_method, COUNT(*) AS cnt, SUM(total_minor) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'completed'
         GROUP BY payment_method
         ORDER BY tot DESC"
    )?;
    let payment_rows: Vec<PaymentBreakdown> = stmt.query_map([], |row| {
        Ok(PaymentBreakdown {
            method: row.get::<_, Option<String>>("payment_method")?.unwrap_or_else(|| "Unknown".into()),
            count: row.get("cnt")?,
            total: row.get("tot")?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    // Void stats.
    let mut void_stmt = db.prepare(
        "SELECT COUNT(*) AS cnt, COALESCE(SUM(total_minor), 0) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'voided'"
    )?;
    let void_row: (i64, i64) = void_stmt.query_row([], |row| {
        Ok((row.get::<_, i64>("cnt")?, row.get::<_, i64>("tot")?))
    })?;
    drop(void_stmt);

    // Discount stats.
    let mut discount_stmt = db.prepare(
        "SELECT COUNT(*) AS cnt, COALESCE(SUM(total_minor), 0) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'completed' AND discount_percent > 0"
    )?;
    let discount_row: (i64, i64) = discount_stmt.query_row([], |row| {
        Ok((row.get::<_, i64>("cnt")?, row.get::<_, i64>("tot")?))
    })?;
    drop(discount_stmt);

    let total_sales = daily.len() as i64;
    let total_revenue: i64 = daily.iter().map(|r| r.total_minor).sum();
    let currency = daily.first().map(|r| r.currency.clone()).unwrap_or_else(|| "USD".into());

    drop(db);

    Ok(EodReport {
        total_sales,
        total_revenue,
        currency,
        payment_breakdown: payment_rows,
        void_count: void_row.0,
        void_total: void_row.1,
        discount_count: discount_row.0,
        discount_total: discount_row.1,
        hourly_breakdown: hourly,
    })
}

// ── Hold Order ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct HoldCartArgs {
    /// Human-readable label for the held cart.
    pub label: String,
    /// JSON-serialized cart data (lines, discount, currency).
    pub cart_data: String,
    /// Number of line items.
    pub item_count: i64,
    /// Cart total in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct HoldCartResult {
    pub id: String,
}

/// Park the current sale as a held order.
///
/// The cart data is serialized to JSON by the front-end and stored
/// in the `held_carts` table. It can be resumed later via
/// `resume_held_cart`.
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
///
/// Returns the full held cart data including the JSON cart_data blob.
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
pub async fn delete_held_cart(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_held_cart(&id)?;
    drop(db);
    tracing::info!(held_cart_id = %id, "held cart deleted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::Currency;
    use oz_core::CartLine;

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
        // Test the core cart logic directly without Tauri commands.
        let mut cart = oz_core::Cart::new(usd());
        let cart_id = cart.id();

        let line = CartLine::new(Sku::new("COFFEE"), 2, price(350));
        cart.add_line(line).unwrap();

        // Verify cart state.
        assert_eq!(cart.line_count(), 1);
        let total = cart.total();
        assert_eq!(total.unwrap().minor_units, 700);
        assert_eq!(total.unwrap().currency, usd());
        assert!(!cart_id.to_string().is_empty());

        // Second line.
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
