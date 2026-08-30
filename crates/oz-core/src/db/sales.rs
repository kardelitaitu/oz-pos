/*
last audited 25-07-26 by RSA-Agent (oz-core slice B2: sales deep read; CLI-1 support 25-07-26: new tx-aware create_sale_in_tx + shared validate_sale_money/insert_sale_with_lines helpers)
crate: oz-core | status: SAFE | lint: CLEAN
findings: money paths exemplary (MONEY-01..04 + TAX-02..06 in-line, checked arithmetic at every IPC boundary, explicit rounding modes, TAX-06 exclusive-total correction); COR-7 MEDIUM: complete_sale_deduction inserts payment splits WITHOUT the idempotency_key column it carries — bypasses create_payments dedup persistence; COR-8 LOW: void_sale comment claims optimistic concurrency but UPDATE has no version CAS (WHERE id only); COR-9 INFO: receipt-barcode lookup swallows DB errors via .ok(); COR-10 INFO: PartialStockResult travels inside Validation.message JSON (documented tradeoff)
next: persist idempotency_key on the sale-path payment insert (COR-7); add version CAS to void_sale (COR-8) | perf: batch SKU lookup avoids N+1; popularity recompute outside tx
*/
//! Sale CRUD — create, list, get, update status, held carts, exports.

use std::collections::HashMap;

use rusqlite::params;

use crate::error::CoreError;
use crate::money::Currency;
use crate::tax_rate::{RoundingMode, TaxRate};
use crate::{AuditEntry, Money, Sale, SaleLine, SaleStatus};

use super::Store;

/// Input for cart-level tax computation (used by IPC command).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CartLineTaxInput {
    /// Product SKU for rate lookup.
    pub sku: String,
    /// Quantity in this line.
    pub qty: i64,
    /// Unit price in minor units.
    pub unit_price_minor: i64,
}

/// Result of cart-level tax computation, including whether any EXCLUSIVE
/// rate applied. The frontend must add `tax_minor` to the payable total
/// ONLY when `has_exclusive` is true — inclusive tax is already embedded
/// in the displayed price, so adding it again would double-charge.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartTaxResult {
    /// Total tax across all lines/rates, in minor units.
    pub tax_minor: i64,
    /// True when at least one applied rate is exclusive (tax added on top
    /// of the price). When false, all rates were inclusive or none applied.
    pub has_exclusive: bool,
}

// ── Export types ─────────────────────────────────────────────────────

/// Row returned by [`Store::export_daily_summary`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailySummaryRow {
    /// Sale unique identifier.
    pub sale_id: String,
    /// Total sale amount in minor units (e.g. cents).
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Number of line items in the sale.
    pub line_count: i64,
    /// Sale status (e.g. "active", "completed", "voided").
    pub status: String,
    /// RFC-3339 timestamp of when the sale was created.
    pub created_at: String,
}

/// Row returned by [`Store::export_sales_by_hour`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct SalesByHourRow {
    /// Hour of day (0–23).
    pub hour: i64,
    /// Total value of all sales in this hour, in minor units.
    pub total_minor: i64,
    /// Number of sales transacted in this hour.
    pub sale_count: i64,
}

/// Summary row for a held (parked) cart.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeldCartRow {
    /// Held cart unique identifier.
    pub id: String,
    /// User-assigned label for the cart.
    pub label: String,
    /// Number of line items in the cart.
    pub item_count: i64,
    /// Cart total in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// RFC-3339 timestamp of when the cart was parked.
    pub created_at: String,
    /// Type of cart: 'hold' or 'open_bill'.
    pub bill_type: String,
    /// Customer name (set when bill_type = 'open_bill').
    pub customer_name: Option<String>,
}

/// Full held cart data including the JSON cart_data blob.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeldCartFull {
    /// Held cart unique identifier.
    pub id: String,
    /// User-assigned label for the cart.
    pub label: String,
    /// JSON-encoded cart state (line items, discounts, etc.).
    pub cart_data: String,
    /// Number of line items in the cart.
    pub item_count: i64,
    /// Cart total in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// RFC-3339 timestamp of when the cart was parked.
    pub created_at: String,
    /// Type of cart: 'hold' or 'open_bill'.
    pub bill_type: String,
    /// Customer name (set when bill_type = 'open_bill').
    pub customer_name: Option<String>,
    /// ADR-19 §6.3: deduction location UUID locked at cart-start time.
    /// `None` for pre-095 held carts or legacy single-location deployments.
    pub deduction_location_id: Option<String>,
}

// ── Sale Deduction (ADR-19) ────────────────────────────────────────

/// MONEY-04: payment splits must cover the recorded sale total before a
/// sale may be persisted.
///
/// Over-tender is allowed (the difference becomes change back to the
/// customer); under-payment is rejected so a hostile IPC caller (or a buggy
/// front-end) cannot complete a sale for less than the ledger total. A
/// negative split is never legitimate and is rejected even when the sum
/// happens to cover the total. Summing uses checked arithmetic so a huge
/// split list cannot overflow past the total.
/// Insert one sale line with the HPP cost snapshot (ADR #36 reporting).
///
/// The product's cost is frozen at write time so historical margins never
/// change when `products.cost_minor` is edited later. NULL when the product
/// is missing or has no cost set — the reporting layer falls back to the
/// product's current cost (and 0) via COALESCE.
fn insert_sale_line(tx: &rusqlite::Transaction<'_>, line: &SaleLine) -> Result<(), CoreError> {
    let unit_cur =
        std::str::from_utf8(&line.unit_price.currency.0).map_err(|e| CoreError::Validation {
            field: "currency",
            message: format!("invalid UTF-8 in currency bytes: {e}"),
        })?;
    // `products.cost_minor` is `NOT NULL DEFAULT 0` — 0 means "cost not
    // set". Normalize it to NULL so an unset snapshot can never shadow a
    // later-set product cost in the reporting COALESCE fallback.
    let cost_minor = match tx.query_row(
        "SELECT cost_minor FROM products WHERE sku = ?1",
        params![line.sku],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(v) if v > 0 => Some(v),
        Ok(_) | Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(CoreError::Db(e)),
    };
    tx.execute(
        "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position,
                                 tax_minor, tax_rate_id, tax_breakdown_json,
                                 serial_number, course, modifiers_json, cost_minor)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            line.id,
            line.sale_id,
            line.sku,
            line.qty,
            line.unit_price.minor_units,
            line.line_total.minor_units,
            unit_cur,
            line.line_position,
            line.tax_amount.minor_units,
            line.tax_rate_id,
            line.tax_breakdown_json,
            line.serial_number,
            line.course,
            line.modifiers_json,
            cost_minor,
        ],
    )?;
    Ok(())
}

fn validate_payment_splits_cover_total(
    splits: &[crate::PaymentSplitArg],
    total_minor: i64,
) -> Result<(), CoreError> {
    let mut sum: i64 = 0;
    for split in splits {
        if split.amount_minor < 0 {
            return Err(CoreError::Validation {
                field: "payments",
                message: format!(
                    "payment split amount must be non-negative, got {}",
                    split.amount_minor
                ),
            });
        }
        sum = sum
            .checked_add(split.amount_minor)
            .ok_or_else(|| CoreError::Validation {
                field: "payments",
                message: "payment split total overflow".into(),
            })?;
    }
    if sum < total_minor {
        return Err(CoreError::Validation {
            field: "payments",
            message: format!("payment splits ({sum}) do not cover the sale total ({total_minor})"),
        });
    }
    Ok(())
}

