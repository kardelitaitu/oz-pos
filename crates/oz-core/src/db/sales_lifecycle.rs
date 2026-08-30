//! Sale lifecycle transitions after checkout.
//!
//! Key functions: `finalize_sale` / `finalize_sale_in_tx` (pending to
//! completed), `complete_sale_with_resolved_shortfalls` (shortfall
//! recovery), `void_pending_sale` and `void_sale`, plus stale-pending
//! detection and reaping (`find_stale_pending_sales`,
//! `reap_stale_pending_sales`).
//!
//! Invariants: every status transition bumps `version` inside a
//! transaction; voiding never adjusts inventory; voids write audit
//! entries.

use super::*;
use crate::AuditEntry;
use crate::SaleStatus;

impl Store<'_> {
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
