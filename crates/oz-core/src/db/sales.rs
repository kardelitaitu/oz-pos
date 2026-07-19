//! Sale CRUD — create, list, get, update status, held carts, exports.

use rusqlite::params;

use crate::error::CoreError;
use crate::money::Currency;
use crate::tax_rate::TaxRate;
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
    ///    JSON on the `sales` row, transitions to Active→Completed, COMMIT.
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
        _staff_user_id: &str,
        _terminal_id: Option<&str>,
    ) -> Result<crate::sale_deduction::CompleteSaleResult, CoreError> {
        use crate::inventory_transaction::InventoryTransactionId;
        use crate::sale_deduction::{Shortfall, StockDeduction};

        // ADR-19 §5.2: single transaction prevents two concurrent sales from
        // racing on the same inventory row. Same pattern as create_sale().
        let tx = self.conn.unchecked_transaction()?;

        // ── Resolve primary deduction location ─────────────────────
        let primary_location = crate::location_resolver::resolve_primary_location(
            &tx,
            workspace_instance_id.unwrap_or("default"),
            None,
        )
        .unwrap_or_else(|_| crate::location_resolver::get_default_location_id());

        // ── Phase 1: stock check + shortfall collection ────────────
        let mut deductions: Vec<StockDeduction> = Vec::with_capacity(sale.lines.len());
        let mut shortfalls: Vec<Shortfall> = Vec::new();

        for line in &sale.lines {
            let product_info: Option<(String, String)> = tx
                .query_row(
                    "SELECT id, product_type FROM products WHERE sku = ?1",
                    rusqlite::params![line.sku],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if product_info.is_none() {
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
            }
            let (pid, ptype_str) = product_info.unwrap();
            let ptype = crate::product::ProductType::parse_str(&ptype_str).unwrap_or_default();
            let tracks_inventory = ptype.tracks_inventory();
            let recipe = self.get_recipe_ingredients(&pid)?;
            let has_recipe = !recipe.is_empty();

            if !tracks_inventory && !has_recipe {
                // Skip checking stock for service products that do not have a recipe.
                continue;
            }

            // 1. Check composite product stock if it tracks inventory
            if tracks_inventory {
                let available: i64 = tx
                    .query_row(
                        "SELECT COALESCE(qty, 0) FROM stock_summary \
                         WHERE item_id = ?1 AND location_id = ?2",
                        rusqlite::params![pid, primary_location.as_str()],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                if available < line.qty {
                    let deficit = line.qty - available;
                    let alternatives = if let Some(ws_id) = workspace_instance_id {
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
                        deficit,
                        primary_location_id: primary_location.clone(),
                        alternatives,
                    });
                } else {
                    deductions.push(StockDeduction {
                        sku: line.sku.clone(),
                        location_id: primary_location.clone(),
                        delta: -line.qty,
                    });
                }
            }

            // 2. Check BOM ingredients if has recipe
            if has_recipe {
                for ingredient in recipe {
                    // Load ingredient product details
                    let ing_info: Option<(String, String, String)> = tx
                        .query_row(
                            "SELECT sku, name, product_type FROM products WHERE id = ?1",
                            rusqlite::params![ingredient.ingredient_product_id],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        )
                        .ok();

                    if let Some((ing_sku, ing_name, ing_ptype_str)) = ing_info {
                        let ing_ptype = crate::product::ProductType::parse_str(&ing_ptype_str)
                            .unwrap_or_default();
                        if ing_ptype.tracks_inventory() {
                            let required_qty = line.qty * ingredient.quantity_required;
                            let available: i64 = tx
                                .query_row(
                                    "SELECT COALESCE(qty, 0) FROM stock_summary \
                                     WHERE item_id = ?1 AND location_id = ?2",
                                    rusqlite::params![
                                        ingredient.ingredient_product_id,
                                        primary_location.as_str()
                                    ],
                                    |row| row.get(0),
                                )
                                .unwrap_or(0);

                            if available < required_qty {
                                let deficit = required_qty - available;
                                let alternatives = if let Some(ws_id) = workspace_instance_id {
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
                                    deficit,
                                    primary_location_id: primary_location.clone(),
                                    alternatives,
                                });
                            } else {
                                deductions.push(StockDeduction {
                                    sku: ing_sku,
                                    location_id: primary_location.clone(),
                                    delta: -required_qty,
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
                serde_json::json!({
                    "sale_line_id": line.id,
                    "sku": line.sku,
                    "deductions": [{
                        "location_id": primary_location.as_str(),
                        "qty": line.qty,
                        "sold_at": now
                    }]
                })
            }).collect::<Vec<_>>()
        })
        .to_string();

        // ── Persist sale + payments ───────────────────────────────
        let cur_str =
            std::str::from_utf8(&sale.currency.0).expect("currency bytes are valid UTF-8");

        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                                 tendered_minor, discount_percent, discount_label, user_id,
                                 created_at, updated_at, subtotal_minor, tax_total_minor,
                                 customer_id, deduction_locations, version)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1)",
            rusqlite::params![
                sale.id, sale.total.minor_units, cur_str, sale.line_count,
                sale.payment_method, sale.tendered_minor,
                sale.discount_percent, sale.discount_label, sale.user_id,
                sale.created_at, now,
                sale.subtotal.minor_units, sale.tax_total.minor_units,
                sale.customer_id, deduction_json,
            ],
        )?;

        for line in &sale.lines {
            let unit_cur = std::str::from_utf8(&line.unit_price.currency.0)
                .expect("currency bytes are valid UTF-8");
            tx.execute(
                "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor,
                                         currency, line_position, tax_minor, tax_rate_id,
                                         serial_number)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
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
                    line.serial_number,
                ],
            )?;
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
            let product_info: Option<(String, String)> = tx
                .query_row(
                    "SELECT id, product_type FROM products WHERE sku = ?1",
                    rusqlite::params![line.sku],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

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
                        let ing_info: Option<(String, String)> = tx
                            .query_row(
                                "SELECT sku, product_type FROM products WHERE id = ?1",
                                rusqlite::params![ingredient.ingredient_product_id],
                                |row| Ok((row.get(0)?, row.get(1)?)),
                            )
                            .ok();

                        if let Some((ing_sku, ing_ptype_str)) = ing_info {
                            let ing_ptype = crate::product::ProductType::parse_str(&ing_ptype_str)
                                .unwrap_or_default();
                            if ing_ptype.tracks_inventory() {
                                let required_qty = line.qty * ingredient.quantity_required;
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
        let cur_str =
            std::str::from_utf8(&sale.currency.0).expect("currency bytes are valid UTF-8");

        let deduction_json = serde_json::json!({
            "version": 1,
            "lines": line_deductions,
        })
        .to_string();

        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                                 tendered_minor, discount_percent, discount_label, user_id,
                                 created_at, updated_at, subtotal_minor, tax_total_minor,
                                 customer_id, deduction_locations, version)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1)",
            rusqlite::params![
                sale.id, sale.total.minor_units, cur_str, sale.line_count,
                sale.payment_method, sale.tendered_minor,
                sale.discount_percent, sale.discount_label, sale.user_id,
                sale.created_at, now,
                sale.subtotal.minor_units, sale.tax_total.minor_units,
                sale.customer_id, deduction_json,
            ],
        )?;

        for line in &sale.lines {
            let unit_cur = std::str::from_utf8(&line.unit_price.currency.0)
                .expect("currency bytes are valid UTF-8");
            tx.execute(
                "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor,
                                         currency, line_position, tax_minor, tax_rate_id,
                                         serial_number)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
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
                    line.serial_number,
                ],
            )?;
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
}