fn stock_at_locations(
    tx: &rusqlite::Transaction<'_>,
    product_id: &str,
    locations: &[crate::inventory::LocationId],
) -> Result<Vec<crate::sale_deduction::LocationStock>, CoreError> {
    locations
        .iter()
        .map(|location_id| {
            let qty: i64 = tx
                .query_row(
                    "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = ?1 AND location_id = ?2",
                    rusqlite::params![product_id, location_id.as_str()],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let location_name: String = tx
                .query_row(
                    "SELECT name FROM inventory_locations WHERE id = ?1",
                    rusqlite::params![location_id.as_str()],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| location_id.as_str().to_owned());
            Ok(crate::sale_deduction::LocationStock {
                location_id: location_id.clone(),
                location_name,
                qty_available: qty,
            })
        })
        .collect()
}
/// Validate the non-negative money/qty class guarded by MONEY-06/MONEY-07
/// (shared by `create_sale` and `create_sale_in_tx`).
fn validate_sale_money(sale: &Sale) -> Result<(), CoreError> {
    for line in &sale.lines {
        if line.qty < 0 {
            return Err(CoreError::Validation {
                field: "qty",
                message: format!("sale line quantity must be positive, got {}", line.qty),
            });
        }
        if line.line_total.minor_units < 0 {
            return Err(CoreError::Validation {
                field: "line_total",
                message: format!(
                    "sale line total must be non-negative, got {}",
                    line.line_total.minor_units
                ),
            });
        }
        if line.tax_amount.minor_units < 0 {
            return Err(CoreError::Validation {
                field: "tax_amount",
                message: format!(
                    "sale line tax must be non-negative, got {}",
                    line.tax_amount.minor_units
                ),
            });
        }
    }
    if sale.total.minor_units < 0 {
        return Err(CoreError::Validation {
            field: "total",
            message: format!(
                "sale total must be non-negative, got {}",
                sale.total.minor_units
            ),
        });
    }
    if sale.subtotal.minor_units < 0 {
        return Err(CoreError::Validation {
            field: "subtotal",
            message: format!(
                "sale subtotal must be non-negative, got {}",
                sale.subtotal.minor_units
            ),
        });
    }
    if sale.tax_total.minor_units < 0 {
        return Err(CoreError::Validation {
            field: "tax_total",
            message: format!(
                "sale tax total must be non-negative, got {}",
                sale.tax_total.minor_units
            ),
        });
    }
    if let Some(tendered) = sale.tendered_minor
        && tendered < 0
    {
        return Err(CoreError::Validation {
            field: "tendered_minor",
            message: format!("tendered amount must be non-negative, got {tendered}"),
        });
    }
    Ok(())
}

/// Insert the sale row plus its line rows inside the caller's transaction
/// (shared by `create_sale` and `create_sale_in_tx`).
fn insert_sale_with_lines(
    tx: &rusqlite::Transaction<'_>,
    sale: &Sale,
    cur_str: &str,
    status_str: &str,
) -> Result<(), CoreError> {
    tx.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor,
                            discount_percent, discount_label, user_id, created_at, updated_at,
                            subtotal_minor, tax_total_minor, customer_id, version, tenant_id,
                            base_currency, base_total_minor, tender_rate_millionths,
                            tip_minor, service_charge_minor)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1, 'default',
                 ?16, ?17, ?18, ?19, ?20)",
        params![
            sale.id, sale.total.minor_units, cur_str, sale.line_count,
            status_str, sale.payment_method, sale.tendered_minor,
            sale.discount_percent, sale.discount_label, sale.user_id,
            sale.created_at, sale.updated_at,
            sale.subtotal.minor_units, sale.tax_total.minor_units,
            sale.customer_id,
            sale.base_currency, sale.base_total_minor, sale.tender_rate_millionths,
            sale.tip_minor, sale.service_charge_minor,
        ],
    )?;

    for line in &sale.lines {
        insert_sale_line(tx, line)?;
    }
    Ok(())
}
impl Store<'_> {
    /// Complete a sale with location-aware stock deduction (ADR-19 §6).
    ///
    /// This is the shared implementation used by both desktop and tablet
    /// POS commands. It performs the following inside a single `BEGIN IMMEDIATE`:
    ///
    /// 1. Creates an `inventory_transaction` audit session (§9a).
    /// 2. Resolves the primary deduction location via
    ///    [`resolve_primary_location`](crate::location_resolver::resolve_primary_location)
    ///    (tier 1 → explicit override, tier 2 → single-binding, tier 3 →
    ///    multi-binding primary, tier 4 → canonical default).
    /// 3. For each sale line, checks stock at the resolved(primary) location.
    ///    Collects ALL shortfalls before any writes.
    /// 4. If ANY shortfalls exist: ROLLBACK, return
    ///    [`PartialStockResult`](crate::sale_deduction::PartialStockResult)
    ///    with per-SKU shortfall details and available alternatives.
    /// 5. If ALL lines sufficed: calls [`adjust_stock_batch`](crate::db::Store::adjust_stock_batch)
    ///    atomically, creates the sale + payments, writes `deduction_locations`
    ///    JSON on the `sales` row with status = 'pending', COMMIT.
    ///
    /// The `workspace_instance_id` is used to resolve the primary location.
    /// Pass `None` for legacy single-location deployments — the canonical
    /// default UUID is used.
    #[allow(clippy::too_many_arguments)]
    pub fn complete_sale_deduction(
        &self,
        sale: &Sale,
        workspace_instance_id: Option<&str>,
        payment_splits: &[crate::PaymentSplitArg],
        staff_user_id: &str,
        terminal_id: Option<&str>,
    ) -> Result<crate::sale_deduction::CompleteSaleResult, CoreError> {
        let location = crate::location_resolver::resolve_primary_location(
            self.conn,
            workspace_instance_id.unwrap_or("default"),
            None,
        )
        .unwrap_or_else(|_| crate::location_resolver::get_default_location_id());
        self.complete_sale_deduction_with_locations(
            sale,
            workspace_instance_id,
            &[location],
            payment_splits,
            staff_user_id,
            terminal_id,
        )
    }
}

/// Batch-lookup product (id, product_type) for multiple SKUs in a single query.
///
/// Returns a `HashMap<SKU, (product_id, product_type)>`. SKUs not found
/// in the database are absent from the map (caller handles missing entries).
fn batch_lookup_products(
    conn: &rusqlite::Connection,
    skus: &[&str],
) -> Result<HashMap<String, (String, String)>, CoreError> {
    if skus.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders: Vec<String> = (0..skus.len()).map(|i| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT sku, id, product_type FROM products WHERE sku IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = skus
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (sku, id, ptype) = row?;
        map.insert(sku, (id, ptype));
    }
    Ok(map)
}

impl Store<'_> {
    /// Complete a sale by greedily allocating each tracked line across the
    /// topology-selected locations in route order. All stock checks,
    /// deductions, sale persistence, and deduction provenance remain inside
    /// one SQLite transaction; an underfunded route set rolls back entirely.
    #[allow(clippy::too_many_arguments)]
    pub fn complete_sale_deduction_with_locations(
        &self,
        sale: &Sale,
        workspace_instance_id: Option<&str>,
        stock_locations: &[crate::inventory::LocationId],
        payment_splits: &[crate::PaymentSplitArg],
        _staff_user_id: &str,
        _terminal_id: Option<&str>,
    ) -> Result<crate::sale_deduction::CompleteSaleResult, CoreError> {
        use crate::inventory_transaction::InventoryTransactionId;
        use crate::sale_deduction::{Shortfall, StockDeduction};

        // MONEY-03 follow-up: a negative line qty would record a negative
        // ledger total AND credit stock (the deduction delta is `-qty`, which
        // is positive when qty is negative). CartLine asserts qty > 0, but this
        // is the ledger boundary — reject hostile or hand-built sales up front.
        for line in &sale.lines {
            if line.qty < 0 {
                return Err(CoreError::Validation {
                    field: "qty",
                    message: format!("sale line quantity must be positive, got {}", line.qty),
                });
            }
        }

        // ADR-19 §5.2: single transaction prevents two concurrent sales from
        // racing on the same inventory row. Same pattern as create_sale().
        let tx = self.conn.unchecked_transaction()?;

        // ── Resolve topology route order ──────────────────────────
        let default_location = crate::location_resolver::get_default_location_id();
        let stock_locations = if stock_locations.is_empty() {
            std::slice::from_ref(&default_location)
        } else {
            stock_locations
        };
        let primary_location = stock_locations[0].clone();

        // ── Phase 1: stock check + shortfall collection ────────────
        let mut deductions: Vec<StockDeduction> = Vec::with_capacity(sale.lines.len());
        let mut line_deductions: HashMap<String, Vec<crate::sale_deduction::LocationAllocation>> =
            HashMap::new();
        let mut shortfalls: Vec<Shortfall> = Vec::new();

        // Batch-lookup all SKUs in a single query (avoids N+1 per-line SELECT).
        let skus: Vec<&str> = sale.lines.iter().map(|l| l.sku.as_str()).collect();
        let product_map = batch_lookup_products(&tx, &skus)?;

        for line in &sale.lines {
            let Some((pid, ptype_str)) = product_map.get(line.sku.as_str()) else {
                shortfalls.push(Shortfall {
                    sku: line.sku.clone(),
                    product_name: line.sku.clone(),
                    requested_qty: line.qty,
                    primary_qty_available: 0,
                    deficit: line.qty,
                    primary_location_id: primary_location.clone(),
                    alternatives: vec![],
                });
                continue;
            };
            let ptype = crate::product::ProductType::parse_str(ptype_str).unwrap_or_default();
            let tracks_inventory = ptype.tracks_inventory();
            let recipe = self.get_recipe_ingredients(pid)?;
            let has_recipe = !recipe.is_empty();

            if !tracks_inventory && !has_recipe {
                // Skip checking stock for service products that do not have a recipe.
                continue;
            }

            // 1. Check composite product stock if it tracks inventory
            if tracks_inventory {
                let availability = stock_at_locations(&tx, pid, stock_locations)?;
                if let Some(allocations) =
                    crate::sale_deduction::allocate_stock_in_route_order(line.qty, &availability)
                {
                    deductions.extend(allocations.iter().map(|allocation| StockDeduction {
                        sku: line.sku.clone(),
                        location_id: allocation.location_id.clone(),
                        delta: -allocation.qty,
                    }));
                    line_deductions.insert(line.id.to_string(), allocations);
                } else {
                    let available = availability
                        .first()
                        .map(|location| location.qty_available)
                        .unwrap_or(0);
                    let alternatives = if stock_locations.len() > 1 {
                        availability
                            .into_iter()
                            .skip(1)
                            .filter(|a| a.qty_available > 0)
                            .collect()
                    } else if let Some(ws_id) = workspace_instance_id {
                        crate::location_resolver::resolve_location_chain_for_sku(
                            &tx, ws_id, &line.sku, line.qty,
                        )
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|a| a.location_id != primary_location)
                        .collect()
                    } else {
                        vec![]
                    };

                    shortfalls.push(Shortfall {
                        sku: line.sku.clone(),
                        product_name: line.sku.clone(),
                        requested_qty: line.qty,
                        primary_qty_available: available,
                        deficit: line.qty.saturating_sub(available),
                        primary_location_id: primary_location.clone(),
                        alternatives,
                    });
                }
            }

            // 2. Check BOM ingredients if has recipe
            if has_recipe {
                for ingredient in recipe {
                    // Load ingredient product details
                    let ing_info: Option<(String, String, String)> = match tx.query_row(
                        "SELECT sku, name, product_type FROM products WHERE id = ?1",
                        rusqlite::params![ingredient.ingredient_product_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    ) {
                        Ok(val) => Some(val),
                        Err(rusqlite::Error::QueryReturnedNoRows) => None,
                        Err(e) => return Err(CoreError::Db(e)),
                    };

                    if let Some((ing_sku, ing_name, ing_ptype_str)) = ing_info {
                        let ing_ptype = crate::product::ProductType::parse_str(&ing_ptype_str)
                            .unwrap_or_default();
                        if ing_ptype.tracks_inventory() {
                            // MONEY-03: line.qty arrives from untrusted IPC input
                            // and must use checked arithmetic like `compute_line_tax`
                            // (TAX-04). Dev/test builds disable overflow checks, so
                            // a bare `*` silently wraps and completes the sale with
                            // a corrupt stock delta.
                            let required_qty = line
                                .qty
                                .checked_mul(ingredient.quantity_required)
                                .ok_or_else(|| CoreError::Validation {
                                field: "qty",
                                message: "ingredient deduction quantity overflow".into(),
                            })?;
                            let availability = stock_at_locations(
                                &tx,
                                &ingredient.ingredient_product_id,
                                stock_locations,
                            )?;
                            if let Some(allocations) =
                                crate::sale_deduction::allocate_stock_in_route_order(
                                    required_qty,
                                    &availability,
                                )
                            {
                                deductions.extend(allocations.iter().map(|allocation| {
                                    StockDeduction {
                                        sku: ing_sku.clone(),
                                        location_id: allocation.location_id.clone(),
                                        delta: -allocation.qty,
                                    }
                                }));
                            } else {
                                let available = availability
                                    .first()
                                    .map(|location| location.qty_available)
                                    .unwrap_or(0);
                                let alternatives = if stock_locations.len() > 1 {
                                    availability
                                        .into_iter()
                                        .skip(1)
                                        .filter(|a| a.qty_available > 0)
                                        .collect()
                                } else if let Some(ws_id) = workspace_instance_id {
                                    crate::location_resolver::resolve_location_chain_for_sku(
                                        &tx,
                                        ws_id,
                                        &ing_sku,
                                        required_qty,
                                    )
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|a| a.location_id != primary_location)
                                    .collect()
                                } else {
                                    vec![]
                                };

                                shortfalls.push(Shortfall {
                                    sku: ing_sku,
                                    product_name: ing_name,
                                    requested_qty: required_qty,
                                    primary_qty_available: available,
                                    deficit: required_qty.saturating_sub(available),
                                    primary_location_id: primary_location.clone(),
                                    alternatives,
                                });
                            }
                        }
                    }
                }
            }
        }

        // ── Shortfall path: rollback, return PartialStockResult ───
        if !shortfalls.is_empty() {
            tx.rollback()?;
            // Return as PartialStockResult via a dedicated error type is
            // cleaner, but for now we use the standard result type pattern.
            // The caller (Tauri command) matches on the return variant.
            return Err(CoreError::Validation {
                field: "stock",
                message: serde_json::to_string(&crate::sale_deduction::PartialStockResult {
                    requires_resolution: true,
                    shortfalls,
                })
                .unwrap_or_else(|_| "shortfalls serialization failed".into()),
            });
        }

        // MONEY-04: validate payment splits against the ledger total AFTER
        // stock resolution (so the PartialStockResult dialog keeps precedence)
        // but BEFORE any write — the error path rolls the whole tx back.
        validate_payment_splits_cover_total(payment_splits, sale.total.minor_units)?;

        // ── Phase 2: execute deductions ───────────────────────────
        let deduct_tx_id = InventoryTransactionId::new();
        let term_id = _terminal_id.map(crate::terminal::TerminalId::from);
        let user_id = crate::user::UserId::from(_staff_user_id.to_owned());
        self.adjust_stock_batch(
            &tx,
            &deductions,
            Some("sale"),
            None,
            term_id.as_ref(),
            Some(&user_id),
        )?;

        // ── Write deduction_locations JSON ────────────────────────
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let deduction_json = serde_json::json!({
            "version": 1,
            "lines": sale.lines.iter().map(|line| {
                let deductions = line_deductions
                    .get(&line.id.to_string())
                    .map(|allocations| {
                        allocations
                            .iter()
                            .map(|allocation| serde_json::json!({
                                "location_id": allocation.location_id.as_str(),
                                "qty": allocation.qty,
                                "sold_at": now,
                            }))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| vec![serde_json::json!({
                        "location_id": primary_location.as_str(),
                        "qty": line.qty,
                        "sold_at": now,
                    })]);
                serde_json::json!({
                    "sale_line_id": line.id,
                    "sku": line.sku,
                    "deductions": deductions,
                })
            }).collect::<Vec<_>>()
        })
        .to_string();

        // ── Persist sale + payments ───────────────────────────────
        let cur_str = std::str::from_utf8(&sale.currency.0).map_err(|e| CoreError::Validation {
            field: "currency",
            message: format!("invalid UTF-8 in currency bytes: {e}"),
        })?;

        // ADR-20 §6: pending_expires_at = NOW + 30 min for stale-reaper.
        let pending_expires_at = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(30))
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .unwrap_or_else(|| now.clone());

        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                                 tendered_minor, discount_percent, discount_label, user_id,
                                 created_at, updated_at, subtotal_minor, tax_total_minor,
                                 customer_id, deduction_locations, version,
                                 pending_expires_at, tenant_id,
                                 base_currency, base_total_minor, tender_rate_millionths,
                                 tip_minor, service_charge_minor)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1, ?16, 'default',
                     ?17, ?18, ?19, ?20, ?21)",
            rusqlite::params![
                sale.id, sale.total.minor_units, cur_str, sale.line_count,
                sale.payment_method, sale.tendered_minor,
                sale.discount_percent, sale.discount_label, sale.user_id,
                sale.created_at, now,
                sale.subtotal.minor_units, sale.tax_total.minor_units,
                sale.customer_id, deduction_json, pending_expires_at,
                sale.base_currency, sale.base_total_minor, sale.tender_rate_millionths,
                sale.tip_minor, sale.service_charge_minor,
            ],
        )?;

        for line in &sale.lines {
            insert_sale_line(&tx, line)?;
        }

        // Create payment records.
        if !payment_splits.is_empty() {
            for split in payment_splits {
                let payment_id = uuid::Uuid::now_v7().to_string();
                tx.execute(
                    "INSERT INTO payments (id, sale_id, method, amount_minor, currency,
                                           gateway_reference, gateway_status, gateway_response,
                                           created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        payment_id,
                        sale.id,
                        split.method,
                        split.amount_minor,
                        cur_str,
                        split.gateway_reference,
                        split.gateway_status,
                        split.gateway_response,
                        now,
                    ],
                )?;
            }
        }

        tx.commit()?;

        Ok(crate::sale_deduction::CompleteSaleResult {
            sale_id: sale.id.clone(),
            status: SaleStatus::Pending,
            receipt_number: sale.id.clone(),
            deduct_tx_id,
        })
    }

    /// Transition a pending sale's status to `completed` after payment capture is successful.
    pub fn finalize_sale(&self, sale_id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE sales SET status = 'completed', updated_at = ?1, version = version + 1 \
             WHERE id = ?2 AND status = 'pending'",
            rusqlite::params![
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                sale_id
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Same as [`Store::finalize_sale`] but inside a caller-owned transaction
    /// (used by the sync daemon's atomic remote-apply path — a nested
    /// `unchecked_transaction` there would fail with "cannot start a
    /// transaction within a transaction").
    pub fn finalize_sale_in_tx(
        tx: &rusqlite::Transaction<'_>,
        sale_id: &str,
    ) -> Result<(), CoreError> {
        tx.execute(
            "UPDATE sales SET status = 'completed', updated_at = ?1, version = version + 1 \
             WHERE id = ?2 AND status = 'pending'",
            rusqlite::params![
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                sale_id
            ],
        )?;
        Ok(())
    }

    /// Complete a sale with cashier-resolved shortfalls (ADR-19 §6b).
    ///
    /// This is the second command in the two-command shortfall resolution flow.
    /// After [`complete_sale_deduction`](Self::complete_sale_deduction) returns a
    /// [`PartialStockResult`](crate::sale_deduction::PartialStockResult), the cashier
    /// resolves each shortfall via the Stock Shortfall dialog (pick alternative
    /// locations, split fulfillment, or manager override).
    ///
    /// The function:
    ///   1. Opens `BEGIN IMMEDIATE` (fresh transaction — the first attempt was rolled back).
    ///   2. Re-checks stock at ALL specified locations for each SKU in the resolutions.
    ///      If any location now has insufficient stock (another terminal sold the item
    ///      while the dialog was shown), returns [`CoreError::InsufficientStockAtLocation`].
    ///   3. Executes all deductions via [`adjust_stock_batch`](Self::adjust_stock_batch).
    ///   4. Writes `deduction_locations` JSON with per-line per-location breakdown.
    ///   5. Creates the sale row with `status = 'pending'`.
    ///   6. Creates payment records.
    ///   7. COMMIT.
    ///
    /// Returns [`CompleteSaleResult`](crate::sale_deduction::CompleteSaleResult) on success.
    #[allow(clippy::too_many_arguments)]
    pub fn complete_sale_with_resolved_shortfalls(
        &self,
        sale: &Sale,
        workspace_instance_id: Option<&str>,
        payment_splits: &[crate::PaymentSplitArg],
        staff_user_id: &str,
        terminal_id: Option<&str>,
        resolutions: &[crate::sale_deduction::ResolvedShortfall],
    ) -> Result<crate::sale_deduction::CompleteSaleResult, CoreError> {
        use crate::inventory_transaction::InventoryTransactionId;
        use crate::sale_deduction::ResolvedShortfall;

        // MONEY-03 follow-up: same negative-qty rejection as
        // complete_sale_deduction — a negative qty would credit stock.
        for line in &sale.lines {
            if line.qty < 0 {
                return Err(CoreError::Validation {
                    field: "qty",
                    message: format!("sale line quantity must be positive, got {}", line.qty),
                });
            }
        }

        // ── BEGIN IMMEDIATE ───────────────────────────────────────
        let tx = self.conn.unchecked_transaction()?;

        // ── Phase 1: Build deduction list from resolutions ────────
        let mut deductions: Vec<crate::sale_deduction::StockDeduction> = Vec::new();
        // Track per-line per-location breakdown for deduction_locations JSON
        let mut line_deductions: Vec<serde_json::Value> = Vec::with_capacity(sale.lines.len());

        // Build a lookup: sku → ResolvedShortfall
        let resolutions_by_sku: std::collections::HashMap<&str, &ResolvedShortfall> =
            resolutions.iter().map(|r| (r.sku.as_str(), r)).collect();

        // Resolve primary/default location once for non-resolution lines.
        let primary_location = crate::location_resolver::resolve_primary_location(
            &tx,
            workspace_instance_id.unwrap_or("default"),
            None,
        )
        .unwrap_or_else(|_| crate::location_resolver::get_default_location_id());

        for line in &sale.lines {
            // Check product info to determine if this line tracks inventory
            let product_info: Option<(String, String)> = match tx.query_row(
                "SELECT id, product_type FROM products WHERE sku = ?1",
                rusqlite::params![line.sku],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ) {
                Ok(val) => Some(val),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(e) => return Err(CoreError::Db(e)),
            };

            let tracks_inventory = product_info
                .as_ref()
                .map(|(_, pt)| {
                    crate::product::ProductType::parse_str(pt)
                        .unwrap_or_default()
                        .tracks_inventory()
                })
                .unwrap_or(false);

            let recipe = match product_info.as_ref() {
                Some((pid, _)) => self.get_recipe_ingredients(pid).unwrap_or_default(),
                None => vec![],
            };
            let has_recipe = !recipe.is_empty();

            let needs_stock = tracks_inventory || has_recipe;

            // If this line has a resolution, use the resolved allocations.
            // Otherwise, for tracked lines, deduct from the primary location.
            if let Some(resolution) = resolutions_by_sku.get(line.sku.as_str()) {
                // Validate allocation sums match requested qty
                let alloc_sum: i64 = resolution.allocations.iter().map(|a| a.qty).sum();
                if alloc_sum != line.qty {
                    tx.rollback()?;
                    return Err(CoreError::Validation {
                        field: "resolutions",
                        message: format!(
                            "SKU {}: allocation sum {} does not match requested qty {}",
                            line.sku, alloc_sum, line.qty
                        ),
                    });
                }

                // Check stock at each location and build deduction entries
                for alloc in &resolution.allocations {
                    if alloc.qty <= 0 {
                        continue;
                    }

                    // Resolve product_id from SKU
                    let product_id: String = tx
                        .query_row(
                            "SELECT id FROM products WHERE sku = ?1",
                            rusqlite::params![line.sku],
                            |row| row.get(0),
                        )
                        .map_err(|_| CoreError::NotFound {
                            entity: "product",
                            id: line.sku.clone(),
                        })?;

                    // Re-check availability at this location
                    let available: i64 = tx
                        .query_row(
                            "SELECT COALESCE(qty, 0) FROM stock_summary \
                             WHERE item_id = ?1 AND location_id = ?2",
                            rusqlite::params![product_id, alloc.location_id.as_str()],
                            |row| row.get(0),
                        )
                        .unwrap_or(0);

                    if available < alloc.qty {
                        // Allow negative stock check: does this binding allow it?
                        let allow_neg = if let Some(ws_id) = workspace_instance_id {
                            tx.query_row(
                                "SELECT COALESCE(allow_negative_stock, 0) \
                                 FROM workspace_inventory_locations \
                                 WHERE instance_id = ?1 AND location_id = ?2",
                                rusqlite::params![ws_id, alloc.location_id.as_str()],
                                |row| row.get::<_, i64>(0),
                            )
                            .unwrap_or(0)
                                == 1
                        } else {
                            false
                        };

                        if !allow_neg {
                            tx.rollback()?;
                            return Err(CoreError::InsufficientStockAtLocation {
                                sku: line.sku.clone(),
                                location_id: alloc.location_id.clone(),
                                requested_delta: alloc.qty,
                                available_qty: available,
                            });
                        }
                    }

                    deductions.push(crate::sale_deduction::StockDeduction {
                        sku: line.sku.clone(),
                        location_id: alloc.location_id.clone(),
                        delta: -alloc.qty,
                    });
                }
            } else if needs_stock {
                // Lines NOT in resolutions but that track inventory still need
                // stock deduction because the entire first sale transaction was
                // rolled back. Deduct from the primary location.
                if tracks_inventory {
                    deductions.push(crate::sale_deduction::StockDeduction {
                        sku: line.sku.clone(),
                        location_id: primary_location.clone(),
                        delta: -line.qty,
                    });
                }

                // BOM ingredients for non-resolution lines
                if has_recipe {
                    for ingredient in recipe {
                        let ing_info: Option<(String, String)> = match tx.query_row(
                            "SELECT sku, product_type FROM products WHERE id = ?1",
                            rusqlite::params![ingredient.ingredient_product_id],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        ) {
                            Ok(val) => Some(val),
                            Err(rusqlite::Error::QueryReturnedNoRows) => None,
                            Err(e) => return Err(CoreError::Db(e)),
                        };

                        if let Some((ing_sku, ing_ptype_str)) = ing_info {
                            let ing_ptype = crate::product::ProductType::parse_str(&ing_ptype_str)
                                .unwrap_or_default();
                            if ing_ptype.tracks_inventory() {
                                // MONEY-03: same overflow contract as the primary
                                // deduction path — the non-resolution BOM branch
                                // must reject an overflowing line qty up front.
                                let required_qty = line
                                    .qty
                                    .checked_mul(ingredient.quantity_required)
                                    .ok_or_else(|| CoreError::Validation {
                                        field: "qty",
                                        message: "ingredient deduction quantity overflow".into(),
                                    })?;
                                deductions.push(crate::sale_deduction::StockDeduction {
                                    sku: ing_sku,
                                    location_id: primary_location.clone(),
                                    delta: -required_qty,
                                });
                            }
                        }
                    }
                }
            }

            // Build deduction_locations entry for this line
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            if let Some(resolution) = resolutions_by_sku.get(line.sku.as_str()) {
                let deductions_entry: Vec<serde_json::Value> = resolution
                    .allocations
                    .iter()
                    .filter(|a| a.qty > 0)
                    .map(|a| {
                        serde_json::json!({
                            "location_id": a.location_id.as_str(),
                            "qty": a.qty,
                            "sold_at": now
                        })
                    })
                    .collect();

                line_deductions.push(serde_json::json!({
                    "sale_line_id": line.id,
                    "sku": line.sku,
                    "deductions": deductions_entry,
                }));
            } else {
                // Non-resolution lines: single-location deduction at primary
                line_deductions.push(serde_json::json!({
                    "sale_line_id": line.id,
                    "sku": line.sku,
                    "deductions": [{
                        "location_id": primary_location.as_str(),
                        "qty": line.qty,
                        "sold_at": now
                    }]
                }));
            }
        }

        // MONEY-04: same ledger-integrity contract as complete_sale_deduction.
        validate_payment_splits_cover_total(payment_splits, sale.total.minor_units)?;

        // ── Phase 2: Execute deductions ───────────────────────────
        let deduct_tx_id = InventoryTransactionId::new();
        let term_id = terminal_id.map(crate::terminal::TerminalId::from);
        let user_id = crate::user::UserId::from(staff_user_id.to_owned());
        self.adjust_stock_batch(
            &tx,
            &deductions,
            Some("sale"),
            None,
            term_id.as_ref(),
            Some(&user_id),
        )?;

        // ── Phase 3: Persist sale + payments ──────────────────────
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let cur_str = std::str::from_utf8(&sale.currency.0).map_err(|e| CoreError::Validation {
            field: "currency",
            message: format!("invalid UTF-8 in currency bytes: {e}"),
        })?;

        let deduction_json = serde_json::json!({
            "version": 1,
            "lines": line_deductions,
        })
        .to_string();

        // ADR-20 §6: pending_expires_at = NOW + 30 min for stale-reaper.
        let pending_expires_at = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(30))
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .unwrap_or_else(|| now.clone());

        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                                 tendered_minor, discount_percent, discount_label, user_id,
                                 created_at, updated_at, subtotal_minor, tax_total_minor,
                                 customer_id, deduction_locations, version,
                                 pending_expires_at, tenant_id,
                                 base_currency, base_total_minor, tender_rate_millionths,
                                 tip_minor, service_charge_minor)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1, ?16, 'default',
                     ?17, ?18, ?19, ?20, ?21)",
            rusqlite::params![
                sale.id, sale.total.minor_units, cur_str, sale.line_count,
                sale.payment_method, sale.tendered_minor,
                sale.discount_percent, sale.discount_label, sale.user_id,
                sale.created_at, now,
                sale.subtotal.minor_units, sale.tax_total.minor_units,
                sale.customer_id, deduction_json, pending_expires_at,
                sale.base_currency, sale.base_total_minor, sale.tender_rate_millionths,
                sale.tip_minor, sale.service_charge_minor,
            ],
        )?;

        for line in &sale.lines {
            insert_sale_line(&tx, line)?;
        }

        if !payment_splits.is_empty() {
            for split in payment_splits {
                let payment_id = uuid::Uuid::now_v7().to_string();
                tx.execute(
                    "INSERT INTO payments (id, sale_id, method, amount_minor, currency,
                                           gateway_reference, gateway_status, gateway_response,
                                           created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        payment_id,
                        sale.id,
                        split.method,
                        split.amount_minor,
                        cur_str,
                        split.gateway_reference,
                        split.gateway_status,
                        split.gateway_response,
                        now,
                    ],
                )?;
            }
        }

        tx.commit()?;

        // ADR #37 D3: recompute popularity for every sold SKU — the sale
        // ledger rows are durable now, so the sales signal (decayed units)
        // reflects this transaction. Runs outside the tx (read-only pass).
        for line in &sale.lines {
            if let Err(e) = self.recompute_popularity(line.sku.as_str()) {
                tracing::warn!(sku = %line.sku, error = %e, "popularity recompute failed after sale");
            }
        }

        Ok(crate::sale_deduction::CompleteSaleResult {
            sale_id: sale.id.clone(),
            status: foundation::SaleStatus::Pending,
            receipt_number: sale.id.clone(),
            deduct_tx_id,
        })
    }

    /// Void a pending sale and restore the reserved/deducted stock back to original locations.
    pub fn void_pending_sale(&self, sale_id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        let deduction_locations_json: String = tx
            .query_row(
                "SELECT deduction_locations FROM sales WHERE id = ?1 AND status = 'pending'",
                rusqlite::params![sale_id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound {
                    entity: "pending sale",
                    id: sale_id.to_owned(),
                },
                other => CoreError::Db(other),
            })?;

        let v: serde_json::Value =
            serde_json::from_str(&deduction_locations_json).map_err(|e| CoreError::Validation {
                field: "deduction_locations",
                message: e.to_string(),
            })?;

        if let Some(lines) = v["lines"].as_array() {
            for line in lines {
                let sku = line["sku"].as_str().ok_or_else(|| CoreError::Validation {
                    field: "sku",
                    message: "missing sku in deduction_locations".into(),
                })?;
                if let Some(deductions) = line["deductions"].as_array() {
                    for d in deductions {
                        let loc_id =
                            d["location_id"]
                                .as_str()
                                .ok_or_else(|| CoreError::Validation {
                                    field: "location_id",
                                    message: "missing location_id in deductions".into(),
                                })?;
                        let qty = d["qty"].as_i64().ok_or_else(|| CoreError::Validation {
                            field: "qty",
                            message: "missing qty in deductions".into(),
                        })?;

                        // Credit stock back (positive delta)
                        self.adjust_stock_at_location_with_reason(
                            &tx,
                            sku,
                            qty,
                            &crate::inventory::LocationId::from(loc_id),
                            Some("void_pending"),
                            None,
                            None,
                            None,
                        )?;
                    }
                }
            }
        }

        tx.execute(
            "UPDATE sales SET status = 'voided', updated_at = ?1, version = version + 1 \
             WHERE id = ?2 AND status = 'pending'",
            rusqlite::params![
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                sale_id
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Find all pending sales whose `pending_expires_at` is in the past.
    ///
    /// ADR-20 §6: uses the partial index `idx_sales_pending_expires` (created
    /// by migration 096) for efficient lookups. A sale is considered "stale"
    /// when `pending_expires_at < datetime('now')` — the 30-min expiry window
    /// was set at creation time in `complete_sale_deduction`.
    /// Find all pending sales whose `pending_expires_at` is in the past.
    ///
    /// ADR-20 §6: uses the partial index `idx_sales_pending_expires` (created
    /// by migration 096) for efficient lookups. A sale is considered "stale"
    /// when `pending_expires_at < NOW` — the 30-min expiry window was set at
    /// creation time in `complete_sale_deduction`.
    ///
    /// The threshold is computed in Rust using the exact same format
    /// (`chrono::SecondsFormat::Millis`) as the stored `pending_expires_at`
    /// values, avoiding format mismatches with SQLite's `strftime`.
    pub fn find_stale_pending_sales(&self) -> Result<Vec<String>, CoreError> {
        let now_rfc = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut stmt = self.conn.prepare(
            "SELECT id FROM sales \
             WHERE status = 'pending' \
               AND pending_expires_at IS NOT NULL \
               AND pending_expires_at < ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![now_rfc], |row| row.get(0))?;
        rows.map(|r| r.map_err(CoreError::from)).collect()
    }

    /// Auto-void all stale pending sales whose `pending_expires_at` has passed.
    ///
    /// ADR-20 §6: intended to be called every 60 seconds by a background
    /// worker. Each stale sale is voided via [`void_pending_sale`](Self::void_pending_sale)
    /// which credits stock back to original deduction locations.
    ///
    /// Returns the number of stale sales that were voided.
    ///
    /// # Errors
    ///
    /// - Returns `CoreError::Db` if the query fails.
    /// - Individual void failures are logged but do NOT abort the batch.
    pub fn reap_stale_pending_sales(&self) -> Result<u32, CoreError> {
        let stale = self.find_stale_pending_sales()?;
        let mut count = 0u32;
        for sale_id in &stale {
            if let Err(e) = self.void_pending_sale(sale_id) {
                // Log but don't abort — other stale sales may succeed.
                tracing::warn!("failed to void stale pending sale {}: {}", sale_id, e);
            } else {
                count += 1;
            }
        }
        Ok(count)
    }
}

// ── Sale CRUD ────────────────────────────────────────────────────

impl Store<'_> {
    fn row_to_sale_line(row: &rusqlite::Row) -> rusqlite::Result<SaleLine> {
        let unit_cur_str: String = row.get("currency")?;
        let currency: Currency = unit_cur_str.parse::<Currency>().map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
            )
        })?;
        Ok(SaleLine {
            id: row.get("id")?,
            sale_id: row.get("sale_id")?,
            sku: row.get("sku")?,
            qty: row.get("qty")?,
            unit_price: Money {
                minor_units: row.get("unit_minor")?,
                currency,
            },
            line_total: Money {
                minor_units: row.get("line_minor")?,
                currency,
            },
            line_position: row.get("line_position")?,
            tax_amount: Money {
                minor_units: row.get("tax_minor")?,
                currency,
            },
            tax_rate_id: row.get("tax_rate_id")?,
            tax_breakdown_json: row.get("tax_breakdown_json")?,
            serial_number: row.get("serial_number")?,
            course: row.get("course")?,
            modifiers_json: row.get("modifiers_json")?,
        })
    }

    /// Persist a [`Sale`] (header + all line items) inside a single transaction.
    pub fn create_sale(&self, sale: &Sale) -> Result<(), CoreError> {
        // MONEY-07: this legacy global-db door deserializes a Sale straight from
        // import/CLI JSON (oz-cli) — CartLine::new's qty > 0 assert never runs.
        // Reject the same negative money/qty class MONEY-06 guards on the
        // complete_sale* entry points, or a hostile import writes negative
        // ledger rows. Zero-total (free) sales with empty lines stay legal.
        validate_sale_money(sale)?;

        let cur_str = std::str::from_utf8(&sale.currency.0).map_err(|e| CoreError::Validation {
            field: "currency",
            message: format!("invalid UTF-8 in currency bytes: {e}"),
        })?;
        let status_str = sale.status.as_stored_str();

        let tx = self.conn.unchecked_transaction()?;

        insert_sale_with_lines(&tx, sale, cur_str, status_str)?;

        tx.commit()?;
        Ok(())
    }

    /// Tx-aware variant of [`Self::create_sale`] for callers already inside
    /// a transaction (CLI `.ozpkg` import — CLI-1 fix).
    ///
    /// The caller's transaction wraps the sale insert plus its line rows,
    /// so a multi-type import commits atomically and the pre-fix nested
    /// "cannot start a transaction within a transaction" failure is
    /// impossible.
    ///
    /// # Invariant
    ///
    /// `tx` must be an open transaction on the same connection this `Store`
    /// wraps. (`self` is not dereferenced — it exists so the method stays on
    /// the Store facade alongside `create_sale`.)
    pub fn create_sale_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        sale: &Sale,
    ) -> Result<(), CoreError> {
        validate_sale_money(sale)?;

        let cur_str = std::str::from_utf8(&sale.currency.0).map_err(|e| CoreError::Validation {
            field: "currency",
            message: format!("invalid UTF-8 in currency bytes: {e}"),
        })?;
        let status_str = sale.status.as_stored_str();

        insert_sale_with_lines(tx, sale, cur_str, status_str)
    }

    /// List all sales ordered by creation date (most recent first), without line items.
    pub fn list_sales(&self) -> Result<Vec<Sale>, CoreError> {
        self.list_sales_sql("FROM sales ORDER BY created_at DESC")
    }

    /// List sales, optionally restricted to the last `days` days (C1.2 Free-tier
    /// sales-history cap).
    ///
    /// Returns the sales plus whether the cap was applied (`days.is_some()`), so
    /// the caller can surface an upgrade teaser when history was truncated.
    /// `created_at` is stored as RFC-3339 text, so the lexicographic comparison
    /// `created_at >= date('now', '-N days')` keeps every sale on/after that
    /// day's midnight.
    pub fn list_sales_with_history_cap(
        &self,
        days: Option<i64>,
    ) -> Result<(Vec<Sale>, bool), CoreError> {
        let mut clause = String::from("FROM sales");
        if let Some(d) = days {
            clause.push_str(&format!(" WHERE created_at >= date('now', '-{d} days')"));
        }
        clause.push_str(" ORDER BY created_at DESC");
        Ok((self.list_sales_sql(&clause)?, days.is_some()))
    }

    /// Shared sale-list query: the given `FROM …` clause is appended to the
    /// standard sale column projection.
    fn list_sales_sql(&self, from_clause: &str) -> Result<Vec<Sale>, CoreError> {
        let sql = format!(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version,
                    base_currency, base_total_minor, tender_rate_millionths,
                    tip_minor, service_charge_minor
             {from_clause}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse::<Currency>().map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
                )
            })?;
            let status = SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency,
                },
                line_count: row.get("line_count")?,
                currency,
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row
                    .get::<_, Option<i64>>("discount_percent")
                    .unwrap_or(Some(0))
                    .unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(),
                subtotal: Money {
                    minor_units: row.get("subtotal_minor")?,
                    currency,
                },
                tax_total: Money {
                    minor_units: row.get("tax_total_minor")?,
                    currency,
                },
                customer_id: row.get("customer_id")?,
                base_currency: row.get("base_currency")?,
                base_total_minor: row.get("base_total_minor")?,
                tender_rate_millionths: row.get("tender_rate_millionths")?,
                tip_minor: row.get("tip_minor")?,
                service_charge_minor: row.get("service_charge_minor")?,
                version: row.get("version").unwrap_or(1),
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List sales visible to one store (soft-scoping layer, migration
    /// 069/117), most recent first, without line items.
    ///
    /// A store sees the shared global sales (`store_id IS NULL`) plus its
    /// own tagged rows — never another store's rows. In the per-store
    /// database model every row is NULL, so this degenerates to the global
    /// list; it is the enforcement surface for shared/cloud databases
    /// where `store_id` is the soft-scoping column.
    pub fn list_sales_for_store(&self, store_id: &str) -> Result<Vec<Sale>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version,
                    base_currency, base_total_minor, tender_rate_millionths,
                    tip_minor, service_charge_minor
             FROM sales
             WHERE store_id IS NULL OR store_id = ?1
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![store_id], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse::<Currency>().map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
                )
            })?;
            let status = SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency,
                },
                line_count: row.get("line_count")?,
                currency,
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row
                    .get::<_, Option<i64>>("discount_percent")
                    .unwrap_or(Some(0))
                    .unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(),
                subtotal: Money {
                    minor_units: row.get("subtotal_minor")?,
                    currency,
                },
                tax_total: Money {
                    minor_units: row.get("tax_total_minor")?,
                    currency,
                },
                customer_id: row.get("customer_id")?,
                base_currency: row.get("base_currency")?,
                base_total_minor: row.get("base_total_minor")?,
                tender_rate_millionths: row.get("tender_rate_millionths")?,
                tip_minor: row.get("tip_minor")?,
                service_charge_minor: row.get("service_charge_minor")?,
                version: row.get("version").unwrap_or(1),
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List sales filtered by user_id (most recent first).
    ///
    /// Multi-terminal: when combined with the shifts table (which maps
    /// user_id + terminal_id), this enables terminal-grouped reporting.
    /// Example: SELECT terminal_id, SUM(total_minor) FROM sales JOIN shifts
    /// ON sales.user_id = shifts.user_id WHERE shifts.status = 'closed'
    /// GROUP BY terminal_id;
    pub fn list_sales_by_user(&self, user_id: &str) -> Result<Vec<Sale>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version,
                    base_currency, base_total_minor, tender_rate_millionths,
                    tip_minor, service_charge_minor
             FROM sales
             WHERE user_id = ?1
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse::<Currency>().map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
                )
            })?;
            let status = SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency,
                },
                line_count: row.get("line_count")?,
                currency,
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row.get("discount_percent")?,
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                subtotal: Money {
                    minor_units: row.get("subtotal_minor")?,
                    currency,
                },
                tax_total: Money {
                    minor_units: row.get("tax_total_minor")?,
                    currency,
                },
                customer_id: row.get("customer_id")?,
                base_currency: row.get("base_currency")?,
                base_total_minor: row.get("base_total_minor")?,
                tender_rate_millionths: row.get("tender_rate_millionths")?,
                tip_minor: row.get("tip_minor")?,
                service_charge_minor: row.get("service_charge_minor")?,
                version: row.get("version")?,
                lines: vec![],
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List sales for one customer (most recent first), without line items.
    ///
    /// CUST-05: powers the customer history view. The result is bounded and
    /// sorted explicitly; the total count lets the caller paginate. Returns
    /// an empty vector (and total 0) when the customer has no sales yet.
    pub fn list_sales_for_customer(
        &self,
        customer_id: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(Vec<Sale>, u64), CoreError> {
        let bounded = limit.clamp(1, 100);
        let total: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sales WHERE customer_id = ?1",
            params![customer_id],
            |row| row.get(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version,
                    base_currency, base_total_minor, tender_rate_millionths,
                    tip_minor, service_charge_minor
             FROM sales WHERE customer_id = ?1
             ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![customer_id, bounded, offset], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse::<Currency>().map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
                )
            })?;
            let status = SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency,
                },
                line_count: row.get("line_count")?,
                currency,
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row
                    .get::<_, Option<i64>>("discount_percent")
                    .unwrap_or(Some(0))
                    .unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(),
                subtotal: Money {
                    minor_units: row.get("subtotal_minor")?,
                    currency,
                },
                tax_total: Money {
                    minor_units: row.get("tax_total_minor")?,
                    currency,
                },
                customer_id: row.get("customer_id")?,
                base_currency: row.get("base_currency")?,
                base_total_minor: row.get("base_total_minor")?,
                tender_rate_millionths: row.get("tender_rate_millionths")?,
                tip_minor: row.get("tip_minor")?,
                service_charge_minor: row.get("service_charge_minor")?,
                version: row.get("version").unwrap_or(1),
            })
        })?;
        let items = rows
            .map(|r| Ok(r?))
            .collect::<Result<Vec<_>, CoreError>>()?;
        Ok((items, total))
    }

    /// Look up a single sale by id, including all line items.
    pub fn get_sale(&self, id: &str) -> Result<Option<Sale>, CoreError> {
        let mut sale_stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version,
                    base_currency, base_total_minor, tender_rate_millionths,
                    tip_minor, service_charge_minor
             FROM sales WHERE id = ?1",
        )?;

        let sale_result = sale_stmt.query_row(params![id], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse::<Currency>().map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()).into(),
                )
            })?;
            let status = SaleStatus::from_stored_str(&status_str).unwrap_or(SaleStatus::Pending);
            Ok(Sale {
                id: row.get("id")?,
                status,
                total: Money {
                    minor_units: row.get("total_minor")?,
                    currency,
                },
                line_count: row.get("line_count")?,
                currency,
                payment_method: row.get("payment_method")?,
                tendered_minor: row.get("tendered_minor")?,
                discount_percent: row
                    .get::<_, Option<i64>>("discount_percent")
                    .unwrap_or(Some(0))
                    .unwrap_or(0),
                discount_label: row.get("discount_label")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                lines: Vec::new(),
                subtotal: Money {
                    minor_units: row.get("subtotal_minor")?,
                    currency,
                },
                tax_total: Money {
                    minor_units: row.get("tax_total_minor")?,
                    currency,
                },
                customer_id: row.get("customer_id")?,
                base_currency: row.get("base_currency")?,
                base_total_minor: row.get("base_total_minor")?,
                tender_rate_millionths: row.get("tender_rate_millionths")?,
                tip_minor: row.get("tip_minor")?,
                service_charge_minor: row.get("service_charge_minor")?,
                version: row.get("version").unwrap_or(1),
            })
        });

        let mut sale = match sale_result {
            Ok(s) => s,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut line_stmt = self.conn.prepare(
            "SELECT id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position,
                    tax_minor, tax_rate_id, tax_breakdown_json, serial_number, course,
                    modifiers_json
             FROM sale_lines WHERE sale_id = ?1 ORDER BY line_position",
        )?;
        let line_rows = line_stmt.query_map(params![id], Self::row_to_sale_line)?;
        for line in line_rows {
            sale.lines.push(line?);
        }

        Ok(Some(sale))
    }

    /// Update the status of a sale, validating the state machine transition.
    pub fn update_sale_status(&self, id: &str, to: SaleStatus) -> Result<Sale, CoreError> {
        let result = self.conn.query_row(
            "SELECT status FROM sales WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        );

        let current_str = match result {
            Ok(s) => s,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(CoreError::NotFound {
                    entity: "sale",
                    id: id.to_owned(),
                });
            }
            Err(e) => return Err(e.into()),
        };

        let current =
            SaleStatus::from_stored_str(&current_str).ok_or_else(|| CoreError::Validation {
                field: "status",
                message: format!("invalid stored status: {current_str}"),
            })?;

        if !SaleStatus::can_transition_to(current, to) {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("cannot transition from {:?} to {:?}", current, to),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let status_str = to.as_stored_str();
        let rows = self.conn.execute(
            "UPDATE sales SET status = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
            params![status_str, now, id],
        )?;
        if rows == 0 {
            return Err(CoreError::Conflict {
                entity: "sale",
                field: "version",
            });
        }

        self.get_sale(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: id.to_owned(),
        })
    }
}