// ── Sale CRUD ────────────────────────────────────────────────────

impl Store<'_> {
    fn row_to_sale_line(row: &rusqlite::Row) -> rusqlite::Result<SaleLine> {
        let unit_cur_str: String = row.get("currency")?;
        let currency: Currency = unit_cur_str.parse().expect("valid currency in DB");
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
            serial_number: row.get("serial_number")?,
        })
    }

    /// Persist a [`Sale`] (header + all line items) inside a single transaction.
    pub fn create_sale(&self, sale: &Sale) -> Result<(), CoreError> {
        let cur_str =
            std::str::from_utf8(&sale.currency.0).expect("currency bytes are valid UTF-8");
        let status_str = sale.status.as_stored_str();

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor,
                                discount_percent, discount_label, user_id, created_at, updated_at,
                                subtotal_minor, tax_total_minor, customer_id, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1)",
            params![
                sale.id, sale.total.minor_units, cur_str, sale.line_count,
                status_str, sale.payment_method, sale.tendered_minor,
                sale.discount_percent, sale.discount_label, sale.user_id,
                sale.created_at, sale.updated_at,
                sale.subtotal.minor_units, sale.tax_total.minor_units,
                sale.customer_id,
            ],
        )?;

        for line in &sale.lines {
            let unit_cur = std::str::from_utf8(&line.unit_price.currency.0)
                .expect("currency bytes are valid UTF-8");
            tx.execute(
                "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position,
                                        tax_minor, tax_rate_id, serial_number)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    line.id, line.sale_id, line.sku, line.qty,
                    line.unit_price.minor_units, line.line_total.minor_units,
                    unit_cur, line.line_position,
                    line.tax_amount.minor_units, line.tax_rate_id,
                    line.serial_number,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// List all sales ordered by creation date (most recent first), without line items.
    pub fn list_sales(&self) -> Result<Vec<Sale>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version
             FROM sales ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse().expect("valid currency in DB");
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
                version: row.get("version").unwrap_or(1),
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single sale by id, including all line items.
    pub fn get_sale(&self, id: &str) -> Result<Option<Sale>, CoreError> {
        let mut sale_stmt = self.conn.prepare(
            "SELECT id, total_minor, currency, line_count, status,
                    payment_method, tendered_minor, discount_percent, discount_label,
                    user_id, created_at, updated_at,
                    subtotal_minor, tax_total_minor, customer_id, version
             FROM sales WHERE id = ?1",
        )?;

        let sale_result = sale_stmt.query_row(params![id], |row| {
            let cur_str: String = row.get("currency")?;
            let status_str: String = row.get("status")?;
            let currency: Currency = cur_str.parse().expect("valid currency in DB");
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
                    tax_minor, tax_rate_id, serial_number
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
    pub fn compute_sale_tax(
        &self,
        sale: &mut Sale,
        lua_overrides: &[(String, i64, bool)],
    ) -> Result<(), CoreError> {
        let currency = sale.currency;
        let mut total_tax: Option<Money> = None;
        let mut subtotal: Option<Money> = None;

        for line in &mut sale.lines {
            let line_subtotal = line.line_total;
            let mut line_tax = Money::zero(currency);

            // Check for a Lua plugin override first.
            let override_idx = lua_overrides
                .iter()
                .position(|(sku, _, _)| sku == &line.sku);

            if let Some(idx) = override_idx {
                let (_, rate_bps, is_inclusive) = &lua_overrides[idx];
                let rbps = *rate_bps;
                let tax = if *is_inclusive {
                    let divisor = 10_000 + rbps;
                    let tax_minor = line_subtotal.minor_units * rbps / divisor;
                    Money {
                        minor_units: tax_minor,
                        currency: line_subtotal.currency,
                    }
                } else {
                    let tax_minor = line_subtotal.minor_units * rbps / 10_000;
                    Money {
                        minor_units: tax_minor,
                        currency: line_subtotal.currency,
                    }
                };
                line_tax = line_tax
                    .checked_add(tax)
                    .unwrap_or_else(|| Money::zero(currency));
                // No DB tax_rate_id for override lines.
                line.tax_rate_id = None;
            } else {
                let rates = self.resolve_best_tax_rates_for_sku(&line.sku)?;

                for rate in &rates {
                    let tax = if rate.is_inclusive {
                        let divisor = 10_000 + rate.rate_bps;
                        let tax_minor = line_subtotal.minor_units * rate.rate_bps / divisor;
                        Money {
                            minor_units: tax_minor,
                            currency: line_subtotal.currency,
                        }
                    } else {
                        let tax_minor = line_subtotal.minor_units * rate.rate_bps / 10_000;
                        Money {
                            minor_units: tax_minor,
                            currency: line_subtotal.currency,
                        }
                    };
                    line_tax = line_tax
                        .checked_add(tax)
                        .unwrap_or_else(|| Money::zero(currency));
                }

                line.tax_rate_id = rates.first().map(|r| r.id.clone());
            }

            line.tax_amount = line_tax;

            total_tax = match total_tax {
                None => Some(line_tax),
                Some(acc) => acc.checked_add(line_tax),
            };

            subtotal = match subtotal {
                None => Some(line.line_total),
                Some(acc) => acc.checked_add(line.line_total),
            };
        }

        sale.subtotal = subtotal.unwrap_or_else(|| Money::zero(currency));
        sale.tax_total = total_tax.unwrap_or_else(|| Money::zero(currency));
        Ok(())
    }

    /// Compute the total tax for a set of cart lines (live preview).
    ///
    /// For each cart line resolves ALL applicable tax rates and sums
    /// their contributions. Returns the total tax amount.
    pub fn compute_cart_tax(
        &self,
        lines: &[CartLineTaxInput],
        currency: Currency,
    ) -> Result<Money, CoreError> {
        let mut total_tax: Option<Money> = None;

        for line in lines {
            let line_total_minor = line.qty * line.unit_price_minor;
            let rates = self.resolve_best_tax_rates_for_sku(&line.sku)?;

            for rate in &rates {
                let tax_minor = if rate.is_inclusive {
                    let divisor = 10_000 + rate.rate_bps;
                    line_total_minor * rate.rate_bps / divisor
                } else {
                    line_total_minor * rate.rate_bps / 10_000
                };
                let tax = Money {
                    minor_units: tax_minor,
                    currency,
                };
                total_tax = match total_tax {
                    None => Some(tax),
                    Some(acc) => acc.checked_add(tax),
                };
            }
        }

        Ok(total_tax.unwrap_or_else(|| Money::zero(currency)))
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

        // 3. Default store-wide tax rate.
        let all_rates = self.list_tax_rates()?;
        if let Some(default) = all_rates.into_iter().find(|r| r.is_default) {
            return Ok(vec![default]);
        }

        Ok(Vec::new())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::{Cart, CartLine, Sku};
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn make_cart() -> Cart {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
            .unwrap();
        cart
    }

    // ── Sale CRUD ────────────────────────────────────────────────

    #[test]
    fn create_sale_persists_header() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.id, sale.id);
        assert_eq!(loaded.status, SaleStatus::Pending);
        assert_eq!(loaded.total.minor_units, 1150);
        assert_eq!(loaded.line_count, 2);
    }

    #[test]
    fn create_sale_persists_lines() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.lines.len(), 2);
        assert_eq!(loaded.lines[0].sku, "COFFEE");
        assert_eq!(loaded.lines[0].qty, 2);
        assert_eq!(loaded.lines[0].unit_price.minor_units, 350);
        assert_eq!(loaded.lines[0].line_total.minor_units, 700);
        assert_eq!(loaded.lines[0].line_position, 1);
        assert_eq!(loaded.lines[1].sku, "BAGEL");
        assert_eq!(loaded.lines[1].line_position, 2);
    }

    #[test]
    fn create_sale_empty_cart() {
        let conn = fresh();
        let cart = Cart::new(usd());
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();
        let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.line_count, 0);
        assert_eq!(loaded.lines.len(), 0);
        assert_eq!(loaded.total.minor_units, 0);
    }

    #[test]
    fn list_sales_empty_db() {
        let conn = fresh();
        let sales = store(&conn).list_sales().unwrap();
        assert!(sales.is_empty());
    }

    #[test]
    fn list_sales_returns_all() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let mut cart2 = Cart::new(usd());
        cart2
            .add_line(CartLine::new(Sku::new("TEA"), 1, price(200)))
            .unwrap();
        let sale2 = Sale::from_cart(&cart2).unwrap();
        store(&conn).create_sale(&sale2).unwrap();

        let sales = store(&conn).list_sales().unwrap();
        assert_eq!(sales.len(), 2);
        // Most recent first.
        assert_eq!(sales[0].id, sale2.id);
        assert_eq!(sales[1].id, sale.id);
        // Lines should be empty (not loaded).
        assert!(sales[0].lines.is_empty());
    }

    #[test]
    fn get_sale_not_found() {
        let conn = fresh();
        let result = store(&conn).get_sale("nope").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_sale_status_active() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        let updated = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        assert_eq!(updated.status, SaleStatus::Active);
        assert!(!updated.updated_at.is_empty());
    }

    #[test]
    fn update_sale_status_full_flow() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        // Pending -> Active.
        let s = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        assert_eq!(s.status, SaleStatus::Active);

        // Active -> Completed.
        let s = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();
        assert_eq!(s.status, SaleStatus::Completed);

        // Terminal -> rejected (Completed -> Voided is invalid).
        let err = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Voided)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));
    }

    #[test]
    fn update_sale_status_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_sale_status("nope", SaleStatus::Active)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_sale_status_invalid_transition() {
        let conn = fresh();
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        store(&conn).create_sale(&sale).unwrap();

        // Pending -> Completed is invalid.
        let err = store(&conn)
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));
    }

    // ── Export / Report ───────────────────────────────────────────

    #[test]
    fn export_daily_summary_empty() {
        let conn = fresh();
        let rows = store(&conn).export_daily_summary().unwrap();
        assert!(rows.is_empty(), "no sales today → empty");
    }

    #[test]
    fn export_sales_by_hour_empty() {
        let conn = fresh();
        let rows = store(&conn).export_sales_by_hour().unwrap();
        assert!(rows.is_empty());
    }

    // ── Held Carts ───────────────────────────────────────────────

    #[test]
    fn hold_cart_creates_and_list() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart("Cart 1", r#"{"items":[]}"#, 0, 0, "USD", "hold", None, None)
            .unwrap();
        assert!(!id.is_empty());

        let carts = s.list_held_carts().unwrap();
        assert_eq!(carts.len(), 1);
        assert_eq!(carts[0].label, "Cart 1");
        assert_eq!(carts[0].total_minor, 0);
    }

    #[test]
    fn hold_cart_roundtrips_deduction_location_id() {
        let conn = fresh();
        let s = store(&conn);
        // Need to insert an inventory location first (FK constraint).
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-wh-a', 'Warehouse A', 'warehouse')",
            [],
        )
        .unwrap();

        let id = s
            .hold_cart(
                "Loc-Locked",
                r#"{"items":[{"sku":"COFFEE","qty":2}]}"#,
                2,
                700,
                "USD",
                "hold",
                None,
                Some("loc-wh-a"),
            )
            .unwrap();

        // Verify get_held_cart returns the deduction_location_id.
        let full = s.get_held_cart(&id).unwrap().unwrap();
        assert_eq!(
            full.deduction_location_id.as_deref(),
            Some("loc-wh-a"),
            "deduction_location_id must roundtrip through hold_cart → get_held_cart"
        );
    }

    #[test]
    fn hold_cart_with_items() {
        let conn = fresh();
        let s = store(&conn);
        s.hold_cart(
            "Active Cart",
            r#"{"lines":[{"sku":"COFFEE","qty":2}]}"#,
            2,
            700,
            "USD",
            "hold",
            None,
            None,
        )
        .unwrap();

        let carts = s.list_held_carts().unwrap();
        assert_eq!(carts.len(), 1);
        assert_eq!(carts[0].item_count, 2);
        assert_eq!(carts[0].total_minor, 700);
        assert_eq!(carts[0].currency, "USD");
    }

    #[test]
    fn get_held_cart_found() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart(
                "Test Cart",
                "{\"data\":\"value\"}",
                3,
                1500,
                "EUR",
                "hold",
                None,
                None,
            )
            .unwrap();

        let full = s.get_held_cart(&id).unwrap().unwrap();
        assert_eq!(full.label, "Test Cart");
        assert_eq!(full.cart_data, "{\"data\":\"value\"}");
        assert_eq!(full.item_count, 3);
        assert_eq!(full.total_minor, 1500);
        assert_eq!(full.currency, "EUR");
        assert!(!full.created_at.is_empty());
    }

    #[test]
    fn get_held_cart_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.get_held_cart("nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_held_carts_empty() {
        let conn = fresh();
        let s = store(&conn);
        let carts = s.list_held_carts().unwrap();
        assert!(carts.is_empty());
    }

    #[test]
    fn delete_held_cart_removes() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart("Delete Me", "{}", 0, 0, "USD", "hold", None, None)
            .unwrap();
        s.delete_held_cart(&id).unwrap();
        let result = s.get_held_cart(&id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_held_cart_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.delete_held_cart("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "held_cart"));
    }

    #[test]
    fn hold_cart_strips_label_whitespace() {
        let conn = fresh();
        let s = store(&conn);
        let id = s
            .hold_cart("  My Cart  ", "{}", 0, 0, "USD", "hold", None, None)
            .unwrap();
        let full = s.get_held_cart(&id).unwrap().unwrap();
        assert_eq!(full.label, "My Cart", "label should be trimmed");
    }

    // ── Open Bills ───────────────────────────────────────────────

    #[test]
    fn open_bill_persists_across_shifts() {
        let conn = fresh();
        let s = store(&conn);

        // Seed two users and a terminal.
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
               ('role-cashier', 'cashier', 'Cashier', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
               ('user-morning', 'alice', 'hash', 'Alice', 'role-cashier', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
               ('user-evening', 'bob', 'hash', 'Bob', 'role-cashier', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();

        // ── Morning shift ──
        let shift_morning = s.open_shift("user-morning", None, 200).unwrap();

        // Create an open bill.
        let _bill_id = s
            .hold_cart(
                "Table 4 — John",
                r#"{"lines":[{"sku":"STEAK","qty":1,"unit_price":1500}]}"#,
                1,
                1500,
                "USD",
                "open_bill",
                Some("John"),
                None,
            )
            .unwrap();

        // Open bill shows up immediately.
        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1, "open bill visible in same shift");
        assert_eq!(open[0].customer_name.as_deref(), Some("John"));

        // Close morning shift.
        s.close_shift(&shift_morning.id, 1700, None).unwrap();

        // ── Evening shift (different user) ──
        let _shift_evening = s.open_shift("user-evening", None, 500).unwrap();

        // The open bill is still listed — it is NOT scoped to a shift.
        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1, "open bill visible across shifts");
        assert_eq!(open[0].customer_name.as_deref(), Some("John"));
        assert_eq!(open[0].total_minor, 1500);
        assert_eq!(open[0].currency, "USD");
    }

    #[test]
    fn open_bill_list_excludes_hold_carts() {
        let conn = fresh();
        let s = store(&conn);

        s.hold_cart("Hold 1", "{}", 0, 0, "USD", "hold", None, None)
            .unwrap();
        s.hold_cart("Hold 2", "{}", 0, 0, "USD", "hold", None, None)
            .unwrap();
        s.hold_cart(
            "Table 7 — Mary",
            r#"{"lines":[]}"#,
            2,
            850,
            "USD",
            "open_bill",
            Some("Mary"),
            None,
        )
        .unwrap();

        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1, "only open bills, not hold carts");
        assert_eq!(open[0].customer_name.as_deref(), Some("Mary"));
    }

    #[test]
    fn open_bill_created_without_customer_name() {
        let conn = fresh();
        let s = store(&conn);

        let id = s
            .hold_cart("Walk-in", "{}", 0, 0, "USD", "open_bill", None, None)
            .unwrap();

        let open = s.list_open_bills().unwrap();
        assert_eq!(open.len(), 1);
        assert!(open[0].customer_name.is_none());
        assert_eq!(open[0].label, "Walk-in");

        // Verify full record.
        let full = s.get_held_cart(&id).unwrap().unwrap();
        assert_eq!(full.bill_type, "open_bill");
        assert!(full.customer_name.is_none());
    }

    // ── Void Sale ────────────────────────────────────────────────

    #[test]
    fn void_sale_changes_status_and_logs_audit() {
        let conn = fresh();
        let s = store(&conn);

        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();

        s.void_sale(&sale.id, "user-2", "customer request").unwrap();

        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.status, SaleStatus::Voided);

        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.void' AND target_id = ?1",
                rusqlite::params![sale.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 1);
    }

    #[test]
    fn void_sale_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.void_sale("nonexistent", "user-1", "test").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn void_sale_only_active_can_be_voided() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        // Sale is Pending, not Active — void should fail with validation error.
        let err = s.void_sale(&sale.id, "user-1", "test").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn void_sale_completed_cannot_be_voided() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        // Move to Active, then Completed.
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let err = s.void_sale(&sale.id, "user-1", "test").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    // ── Export with data ─────────────────────────────────────────

    #[test]
    fn export_daily_summary_with_sales() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        // Export uses date('now') so it should find the sale we just created.
        let rows = s.export_daily_summary().unwrap();
        assert!(!rows.is_empty(), "should find today's sale");
        assert_eq!(rows[0].total_minor, 1150);
    }

    #[test]
    fn create_sale_with_user_and_discount() {
        let conn = fresh();
        let s = store(&conn);
        let mut cart = make_cart();
        cart.set_discount(
            foundation::Percentage::new(10).unwrap(),
            Some("Loyalty".into()),
        );
        let sale = Sale::from_cart(&cart).unwrap();
        // Add user_id to the sale.
        let sale_with_user = Sale {
            user_id: Some("cashier-1".into()),
            customer_id: None,
            version: 1,
            ..sale
        };
        s.create_sale(&sale_with_user).unwrap();

        let loaded = s.get_sale(&sale_with_user.id).unwrap().unwrap();
        assert_eq!(loaded.user_id, Some("cashier-1".into()));
    }

    #[test]
    fn create_sale_discount_persisted() {
        let conn = fresh();
        let s = store(&conn);
        let mut cart = make_cart();
        cart.set_discount(foundation::Percentage::new(15).unwrap(), Some("VIP".into()));
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.discount_percent, 15);
        assert_eq!(loaded.discount_label, Some("VIP".into()));
    }

    // ── Export with data ─────────────────────────────────────────

    #[test]
    fn export_sales_by_hour_with_sales() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let rows = s.export_sales_by_hour().unwrap();
        assert!(!rows.is_empty(), "should find today's hourly aggregation");
        assert_eq!(rows[0].sale_count, 1);
        assert_eq!(rows[0].total_minor, 1150);
    }

    // ── Status transition edge cases ─────────────────────────────

    #[test]
    fn update_sale_status_invalid_stored_status() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        // Set a status that is valid at the SQL CHECK level ('refunded' is
        // in the CHECK constraint from migration 096) but NOT recognized
        // by SaleStatus::from_stored_str — this tests the Rust-layer
        // defensive guard against unknown stored values.
        conn.execute(
            "UPDATE sales SET status = 'refunded' WHERE id = ?1",
            rusqlite::params![sale.id],
        )
        .unwrap();

        let err = s
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    // ── Void edge cases ──────────────────────────────────────────

    #[test]
    fn void_sale_with_unknown_sku() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart(); // COFFEE x 2 (350) + BAGEL x 1 (450)
        let sale = Sale::from_cart(&cart).unwrap();
        // Do NOT create products — product_id_by_sku will return None.
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();

        // Void should succeed and skip stock restoration for unknown SKUs.
        let result = s.void_sale(&sale.id, "user-1", "no product record");
        assert!(
            result.is_ok(),
            "void should succeed even when SKU has no product record"
        );
        let loaded = result.unwrap();
        assert_eq!(loaded.status, SaleStatus::Voided);
    }

    // ── Tax Computation ─────────────────────────────

    fn seed_tax_rate(
        conn: &Connection,
        name: &str,
        rate_bps: i64,
        is_default: bool,
        is_inclusive: bool,
    ) -> String {
        let s = store(conn);
        s.create_tax_rate(name, rate_bps, is_default, is_inclusive)
            .unwrap()
            .id
    }

    fn seed_product_with_category(conn: &Connection, sku: &str, category_id: Option<&str>) {
        let s = store(conn);
        let currency: crate::money::Currency = "USD".parse().unwrap();
        let money = crate::Money {
            minor_units: 1000,
            currency,
        };
        s.create_product(sku, sku, money, category_id, None, 100, None)
            .unwrap();
    }

    fn make_single_line_sale(sku: &str, qty: i64, unit_minor: i64) -> Sale {
        let line_id = uuid::Uuid::now_v7().to_string();
        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Sale {
            id: sale_id.clone(),
            total: price(unit_minor * qty),
            currency: usd(),
            line_count: 1,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(unit_minor * qty),
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![SaleLine {
                id: line_id,
                sale_id,
                sku: sku.into(),
                qty,
                unit_price: price(unit_minor),
                line_total: price(unit_minor * qty),
                line_position: 1,
                tax_amount: price(0),
                tax_rate_id: None,
                serial_number: None,
            }],
        }
    }

    #[test]
    fn compute_tax_no_rates() {
        let conn = fresh();
        let s = store(&conn);
        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        assert_eq!(sale.subtotal.minor_units, 700);
        assert_eq!(sale.tax_total.minor_units, 0);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 0);
        assert!(sale.lines[0].tax_rate_id.is_none());
    }

    #[test]
    fn compute_tax_default_rate_exclusive() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // exclusive: tax = 700 * 1000 / 10000 = 70
        assert_eq!(sale.subtotal.minor_units, 700);
        assert_eq!(sale.tax_total.minor_units, 70);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 70);
        assert!(sale.lines[0].tax_rate_id.is_some());
    }

    #[test]
    fn compute_tax_default_rate_inclusive() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "GST 10%", 1000, true, true);

        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // inclusive: tax = 700 * 1000 / (10000 + 1000) = 700000 / 11000 = 63
        assert_eq!(sale.subtotal.minor_units, 700);
        assert_eq!(sale.tax_total.minor_units, 63);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 63);
    }

    #[test]
    fn compute_tax_product_level_wins() {
        let conn = fresh();
        let s = store(&conn);
        let _default_id = seed_tax_rate(&conn, "Default 5%", 500, true, false);
        let product_id = seed_tax_rate(&conn, "Product 10%", 1000, false, false);
        seed_product_with_category(&conn, "COFFEE", None);
        s.set_product_tax_rates("COFFEE", std::slice::from_ref(&product_id))
            .unwrap();

        let mut sale = make_single_line_sale("COFFEE", 1, 1000);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // product rate (10%) wins over default (5%): tax = 1000 * 1000 / 10000 = 100
        assert_eq!(sale.tax_total.minor_units, 100);
        assert_eq!(
            sale.lines[0].tax_rate_id.as_deref(),
            Some(product_id.as_str())
        );
    }

    #[test]
    fn compute_tax_category_level_wins_over_default() {
        let conn = fresh();
        let s = store(&conn);
        let _default_id = seed_tax_rate(&conn, "Default 5%", 500, true, false);
        let cat_id = seed_tax_rate(&conn, "Category 8%", 800, false, false);
        s.create_category("cat-1", "Beverages", "#fff", "").unwrap();
        s.set_category_tax_rates("cat-1", std::slice::from_ref(&cat_id))
            .unwrap();
        seed_product_with_category(&conn, "COFFEE", Some("cat-1"));

        let mut sale = make_single_line_sale("COFFEE", 1, 1000);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // category rate (8%) wins over default (5%): tax = 1000 * 800 / 10000 = 80
        assert_eq!(sale.tax_total.minor_units, 80);
        assert_eq!(sale.lines[0].tax_rate_id.as_deref(), Some(cat_id.as_str()));
    }

    #[test]
    fn compute_tax_multi_line() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

        let line1 = SaleLine {
            id: uuid::Uuid::now_v7().to_string(),
            sale_id: "sale-1".into(),
            sku: "COFFEE".into(),
            qty: 2,
            unit_price: price(350),
            line_total: price(700),
            line_position: 1,
            tax_amount: price(0),
            tax_rate_id: None,
            serial_number: None,
        };
        let line2 = SaleLine {
            id: uuid::Uuid::now_v7().to_string(),
            sale_id: "sale-1".into(),
            sku: "BAGEL".into(),
            qty: 1,
            unit_price: price(450),
            line_total: price(450),
            line_position: 2,
            tax_amount: price(0),
            tax_rate_id: None,
            serial_number: None,
        };
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut sale = Sale {
            id: "sale-1".into(),
            total: price(1150),
            currency: usd(),
            line_count: 2,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(1150),
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![line1, line2],
        };

        s.compute_sale_tax(&mut sale, &[]).unwrap();
        // line1: 700 * 1000 / 10000 = 70
        // line2: 450 * 1000 / 10000 = 45
        // total tax = 115
        assert_eq!(sale.subtotal.minor_units, 1150);
        assert_eq!(sale.tax_total.minor_units, 115);
        assert_eq!(sale.lines[0].tax_amount.minor_units, 70);
        assert_eq!(sale.lines[1].tax_amount.minor_units, 45);
    }

    #[test]
    fn compute_tax_persisted_after_create() {
        let conn = fresh();
        let s = store(&conn);
        seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

        let mut sale = make_single_line_sale("COFFEE", 2, 350);
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        s.create_sale(&sale).unwrap();

        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.subtotal.minor_units, 700);
        assert_eq!(loaded.tax_total.minor_units, 70);
        assert_eq!(loaded.lines[0].tax_amount.minor_units, 70);
        assert!(loaded.lines[0].tax_rate_id.is_some());
    }

    #[test]
    fn compute_tax_empty_sale_no_crash() {
        let conn = fresh();
        let s = store(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut sale = Sale {
            id: "empty".into(),
            total: price(0),
            currency: usd(),
            line_count: 0,
            status: SaleStatus::Pending,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            version: 1,
            lines: vec![],
        };
        s.compute_sale_tax(&mut sale, &[]).unwrap();
        assert_eq!(sale.subtotal.minor_units, 0);
        assert_eq!(sale.tax_total.minor_units, 0);
    }

    #[test]
    fn void_sale_succeeds_regardless_of_stock() {
        let conn = fresh();
        let s = store(&conn);

        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();

        // Void succeeds without touching stock at all.
        s.void_sale(&sale.id, "user-1", "no stock impact").unwrap();

        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.status, SaleStatus::Voided);
    }

    // ── complete_sale_deduction (ADR-19) ─────────────────────────

    /// Seed a product and stock so that complete_sale_deduction can succeed.
    fn seed_product_with_stock(conn: &Connection, sku: &str, qty: i64) -> String {
        use crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
        let product_id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES (?1, ?2, ?3, 1000, 'USD', 'retail')",
            rusqlite::params![product_id, sku, sku],
        )
        .unwrap();
        // Seed stock at the canonical default location so the resolver finds it.
        conn.execute(
            "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) \
             VALUES (?1, ?2, ?3)",
            rusqlite::params![product_id, CANONICAL_DEFAULT_LOCATION_UUID, qty],
        )
        .unwrap();
        product_id
    }

    /// Helper: seed a product with stock at TWO locations for split-fulfillment tests.
    fn setup_locations_with_stock(
        conn: &Connection,
        sku: &str,
        loc_a_id: &str,
        loc_a_qty: i64,
        loc_b_id: &str,
        loc_b_qty: i64,
    ) -> String {
        let product_id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES (?1, ?2, ?3, 1000, 'USD', 'retail')",
            rusqlite::params![product_id, sku, sku],
        )
        .unwrap();
        // Seed both locations into inventory_locations (creates IF NOT EXISTS).
        for loc_id in &[loc_a_id, loc_b_id] {
            conn.execute(
                "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES (?1, ?2, 'warehouse')",
                rusqlite::params![loc_id, loc_id],
            )
            .unwrap();
        }
        // Seed stock at both locations
        conn.execute(
            "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) VALUES (?1, ?2, ?3)",
            rusqlite::params![product_id, loc_a_id, loc_a_qty],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) VALUES (?1, ?2, ?3)",
            rusqlite::params![product_id, loc_b_id, loc_b_qty],
        )
        .unwrap();
        // Ensure canonical default location exists in inventory_locations (but don't
        // auto-seed stock — callers explicitly manage stock via loc_a/loc_b params).
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) \
             VALUES (?1, 'Default', 'store')",
            rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
        )
        .unwrap();
        product_id
    }

    #[test]
    fn complete_sale_deduction_sufficient_stock_succeeds() {
        let conn = fresh();
        let s = store(&conn);
        seed_product_with_stock(&conn, "COFFEE", 10);
        seed_product_with_stock(&conn, "BAGEL", 5);

        let sale = make_single_line_sale("COFFEE", 2, 350);
        let result = s
            .complete_sale_deduction(&sale, None, &[], "cashier-1", None)
            .unwrap();

        assert_eq!(result.sale_id, sale.id);
        assert_eq!(result.status, SaleStatus::Pending);
        assert!(!result.deduct_tx_id.as_str().is_empty());

        // Verify the sale row exists and is completed.
        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.status, SaleStatus::Pending);

        // Verify stock was deducted.
        let remaining: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary \
                 WHERE item_id = (SELECT id FROM products WHERE sku = 'COFFEE') \
                 AND location_id = ?1",
                rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 8, "10 - 2 = 8");
    }

    #[test]
    fn complete_sale_deduction_shortfall_returns_error_with_partial_result() {
        let conn = fresh();
        let s = store(&conn);
        seed_product_with_stock(&conn, "COFFEE", 1); // only 1 available, need 2

        let sale = make_single_line_sale("COFFEE", 2, 350);
        let err = s
            .complete_sale_deduction(&sale, None, &[], "cashier-1", None)
            .unwrap_err();

        // Should be a Validation error with serialized PartialStockResult.
        match &err {
            CoreError::Validation { field, message } if *field == "stock" => {
                let psr: crate::sale_deduction::PartialStockResult =
                    serde_json::from_str(message).unwrap();
                assert!(psr.requires_resolution);
                assert_eq!(psr.shortfalls.len(), 1);
                assert_eq!(psr.shortfalls[0].sku, "COFFEE");
                assert_eq!(psr.shortfalls[0].requested_qty, 2);
                assert_eq!(psr.shortfalls[0].primary_qty_available, 1);
                assert_eq!(psr.shortfalls[0].deficit, 1);
            }
            other => panic!("expected Validation error with field=stock, got {other:?}"),
        }

        // Stock should NOT have been deducted (transaction rolled back).
        let remaining: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary \
                 WHERE item_id = (SELECT id FROM products WHERE sku = 'COFFEE') \
                 AND location_id = ?1",
                rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 1, "stock unchanged after shortfall rollback");
    }

    #[test]
    fn complete_sale_deduction_empty_lines_succeeds() {
        let conn = fresh();
        let s = store(&conn);

        let sale = make_single_line_sale("COFFEE", 0, 0);
        let mut empty_sale = sale;
        empty_sale.lines.clear();
        empty_sale.line_count = 0;
        empty_sale.total = price(0);

        let result = s
            .complete_sale_deduction(&empty_sale, None, &[], "cashier-1", None)
            .unwrap();
        assert_eq!(result.status, SaleStatus::Pending);
    }

    #[test]
    fn complete_sale_deduction_unknown_sku_shortfall() {
        let conn = fresh();
        let s = store(&conn);
        // Do NOT seed any product — the SKU is unknown.

        let sale = make_single_line_sale("GHOST", 2, 350);
        let err = s
            .complete_sale_deduction(&sale, None, &[], "cashier-1", None)
            .unwrap_err();

        match &err {
            CoreError::Validation { field, message } if *field == "stock" => {
                let psr: crate::sale_deduction::PartialStockResult =
                    serde_json::from_str(message).unwrap();
                assert_eq!(psr.shortfalls.len(), 1);
                assert_eq!(psr.shortfalls[0].sku, "GHOST");
                assert_eq!(psr.shortfalls[0].primary_qty_available, 0);
            }
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    #[test]
    fn complete_sale_deduction_multi_line_partial_shortfall() {
        let conn = fresh();
        let s = store(&conn);
        seed_product_with_stock(&conn, "COFFEE", 10);
        seed_product_with_stock(&conn, "BAGEL", 0); // no stock for BAGEL

        // Build a 2-line sale manually.
        let cart = make_cart();
        let mut sale = Sale::from_cart(&cart).unwrap();
        // Override qty for BAGEL to exceed available stock.
        sale.lines[1].qty = 1;

        let err = s
            .complete_sale_deduction(&sale, None, &[], "cashier-1", None)
            .unwrap_err();

        match &err {
            CoreError::Validation { field, message } if *field == "stock" => {
                let psr: crate::sale_deduction::PartialStockResult =
                    serde_json::from_str(message).unwrap();
                assert!(psr.requires_resolution);
                // Only BAGEL should be listed as a shortfall (COFFEE sufficed).
                assert_eq!(psr.shortfalls.len(), 1);
                assert_eq!(psr.shortfalls[0].sku, "BAGEL");
            }
            other => panic!("expected Validation error, got {other:?}"),
        }

        // COFFEE stock should NOT have been deducted (full rollback).
        let coffee_qty: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary \
                 WHERE item_id = (SELECT id FROM products WHERE sku = 'COFFEE') \
                 AND location_id = ?1",
                rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(coffee_qty, 10, "COFFEE stock unchanged (full rollback)");
    }

    #[test]
    fn complete_sale_deduction_with_payment_splits() {
        let conn = fresh();
        let s = store(&conn);
        seed_product_with_stock(&conn, "COFFEE", 10);

        let sale = make_single_line_sale("COFFEE", 2, 350);
        let splits = vec![crate::PaymentSplitArg {
            method: "cash".into(),
            amount_minor: 500,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
        }];
        let result = s
            .complete_sale_deduction(&sale, None, &splits, "cashier-1", None)
            .unwrap();
        assert_eq!(result.status, SaleStatus::Pending);

        // Verify payment was recorded.
        let payment_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM payments WHERE sale_id = ?1",
                rusqlite::params![sale.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(payment_count, 1, "one payment row created");
    }

    // ── complete_sale_with_resolved_shortfalls (ADR-19 §6b) ─────

    /// Resolution with sufficient stock at an alternative location success.
    #[test]
    fn complete_sale_with_resolved_shortfalls_splits_across_locations() {
        let conn = fresh();
        let s = store(&conn);
        let loc_a = "loc-a";
        let loc_b = "loc-b";
        setup_locations_with_stock(&conn, "COFFEE", loc_a, 5, loc_b, 10);

        let sale = make_single_line_sale("COFFEE", 12, 350);
        let resolution = crate::sale_deduction::ResolvedShortfall {
            sku: "COFFEE".into(),
            allocations: vec![
                crate::sale_deduction::LocationAllocation {
                    location_id: crate::inventory::LocationId::from(loc_a),
                    qty: 5,
                },
                crate::sale_deduction::LocationAllocation {
                    location_id: crate::inventory::LocationId::from(loc_b),
                    qty: 7,
                },
            ],
        };
        let result = s
            .complete_sale_with_resolved_shortfalls(
                &sale,
                None,
                &[],
                "cashier-1",
                None,
                &[resolution],
            )
            .unwrap();
        assert_eq!(result.status, SaleStatus::Pending);

        // Verify stock deducted correctly from both locations
        let stock_a: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
                rusqlite::params![loc_a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_a, 0, "loc-a had 5, deducted 5 → 0");

        let stock_b: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
                rusqlite::params![loc_b],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_b, 3, "loc-b had 10, deducted 7 → 3");

        // Verify sale persisted with deduction_locations JSON
        let dl: String = conn
            .query_row(
                "SELECT deduction_locations FROM sales WHERE id = ?1",
                rusqlite::params![sale.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            dl.contains(loc_a),
            "deduction_locations should reference loc-a"
        );
        assert!(
            dl.contains(loc_b),
            "deduction_locations should reference loc-b"
        );
    }

    /// Resolution sum validation rejects mismatch.
    #[test]
    fn complete_sale_with_resolved_shortfalls_rejects_bad_allocation_sum() {
        let conn = fresh();
        let s = store(&conn);
        let loc = "loc-a";
        setup_locations_with_stock(&conn, "TEA", loc, 10, "loc-other", 0);

        let sale = make_single_line_sale("TEA", 5, 200);
        // Allocation sum = 3, but requested = 5 → error
        let resolution = crate::sale_deduction::ResolvedShortfall {
            sku: "TEA".into(),
            allocations: vec![crate::sale_deduction::LocationAllocation {
                location_id: crate::inventory::LocationId::from(loc),
                qty: 3,
            }],
        };
        let err = s
            .complete_sale_with_resolved_shortfalls(
                &sale,
                None,
                &[],
                "cashier-1",
                None,
                &[resolution],
            )
            .unwrap_err();
        assert!(
            matches!(&err, CoreError::Validation { field, .. } if field == &"resolutions"),
            "expected Validation error for bad allocation sum, got: {err}"
        );
    }

    /// Insufficient stock at resolved location returns error.
    #[test]
    fn complete_sale_with_resolved_shortfalls_fails_on_second_check() {
        let conn = fresh();
        let s = store(&conn);
        let loc = "loc-a";
        setup_locations_with_stock(&conn, "CHA", loc, 2, "loc-other", 0);

        let sale = make_single_line_sale("CHA", 5, 150);
        // Try to allocate 5 from loc-a which only has 2 → error
        let resolution = crate::sale_deduction::ResolvedShortfall {
            sku: "CHA".into(),
            allocations: vec![crate::sale_deduction::LocationAllocation {
                location_id: crate::inventory::LocationId::from(loc),
                qty: 5,
            }],
        };
        let err = s
            .complete_sale_with_resolved_shortfalls(
                &sale,
                None,
                &[],
                "cashier-1",
                None,
                &[resolution],
            )
            .unwrap_err();
        assert!(
            matches!(&err, CoreError::InsufficientStockAtLocation { .. }),
            "expected InsufficientStockAtLocation for over-allocation, got: {err}"
        );
    }

    /// Non-resolution lines still get deducted from primary location.
    #[test]
    fn complete_sale_with_resolved_shortfalls_deducts_unresolved_lines_at_primary() {
        use crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
        let conn = fresh();
        let s = store(&conn);
        setup_locations_with_stock(
            &conn,
            "COFFEE",
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            20,
            "loc-wh",
            50,
        );
        setup_locations_with_stock(
            &conn,
            "BAGEL",
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            10,
            "loc-wh",
            30,
        );

        // Only COFFEE has a resolution; BAGEL should be deducted from primary (default UUID)
        let sale = {
            let mut cart = Cart::new(usd());
            cart.add_line(CartLine::new(Sku::new("COFFEE"), 3, price(350)))
                .unwrap();
            cart.add_line(CartLine::new(Sku::new("BAGEL"), 2, price(450)))
                .unwrap();
            Sale::from_cart(&cart).unwrap()
        };

        let resolution = crate::sale_deduction::ResolvedShortfall {
            sku: "COFFEE".into(),
            allocations: vec![crate::sale_deduction::LocationAllocation {
                location_id: crate::inventory::LocationId::from("loc-wh"),
                qty: 3,
            }],
        };
        let result = s
            .complete_sale_with_resolved_shortfalls(
                &sale,
                None,
                &[],
                "cashier-1",
                None,
                &[resolution],
            )
            .unwrap();
        assert_eq!(result.status, SaleStatus::Pending);

        // COFFEE deducted 3 from loc-wh (50 → 47)
        let coffee_wh: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
                rusqlite::params!["loc-wh"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(coffee_wh, 47, "loc-wh had 50, deducted 3 → 47");

        // BAGEL deductible from canonical default (10 seeded, 2 deducted → 8)
        let bagel_def: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'BAGEL') AND location_id = ?1",
                rusqlite::params![CANONICAL_DEFAULT_LOCATION_UUID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(bagel_def, 8, "canonical default had 10, deducted 2 → 8");
    }

    /// Empty resolutions (no shortfalls) still deducts all stock from primary.
    #[test]
    fn complete_sale_with_resolved_shortfalls_empty_resolutions_deducts_at_primary() {
        let conn = fresh();
        let s = store(&conn);
        setup_locations_with_stock(
            &conn,
            "COFFEE",
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            10,
            "loc-wh",
            20,
        );

        let sale = make_single_line_sale("COFFEE", 3, 350);
        let result = s
            .complete_sale_with_resolved_shortfalls(&sale, None, &[], "cashier-1", None, &[])
            .unwrap();
        assert_eq!(result.status, SaleStatus::Pending);

        // COFFEE deducted 3 from canonical default
        let stock: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
                rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock, 7, "canonical default had 10, deducted 3 → 7");
        // loc-wh should be untouched
        let stock_wh: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
                rusqlite::params!["loc-wh"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_wh, 20, "loc-wh untouched");
    }

    // ── ADR-19 §16.2 acceptance tests ──────────────────────────────

    /// Multi-binding sale with insufficient primary stock → shortfall
    /// returned AND no sale row persists (full rollback).
    #[test]
    fn complete_sale_partial_shortfall_rolls_back_sale_row() {
        let conn = fresh();
        let s = store(&conn);

        // ── set up a multi-binding workspace ────────────────────────
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES
                ('loc-pri', 'Primary', 'store'),
                ('loc-sec', 'Secondary', 'warehouse');
             INSERT OR IGNORE INTO store_profiles (id, name, is_primary) VALUES ('store-1', 'Test Store', 1);
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name)
                VALUES ('ws-multi-test',
                    (SELECT key FROM workspace_types LIMIT 1),
                    'store-1', 'Multi-Test');
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order)
                VALUES ('wsl-pri', 'ws-multi-test', 'loc-pri', 1, 0),
                       ('wsl-sec', 'ws-multi-test', 'loc-sec', 0, 1);",
        )
        .unwrap();
        let product_id = seed_product_with_stock(&conn, "COFFEE", 0);
        // Stock only at the secondary location (primary has 0).
        conn.execute(
            "INSERT OR REPLACE INTO stock_summary (item_id, location_id, qty) VALUES (?1, 'loc-sec', 5)",
            rusqlite::params![product_id],
        )
        .unwrap();

        let sale = make_single_line_sale("COFFEE", 2, 350);
        let err = s
            .complete_sale_deduction(&sale, Some("ws-multi-test"), &[], "cashier-1", None)
            .unwrap_err();

        match &err {
            CoreError::Validation { field, message } if *field == "stock" => {
                let psr: crate::sale_deduction::PartialStockResult =
                    serde_json::from_str(message).unwrap();
                assert!(psr.requires_resolution);
                assert_eq!(psr.shortfalls.len(), 1);
                assert_eq!(psr.shortfalls[0].sku, "COFFEE");
                assert_eq!(
                    psr.shortfalls[0].primary_location_id,
                    crate::inventory::LocationId::from("loc-pri")
                );
                assert_eq!(psr.shortfalls[0].primary_qty_available, 0);
                assert_eq!(psr.shortfalls[0].deficit, 2);
                // Should have loc-sec as an alternative
                assert!(
                    psr.shortfalls[0]
                        .alternatives
                        .iter()
                        .any(|a| a.location_id == crate::inventory::LocationId::from("loc-sec")),
                    "expected loc-sec as alternative"
                );
            }
            other => panic!("expected Validation error with field=stock, got {other:?}"),
        }

        // Verify NO sale row was created (full rollback).
        let sale_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sales WHERE id = ?1",
                rusqlite::params![sale.id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(
            !sale_exists,
            "sale row must not exist after shortfall rollback"
        );
    }

    /// Void of a multi-location pending sale credits stock back to
    /// each original deduction source (ADR-19 §5.3 / §16.2).
    #[test]
    fn void_sale_credits_back_to_original_deduction_source() {
        let conn = fresh();
        let s = store(&conn);
        let loc_a = "loc-v-a";
        let loc_b = "loc-v-b";

        // ── create a sale with split-location deduction_locations ───
        setup_locations_with_stock(&conn, "TEA", loc_a, 10, loc_b, 5);
        let sale = make_single_line_sale("TEA", 8, 200);
        let resolution = crate::sale_deduction::ResolvedShortfall {
            sku: "TEA".into(),
            allocations: vec![
                crate::sale_deduction::LocationAllocation {
                    location_id: crate::inventory::LocationId::from(loc_a),
                    qty: 5,
                },
                crate::sale_deduction::LocationAllocation {
                    location_id: crate::inventory::LocationId::from(loc_b),
                    qty: 3,
                },
            ],
        };
        s.complete_sale_with_resolved_shortfalls(
            &sale,
            None,
            &[],
            "cashier-1",
            None,
            &[resolution],
        )
        .unwrap();

        // Confirm stock was deducted.
        let stock_a_before: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
                rusqlite::params![loc_a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_a_before, 5, "loc-a had 10, deducted 5 → 5");

        let stock_b_before: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
                rusqlite::params![loc_b],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_b_before, 2, "loc-b had 5, deducted 3 → 2");

        // ── void the pending sale ───────────────────────────────────
        s.void_pending_sale(&sale.id).unwrap();

        // Verify stock was credited BACK to each location.
        let stock_a_after: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
                rusqlite::params![loc_a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_a_after, 10, "loc-a credited back to original 10");

        let stock_b_after: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
                 (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
                rusqlite::params![loc_b],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock_b_after, 5, "loc-b credited back to original 5");

        // Verify sale status is voided.
        let loaded = s.get_sale(&sale.id).unwrap().unwrap();
        assert_eq!(loaded.status, SaleStatus::Voided);
    }

    /// Two threads attempting complete_sale_deduction on the same SKU:
    /// one succeeds, the other fails with a constraint/serialization error
    /// thanks to BEGIN IMMEDIATE (ADR-19 §5.2).
    #[test]
    fn concurrent_complete_sale_serialized_by_begin_immediate() {
        // Use a file-based DB so two connections can access it concurrently.
        let dir = std::env::temp_dir().join(format!("oz_concurrent_{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");

        // Clone the schema from a fresh_db() snapshot into the file DB.
        {
            let mut file_conn = rusqlite::Connection::open(&db_path).unwrap();
            {
                let template = crate::migrations::fresh_db();
                let backup = rusqlite::backup::Backup::new(&template, &mut file_conn).unwrap();
                backup
                    .run_to_completion(10, std::time::Duration::from_millis(0), None)
                    .unwrap();
            }
            let pid = uuid::Uuid::now_v7().to_string();
            file_conn
                .execute(
                    "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
                     VALUES (?1, 'COFFEE', 'Coffee', 1000, 'USD', 'retail')",
                    rusqlite::params![pid],
                )
                .unwrap();
            file_conn
                .execute(
                    "INSERT INTO stock_summary (item_id, location_id, qty) VALUES (?1, ?2, 2)",
                    rusqlite::params![pid, crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
                )
                .unwrap();
        }

        let sale = std::sync::Arc::new(make_single_line_sale("COFFEE", 2, 350));

        let mut handles = Vec::new();
        for i in 0..2 {
            let p = db_path.clone();
            let sl = sale.clone();
            handles.push(std::thread::spawn(move || {
                let conn = rusqlite::Connection::open(&p).unwrap();
                let store = Store::new(&conn);
                let result = store.complete_sale_deduction(&sl, None, &[], "cashier-1", None);
                (i, result)
            }));
        }

        let mut success_count = 0;
        let mut failure_count = 0;
        for h in handles {
            match h.join().unwrap() {
                (_, Ok(_)) => success_count += 1,
                (i, Err(e)) => {
                    failure_count += 1;
                    tracing::info!(thread = i, error = %e, "concurrent sale failed as expected");
                }
            }
        }

        assert_eq!(
            success_count, 1,
            "exactly one thread should succeed with BEGIN IMMEDIATE"
        );
        assert!(
            failure_count >= 1,
            "second thread should fail with serialization error"
        );

        // Clean up.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn void_pending_sale_nonexistent_sale_errors() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.void_pending_sale("nonexistent").unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "pending sale",
                ..
            }
        ));
    }

    #[test]
    fn void_pending_sale_malformed_deduction_locations_errors() {
        let conn = fresh();
        let s = store(&conn);
        let cart = make_cart();
        let sale = Sale::from_cart(&cart).unwrap();

        // Insert a sale with malformed JSON in deduction_locations
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                                tendered_minor, discount_percent, discount_label, user_id,
                                created_at, updated_at, subtotal_minor, tax_total_minor,
                                deduction_locations, version)
             VALUES (?1, 1000, 'USD', 1, 'pending', 'CASH', 1000, 0, NULL, 'user-1',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1000, 0, 'not-valid-json', 1)",
            rusqlite::params![sale.id],
        )
        .unwrap();

        let err = s.void_pending_sale(&sale.id).unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "deduction_locations",
                ..
            }
        ));
    }

    #[test]
    fn void_pending_sale_twice_errors() {
        let conn = fresh();
        let s = store(&conn);

        // Seed a product and stock
        conn.execute(
            "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('prod-test', 'TEST-1', 'Test', 5000, 'IDR', 'retail')",
            [],
        )
        .unwrap();
        let default_loc = crate::location_resolver::get_default_location_id();
        conn.execute(
            "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) \
             VALUES ('prod-test', ?1, 10)",
            rusqlite::params![default_loc.as_str()],
        )
        .unwrap();

        // Use Sale::from_cart to create a sale — the only public constructor.
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("TEST-1"), 3, price(5000)))
            .unwrap();
        let sale = Sale::from_cart(&cart).unwrap();

        s.complete_sale_deduction(&sale, None, &[], "staff-1", None)
            .unwrap();

        // First void succeeds
        s.void_pending_sale(&sale.id).unwrap();

        // Second void fails — sale is now 'voided', not 'pending'
        let err = s.void_pending_sale(&sale.id).unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "pending sale",
                ..
            }
        ));
    }
}