// ── Export / Report queries ─────────────────────────────────────────

impl Store<'_> {
    /// Query all sales for today, ordered chronologically.
    pub fn export_daily_summary(&self) -> Result<Vec<DailySummaryRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status, created_at
             FROM sales WHERE date(created_at) = date('now') ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DailySummaryRow {
                sale_id: row.get("id")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                line_count: row.get("line_count")?,
                status: row.get("status")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Query sales volume grouped by hour of day (for today).
    pub fn export_sales_by_hour(&self) -> Result<Vec<SalesByHourRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%H', created_at) AS INTEGER) AS hour,
                    SUM(total_minor) AS total_minor, COUNT(*) AS sale_count
             FROM sales WHERE date(created_at) = date('now')
             GROUP BY hour ORDER BY hour",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SalesByHourRow {
                hour: row.get("hour")?,
                total_minor: row.get("total_minor")?,
                sale_count: row.get("sale_count")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }
}

// ── Held Carts ──────────────────────────────────────────────────────

impl Store<'_> {
    /// Persist a cart as a held (parked) order or open bill.
    ///
    /// `bill_type` should be `"hold"` or `"open_bill"`. When `customer_name`
    /// is set, it is stored alongside the cart for open-bill listing.
    ///
    /// `deduction_location_id` — ADR-19 §5.3 / §6.3: the deduction location
    /// locked on the active cart at cart-start time. Pass `None` for legacy
    /// single-location deployments. When restoring a held cart, the caller
    /// should set this value on the new active cart via `save_active_cart`.
    #[allow(clippy::too_many_arguments)]
    pub fn hold_cart(
        &self,
        label: &str,
        cart_data: &str,
        item_count: i64,
        total_minor: i64,
        currency: &str,
        bill_type: &str,
        customer_name: Option<&str>,
        deduction_location_id: Option<&str>,
    ) -> Result<String, CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO held_carts (id, label, cart_data, item_count, total_minor, currency, bill_type, customer_name, deduction_location_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                label.trim(),
                cart_data,
                item_count,
                total_minor,
                currency,
                bill_type,
                customer_name,
                deduction_location_id,
            ],
        )?;
        Ok(id)
    }

    /// List all held (parked) orders, most recent first.
    pub fn list_held_carts(&self) -> Result<Vec<HeldCartRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, item_count, total_minor, currency, created_at, bill_type, customer_name
             FROM held_carts ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HeldCartRow {
                id: row.get("id")?,
                label: row.get("label")?,
                item_count: row.get("item_count")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                created_at: row.get("created_at")?,
                bill_type: row.get("bill_type")?,
                customer_name: row.get("customer_name")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List only open bills (bill_type = 'open_bill'), most recent first.
    pub fn list_open_bills(&self) -> Result<Vec<HeldCartRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, item_count, total_minor, currency, created_at, bill_type, customer_name
             FROM held_carts WHERE bill_type = 'open_bill' ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HeldCartRow {
                id: row.get("id")?,
                label: row.get("label")?,
                item_count: row.get("item_count")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                created_at: row.get("created_at")?,
                bill_type: row.get("bill_type")?,
                customer_name: row.get("customer_name")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a held cart by id.
    pub fn get_held_cart(&self, id: &str) -> Result<Option<HeldCartFull>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, cart_data, item_count, total_minor, currency, created_at, bill_type, customer_name, deduction_location_id
             FROM held_carts WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(HeldCartFull {
                id: row.get("id")?,
                label: row.get("label")?,
                cart_data: row.get("cart_data")?,
                item_count: row.get("item_count")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                created_at: row.get("created_at")?,
                bill_type: row.get("bill_type")?,
                customer_name: row.get("customer_name")?,
                deduction_location_id: row.get("deduction_location_id")?,
            })
        });
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a held cart by id.
    pub fn delete_held_cart(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM held_carts WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "held_cart",
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}

// ── Receipt Barcodes ──────────────────────────────────────────────────

impl Store<'_> {
    /// Store a receipt barcode mapping for a sale.
    pub fn save_receipt_barcode(&self, sale_id: &str, barcode: &str) -> Result<(), CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO receipt_barcodes (id, sale_id, barcode) VALUES (?1, ?2, ?3)",
            params![id, sale_id, barcode],
        )?;
        Ok(())
    }

    /// Look up a sale by its receipt barcode.
    pub fn lookup_sale_by_receipt_barcode(&self, barcode: &str) -> Result<Option<Sale>, CoreError> {
        let sale_id: Option<String> = self
            .conn
            .query_row(
                "SELECT sale_id FROM receipt_barcodes WHERE barcode = ?1",
                params![barcode],
                |row| row.get(0),
            )
            .ok();

        match sale_id {
            Some(id) => self.get_sale(&id),
            None => Ok(None),
        }
    }
}

// ── Void Sale ───────────────────────────────────────────────────────

impl Store<'_> {
    /// Void a sale — sets status to Voided and logs an audit entry.
    /// Does NOT adjust inventory; stock is managed independently.
    pub fn void_sale(&self, sale_id: &str, user_id: &str, reason: &str) -> Result<Sale, CoreError> {
        let sale = self.get_sale(sale_id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: sale_id.to_owned(),
        })?;

        if sale.status != SaleStatus::Active {
            return Err(CoreError::Validation {
                field: "status",
                message: format!(
                    "only active sales can be voided (current: {:?})",
                    sale.status
                ),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let tx = self.conn.unchecked_transaction()?;

        // 1. Update status to Voided with optimistic concurrency (ADR #6).
        let rows = tx.execute(
            "UPDATE sales SET status = 'voided', updated_at = ?1, version = version + 1 WHERE id = ?2",
            rusqlite::params![now, sale_id],
        )?;
        if rows == 0 {
            return Err(CoreError::Conflict {
                entity: "sale",
                field: "version",
            });
        }

        // 2. Audit log entry.
        let details = serde_json::json!({
            "reason": reason,
            "total_minor": sale.total.minor_units,
        })
        .to_string();
        let audit = AuditEntry::new(
            user_id,
            "sale.void",
            Some("sale"),
            Some(sale_id),
            Some(details),
            "success",
        );
        self.log_audit(&audit)?;

        tx.commit()?;

        self.get_sale(sale_id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: sale_id.to_owned(),
        })
    }
}

// ── Tax Computation ───────────────────────────────────────────────────

/// Compute the tax contribution for one line/rate pair with integer-safe
/// arithmetic (TAX-04) and an explicit rounding policy (TAX-05).
///
/// Exclusive tax: `base * bps / 10_000`; inclusive tax:
/// `base * bps / (10_000 + bps)`. Uses checked multiplication and
/// division so an extreme (bounded) rate or large amount cannot panic
/// or silently truncate through overflow. The fractional result is then
/// reduced per `mode` ([`RoundingMode::Truncate`] for legacy behavior,
/// [`RoundingMode::HalfUp`] as the recommended default).
fn compute_line_tax(
    base_minor: i64,
    rate_bps: i64,
    is_inclusive: bool,
    currency: Currency,
    mode: RoundingMode,
) -> Result<Money, CoreError> {
    let numerator = base_minor
        .checked_mul(rate_bps)
        .ok_or_else(|| CoreError::Validation {
            field: "tax",
            message: "tax multiplication overflow".into(),
        })?;
    let divisor = if is_inclusive {
        10_000i64
            .checked_add(rate_bps)
            .ok_or_else(|| CoreError::Validation {
                field: "rate_bps",
                message: "inclusive divisor overflow".into(),
            })?
    } else {
        10_000i64
    };
    let tax_minor = mode
        .divide(numerator, divisor)
        .ok_or_else(|| CoreError::Validation {
            field: "tax",
            message: "tax rounding overflow".into(),
        })?;
    Ok(Money {
        minor_units: tax_minor,
        currency,
    })
}

impl Store<'_> {
    /// Compute tax breakdown for a sale in-place.
    ///
    /// For each line resolves ALL applicable tax rates via the chain:
    /// 1. Product-level tax rates (`get_product_tax_rates`)
    /// 2. Category-level tax rates (via the product's `category_id`)
    /// 3. Default store-wide tax rate (where `is_default = true`)
    ///
    /// `lua_overrides` — per-SKU tax rate overrides from plugins.
    /// When a SKU is present in `lua_overrides` its `(rate_bps, is_inclusive)`
    /// values are used instead of the DB-resolved rates for that line.
    ///
    /// All rates for a line contribute to its total tax. Stores the
    /// first rate's id in `tax_rate_id` for backward compatibility.
    /// Updates each line's `tax_amount`, then sets `sale.subtotal`
    /// and `sale.tax_total`.
    ///
    /// `mode` controls how fractional per-rate results are rounded
    /// (TAX-05): pass [`RoundingMode::HalfUp`] for new sales and
    /// [`RoundingMode::Truncate`] when reproducing legacy behavior.
    pub fn compute_sale_tax(
        &self,
        sale: &mut Sale,
        lua_overrides: &[(String, i64, bool)],
        mode: RoundingMode,
    ) -> Result<(), CoreError> {
        let currency = sale.currency;
        let mut total_tax: Option<Money> = None;
        let mut subtotal: Option<Money> = None;
        // TAX-06: exclusive-tax contributions tracked separately so the
        // sale total reflects the true collectible amount. Inclusive tax
        // is embedded in the displayed price (total already includes it);
        // exclusive tax must be added to the total.
        let mut exclusive_tax: Option<Money> = None;

        // MONEY-02 follow-up: reject negative line totals in a pre-pass so a
        // hand-built `Sale` cannot record negative tax on the ledger, and so
        // the error path leaves no partially-mutated Sale behind. CartLine
        // asserts qty > 0 so this is unreachable from the front-end, but this
        // is the tax boundary.
        for line in &sale.lines {
            if line.line_total.minor_units < 0 {
                return Err(CoreError::Validation {
                    field: "line_total",
                    message: format!(
                        "line total must be non-negative, got {}",
                        line.line_total.minor_units
                    ),
                });
            }
        }

        for line in &mut sale.lines {
            let line_subtotal = line.line_total;
            let mut line_tax = Money::zero(currency);
            // TAX-02: per-rate breakdown persisted on the line so multi-rate
            // detail survives (state + local, etc.) even if a rate is later
            // archived/renamed. `tax_rate_id` keeps only the FIRST rate id.
            let mut line_breakdown: Vec<serde_json::Value> = Vec::new();

            // Check for a Lua plugin override first.
            let override_idx = lua_overrides
                .iter()
                .position(|(sku, _, _)| sku == &line.sku);

            if let Some(idx) = override_idx {
                let (_, rate_bps, is_inclusive) = &lua_overrides[idx];
                let rbps = *rate_bps;
                let tax = compute_line_tax(
                    line_subtotal.minor_units,
                    rbps,
                    *is_inclusive,
                    line_subtotal.currency,
                    mode,
                )?;
                line_tax = line_tax
                    .checked_add(tax)
                    .ok_or_else(|| CoreError::Validation {
                        field: "tax",
                        message: "line tax overflow".into(),
                    })?;
                // TAX-06: track exclusive tax for the total correction.
                if !is_inclusive {
                    exclusive_tax = Some(match exclusive_tax {
                        None => tax,
                        Some(acc) => acc.checked_add(tax).ok_or_else(|| CoreError::Validation {
                            field: "tax",
                            message: "exclusive tax accumulation overflow".into(),
                        })?,
                    });
                }
                // No DB tax_rate_id for override lines.
                line.tax_rate_id = None;
                line_breakdown.push(serde_json::json!({
                    "rate_id": null,
                    "rate_bps": rbps,
                    "is_inclusive": *is_inclusive,
                    "tax_minor": tax.minor_units,
                }));
            } else {
                let rates = self.resolve_best_tax_rates_for_sku(&line.sku)?;

                for rate in &rates {
                    let tax = compute_line_tax(
                        line_subtotal.minor_units,
                        rate.rate_bps,
                        rate.is_inclusive,
                        line_subtotal.currency,
                        mode,
                    )?;
                    line_tax = line_tax
                        .checked_add(tax)
                        .ok_or_else(|| CoreError::Validation {
                            field: "tax",
                            message: "line tax overflow".into(),
                        })?;
                    // TAX-06: track exclusive tax for the total correction.
                    if !rate.is_inclusive {
                        exclusive_tax = Some(match exclusive_tax {
                            None => tax,
                            Some(acc) => {
                                acc.checked_add(tax).ok_or_else(|| CoreError::Validation {
                                    field: "tax",
                                    message: "exclusive tax accumulation overflow".into(),
                                })?
                            }
                        });
                    }
                    line_breakdown.push(serde_json::json!({
                        "rate_id": rate.id,
                        "rate_bps": rate.rate_bps,
                        "is_inclusive": rate.is_inclusive,
                        "tax_minor": tax.minor_units,
                    }));
                }

                line.tax_rate_id = rates.first().map(|r| r.id.clone());
            }

            line.tax_breakdown_json =
                if line_breakdown.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&line_breakdown).map_err(|e| {
                        CoreError::Internal(format!("serializing tax breakdown: {e}"))
                    })?)
                };

            line.tax_amount = line_tax;

            total_tax = match total_tax {
                None => Some(line_tax),
                Some(acc) => {
                    Some(
                        acc.checked_add(line_tax)
                            .ok_or_else(|| CoreError::Validation {
                                field: "tax",
                                message: "sale tax total overflow".into(),
                            })?,
                    )
                }
            };

            subtotal =
                match subtotal {
                    None => Some(line.line_total),
                    Some(acc) => Some(acc.checked_add(line.line_total).ok_or_else(|| {
                        CoreError::Validation {
                            field: "subtotal",
                            message: "sale subtotal overflow".into(),
                        }
                    })?),
                };
        }

        // A sale always has ≥ 1 line (the loop above runs once per line), so
        // `subtotal`/`total_tax` are always `Some` here — overflow would have
        // already returned a `Validation` error. `unwrap_or_else` is defensive
        // only; it must NOT silently zero real money, which is why overflow
        // is propagated above instead of folded into `None`.
        sale.subtotal = subtotal.unwrap_or_else(|| Money::zero(currency));
        sale.tax_total = total_tax.unwrap_or_else(|| Money::zero(currency));

        // TAX-06: when exclusive tax was computed, the sale total must
        // include it. `Sale::from_cart` sets `total` from the cart total
        // (post-discount, pre-tax); the customer pays the discounted
        // subtotal PLUS the exclusive tax on top. Adding it here makes
        // `sales.total_minor` the true collectible amount, matching the
        // receipt's "grand total (subtotal + tax)" contract.
        if let Some(et) = exclusive_tax {
            sale.total = sale
                .total
                .checked_add(et)
                .ok_or_else(|| CoreError::Validation {
                    field: "total",
                    message: "sale total overflow from exclusive tax".into(),
                })?;
        }

        Ok(())
    }

    /// Compute the total tax for a set of cart lines (live preview).
    ///
    /// For each cart line resolves ALL applicable tax rates and sums
    /// their contributions. Returns the total tax amount plus whether any
    /// EXCLUSIVE rate applied (see [`CartTaxResult`]).
    ///
    /// `mode` controls how fractional per-rate results are rounded
    /// (TAX-05): pass [`RoundingMode::HalfUp`] for new sales and
    /// [`RoundingMode::Truncate`] when reproducing legacy behavior.
    pub fn compute_cart_tax(
        &self,
        lines: &[CartLineTaxInput],
        currency: Currency,
        mode: RoundingMode,
    ) -> Result<CartTaxResult, CoreError> {
        let mut total_tax: Option<Money> = None;
        let mut has_exclusive = false;

        for line in lines {
            // MONEY-02: negative qty/price would produce a negative line total
            // and a negative "tax" preview (the front-end renders it raw). The
            // cart model never allows negative qty/price, so reject them with a
            // structured Validation error naming the offending field.
            if line.qty < 0 {
                return Err(CoreError::Validation {
                    field: "qty",
                    message: format!("qty must be positive, got {}", line.qty),
                });
            }
            if line.unit_price_minor < 0 {
                return Err(CoreError::Validation {
                    field: "price",
                    message: format!(
                        "unit price must be non-negative, got {}",
                        line.unit_price_minor
                    ),
                });
            }
            // MONEY-01: the line total comes from untrusted IPC input and must
            // use checked arithmetic like `compute_line_tax` (TAX-04). The
            // workspace disables overflow-checks for dev/test builds, so a
            // bare `*` silently wraps and feeds a wrong tax to the register.
            let line_total_minor =
                line.qty.checked_mul(line.unit_price_minor).ok_or_else(|| {
                    CoreError::Validation {
                        field: "tax",
                        message: "cart line total overflow".into(),
                    }
                })?;
            let rates = self.resolve_best_tax_rates_for_sku(&line.sku)?;

            for rate in &rates {
                let tax = compute_line_tax(
                    line_total_minor,
                    rate.rate_bps,
                    rate.is_inclusive,
                    currency,
                    mode,
                )?;
                if !rate.is_inclusive {
                    has_exclusive = true;
                }
                total_tax = match total_tax {
                    None => Some(tax),
                    Some(acc) => {
                        Some(acc.checked_add(tax).ok_or_else(|| CoreError::Validation {
                            field: "tax",
                            message: "cart tax overflow".into(),
                        })?)
                    }
                };
            }
        }

        let tax = total_tax.unwrap_or_else(|| Money::zero(currency));
        Ok(CartTaxResult {
            tax_minor: tax.minor_units,
            has_exclusive,
        })
    }

    /// Resolve all applicable tax rates for a SKU using the chain:
    /// product rates → category rates → default rate.
    ///
    /// Returns ALL rates at the first matching level (e.g. all product-
    /// level rates). Returns an empty vec when no rate is configured.
    pub fn resolve_best_tax_rates_for_sku(&self, sku: &str) -> Result<Vec<TaxRate>, CoreError> {
        // 1. Product-level tax rates — return ALL assigned rates.
        let product_rate_ids = self.get_product_tax_rates(sku)?;
        if !product_rate_ids.is_empty() {
            let mut rates = Vec::with_capacity(product_rate_ids.len());
            for id in &product_rate_ids {
                if let Some(rate) = self.get_tax_rate(id)? {
                    rates.push(rate);
                }
            }
            if !rates.is_empty() {
                return Ok(rates);
            }
        }

        // 2. Category-level tax rates (via product.category_id).
        let product_id = self.product_id_by_sku(sku)?;
        if let Some(pid) = product_id {
            let category_id: Option<String> = self
                .conn
                .query_row(
                    "SELECT category_id FROM products WHERE id = ?1",
                    params![pid],
                    |row| row.get(0),
                )
                .ok()
                .and_then(|v| v);

            if let Some(cid) = category_id {
                let cat_rate_ids = self.get_category_tax_rates(&cid)?;
                if !cat_rate_ids.is_empty() {
                    let mut rates = Vec::with_capacity(cat_rate_ids.len());
                    for id in &cat_rate_ids {
                        if let Some(rate) = self.get_tax_rate(id)? {
                            rates.push(rate);
                        }
                    }
                    if !rates.is_empty() {
                        return Ok(rates);
                    }
                }
            }
        }

        // 3. Default store-wide tax rate (where `is_default = 1`).
        if let Some(rate) = self.get_default_tax_rate()? {
            return Ok(vec![rate]);
        }

        Ok(Vec::new())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "sales_tests.rs"]
mod tests;
