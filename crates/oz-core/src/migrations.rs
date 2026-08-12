//! Migration definitions for OZ-POS.
//!
//! Migrations are `.sql` files under `crates/oz-core/migrations/`. They are
//! embedded at compile time via [`include_str!`] and run in the
//! compile-time array order of [`ALL`] on first startup by the generic
//! runner in `platform-core`. The array order is canonical — not
//! lexicographic filename order — and is enforced by
//! [`migration_prefixes_are_monotonic_after_legacy_block`] and the
//! registry↔filesystem parity test [`migration_registry_matches_filesystem`].
//!
//! # Forward-only contract
//!
//! Production migrations are **forward-only**. They must be written so that
//! re-running them is a no-op (the runner tracks applied IDs), and they are
//! never reversed in the field: destructive/data migrations require a
//! backup-plus-forward-repair procedure, never ad-hoc down SQL (DB-03).
//! The generic [`platform_core::database::rollback`] helper exists for
//! synthetic/test tables only — the core registry carries no down SQL.

use platform_core::database::Migration;

/// All migrations in the order they should be applied.
///
/// The list is exhaustive at compile time; adding a new migration means
/// adding a new entry here AND a new file in `crates/oz-core/migrations/`.
///
/// # Numbering gaps
///
/// The following migration numbers are intentionally absent — they held
/// migrations that were removed, renumbered, or merged during development:
///
/// - 026 — removed (absorbed into 025_store_profiles rework)
/// - 034 — removed (merged into 035_workspaces)
/// - 044 — removed (redundant with 043_price_updated_at)
/// - 056–059 — reserved for workspace-instance lifecycle (ultimately landed as 060)
/// - 062 — removed (merged into 063_stock_movements)
/// - 088 — removed (merged into 089_stock_summary_composite_pk)
///
/// The runner uses a tracking table (`schema_migrations`), not sequential
/// numbering, so gaps are safe. Do not re-use a gap number — always append
/// the next available integer.
///
/// # Shared-prefix convention (legacy)
///
/// Migrations 046 and 047 each have multiple files sharing the same numeric
/// prefix (`046_gift_cards`, `046_suppliers`, `046_stock_counts`,
/// `046_track_serial` and `047_purchase_orders`, `047_stock_transfers`,
/// `047_receipt_barcodes`). This is a legacy pattern from early development
/// when domain-adjacent migrations were batched under one number. New
/// migrations MUST use a unique sequential prefix. The runner processes
/// migrations in compile-time array order, so the shared prefixes have no
/// functional impact.
pub const ALL: &[Migration] = &[
    Migration {
        id: "001_sales.sql",
        sql: include_str!("../migrations/001_sales.sql"),
    },
    Migration {
        id: "002_products.sql",
        sql: include_str!("../migrations/002_products.sql"),
    },
    Migration {
        id: "003_barcode.sql",
        sql: include_str!("../migrations/003_barcode.sql"),
    },
    Migration {
        id: "004_sale_status.sql",
        sql: include_str!("../migrations/004_sale_status.sql"),
    },
    Migration {
        id: "005_line_count_check.sql",
        sql: include_str!("../migrations/005_line_count_check.sql"),
    },
    Migration {
        id: "006_currencies.sql",
        sql: include_str!("../migrations/006_currencies.sql"),
    },
    Migration {
        id: "007_customers.sql",
        sql: include_str!("../migrations/007_customers.sql"),
    },
    Migration {
        id: "008_payments.sql",
        sql: include_str!("../migrations/008_payments.sql"),
    },
    Migration {
        id: "009_tax_rates.sql",
        sql: include_str!("../migrations/009_tax_rates.sql"),
    },
    Migration {
        id: "010_audit_log.sql",
        sql: include_str!("../migrations/010_audit_log.sql"),
    },
    Migration {
        id: "011_discounts.sql",
        sql: include_str!("../migrations/011_discounts.sql"),
    },
    Migration {
        id: "012_product_taxes.sql",
        sql: include_str!("../migrations/012_product_taxes.sql"),
    },
    Migration {
        id: "013_held_carts.sql",
        sql: include_str!("../migrations/013_held_carts.sql"),
    },
    Migration {
        id: "014_user_id_on_sales.sql",
        sql: include_str!("../migrations/014_user_id_on_sales.sql"),
    },
    Migration {
        id: "015_product_variants.sql",
        sql: include_str!("../migrations/015_product_variants.sql"),
    },
    Migration {
        id: "016_terminals.sql",
        sql: include_str!("../migrations/016_terminals.sql"),
    },
    Migration {
        id: "017_tax_inclusive_category.sql",
        sql: include_str!("../migrations/017_tax_inclusive_category.sql"),
    },
    Migration {
        id: "018_offline_queue.sql",
        sql: include_str!("../migrations/018_offline_queue.sql"),
    },
    Migration {
        id: "019_refunds.sql",
        sql: include_str!("../migrations/019_refunds.sql"),
    },
    Migration {
        id: "020_tax_on_sales.sql",
        sql: include_str!("../migrations/020_tax_on_sales.sql"),
    },
    Migration {
        id: "021_shifts.sql",
        sql: include_str!("../migrations/021_shifts.sql"),
    },
    Migration {
        id: "022_payments_table.sql",
        sql: include_str!("../migrations/022_payments_table.sql"),
    },
    Migration {
        id: "023_cash_payouts.sql",
        sql: include_str!("../migrations/023_cash_payouts.sql"),
    },
    Migration {
        id: "024_audit_log_triggers.sql",
        sql: include_str!("../migrations/024_audit_log_triggers.sql"),
    },
    Migration {
        id: "025_store_profiles.sql",
        sql: include_str!("../migrations/025_store_profiles.sql"),
    },
    Migration {
        id: "027_payment_gateway_fields.sql",
        sql: include_str!("../migrations/027_payment_gateway_fields.sql"),
    },
    Migration {
        id: "028_terminal_feature_overrides.sql",
        sql: include_str!("../migrations/028_terminal_feature_overrides.sql"),
    },
    Migration {
        id: "029_promotions.sql",
        sql: include_str!("../migrations/029_promotions.sql"),
    },
    Migration {
        id: "030_product_bundles.sql",
        sql: include_str!("../migrations/030_product_bundles.sql"),
    },
    Migration {
        id: "031_loyalty.sql",
        sql: include_str!("../migrations/031_loyalty.sql"),
    },
    Migration {
        id: "032_kds_orders.sql",
        sql: include_str!("../migrations/032_kds_orders.sql"),
    },
    Migration {
        id: "033_tables.sql",
        sql: include_str!("../migrations/033_tables.sql"),
    },
    Migration {
        id: "035_workspaces.sql",
        sql: include_str!("../migrations/035_workspaces.sql"),
    },
    Migration {
        id: "036_open_bills.sql",
        sql: include_str!("../migrations/036_open_bills.sql"),
    },
    Migration {
        id: "037_active_carts.sql",
        sql: include_str!("../migrations/037_active_carts.sql"),
    },
    Migration {
        id: "038_user_preferences.sql",
        sql: include_str!("../migrations/038_user_preferences.sql"),
    },
    Migration {
        id: "039_category_icon.sql",
        sql: include_str!("../migrations/039_category_icon.sql"),
    },
    Migration {
        id: "040_user_workspaces.sql",
        sql: include_str!("../migrations/040_user_workspaces.sql"),
    },
    Migration {
        id: "041_credit_reminders.sql",
        sql: include_str!("../migrations/041_credit_reminders.sql"),
    },
    Migration {
        id: "042_customer_id_on_sales.sql",
        sql: include_str!("../migrations/042_customer_id_on_sales.sql"),
    },
    Migration {
        id: "043_price_updated_at.sql",
        sql: include_str!("../migrations/043_price_updated_at.sql"),
    },
    Migration {
        id: "045_serial_number.sql",
        sql: include_str!("../migrations/045_serial_number.sql"),
    },
    Migration {
        id: "046_gift_cards.sql",
        sql: include_str!("../migrations/046_gift_cards.sql"),
    },
    Migration {
        id: "046_suppliers.sql",
        sql: include_str!("../migrations/046_suppliers.sql"),
    },
    Migration {
        id: "046_stock_counts.sql",
        sql: include_str!("../migrations/046_stock_counts.sql"),
    },
    Migration {
        id: "047_purchase_orders.sql",
        sql: include_str!("../migrations/047_purchase_orders.sql"),
    },
    Migration {
        id: "046_track_serial.sql",
        sql: include_str!("../migrations/046_track_serial.sql"),
    },
    Migration {
        id: "047_stock_transfers.sql",
        sql: include_str!("../migrations/047_stock_transfers.sql"),
    },
    Migration {
        id: "047_receipt_barcodes.sql",
        sql: include_str!("../migrations/047_receipt_barcodes.sql"),
    },
    Migration {
        id: "048_kds_workspace.sql",
        sql: include_str!("../migrations/048_kds_workspace.sql"),
    },
    Migration {
        id: "049_product_type.sql",
        sql: include_str!("../migrations/049_product_type.sql"),
    },
    Migration {
        id: "050_terminal_profiles.sql",
        sql: include_str!("../migrations/050_terminal_profiles.sql"),
    },
    Migration {
        id: "051_product_recipes.sql",
        sql: include_str!("../migrations/051_product_recipes.sql"),
    },
    Migration {
        id: "052_order_modifiers.sql",
        sql: include_str!("../migrations/052_order_modifiers.sql"),
    },
    Migration {
        id: "053_kds_status_check.sql",
        sql: include_str!("../migrations/053_kds_status_check.sql"),
    },
    Migration {
        id: "054_product_cost.sql",
        sql: include_str!("../migrations/054_product_cost.sql"),
    },
    Migration {
        id: "055_offline_queue_tenant.sql",
        sql: include_str!("../migrations/055_offline_queue_tenant.sql"),
    },
    Migration {
        id: "060_workspace_instances.sql",
        sql: include_str!("../migrations/060_workspace_instances.sql"),
    },
    Migration {
        id: "061_tenant_subscription.sql",
        sql: include_str!("../migrations/061_tenant_subscription.sql"),
    },
    Migration {
        id: "063_stock_movements.sql",
        sql: include_str!("../migrations/063_stock_movements.sql"),
    },
    Migration {
        id: "064_kds_store_id.sql",
        sql: include_str!("../migrations/064_kds_store_id.sql"),
    },
    Migration {
        id: "065_version_optimistic.sql",
        sql: include_str!("../migrations/065_version_optimistic.sql"),
    },
    Migration {
        id: "066_store_profile_orphan_guard.sql",
        sql: include_str!("../migrations/066_store_profile_orphan_guard.sql"),
    },
    Migration {
        id: "067_stock_movements_store_id.sql",
        sql: include_str!("../migrations/067_stock_movements_store_id.sql"),
    },
    Migration {
        id: "068_tenant_subscription_api_key.sql",
        sql: include_str!("../migrations/068_tenant_subscription_api_key.sql"),
    },
    Migration {
        id: "069_data_scoping_columns.sql",
        sql: include_str!("../migrations/069_data_scoping_columns.sql"),
    },
    Migration {
        id: "070_reset_machine_id.sql",
        sql: include_str!("../migrations/070_reset_machine_id.sql"),
    },
    Migration {
        id: "071_exchange_rate_minor_units.sql",
        sql: include_str!("../migrations/071_exchange_rate_minor_units.sql"),
    },
    Migration {
        id: "072_stock_movements_archive.sql",
        sql: include_str!("../migrations/072_stock_movements_archive.sql"),
    },
    Migration {
        id: "073_offline_queue_priority.sql",
        sql: include_str!("../migrations/073_offline_queue_priority.sql"),
    },
    Migration {
        id: "074_login_attempts.sql",
        sql: include_str!("../migrations/074_login_attempts.sql"),
    },
    Migration {
        id: "075_global_currency_settings.sql",
        sql: include_str!("../migrations/075_global_currency_settings.sql"),
    },
    Migration {
        id: "076_tenant_id_reference.sql",
        sql: include_str!("../migrations/076_tenant_id_reference.sql"),
    },
    Migration {
        id: "077_kitchen_zone.sql",
        sql: include_str!("../migrations/077_kitchen_zone.sql"),
    },
    // ── ADR #18 Phase 0A: Multi-Location Inventory Foundation ─────
    // These three migrations introduce location-aware stock tracking
    // without touching the Rust API or the workspace-rename cascade.
    // They are added in lexicographic order so the runner picks them up
    // after 077_kitchen_zone.sql. Phase 1 follow-ups will land the
    // composite-PK rebuild of inventory.stock_summary, the stock_transfers
    // CHECK extension (ADR §13 finding 34), and the sale-deduction flow
    // changes (gated on ADR-19 per §13 finding 31).
    Migration {
        id: "078_inventory_locations.sql",
        sql: include_str!("../migrations/078_inventory_locations.sql"),
    },
    Migration {
        id: "079_inventory_location_id.sql",
        sql: include_str!("../migrations/079_inventory_location_id.sql"),
    },
    Migration {
        id: "080_stock_movements_location_id.sql",
        sql: include_str!("../migrations/080_stock_movements_location_id.sql"),
    },
    // ── ADR #18 Phase 1: stock_transfers rebuild (§13 finding 34) ──
    // Extends the CHECK constraint to include 'received_partial' so
    // the §7 step-6 partial-receipt flow doesn't crash on insert. Adds
    // source_location_id / destination_location_id FK columns to
    // inventory_locations; renames the legacy free-text columns to
    // `_old` for backward-compatibility audit (§2d). No Rust API
    // change required — the existing stock_transfer module still
    // accepts and emits the same Rust domain types.
    Migration {
        id: "081_stock_transfers_received_partial.sql",
        sql: include_str!("../migrations/081_stock_transfers_received_partial.sql"),
    },
    // ── ADR #18 Phase 1: workspace-instance-to-location binding (§5) ─
    // Adds a nullable bound_location_id FK on workspace_instances.
    // Nullable (not NOT NULL) per §5 to preserve the "unbound admin
    // console" fallback for legacy single-location deployments. The
    // companion workspace_inventory_locations table (§4) is a separate
    // multi-binding migration in a later phase — together they form
    // §5's split-brain-prevention framework enforced at the
    // application layer (SQLite cannot enforce the XOR constraint
    // without triggers).
    Migration {
        id: "082_workspace_instances_bound_location.sql",
        sql: include_str!("../migrations/082_workspace_instances_bound_location.sql"),
    },
    // ── ADR #18 Phase 1: workspace multi-binding (§4) ─────────────
    // Companion table to migration 082's single-binding
    // `bound_location_id` FK. Allows a single workspace instance
    // to bind to multiple inventory locations (the multi-binding case).
    // §5 split-brain prevention: a workspace MUST NOT have both
    // bound_location_id set AND rows here — enforced at the
    // application layer (SQLite cannot enforce the XOR constraint
    // without triggers).
    Migration {
        id: "083_workspace_inventory_locations.sql",
        sql: include_str!("../migrations/083_workspace_inventory_locations.sql"),
    },
    // ── ADR #18 Phase 2: staff audit trail (§9a + §9b) ───────────
    // inventory_transactions is a session grouping for inventory
    // operations; inventory_transaction_lines is the per-SKU detail.
    // Followup migrations (§9c, §9d) will link stock_movements rows
    // back to the session and add the shift accountability window.
    Migration {
        id: "084_inventory_transaction_audit.sql",
        sql: include_str!("../migrations/084_inventory_transaction_audit.sql"),
    },
    // ── ADR #18 Phase 2: ledger → session linkage (§9c) ─────────
    // ALTER stock_movements + stock_movements_archive add a nullable
    // `inventory_transaction_id` FK pointing at migration 084's audit
    // session table. Nullable because legacy stock_movements rows
    // predate this audit framework. The transactional chain
    // (users ← inventory_transactions.staff_id ← stock_movements.inventory_transaction_id)
    // makes users.id hard-deletable-only-if-no-audit-history —
    // see §9c "on-delete chain note" inline.
    Migration {
        id: "085_stock_movements_inventory_transaction_fk.sql",
        sql: include_str!("../migrations/085_stock_movements_inventory_transaction_fk.sql"),
    },
    // ── ADR #18 Phase 2: staff shift accountability (§9d) ──────────
    // Bundles the inventory_shifts table and the FK column that links
    // inventory_transactions to a shift session. Per §9d, an inventory
    // shift is bound to one location; cross-location active shifts are
    // allowed via the (user_id, location_id) partial unique index —
    // this is the §13 finding 32 v2 amend that fixed the v1 (user_id)-only
    // contradiction with §9d's "one shift = one location" invariant.
    // The inventory_shift_id FK is NULLABLE so legacy transactions
    // (before §3 Rust API) and pre-shift sessions remain valid.
    Migration {
        id: "086_inventory_shifts.sql",
        sql: include_str!("../migrations/086_inventory_shifts.sql"),
    },
    // ── ADR #18 Phase 2: configurable threshold alerts (§9e) ────
    // Bundles §9e-i (stock_thresholds config table) + §9e-ii
    // (stock_alert_events lifecycle table) per the migration 084/086
    // sibling-table pattern. After 087 lands, the alert system has
    // a config baseline AND the lifecycle table; in-memory trigger
    // logic in Rust (Phase 2's runtime work) reads/writes these.
    // §9e-iii low_stock_alerts_at_location is a Rust function, no
    // migration needed.
    Migration {
        id: "087_stock_thresholds_alerts.sql",
        sql: include_str!("../migrations/087_stock_thresholds_alerts.sql"),
    },
    // ── ADR #18 Phase 1: stock_summary composite-PK (§2c) ────────
    // Rebuilds stock_summary with PRIMARY KEY (item_id, location_id).
    // Pairs with §2a's deferred inventory full-rebuild (still in
    // ADD COLUMN form from migration 079 because that one needed to
    // keep migration_069 tests green). This §2c rebuild is independent
    // — the materialised ledger aggregate and the §2a live stock
    // table can be sequenced independently. Without this rebuild,
    // §9e-iii low_stock_alerts_at_location returns aggregated
    // cross-location totals instead of per-location vectors.
    // NOTE: bundles with a Rust-side refactor of
    // crates/oz-core/src/db/stock_summary.rs::rebuild_stock_summary()
    // to `GROUP BY item_id, location_id` — required for correctness
    // since the old `GROUP BY item_id` will fail on the composite PK.
    Migration {
        id: "089_stock_summary_composite_pk.sql",
        sql: include_str!("../migrations/089_stock_summary_composite_pk.sql"),
    },
    // ── ADR #18 Phase 1: purchase order receiving flow (§8) ─────
    // ALTER purchase_orders ADD COLUMN location_id FK to
    // inventory_locations. Nullable per §8 — PO drafts may not yet
    // have a receiving location. The `adjust_stock_at_location_with_reason
    // (sku, +qty, location_id, 'purchase-order', ...)` receive flow
    // requires a non-null location_id at receive time (Rust-layer
    // constraint).
    Migration {
        id: "090_purchase_orders_location_id.sql",
        sql: include_str!("../migrations/090_purchase_orders_location_id.sql"),
    },
    // ── ADR #18 §3 + §13 finding 37: workspace rename cascade ─
    // Renames `inventory` → `warehouse` across all FK-referencing
    // tables atomically. The 8-site cascade per §13-37 also requires
    // file-level renames (ui directory, fluent bundles, manifest,
    // platform/startup, Rust crate) — those accompany this PR outside
    // the SQL migration and are documented inline.
    Migration {
        id: "091_workspace_types_rename.sql",
        sql: include_str!("../migrations/091_workspace_types_rename.sql"),
    },
    // ── ADR #19 Phase 3: sale-deduction runtime foundation ──
    // 092: rebuild_stock_summary GROUP BY (item_id, location_id) at SQL
    // layer (ADR-19 §15 criterion 19-1 — Rust function already aggregates;
    // this lands the equivalent SQL invariant so a fresh install passes
    // §9e-iii low_stock_alerts_at_location per-location vector query
    // even before any Rust code runs).
    Migration {
        id: "092_rebuild_stock_summary_group_by_location.sql",
        sql: include_str!("../migrations/092_rebuild_stock_summary_group_by_location.sql"),
    },
    // 093: adds `deduction_locations` JSON column to `sales` so the
    // `complete_sale_with_resolved_shortfalls` command can record per-line
    // per-location breakdown for void/refund inverse-flow fidelity (§2.4).
    // 094: locks the deduction location on `active_carts` at cart-start
    // time so the payment gateway capture always has a known stock source
    // BEFORE funds are captured (§5.1 pre-capture ordering).
    Migration {
        id: "093_sales_deduction_locations.sql",
        sql: include_str!("../migrations/093_sales_deduction_locations.sql"),
    },
    Migration {
        id: "094_active_carts_location_lock.sql",
        sql: include_str!("../migrations/094_active_carts_location_lock.sql"),
    },
    // 095: adds deduction_location_id FK to held_carts so restoring a held
    // cart also restores its deduction location lock (§6.3). Pre-095 held
    // carts have NULL — the Rust layer enforces non-NULL at runtime for
    // new carts in scoped workspaces.
    Migration {
        id: "095_held_carts_deduction_location.sql",
        sql: include_str!("../migrations/095_held_carts_deduction_location.sql"),
    },
    // 096: ADR-20 — three-phase sale lifecycle with stock reservation
    // before payment capture. Adds 'pending' to the sales.status CHECK
    // constraint, plus pending_expires_at, payment_reference, and
    // captured_at columns. Table is rebuilt because SQLite cannot ALTER
    // CHECK constraints. Also adds a partial index on pending_expires_at
    // for the stale-pending-sale reaper worker.
    Migration {
        id: "096_pending_sale_status.sql",
        sql: include_str!("../migrations/096_pending_sale_status.sql"),
    },
    // 097: ADR-20 §5 — idempotency key support for payment gateway
    // requests. Adds `idempotency_key TEXT` column to payments table
    // with a UNIQUE index (allows multiple NULLs for pre-097 payments).
    // The key is generated as UUIDv7 before each payment gateway request
    // and stored with the payment record; retries with the same key are
    // detected and prevented at the database layer.
    Migration {
        id: "097_payment_idempotency_keys.sql",
        sql: include_str!("../migrations/097_payment_idempotency_keys.sql"),
    },
    // 098: adds idx_customers_name for faster customer name-based lookups.
    Migration {
        id: "098_customers_name_index.sql",
        sql: include_str!("../migrations/098_customers_name_index.sql"),
    },
    // 099: adds idx_inventory_transactions_created for faster audit-log
    // queries ordered by created_at DESC.
    Migration {
        id: "099_inventory_transactions_created_at.sql",
        sql: include_str!("../migrations/099_inventory_transactions_created_at.sql"),
    },
    // ── ADR #22 Phase 0d: Settings delta ledger ──────────────────
    // Adds setting_updated table with per-(key, terminal_id) version
    // tracking. Enables concurrent-edit conflict detection and the
    // settings_updated event consumed by the SettingsContext provider.
    Migration {
        id: "100_setting_updated.sql",
        sql: include_str!("../migrations/100_setting_updated.sql"),
    },
    // 101: adds table_number column to kds_orders so KDS ticket cards
    // can display the assigned table number without the `as unknown` hack.
    Migration {
        id: "101_kds_table_number.sql",
        sql: include_str!("../migrations/101_kds_table_number.sql"),
    },
    // ── 103: KDS priority/rush flag ──────────────────────────────
    Migration {
        id: "103_kds_priority.sql",
        sql: include_str!("../migrations/103_kds_priority.sql"),
    },
    // ── 104: Hardware profiles DB store (TODO 4e) ────────────────
    // Stores per-terminal hardware config in the DB with schema versioning.
    // The JSON files in terminal_profiles/ remain as fallback/cache.
    Migration {
        id: "104_hardware_profiles.sql",
        sql: include_str!("../migrations/104_hardware_profiles.sql"),
    },
    // ── 105: KDS line items (TODO 2a) ────────────────────────────
    // Structured per-item data for kitchen tickets — replaces the flat
    // items_summary string with course, modifier, and per-item status.
    Migration {
        id: "105_kds_line_items.sql",
        sql: include_str!("../migrations/105_kds_line_items.sql"),
    },
    // ── 106: Sale lines course + modifier enrichment (TODO 2a) ───
    // Adds course and modifiers_json columns so the POS → KDS pipeline
    // carries structured item data instead of a flat summary string.
    Migration {
        id: "106_sale_lines_course_modifier.sql",
        sql: include_str!("../migrations/106_sale_lines_course_modifier.sql"),
    },
    // ── 107: Loyalty integrity constraints and tier validation ─────
    // Prevents duplicate earn/redeem projections for one account and sale,
    // and rejects invalid tier configuration at the database boundary.
    Migration {
        id: "107_loyalty_integrity.sql",
        sql: include_str!("../migrations/107_loyalty_integrity.sql"),
    },
    // ── 108: Tax single-default invariant (TAX-02) ─────────────────
    // Normalises any legacy multiple-default rows and adds a partial
    // UNIQUE index so SQLite rejects a second is_default = 1 row,
    // closing the concurrency/failure window in default switching.
    Migration {
        id: "108_tax_single_default.sql",
        sql: include_str!("../migrations/108_tax_single_default.sql"),
    },
    // ── 109: Tax soft-delete flag (TAX-03) ─────────────────────────
    // Adds is_active so archiving a rate preserves historical
    // sale-line linkage instead of hard-deleting; archiving a rate
    // still referenced by historical sales is blocked at the app layer.
    Migration {
        id: "109_tax_soft_delete.sql",
        sql: include_str!("../migrations/109_tax_soft_delete.sql"),
    },
    // ── 110: Per-line tax breakdown (TAX-02 auditability) ─────────
    // Persists the full per-rate breakdown on each sale line so
    // receipts/audit trails can reconstruct multi-rate taxation even
    // after a rate is archived or renamed (today only the first rate
    // id survives on `tax_rate_id`).
    Migration {
        id: "110_sale_line_tax_breakdown.sql",
        sql: include_str!("../migrations/110_sale_line_tax_breakdown.sql"),
    },
    // ── 111: Device-scoped login abuse controls (audit/06 STAFF-07) ─
    // Adds device_id to login_attempts so the rate limiter can combine
    // per-account throttling with per-device and global limits, using
    // exponential backoff instead of a fixed short lock.
    Migration {
        id: "111_login_attempts_device.sql",
        sql: include_str!("../migrations/111_login_attempts_device.sql"),
    },
    // ── 112: Store-local transfer actor FK removal (audit/07 INV-03) ─
    // Transfer actor IDs are derived from the global session identity.
    // Remove the obsolete local users foreign keys so a store database
    // never needs fake authentication rows for a legitimate actor.
    Migration {
        id: "112_stock_transfer_actor_ids.sql",
        sql: include_str!("../migrations/112_stock_transfer_actor_ids.sql"),
    },
    // ── 113: Store-local stock-count actor FK removal (audit/07 INV-03) ─
    // Count and adjustment actor IDs are derived from the global session
    // identity. Remove obsolete local users foreign keys while preserving
    // existing count data and adding non-negative quantity constraints.
    Migration {
        id: "113_stock_count_actor_ids.sql",
        sql: include_str!("../migrations/113_stock_count_actor_ids.sql"),
    },
    // ── 114: Durable sync pull anchor + idempotency ledger (audit/09 SYNC-01) ─
    // sync_pull_state persists the daemon's pull anchor/cursor so remote
    // updates are only fetched since the last applied page; sync_applied_items
    // is a receipt ledger so a replayed remote item is never applied twice
    // (previously every daemon cycle pulled the whole queue and re-applied
    // stock/sale mutations, silently corrupting inventory).
    Migration {
        id: "114_sync_pull_state.sql",
        sql: include_str!("../migrations/114_sync_pull_state.sql"),
    },
    Migration {
        id: "115_audit_review_checkpoints.sql",
        sql: include_str!("../migrations/115_audit_review_checkpoints.sql"),
    },
    // ── DB-08: unique (key, terminal_id, version) (audit/29) ──────
    // setting_updated's version allocation is MAX(version)+1 in app code;
    // two concurrent writers could insert the same version. 116 collapses
    // legacy duplicates and adds a UNIQUE index so the database rejects
    // duplicate versions (fail closed) instead of corrupting the delta
    // ledger.
    Migration {
        id: "116_setting_updated_unique_version.sql",
        sql: include_str!("../migrations/116_setting_updated_unique_version.sql"),
    },
    // ── DB-04 end-state: store_id FK on ADR #4 domain tables (audit/29) ─
    // 069 added nullable store_id to products/sales/sale_lines/customers
    // without any database-level link to the store catalog. 117 rebuilds
    // those four tables with `store_id REFERENCES store_profiles(id)` and
    // quarantines orphaned store_ids to NULL (the documented global
    // sentinel). NULL remains valid — per-store DB files are the primary
    // isolation mechanism, so forcing NOT NULL would break that model.
    Migration {
        id: "117_scoping_store_id_fk.sql",
        sql: include_str!("../migrations/117_scoping_store_id_fk.sql"),
    },
    // ── warehouse_id supersession cleanup ────────────────────────
    // 069 added nullable warehouse_id to inventory/stock_counts as a
    // speculative multi-warehouse hook. ADR #18 superseded it: warehouses
    // are inventory_locations rows with type='warehouse' and 079's
    // inventory.location_id FK is the real catalog link. Zero code reads
    // or writes warehouse_id, so 118 drops the dead columns + their
    // unused index (docs: 118_drop_warehouse_id_superseded.sql).
    Migration {
        id: "118_drop_warehouse_id_superseded.sql",
        sql: include_str!("../migrations/118_drop_warehouse_id_superseded.sql"),
    },
    // ── SYNC-08: durable remote failure/dead-letter ledger ─────────
    Migration {
        id: "119_sync_remote_failures.sql",
        sql: include_str!("../migrations/119_sync_remote_failures.sql"),
    },
    // ── Self-healing repair: re-seed default workspace instances ──────
    // Databases created during the migration 066 regression window can have
    // an empty workspace_instances table (066 dropped rows whose store_id was
    // not yet in store_profiles). Because 066 is already "applied", it never
    // re-runs, so the owner logs in to an empty workspace picker. This
    // idempotently re-seeds the canonical default instances per store using
    // the current workspace_types keys.
    Migration {
        id: "120_reseed_default_workspace_instances.sql",
        sql: include_str!("../migrations/120_reseed_default_workspace_instances.sql"),
    },
    // ── Follow-up: keep 120 immutable, repair multi-store seeding in 121 ──
    // 120's original definition seeds under COALESCE(primary, 'default'); on
    // a multi-store DB with no primary at migration time that lands the
    // canonical instances under store_id = 'default', invisible to the
    // store-scoped picker. Because 120 is already applied on upgraded
    // databases it cannot be edited (audit/29 DB-02), so 121 re-seeds with
    // the improved COALESCE for fresh DBs and re-points the rows 120 seeded
    // under 'default' to the store's own profile for upgraded DBs.
    Migration {
        id: "121_workspace_instances_store_own_profile.sql",
        sql: include_str!("../migrations/121_workspace_instances_store_own_profile.sql"),
    },
    // ── Topology builder: separate controlled business purpose from type/name ──
    Migration {
        id: "122_workspace_instance_purpose.sql",
        sql: include_str!("../migrations/122_workspace_instance_purpose.sql"),
    },
    // ── Topology runtime consumer: persist selected KDS target ────────
    Migration {
        id: "123_kds_target_instance.sql",
        sql: include_str!("../migrations/123_kds_target_instance.sql"),
    },
    // ── Topology runtime consumer: normalized KDS fan-out targets ─────
    Migration {
        id: "124_kds_order_targets.sql",
        sql: include_str!("../migrations/124_kds_order_targets.sql"),
    },
    // ── Sync auth: registered sync terminals (ADR sync-auth-hardening P3) ──
    Migration {
        id: "125_sync_terminals.sql",
        sql: include_str!("../migrations/125_sync_terminals.sql"),
    },
    // ── Sync plan gating: per-tenant cloud sync plans (ADR sync-plan-gating) ──
    Migration {
        id: "126_tenant_plans.sql",
        sql: include_str!("../migrations/126_tenant_plans.sql"),
    },
    // ── Sync plan gating: Stripe customer → tenant mapping for subscription webhooks ──
    Migration {
        id: "127_stripe_customers.sql",
        sql: include_str!("../migrations/127_stripe_customers.sql"),
    },
    Migration {
        id: "128_assignments.sql",
        sql: include_str!("../migrations/128_assignments.sql"),
    },
    Migration {
        id: "129_retire_cashier_kitchen.sql",
        sql: include_str!("../migrations/129_retire_cashier_kitchen.sql"),
    },
    Migration {
        id: "130_user_profiles.sql",
        sql: include_str!("../migrations/130_user_profiles.sql"),
    },
    Migration {
        id: "131_user_profiles_national_id_hash.sql",
        sql: include_str!("../migrations/131_user_profiles_national_id_hash.sql"),
    },
    // ── ADR #36 D1: retail merchandising attributes ──────────────
    // brand/rack_location/notes/unit (free text), is_active (hide without
    // delete), default_supplier_id (FK to suppliers, local-only). cost_minor
    // already exists (054).
    Migration {
        id: "132_product_attributes.sql",
        sql: include_str!("../migrations/132_product_attributes.sql"),
    },
    // ── ADR #37 D3: popularity signal ledger + materialized score ─
    // product_activity (search/edit events) + products.popularity_score.
    // Sales signal reads sale_lines directly; the score is recomputed by the
    // formula in code and is local-only (never synced).
    Migration {
        id: "133_product_activity.sql",
        sql: include_str!("../migrations/133_product_activity.sql"),
    },
    // ── ADR #37 backfill: seed popularity from pre-feature history ──
    // product_activity starts empty on upgrade; 134 seeds one synthetic
    // 'edit' event per product at its last update timestamp (within the
    // decay window) so the retail grid's default popularity sort ranks
    // recently-managed products from day one. Sales need no seeding — the
    // full-catalog pass at store open reads sale_lines directly. Search
    // history was never recorded, so that signal starts cold by design.
    Migration {
        id: "134_popularity_backfill.sql",
        sql: include_str!("../migrations/134_popularity_backfill.sql"),
    },
    // ── ADR #36 reporting: freeze HPP into sale_lines ────────────
    // sale_lines.cost_minor snapshots the product's cost at checkout so
    // historical margins never change when cost_minor is edited later.
    // Backfills existing rows with the current product cost and reports
    // fall back COALESCE(sl.cost_minor, p.cost_minor, 0).
    Migration {
        id: "135_sale_line_cost_snapshot.sql",
        sql: include_str!("../migrations/135_sale_line_cost_snapshot.sql"),
    },
    Migration {
        id: "136_processed_webhooks.sql",
        sql: include_str!("../migrations/136_processed_webhooks.sql"),
    },
];

/// Apply every unapplied migration and configure runtime PRAGMAs.
///
/// After migrations, sets WAL journal mode + busy_timeout for better
/// concurrent-read performance and multi-connection safety, and enables
/// foreign key enforcement (SQLite defaults to OFF). These are idempotent
/// — safe to call on every startup.
pub fn run(conn: &mut rusqlite::Connection) -> Result<(), crate::CoreError> {
    platform_core::database::run(conn, ALL)?;
    // WAL mode enables concurrent reads while a write is in progress.
    // busy_timeout prevents "database is locked" errors when multiple
    // connections contend for the write lock (default is 0 = immediate fail).
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "busy_timeout", "5000")?;
    // synchronous=NORMAL is safe in WAL mode (the WAL itself provides
    // durability) and yields 2–3× faster writes than the default FULL.
    // For a local POS database, only a power loss or hard shutdown
    // (without fsync) loses the most recent transaction, which the
    // offline queue recovers from.
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // Enable foreign key enforcement. SQLite defaults to OFF — the setting
    // is per-connection, so we must set it on every connection open.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

/// Create a fresh in-memory database with all migrations already applied.
///
/// Uses a [`std::sync::LazyLock`]ed pre-migrated snapshot connection.
/// The first call runs all migrations once; subsequent calls clone the
/// snapshot via SQLite's page-level [`rusqlite::backup::Backup`] API —
/// orders of magnitude faster than re-running `execute_batch` per test.
///
/// # Panics
///
/// Panics if the database cannot be created.
#[doc(hidden)]
pub fn fresh_db() -> rusqlite::Connection {
    use std::sync::{LazyLock, Mutex};

    /// Pre-migrated snapshot — built once, cloned for every test.
    static SNAPSHOT: LazyLock<Mutex<rusqlite::Connection>> = LazyLock::new(|| {
        use std::sync::OnceLock;

        fn cached_sql() -> &'static str {
            static SQL: OnceLock<String> = OnceLock::new();
            SQL.get_or_init(|| {
                let mut buf = String::with_capacity(48_000);
                buf.push_str("PRAGMA foreign_keys = ON;\n");
                buf.push_str(
                    "CREATE TABLE IF NOT EXISTS schema_migrations (\n\
                     id         TEXT PRIMARY KEY,\n\
                     applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),\n\
                     checksum   TEXT\n\
                     );\n",
                );
                for mig in ALL {
                    buf.push_str("BEGIN;\n");
                    buf.push_str(mig.sql);
                    buf.push('\n');
                    buf.push_str("INSERT INTO schema_migrations (id) VALUES ('");
                    buf.push_str(mig.id);
                    buf.push_str("');\n");
                    buf.push_str("COMMIT;\n");
                }
                buf
            })
        }

        let conn = rusqlite::Connection::open_in_memory().unwrap(); // SAFETY: in-memory test DB open cannot fail; failure is a harness programming error (see fresh_db # Panics)
        conn.execute_batch(cached_sql()).unwrap(); // SAFETY: SQL is compile-time embedded from `ALL`; syntax errors fail the test suite, not a live process
        Mutex::new(conn)
    });

    let mut fresh = rusqlite::Connection::open_in_memory().unwrap(); // SAFETY: in-memory test DB open cannot fail (fresh_db # Panics)
    {
        let snapshot = SNAPSHOT.lock().unwrap(); // SAFETY: lock is only poisoned if the snapshot init closure panicked, which is a test harness bug
        let backup = rusqlite::backup::Backup::new(&snapshot, &mut fresh).unwrap(); // SAFETY: both connections are valid in-memory SQLite handles; Backup::new cannot fail
        backup
            .run_to_completion(100, std::time::Duration::from_millis(0), None)
            .unwrap(); // SAFETY: page copy between two in-memory DBs cannot fail at runtime
    } // drop Backup (releases &mut fresh borrow), then drop MutexGuard
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn
    }

    #[test]
    fn first_run_applies_all_migrations() {
        let mut conn = fresh();
        run(&mut conn).unwrap();
        let mut stmt = conn.prepare("SELECT id FROM schema_migrations").unwrap();
        let applied: std::collections::HashSet<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for mig in ALL {
            assert!(
                applied.contains(mig.id),
                "missing applied entry for {}",
                mig.id
            );
        }
    }

    #[test]
    fn second_run_is_idempotent() {
        let mut conn = fresh();
        run(&mut conn).unwrap();
        run(&mut conn).unwrap();
        let mut stmt = conn.prepare("SELECT id FROM schema_migrations").unwrap();
        let applied: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(applied.len(), ALL.len());
    }

    #[test]
    fn migration_creates_sales_table() {
        let mut conn = fresh();
        run(&mut conn).unwrap();
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sales'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "expected `sales` table after migration");
    }

    #[test]
    fn all_migrations_have_ids() {
        for mig in ALL {
            assert!(!mig.id.is_empty(), "migration id must not be empty");
            assert!(
                mig.id.ends_with(".sql"),
                "migration id should end with .sql"
            );
        }
    }

    #[test]
    fn all_migrations_have_sql_content() {
        for mig in ALL {
            assert!(!mig.sql.is_empty(), "migration {} has empty SQL", mig.id);
        }
    }

    #[test]
    fn all_migration_ids_are_unique() {
        let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for mig in ALL {
            assert!(ids.insert(mig.id), "duplicate migration id: {}", mig.id);
        }
    }

    #[test]
    fn migration_registry_matches_filesystem() {
        // DB-01: the registry is the source of truth. Every `.sql` file under
        // crates/oz-core/migrations/ must have EXACTLY ONE registry entry,
        // and every registry entry must resolve to a real file. A new SQL
        // file that is never registered (or a registered entry whose file
        // was deleted) silently changes what fresh installs vs upgrades
        // produce, so this must fail at test time.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .expect("migrations directory must exist")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".sql"))
            .collect();
        files.sort();

        let mut registered: Vec<&str> = ALL.iter().map(|m| m.id).collect();
        registered.sort_unstable();

        // Every file on disk must be registered exactly once.
        let mut seen_files: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for file in &files {
            assert!(
                ALL.iter().any(|m| m.id == file),
                "DB-01: migration file {file} exists on disk but has NO registry entry in ALL — add it or the runner will skip it"
            );
            assert!(
                seen_files.insert(file),
                "DB-01: migration file {file} is registered more than once"
            );
        }

        // Every registry entry must have a real file on disk.
        for id in &registered {
            assert!(
                files.iter().any(|f| f == id),
                "DB-01: registry entry {id} has no matching file in migrations/ — remove the entry or restore the file"
            );
        }

        assert_eq!(
            files.len(),
            registered.len(),
            "DB-01: registry/file parity broken — {} files vs {} registered entries",
            files.len(),
            registered.len()
        );
    }

    /// Extract the numeric prefix from a migration id (`"046_gift_cards.sql"` → `046`).
    fn numeric_prefix(id: &str) -> u32 {
        let stem = id.split('.').next().unwrap_or(id);
        stem.split('_').next().unwrap_or("").parse().unwrap_or(0)
    }

    #[test]
    fn migration_prefixes_are_unique_after_legacy_shared_block() {
        // RUST-09: migrations 046 and 047 each intentionally share numeric
        // prefixes (documented legacy batching). New migrations MUST use a
        // unique sequential prefix — a duplicate prefix on any migration
        // after 047 is a registry error that can mask ordering mistakes.
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut in_legacy_block = true;
        for mig in ALL {
            let prefix = numeric_prefix(mig.id);
            if prefix > 47 {
                in_legacy_block = false;
            }
            if !in_legacy_block {
                assert!(
                    seen.insert(prefix),
                    "duplicate migration prefix {prefix:03} on {} (unique prefixes required after 047)",
                    mig.id
                );
            }
        }
    }

    #[test]
    fn migration_prefixes_are_monotonic_after_legacy_block() {
        // RUST-09: the runner applies migrations in compile-time array order,
        // so that order is canonical. After the legacy 046/047 block the
        // numeric prefixes must be strictly increasing — a fresh install and
        // an upgrade must converge on the same schema regardless of entry
        // insertion point.
        let mut prev: Option<u32> = None;
        let mut past_legacy = false;
        for mig in ALL {
            let prefix = numeric_prefix(mig.id);
            if prefix > 47 {
                past_legacy = true;
            }
            if past_legacy {
                if let Some(p) = prev {
                    assert!(
                        prefix > p,
                        "migration {} has prefix {prefix:03} which is not greater than previous {p:03} — array order is canonical (RUST-09)",
                        mig.id
                    );
                }
                prev = Some(prefix);
            }
        }
    }

    #[test]
    fn fresh_install_and_upgrade_path_produce_identical_schema() {
        // RUST-09/RUST-10: applying all migrations to an empty DB (fresh
        // install) must yield the same schema as applying a prefix of the
        // registry and then upgrading through the remainder (an upgrade from
        // an older release). Compare the full table/column/index surface.
        fn schema_fingerprint(
            conn: &rusqlite::Connection,
        ) -> std::collections::BTreeMap<String, Vec<String>> {
            let mut tables: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            let mut stmt = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
                )
                .unwrap();
            let names: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            drop(stmt);
            for name in names {
                let mut cols: Vec<String> = Vec::new();
                let mut cstmt = conn
                    .prepare(&format!("PRAGMA table_info(\"{name}\")"))
                    .unwrap();
                let rows = cstmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, i64>(3)?,
                            r.get::<_, Option<String>>(4)?,
                            r.get::<_, i64>(5)?,
                        ))
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                for (cid, ctype, notnull, dflt, pk) in rows {
                    cols.push(format!("{cid}|{ctype}|{notnull}|{dflt:?}|{pk}"));
                }
                tables.insert(name, cols);
            }
            tables
        }

        // Fresh install: run every migration in one pass.
        let mut fresh_conn = fresh();
        run(&mut fresh_conn).unwrap();
        let fresh_schema = schema_fingerprint(&fresh_conn);

        // Upgrade path: apply the first 80 (a plausible older release), then
        // the remainder through the same registry runner.
        let split = 80usize.min(ALL.len());
        let mut upgrade_conn = fresh();
        platform_core::database::run(&mut upgrade_conn, &ALL[..split]).unwrap();
        platform_core::database::run(&mut upgrade_conn, &ALL[split..]).unwrap();
        let upgrade_schema = schema_fingerprint(&upgrade_conn);

        assert_eq!(
            fresh_schema, upgrade_schema,
            "fresh install and upgrade path diverged — schema drift (RUST-09/RUST-10)"
        );
    }

    // ── Backfill migration 134: seed popularity edit events ──
    //
    // On upgrade, product_activity starts empty so the popularity formula
    // has no search/edit history. 134 seeds one synthetic 'edit' event per
    // product at its most recent update timestamp (within the 90-day decay
    // window), letting the retail grid's default popularity sort rank
    // recently-managed products from day one.
    #[test]
    fn migration_134_backfills_edit_events_from_product_timestamps() {
        // 1. Fully-migrated DB (as an upgrade would): 134 ran against an
        //    empty catalog, so no rows were seeded.
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // 2. Simulate a pre-existing catalog: products updated inside the
        //    window, outside it, and one kept alive only by a recent price
        //    change.
        fn ts(days_ago: i64) -> String {
            chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::days(days_ago))
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        }
        // Capture once: the migration stores timestamps verbatim, so the
        // assertions below must compare against the exact same strings.
        let (recent_updated, old_updated, price_updated, price_created) =
            (ts(2), ts(300), ts(10), ts(400));
        conn.execute_batch(
            &format!(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at, price_updated_at) VALUES
                ('p-recent', 'SKU-RECENT', 'Recent', 1000, 'USD', '{price_created}', '{recent_updated}', ''),
                ('p-old',    'SKU-OLD',    'Old',    1000, 'USD', '{price_created}', '{old_updated}', ''),
                ('p-price',  'SKU-PRICE',  'Price',  1000, 'USD', '{price_created}', '{old_updated}', '{price_updated}');",
            ),
        )
        .unwrap();

        // 3. Mark 134 as not yet applied and re-run migrations (the upgrade
        //    path that ships this migration on an existing store).
        conn.execute(
            "DELETE FROM schema_migrations WHERE id = '134_popularity_backfill.sql'",
            [],
        )
        .unwrap();
        run(&mut conn).unwrap();

        // 4. Exactly the in-window products get one synthetic edit event,
        //    dated at their most recent modification.
        let mut stmt = conn
            .prepare("SELECT sku, created_at FROM product_activity WHERE event_type = 'edit'")
            .unwrap();
        let rows: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        drop(stmt);

        let mut by_sku: std::collections::HashMap<String, String> = rows.into_iter().collect();
        assert_eq!(
            by_sku.len(),
            2,
            "backfill must seed exactly the in-window products (got: {by_sku:?})"
        );
        let recent = by_sku.remove("SKU-RECENT").expect("recent product seeded");
        assert_eq!(
            recent, recent_updated,
            "event must be dated at the product's last update"
        );
        let price = by_sku.remove("SKU-PRICE").expect("price product seeded");
        assert_eq!(
            price, price_updated,
            "price_updated_at wins when it is newer than updated_at"
        );
        assert!(
            !by_sku.contains_key("SKU-OLD"),
            "product last touched outside the decay window must not be seeded"
        );

        // 5. Idempotence: a second re-run must not duplicate rows.
        run(&mut conn).unwrap();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM product_activity", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 2, "migration 134 must run exactly once");
    }

    // ── Backfill migration 135: freeze HPP into sale_lines ──
    //
    // On upgrade, existing sale_lines have no cost snapshot. 135 backfills
    // each line with the product's current cost so pre-existing rows are
    // frozen at what the reports displayed before the migration. A product
    // cost of 0 ("not set") normalizes to NULL.
    #[test]
    fn migration_135_backfills_cost_snapshot_from_product_cost() {
        // 1. Simulate a pre-feature release: apply every migration EXCEPT
        //    135 (the snapshot column does not exist yet), then seed a
        //    catalog with one costed product, one without a cost, and a
        //    completed sale referencing both.
        //
        // 135 is not the last migration in ALL (136_processed_webhooks was
        // appended later), so slice at its actual position rather than
        // ALL.len() - 1 — a naive tail cut would run 135 before the seed
        // data exists and the backfill would find no rows.
        let mut conn = fresh();
        let split = ALL
            .iter()
            .position(|m| m.id == "135_sale_line_cost_snapshot.sql")
            .expect("migration 135 must be registered in ALL");
        platform_core::database::run(&mut conn, &ALL[..split]).unwrap();
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, cost_minor, created_at, updated_at) VALUES
             ('p-1', 'COSTED', 'Costed', 1000, 'USD', 800, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
             ('p-2', 'FREE',   'Free',   1000, 'USD', 0,   '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
             ('s-1', 3000, 'USD', 2, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
             ('sl-1', 's-1', 'COSTED', 2, 1000, 2000, 'USD', 1),
             ('sl-2', 's-1', 'FREE',   1, 1000, 1000, 'USD', 2);",
        )
        .unwrap();

        // 2. Apply the remainder (135) exactly as an upgrade would.
        platform_core::database::run(&mut conn, &ALL[split..]).unwrap();

        // 3. Costed line frozen at the product cost; unset cost → NULL.
        let costed: Option<i64> = conn
            .query_row(
                "SELECT cost_minor FROM sale_lines WHERE id = 'sl-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(costed, Some(800));
        let free: Option<i64> = conn
            .query_row(
                "SELECT cost_minor FROM sale_lines WHERE id = 'sl-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(free, None, "0 product cost normalizes to NULL");
    }

    // ── Repair migration 120: re-seed default workspace instances ──
    //
    // Simulates the migration 066 regression window: a database where
    // workspace_instances was emptied (066 dropped rows whose store_id was
    // not yet in store_profiles) and 066 is already recorded as applied, so
    // it never re-runs. Re-running migrations must restore the default
    // instances via 120, and must do so idempotently (no duplicate rows).
    #[test]
    fn migration_120_reseeds_empty_workspace_instances() {
        // 1. Build a fully-migrated DB (as an upgrade would): defaults are
        //    seeded and 120 is recorded as applied.
        let mut conn = fresh();
        run(&mut conn).unwrap();
        let seeded: i64 = conn
            .query_row("SELECT COUNT(*) FROM workspace_instances", [], |r| r.get(0))
            .unwrap();
        assert!(
            seeded > 0,
            "precondition: defaults seeded by prior migrations"
        );

        // 2. Simulate the broken window: wipe workspace_instances AND drop the
        //    120 row from schema_migrations so the runner treats it as not yet
        //    applied (066 stays "applied", so it will NOT re-run).
        conn.execute("DELETE FROM workspace_instances", []).unwrap();
        conn.execute(
            "DELETE FROM schema_migrations WHERE id = '120_reseed_default_workspace_instances.sql'",
            [],
        )
        .unwrap();
        let after_wipe: i64 = conn
            .query_row("SELECT COUNT(*) FROM workspace_instances", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after_wipe, 0, "simulated broken-window empty table");

        // 3. Re-run migrations — 066 is a no-op, but 120 must re-seed.
        run(&mut conn).unwrap();
        let after_repair: i64 = conn
            .query_row("SELECT COUNT(*) FROM workspace_instances", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            after_repair, seeded,
            "migration 120 must restore the default instances"
        );

        // 4. Idempotency: a second re-run must not duplicate rows.
        run(&mut conn).unwrap();
        let after_second: i64 = conn
            .query_row("SELECT COUNT(*) FROM workspace_instances", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after_second, seeded, "migration 120 must be idempotent");
    }

    //
    // Simulates a multi-store upgrade: migration 120 (original definition)
    // seeded the canonical instances under store_id = 'default' because no
    // profile was primary at migration time. Because 120 is already applied
    // it cannot be re-defined (audit/29 DB-02); 121 must re-point those rows
    // to the store's own non-default profile so the store-scoped picker
    // (wi.store_id = ?) lists them again.
    #[test]
    fn migration_121_repoints_instances_seeded_under_default_store() {
        // 1. Fully-migrated DB (as an upgrade would): 120 seeded the
        //    canonical instances, and 121's re-point found no non-default
        //    profile yet, so they sit under 'default'.
        let mut conn = fresh();
        run(&mut conn).unwrap();
        let canonical: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_instances WHERE id LIKE 'default-%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(canonical > 0, "precondition: canonical instances exist");

        // 2. The app adds a named store profile (no primary yet — 025's
        //    legacy 'default' row has is_primary = 0, matching the real
        //    store-DB bootstrap). Simulate the original-120 seeding state and
        //    mark 121 as not yet applied, as on a real upgrade.
        conn.execute(
            "INSERT INTO store_profiles (id, name, currency, timezone) \
             VALUES ('store-x', 'Store X', 'USD', 'UTC')",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE workspace_instances SET store_id = 'default' WHERE id LIKE 'default-%'",
            [],
        )
        .unwrap();
        conn.execute(
            "DELETE FROM schema_migrations \
             WHERE id = '121_workspace_instances_store_own_profile.sql'",
            [],
        )
        .unwrap();

        // 3. Re-run — 121 must re-point the canonical instances to the
        //    store's own profile.
        run(&mut conn).unwrap();
        let repointed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_instances \
                 WHERE id LIKE 'default-%' AND store_id = 'store-x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            repointed, canonical,
            "121 must re-point every canonical instance to the store's own profile"
        );
        let still_default: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_instances \
                 WHERE id LIKE 'default-%' AND store_id = 'default'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            still_default, 0,
            "no canonical instance may remain under 'default' when a better profile exists"
        );

        // 4. Idempotency: a second run must not change anything.
        run(&mut conn).unwrap();
        let after_second: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_instances \
                 WHERE id LIKE 'default-%' AND store_id = 'store-x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(after_second, repointed, "migration 121 must be idempotent");
    }

    #[test]
    fn migrations_create_expected_tables() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        let expected_tables = [
            "sales",
            "sale_lines",
            "products",
            "categories",
            "inventory",
            "settings",
            "customers",
            "currencies",
            "exchange_rates",
            "tax_rates",
            "audit_log",
            "users",
            "roles",
            "offline_queue",
            "refunds",
            "refund_lines",
            "terminals",
            "product_taxes",
            "held_carts",
            "product_variants",
            "product_recipes",
            "modifier_groups",
            "modifiers",
            "product_modifier_groups",
            "category_taxes",
            "payments",
            "cash_payouts",
            "store_profiles",
            "terminal_feature_overrides",
            "promotions",
            "promotion_applications",
            "loyalty_tiers",
            "loyalty_accounts",
            "loyalty_transactions",
            "gift_cards",
            "gift_card_transactions",
            "suppliers",
            "stock_counts",
            "stock_count_lines",
            "stock_adjustments",
            "purchase_orders",
            "purchase_order_lines",
            "stock_transfers",
            "stock_transfer_lines",
            "terminal_profiles",
            "kds_orders",
            "kds_daily_counters",
            "active_carts",
            "tables",
            "workspaces",
            "workspace_screens",
            "role_workspaces",
            "user_workspaces",
            "workspace_types",
            "workspace_type_screens",
            "workspace_instances",
            "user_workspace_instances",
            "role_workspace_types",
            "login_attempts",
            "user_store_access",
            // ── ADR #18 Phase 1+2 (migrations 078-090) ──
            "inventory_locations",
            "workspace_inventory_locations",
            "inventory_transactions",
            "inventory_transaction_lines",
            "inventory_shifts",
            "stock_thresholds",
            "stock_alert_events",
            // ── ADR #19 Phase 3 (migrations 093-094) ──
            // 093 adds deduction_locations column to sales (no new table).
            // 094 adds deduction_location_id + location_override_at to active_carts (no new table).
            // ── ADR #22 Phase 0d (migration 100) ──
            "setting_updated",
            // ── audit/09 SYNC-01 (migration 114) ──
            "sync_pull_state",
            "sync_applied_items",
            "sync_remote_failures",
            // ── ADR #35 D5 (migration 128) ──
            "assignments",
            "assignment_branches",
            "assignment_workspaces",
        ];

        for table in &expected_tables {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                exists, 1,
                "expected table `{table}` to exist after migration"
            );
        }
    }

    // ── ADR #4 Phase 2: Data Scoping tests ─────────────────────────

    #[test]
    fn migration_069_adds_scoping_columns() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // store_id columns exist on domain tables.
        for table in &["products", "sales", "sale_lines", "customers"] {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = 'store_id'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "{table} missing store_id column");
        }

        // warehouse_id columns are GONE from the end-state schema. 069
        // added them to inventory and stock_counts as a speculative
        // multi-warehouse hook, but migration 118 dropped them as
        // superseded by ADR #18: warehouses are inventory_locations rows
        // with type='warehouse' and 079's inventory.location_id FK is the
        // real catalog link. Asserting absence here pins the cleanup so a
        // future migration cannot silently resurrect the dead column.
        for table in &["inventory", "stock_counts"] {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = 'warehouse_id'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                count, 0,
                "{table} must not have warehouse_id after migration 118"
            );
        }
    }

    #[test]
    fn migration_069_creates_scoping_indexes() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // idx_inventory_warehouse_product is deliberately NOT listed:
        // migration 118 dropped it together with the superseded
        // warehouse_id column (the index existed only for a predicate no
        // query ever issued — see 118_drop_warehouse_id_superseded.sql).
        let expected_indexes = [
            "idx_sales_store_status",
            "idx_sale_lines_store_sale",
            "idx_products_store_category",
            "idx_customers_store",
        ];

        for index in &expected_indexes {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    rusqlite::params![index],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing index {index}");
        }
    }

    #[test]
    fn migration_069_scoping_columns_nullable() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Insert a product without store_id — should default to NULL.
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type)
             VALUES ('prod-scope', 'SKU-SCOPE', 'Scope Test', 100, 'USD', 'retail')",
            [],
        )
        .unwrap();

        let store_id: Option<String> = conn
            .query_row(
                "SELECT store_id FROM products WHERE id = 'prod-scope'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(store_id.is_none(), "store_id should default to NULL");

        // Insert a sale without store_id — should default to NULL.
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status)
             VALUES ('sale-scope', 500, 'USD', 1, 'completed')",
            [],
        )
        .unwrap();

        let sale_store_id: Option<String> = conn
            .query_row(
                "SELECT store_id FROM sales WHERE id = 'sale-scope'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(sale_store_id.is_none(), "store_id should default to NULL");
    }

    // ── ADR #18 Phase 0A: Inventory Locations canonical seeds ──

    #[test]
    fn migration_078_seeds_canonical_default_and_transit_locations() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // ADR-18 §13 finding 36: the canonical UUIDs are FROZEN and
        // propagate uniformly through §2a/§2b/§2d/§5 migrations. A
        // regression that drops one of these seeds breaks every
        // downstream migration that defaults `location_id` to the
        // canonical value (migrations 079, 080, 082, 085, 089).
        // DO NOT replace the literals below with
        // `crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID` (or the
        // transit counterpart) — this test asserts the schema seed value,
        // so substituting the const would make the assertion circular /
        // self-referential (`const == seeded-const`). The const's own
        // docstring in `inventory.rs` documents this exception.
        let default_uuid = "01926b3a-0000-7000-8000-000000000001";
        let transit_uuid = "01926b3a-0000-7000-8000-000000000002";

        // Runtime drift guard — fires only in debug builds. Catches drift
        // between this test-assertion literal and the Rust const even if
        // both prose comments above get deleted by a future automated
        // cleanup pass. The transit counterpart intentionally has no
        // CANONICAL_TRANSIT_LOCATION_UUID const (SQL-only concern per the
        // inventory.rs const docstring), so no equivalent guard needed.
        debug_assert_eq!(
            default_uuid,
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            "test-assertion literal drifted from CANONICAL_DEFAULT_LOCATION_UUID const"
        );

        let default_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM inventory_locations WHERE id = ?1",
                rusqlite::params![default_uuid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            default_count, 1,
            "missing canonical default-location UUID seed (01926b3a-...-001)"
        );

        let transit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM inventory_locations WHERE id = ?1",
                rusqlite::params![transit_uuid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            transit_count, 1,
            "missing canonical transit-location UUID seed (01926b3a-...-002)"
        );

        // Also verify the human-readable names match ADR §1 expectations
        // — app lookup-by-name relies on this.
        let default_name: String = conn
            .query_row(
                "SELECT name FROM inventory_locations WHERE id = ?1",
                rusqlite::params![default_uuid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(default_name, "Default Inventory");

        let transit_name: String = conn
            .query_row(
                "SELECT name FROM inventory_locations WHERE id = ?1",
                rusqlite::params![transit_uuid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(transit_name, "In Transit");
    }

    #[test]
    fn migration_078_inventory_locations_enforces_active_name_uniqueness() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // The partial UNIQUE INDEX idx_inventory_locations_name_unique
        // enforces name uniqueness ONLY for active rows (is_active = 1).
        // Soft-deactivated rows can reuse names. Verify both halves:
        //
        //   * Inserting a second active location with the same name as
        //     a seeded one must fail at the index level.
        //   * Soft-deactivating the seeded row (is_active = 0) lets a
        //     new active row reuse the name.
        let result = conn.execute(
            "INSERT INTO inventory_locations (id, name, type) VALUES ('other-uuid', 'Default Inventory', 'store')",
            [],
        );
        assert!(
            result.is_err(),
            "expected UNIQUE index to reject duplicate active 'Default Inventory' name"
        );

        // Soft-deactivate the canonical default and allow a re-use.
        conn.execute(
            "UPDATE inventory_locations SET is_active = 0 WHERE name = 'Default Inventory'",
            [],
        )
        .unwrap();
        let reuse = conn.execute(
            "INSERT INTO inventory_locations (id, name, type) VALUES ('other-uuid-2', 'Default Inventory', 'store')",
            [],
        );
        assert!(
            reuse.is_ok(),
            "soft-deactivated name should be reusable: {:?}",
            reuse
        );
    }

    #[test]
    fn migration_086_creates_partial_unique_index_for_active_shifts() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // ADR §13 finding 32 v2 amend: the partial UNIQUE index
        // idx_inv_shifts_active_per_user_location must exist with
        // leading-column pair (user_id, location_id) and predicate
        // `WHERE status = 'active'`. Schema-level check (rather than
        // a data-level enforcement test) because seeding users + roles
        // is out of scope for this migration test. The index is the
        // database-layer enforcement of §9d's "at most one active shift
        // per (user_id, location_id) pair" invariant.
        //
        // We verify BOTH the index's presence AND its predicate +
        // leading-columns. A copy-paste regression that drops the
        // WHERE clause would silently pass a presence-only check;
        // assert substring on the index's stored SQL catches that.
        let index_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master
                  WHERE type='index' AND name='idx_inv_shifts_active_per_user_location'",
                [],
                |r| r.get(0),
            )
            .expect("missing §13-32 v2-amend partial UNIQUE index for active inventory shifts");

        assert!(
            index_sql.contains("user_id") && index_sql.contains("location_id"),
            "partial UNIQUE index must index both user_id and location_id, got: {index_sql}"
        );
        assert!(
            index_sql.contains("WHERE") && index_sql.contains("active"),
            "partial UNIQUE index must predicate on active status (v2 amend), got: {index_sql}"
        );
    }

    #[test]
    fn migration_069_scoping_indexes_used_in_query_plan() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Verify the compound index is used for store-scoped status queries.
        let mut stmt = conn
            .prepare(
                "EXPLAIN QUERY PLAN SELECT * FROM sales WHERE store_id = 's1' AND status = 'completed' ORDER BY created_at DESC",
            )
            .unwrap();
        let plans: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        let plan_text = plans.join(" ");
        assert!(
            plan_text.contains("idx_sales_store_status"),
            "expected query plan to use idx_sales_store_status, got: {plan_text}"
        );
    }

    // ── ADR #22 Phase 0d: setting_updated migration ─────────────

    #[test]
    fn migration_100_creates_setting_updated_indexes() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Verify both indexes from 100_setting_updated.sql exist.
        let indexes = [
            "idx_setting_updated_key_version",
            "idx_setting_updated_terminal",
        ];
        for idx in &indexes {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    rusqlite::params![idx],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing index {idx} from migration 100");
        }
    }

    #[test]
    fn migration_100_setting_updated_schema_integrity() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Verify all expected columns exist with correct constraints.
        let columns: Vec<(String, String, i64)> = {
            let mut stmt = conn
                .prepare("SELECT name, type, \"notnull\" FROM pragma_table_info('setting_updated')")
                .unwrap();
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
        };

        // Verify the 6 expected columns exist.
        let expected = ["id", "key", "value", "terminal_id", "version", "created_at"];
        for col in &expected {
            assert!(
                columns.iter().any(|(name, _, _)| name == col),
                "missing column '{col}' in setting_updated"
            );
        }

        // key, value, terminal_id, version must be NOT NULL.
        for col in &["key", "value", "terminal_id", "version"] {
            let notnull = columns
                .iter()
                .find(|(name, _, _)| name == col)
                .map(|(_, _, nn)| *nn)
                .unwrap_or(0);
            assert_eq!(
                notnull, 1,
                "column '{col}' must be NOT NULL in setting_updated"
            );
        }
    }

    // ── TAX-02 (migration 108): single-default invariant ────────────

    #[test]
    fn migration_108_creates_single_default_index() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        let index_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master \
                  WHERE type='index' AND name='idx_tax_rates_single_default'",
                [],
                |r| r.get(0),
            )
            .expect("missing partial UNIQUE index from migration 108");

        assert!(
            index_sql.contains("UNIQUE") && index_sql.contains("WHERE is_default = 1"),
            "single-default index must be partial + unique, got: {index_sql}"
        );
    }

    #[test]
    fn migration_108_rejects_second_default() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Seed one default rate, then a raw second default must be rejected
        // by the partial UNIQUE index (TAX-02 database invariant).
        conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES ('tax-a', 'A', 1000, 1, 0, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let second = conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES ('tax-b', 'B', 1000, 1, 0, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        );
        assert!(
            second.is_err(),
            "expected the partial UNIQUE index to reject a second default rate"
        );
    }

    #[test]
    fn migration_108_normalises_legacy_multiple_defaults() {
        // Apply migrations UP TO (but not including) 108 so the partial
        // UNIQUE index does not yet exist — otherwise the second default
        // insert below would be rejected by the index before the
        // normalisation UPDATE ever runs.
        let mut conn = fresh();
        let idx_108 = ALL
            .iter()
            .position(|m| m.id == "108_tax_single_default.sql")
            .expect("migration 108 registered");
        platform_core::database::run(&mut conn, &ALL[..idx_108]).unwrap();

        // Simulate a pre-108 DB that already has two defaults (the bug
        // TAX-02 fixed). The UPDATE normalisation must keep the OLDEST
        // and clear the other when migration 108 applies.
        conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES ('tax-old', 'Old', 1000, 1, 0, '2026-07-01T00:00:00.000Z', '2026-07-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES ('tax-new', 'New', 1000, 1, 0, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();

        // Apply migration 108 — normalisation + index creation.
        platform_core::database::run(&mut conn, &ALL[idx_108..]).unwrap();

        let defaults: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM tax_rates WHERE is_default = 1")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert_eq!(
            defaults,
            vec!["Old"],
            "legacy multiple-default data must be normalised to the oldest default"
        );
    }

    /// Verify the `setting_updated` table survives a migration re-run
    /// (idempotent — uses `CREATE TABLE IF NOT EXISTS`).
    #[test]
    fn migration_100_is_idempotent_with_existing_data() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Insert a test row to verify it survives re-migration.
        conn.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["test.key", "test-value", "term-test", 1],
        )
        .unwrap();

        // Re-run migrations — must be idempotent.
        run(&mut conn).unwrap();

        // Row must still exist.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM setting_updated WHERE key = 'test.key'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 1,
            "existing delta row should survive migration re-run"
        );
    }

    // ── TAX-03 (migration 109): tax soft-delete flag ─────────────

    #[test]
    fn migration_109_adds_is_active_column_to_tax_rates() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        let col: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('tax_rates') WHERE name = 'is_active'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col, 1, "tax_rates must have is_active column");

        let notnull: i64 = conn
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('tax_rates') WHERE name = 'is_active'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(notnull, 1, "is_active must be NOT NULL");
    }

    #[test]
    fn migration_109_existing_rates_default_to_active() {
        // Rows inserted before 109 (and any future insert that omits the
        // column) must default to is_active = 1.
        let mut conn = fresh();
        run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES ('tax-109-a', 'A', 1000, 1, 0, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let active: i64 = conn
            .query_row(
                "SELECT is_active FROM tax_rates WHERE id = 'tax-109-a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active, 1, "pre-109 rows must default to active");

        // The partial unique index from 108 still works with 109 applied.
        let second = conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, created_at, updated_at)
             VALUES ('tax-109-b', 'B', 1000, 1, 0, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        );
        assert!(second.is_err(), "single-default index must still reject");
    }

    // ── DB-06: populated upgrade fixture (migration 081 rebuild) ───

    #[test]
    fn upgrade_081_rebuild_preserves_populated_stock_transfer_lines() {
        // A pre-081 database with real stock_transfers + stock_transfer_lines.
        // Migration 081 DROPs and recreates stock_transfers while
        // stock_transfer_lines FK-references it ON DELETE CASCADE — without
        // the runner's FK isolation (DB-05) the rebuild would cascade-delete
        // the lines or fail outright.
        let idx081 = ALL
            .iter()
            .position(|m| m.id == "081_stock_transfers_received_partial.sql")
            .unwrap();
        let mut conn = fresh();
        platform_core::database::run(&mut conn, &ALL[..idx081]).unwrap();

        // Seed a role + user (stock_transfers.created_by FK) and one
        // transfer with a line (stock_transfer_lines.transfer_id FK).
        conn.execute_batch(
            "INSERT INTO roles (id, name) VALUES ('role-owner', 'Owner');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active)
                 VALUES ('user-1', 'owner', 'hash', 'Owner', 'role-owner', 1);
             INSERT INTO stock_transfers (id, transfer_number, status,
                 source_location, destination_location, created_by)
                 VALUES ('tf-1', 'TR-0001', 'in_transit', 'Store A', 'Store B', 'user-1');
             INSERT INTO stock_transfer_lines (id, transfer_id, sku, product_name, qty)
                 VALUES ('stl-1', 'tf-1', 'SKU-1', 'Widget', 5);",
        )
        .unwrap();

        // Upgrade through 081 (and everything after).
        platform_core::database::run(&mut conn, &ALL[idx081..]).unwrap();

        // The line survived the rebuild and still points at the transfer.
        let lines: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock_transfer_lines", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(lines, 1, "stock_transfer_lines must survive migration 081");
        let transfer_id: String = conn
            .query_row(
                "SELECT transfer_id FROM stock_transfer_lines WHERE id = 'stl-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(transfer_id, "tf-1");

        // Rebuild backfilled the new FK columns to the canonical location.
        let src_loc: String = conn
            .query_row(
                "SELECT source_location_id FROM stock_transfers WHERE id = 'tf-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(src_loc, "01926b3a-0000-7000-8000-000000000001");

        // The extended status CHECK now accepts received_partial.
        conn.execute(
            "INSERT INTO stock_transfers (id, transfer_number, status, created_by)
             VALUES ('tf-2', 'TR-0002', 'received_partial', 'user-1')",
            [],
        )
        .unwrap();

        // No orphaned FKs anywhere in the upgraded schema.
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            violations, 0,
            "foreign_key_check must be clean after 081 upgrade"
        );
    }

    // ── DB-07: 092 rebuild conserves the multi-location ledger ─────

    #[test]
    fn upgrade_092_rebuild_conserves_multi_location_ledger() {
        // A pre-092 database with a two-location stock_movements ledger.
        // Migration 092 DELETEs stock_summary and rebuilds it from the
        // ledger grouped by (item_id, location_id), then zeroes inventory
        // for over-sold products. Assert conservation + zero-out.
        let idx092 = ALL
            .iter()
            .position(|m| m.id == "092_rebuild_stock_summary_group_by_location.sql")
            .unwrap();
        let mut conn = fresh();
        platform_core::database::run(&mut conn, &ALL[..idx092]).unwrap();

        // Seed products, a second location, movements across both locations
        // (positive + negative deltas), stale summary rows, and inventory.
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency)
                 VALUES ('p1', 'SKU-1', 'Product 1', 100, 'USD'),
                        ('p2', 'SKU-2', 'Product 2', 100, 'USD');
             INSERT INTO inventory_locations (id, name, type)
                 VALUES ('loc-wh', 'Warehouse', 'warehouse');
             INSERT INTO stock_movements (id, item_id, delta, reason, location_id) VALUES
                 ('m1', 'p1',   7, 'restock', '01926b3a-0000-7000-8000-000000000001'),
                 ('m2', 'p1',  -2, 'sale',    '01926b3a-0000-7000-8000-000000000001'),
                 ('m3', 'p1',   3, 'restock', 'loc-wh'),
                 ('m4', 'p2',  -4, 'sale',    '01926b3a-0000-7000-8000-000000000001');
             INSERT INTO stock_summary (item_id, location_id, qty, updated_at) VALUES
                 ('p1', '01926b3a-0000-7000-8000-000000000001', 99, '2026-01-01T00:00:00.000Z'),
                 ('p1', 'loc-wh', 99, '2026-01-01T00:00:00.000Z'),
                 ('p2', '01926b3a-0000-7000-8000-000000000001', 99, '2026-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty) VALUES ('p1', 8), ('p2', 5);",
        )
        .unwrap();

        // Upgrade through 092 (and everything after).
        platform_core::database::run(&mut conn, &ALL[idx092..]).unwrap();

        // stock_summary rebuilt = SUM(delta) per (item_id, location_id).
        let summary: Vec<(String, String, i64)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT item_id, location_id, qty FROM stock_summary ORDER BY location_id, item_id",
                )
                .unwrap();
            stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
        };
        assert_eq!(
            summary,
            vec![
                (
                    "p1".to_string(),
                    "01926b3a-0000-7000-8000-000000000001".to_string(),
                    5
                ),
                (
                    "p2".to_string(),
                    "01926b3a-0000-7000-8000-000000000001".to_string(),
                    -4
                ),
                ("p1".to_string(), "loc-wh".to_string(), 3),
            ]
        );

        // Inventory zeroed only for the over-sold product (net <= 0).
        let p2_qty: i64 = conn
            .query_row(
                "SELECT qty FROM inventory WHERE product_id = 'p2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(p2_qty, 0, "over-sold product must have inventory zeroed");
        let p1_qty: i64 = conn
            .query_row(
                "SELECT qty FROM inventory WHERE product_id = 'p1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(p1_qty, 8, "in-stock product inventory must be preserved");

        // No orphaned FKs after the rebuild.
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            violations, 0,
            "foreign_key_check must be clean after 092 upgrade"
        );
    }

    // ── DB-08: unique (key, terminal_id, version) on setting_updated ─

    #[test]
    fn migration_116_creates_unique_setting_version_index() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        let idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_setting_updated_unique_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx, 1, "migration 116 must create the unique version index");

        // Duplicate (key, terminal_id, version) must be rejected.
        conn.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version) VALUES ('k', 'v1', 't1', 1)",
            [],
        )
        .unwrap();
        let dup = conn.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version) VALUES ('k', 'v2', 't1', 1)",
            [],
        );
        assert!(
            dup.is_err(),
            "duplicate (key, terminal_id, version) must fail"
        );
    }

    #[test]
    fn migration_116_dedupes_legacy_duplicate_versions() {
        // Upgrade path: a pre-116 database that already carries duplicate
        // (key, terminal_id, version) rows (the MAX(version)+1 race).
        let idx116 = ALL
            .iter()
            .position(|m| m.id == "116_setting_updated_unique_version.sql")
            .unwrap();
        let mut conn = fresh();
        platform_core::database::run(&mut conn, &ALL[..idx116]).unwrap();

        conn.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version) VALUES ('k', 'old', 't1', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version) VALUES ('k', 'new', 't1', 1)",
            [],
        )
        .unwrap();

        // Apply 116: duplicate collapsed, keeping the newest row (max id).
        platform_core::database::run(&mut conn, &ALL[idx116..]).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM setting_updated WHERE key = 'k' AND terminal_id = 't1' AND version = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 1,
            "legacy duplicate versions must be collapsed to one row"
        );

        let value: String = conn
            .query_row(
                "SELECT value FROM setting_updated WHERE key = 'k' AND terminal_id = 't1' AND version = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(value, "new", "the most recently written row must survive");
    }

    // ── DB-04 end-state (migration 117): store_id FK on domain tables ─

    /// Assert a `store_id` → `store_profiles` FK is declared on `table`.
    fn assert_store_id_fk(conn: &rusqlite::Connection, table: &str) {
        let mut stmt = conn
            .prepare(&format!("PRAGMA foreign_key_list(\"{table}\")"))
            .unwrap();
        let fks: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(2)?, row.get::<_, String>(3)?))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            fks.iter()
                .any(|(t, from)| t == "store_profiles" && from == "store_id"),
            "{table} must declare store_id REFERENCES store_profiles(id), got FKs: {fks:?}"
        );
    }

    #[test]
    fn migration_117_creates_store_id_foreign_keys() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // The rebuild must carry the FK on all four ADR #4 domain tables.
        for table in &["products", "sales", "sale_lines", "customers"] {
            assert_store_id_fk(&conn, table);
        }

        // NULL remains the valid global sentinel (per-store DB isolation).
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type)
             VALUES ('p-null', 'SKU-NULL', 'Global', 100, 'USD', 'retail')",
            [],
        )
        .unwrap();

        // A store_id that does not exist in store_profiles must be rejected.
        let orphan = conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
             VALUES ('p-orphan', 'SKU-ORPHAN', 'Orphan', 100, 'USD', 'retail', 'ghost-store')",
            [],
        );
        assert!(
            orphan.is_err(),
            "store_id referencing a missing store_profile must fail the FK"
        );

        // A store_id that exists (migration 025 seeds 'default') must pass.
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
             VALUES ('p-scoped', 'SKU-SCOPED', 'Scoped', 100, 'USD', 'retail', 'default')",
            [],
        )
        .unwrap();

        // Re-running migrations stays idempotent after the rebuild.
        run(&mut conn).unwrap();
    }

    #[test]
    fn migration_117_quarantines_orphan_store_ids_on_upgrade() {
        // Upgrade fixture: a pre-117 database that already carries domain
        // rows with store_id values that do not exist in store_profiles
        // (legacy orphans — the exact data the audit flagged). The rebuild
        // must quarantine them to NULL (the documented global sentinel)
        // rather than failing the upgrade or dropping the rows.
        let idx117 = ALL
            .iter()
            .position(|m| m.id == "117_scoping_store_id_fk.sql")
            .unwrap();
        let mut conn = fresh();
        platform_core::database::run(&mut conn, &ALL[..idx117]).unwrap();

        // Seed rows across all four tables with a mix of valid ('default',
        // seeded by migration 025) and orphaned store_ids, plus a NULL.
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
                 VALUES ('p-ok', 'SKU-OK', 'Kept', 100, 'USD', 'retail', 'default'),
                        ('p-orphan', 'SKU-ORPHAN', 'Orphaned', 100, 'USD', 'retail', 'ghost-store'),
                        ('p-null', 'SKU-NULL', 'Global', 100, 'USD', 'retail', NULL);
             INSERT INTO customers (id, name, store_id) VALUES ('c-orphan', 'Orphan Cust', 'ghost-store');
             INSERT INTO sales (id, total_minor, currency, line_count, status, store_id)
                 VALUES ('s-orphan', 100, 'USD', 1, 'completed', 'ghost-store');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, store_id)
                 VALUES ('sl-orphan', 's-orphan', 'SKU-X', 1, 100, 100, 'USD', 1, 'ghost-store');",
        )
        .unwrap();

        // Apply 117 (and everything after).
        platform_core::database::run(&mut conn, &ALL[idx117..]).unwrap();

        // Orphaned store_ids quarantined to NULL; valid + NULL preserved.
        let orphan_sid: Option<String> = conn
            .query_row(
                "SELECT store_id FROM products WHERE id = 'p-orphan'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            orphan_sid.is_none(),
            "orphaned store_id must be quarantined to NULL"
        );

        let ok_sid: Option<String> = conn
            .query_row("SELECT store_id FROM products WHERE id = 'p-ok'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            ok_sid.as_deref(),
            Some("default"),
            "valid store_id must survive"
        );

        let null_sid: Option<String> = conn
            .query_row(
                "SELECT store_id FROM products WHERE id = 'p-null'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(null_sid.is_none(), "NULL global sentinel must survive");

        // Rows survived the rebuild (not dropped), FK is clean.
        let cust_sid: Option<String> = conn
            .query_row(
                "SELECT store_id FROM customers WHERE id = 'c-orphan'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(cust_sid.is_none(), "customer orphan store_id quarantined");
        let sale_sid: Option<String> = conn
            .query_row(
                "SELECT store_id FROM sales WHERE id = 's-orphan'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(sale_sid.is_none(), "sale orphan store_id quarantined");
        let line_sid: Option<String> = conn
            .query_row(
                "SELECT store_id FROM sale_lines WHERE id = 'sl-orphan'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(line_sid.is_none(), "sale_line orphan store_id quarantined");

        // All four rows still exist, FK enforcement is clean, and the
        // scoping indexes survived the rebuild.
        for (table, id) in [
            ("products", "p-orphan"),
            ("customers", "c-orphan"),
            ("sales", "s-orphan"),
            ("sale_lines", "sl-orphan"),
        ] {
            let n: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE id = ?1"),
                    rusqlite::params![id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "{table} row {id} must survive the 117 rebuild");
        }
        let fk_check: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fk_check, 0, "foreign_key_check must be clean after 117");
        for index in [
            "idx_sales_store_status",
            "idx_sale_lines_store_sale",
            "idx_products_store_category",
            "idx_customers_store",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    rusqlite::params![index],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "scoping index {index} must survive the 117 rebuild");
        }
    }

    // ── warehouse_id supersession cleanup (migration 118) ──────────

    #[test]
    fn migration_118_drops_warehouse_id_columns() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // The dead column is gone from both tables in the end-state schema.
        for table in &["inventory", "stock_counts"] {
            let count: i64 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = 'warehouse_id'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                count, 0,
                "{table} must not have warehouse_id after migration 118"
            );
        }

        // The index that existed only for the dead column is gone too.
        let idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_inventory_warehouse_product'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx, 0, "idx_inventory_warehouse_product must be dropped");

        // The real location link from migration 079 survives untouched:
        // the location_id FK column and its query index both remain.
        let loc_col: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('inventory') WHERE name = 'location_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            loc_col, 1,
            "inventory.location_id must survive migration 118"
        );
        let loc_idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_inventory_location_product'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            loc_idx, 1,
            "idx_inventory_location_product must survive migration 118"
        );
    }

    #[test]
    fn migration_118_drop_preserves_inventory_and_count_data_on_upgrade() {
        // Upgrade fixture: a pre-118 database that already carries
        // inventory rows and stock counts whose warehouse_id values were
        // set (the speculative hook never had a writer, but rows may carry
        // them from tests or hand-imports). The DROP COLUMN must remove the
        // column while preserving every other column's data — a rebuild
        // bug here would silently lose qty/count data on upgrade.
        let idx118 = ALL
            .iter()
            .position(|m| m.id == "118_drop_warehouse_id_superseded.sql")
            .unwrap();
        let mut conn = fresh();
        platform_core::database::run(&mut conn, &ALL[..idx118]).unwrap();

        // Seed a product (needed for the inventory FK), an inventory row
        // with a non-NULL warehouse_id, and a stock count with one line.
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type)
                 VALUES ('p-118', 'SKU-118', 'Counted', 100, 'USD', 'retail');
             INSERT INTO inventory (product_id, qty, warehouse_id)
                 VALUES ('p-118', 42, 'wh-1');
             INSERT INTO stock_counts (id, count_number, status, count_type, warehouse_id)
                 VALUES ('sc-118', 'CN-118', 'draft', 'full', 'wh-1');
             INSERT INTO stock_count_lines (id, count_id, sku, product_name, expected_qty)
                 VALUES ('scl-118', 'sc-118', 'SKU-118', 'Counted', 42);",
        )
        .unwrap();

        // Apply 118 (and everything after).
        platform_core::database::run(&mut conn, &ALL[idx118..]).unwrap();

        // The dead column is gone, but the data it sat beside survived.
        let inv_qty: i64 = conn
            .query_row(
                "SELECT qty FROM inventory WHERE product_id = 'p-118'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(inv_qty, 42, "inventory qty must survive the 118 drop");

        let count_status: String = conn
            .query_row(
                "SELECT status FROM stock_counts WHERE id = 'sc-118'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            count_status, "draft",
            "stock count row must survive the 118 drop"
        );

        let line_expected: i64 = conn
            .query_row(
                "SELECT expected_qty FROM stock_count_lines WHERE id = 'scl-118'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            line_expected, 42,
            "stock count line must survive the 118 drop"
        );

        // FK integrity is clean and the runner stays idempotent.
        let fk_check: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fk_check, 0, "foreign_key_check must be clean after 118");
        run(&mut conn).unwrap();
    }

    // ── Cross-store query audit (migration 117 end-state) ──────────

    /// Run `SELECT id FROM {table} WHERE store_id = ?1` — the canonical
    /// store-scoped query shape — and return the matching row ids.
    fn scoped_row_ids(conn: &rusqlite::Connection, table: &str, store: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id FROM {table} WHERE store_id = ?1 ORDER BY id"
            ))
            .unwrap();
        stmt.query_map(rusqlite::params![store], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    /// Run `SELECT id FROM {table} WHERE store_id IS NULL` — the explicit
    /// global-scope predicate that is the ONLY way NULL-sentinel rows are
    /// reachable — and return the matching row ids.
    fn global_row_ids(conn: &rusqlite::Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id FROM {table} WHERE store_id IS NULL ORDER BY id"
            ))
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    /// Seed the shared cross-store audit fixture: two store profiles
    /// (migration 025 already seeds 'default') plus rows owned by
    /// store-a, store-b, and the NULL global sentinel on every ADR #4
    /// scoped table. Used by both the SELECT and UPDATE audit tests so
    /// the fixtures cannot drift apart. `payment_method`/`course` are
    /// seeded for the UPDATE test's mutable-column sweep but are inert
    /// for the SELECT test.
    fn seed_cross_store_fixture(conn: &rusqlite::Connection) {
        conn.execute_batch(
            "INSERT INTO store_profiles (id, name)
                 VALUES ('store-a', 'Store A'), ('store-b', 'Store B');
             INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
                 VALUES ('p-a', 'SKU-A', 'A', 100, 'USD', 'retail', 'store-a'),
                        ('p-b', 'SKU-B', 'B', 100, 'USD', 'retail', 'store-b'),
                        ('p-null', 'SKU-N', 'Global', 100, 'USD', 'retail', NULL);
             INSERT INTO customers (id, name, store_id)
                 VALUES ('c-a', 'Cust A', 'store-a'),
                        ('c-b', 'Cust B', 'store-b'),
                        ('c-null', 'Cust Global', NULL);
             INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, store_id)
                 VALUES ('s-a', 100, 'USD', 1, 'completed', 'cash', 'store-a'),
                        ('s-b', 100, 'USD', 1, 'completed', 'cash', 'store-b'),
                        ('s-null', 100, 'USD', 1, 'completed', 'cash', NULL);
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, course, store_id)
                 VALUES ('sl-a', 's-a', 'SKU-A', 1, 100, 100, 'USD', 1, 'starter', 'store-a'),
                        ('sl-b', 's-b', 'SKU-B', 1, 100, 100, 'USD', 1, 'starter', 'store-b'),
                        ('sl-null', 's-null', 'SKU-N', 1, 100, 100, 'USD', 1, 'starter', NULL);",
        )
        .unwrap();
    }

    #[test]
    fn store_scoped_query_never_returns_null_or_other_store_rows() {
        // DB-04 query-level audit. Migration 117's FK guarantees a non-NULL
        // store_id always references a real store_profile, but the audit
        // also pins the QUERY contract: `WHERE store_id = 'x'` must return
        // exactly store x's rows — never the NULL global-sentinel rows
        // (migration 069's "unscoped / legacy / global shared" state) and
        // never another store's rows. A scoped caller that forgets nothing
        // gets clean isolation at the predicate level too.
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Seed the shared cross-store fixture (store-a / store-b / NULL
        // rows on all four ADR #4 scoped tables).
        seed_cross_store_fixture(&conn);

        // The audit: a store-a scoped query returns EXACTLY the store-a
        // row on every table — no NULL sentinel, no store-b leakage.
        for (table, expected) in [
            ("products", vec!["p-a"]),
            ("customers", vec!["c-a"]),
            ("sales", vec!["s-a"]),
            ("sale_lines", vec!["sl-a"]),
        ] {
            let ids = scoped_row_ids(&conn, table, "store-a");
            assert_eq!(
                ids, expected,
                "{table} store-a scoped query must return only store-a rows, got: {ids:?}"
            );
        }

        // Mirror for store-b — isolation must hold in both directions.
        for (table, expected) in [
            ("products", vec!["p-b"]),
            ("customers", vec!["c-b"]),
            ("sales", vec!["s-b"]),
            ("sale_lines", vec!["sl-b"]),
        ] {
            let ids = scoped_row_ids(&conn, table, "store-b");
            assert_eq!(
                ids, expected,
                "{table} store-b scoped query must return only store-b rows, got: {ids:?}"
            );
        }

        // NULL-sentinel rows are reachable ONLY through the explicit
        // global predicate (store_id IS NULL), never through a scoped
        // query — that is the contract that keeps unscoped rows from
        // leaking into a single store's view.
        for (table, expected) in [
            ("products", vec!["p-null"]),
            ("customers", vec!["c-null"]),
            ("sales", vec!["s-null"]),
            ("sale_lines", vec!["sl-null"]),
        ] {
            let ids = global_row_ids(&conn, table);
            assert_eq!(
                ids, expected,
                "{table} global-sentinel query must return only NULL rows, got: {ids:?}"
            );
        }

        // FK ownership integrity (migration 117): a store_id with no
        // matching store_profiles row is rejected at the database layer,
        // so a scoped query can never be pointed at a phantom store.
        let ghost = conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
             VALUES ('p-ghost', 'SKU-GHOST', 'Ghost', 100, 'USD', 'retail', 'ghost-store')",
            [],
        );
        assert!(
            ghost.is_err(),
            "store_id referencing a missing store_profile must fail the 117 FK"
        );

        // Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
    }

    #[test]
    fn store_deletion_reverts_scoped_rows_to_null_sentinel() {
        // ON DELETE SET NULL contract (migration 117): deleting a store
        // profile must neither block on historical domain rows (RESTRICT)
        // nor destroy them (CASCADE) — their store_id reverts to the NULL
        // global sentinel. The rows stay globally visible and a scoped
        // query for the deleted store returns nothing.
        let mut conn = fresh();
        run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO store_profiles (id, name) VALUES ('store-a', 'Store A')",
            [],
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
                 VALUES ('p-a', 'SKU-A', 'A', 100, 'USD', 'retail', 'store-a'),
                        ('p-null', 'SKU-N', 'Global', 100, 'USD', 'retail', NULL);
             INSERT INTO sales (id, total_minor, currency, line_count, status, store_id)
                 VALUES ('s-a', 100, 'USD', 1, 'completed', 'store-a');",
        )
        .unwrap();

        conn.execute("DELETE FROM store_profiles WHERE id = 'store-a'", [])
            .unwrap();

        // Scoped query for the deleted store returns nothing…
        assert_eq!(
            scoped_row_ids(&conn, "products", "store-a"),
            Vec::<String>::new(),
            "scoped query for a deleted store must return no rows"
        );
        // …but the rows themselves survived, reverted to the NULL sentinel.
        let sid: Option<String> = conn
            .query_row("SELECT store_id FROM products WHERE id = 'p-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(
            sid.is_none(),
            "store-a product must revert to NULL sentinel"
        );
        let sale_sid: Option<String> = conn
            .query_row("SELECT store_id FROM sales WHERE id = 's-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(
            sale_sid.is_none(),
            "store-a sale must revert to NULL sentinel"
        );
        // The NULL sentinel row is untouched and the FK surface is clean.
        assert_eq!(
            global_row_ids(&conn, "products"),
            vec!["p-a", "p-null"],
            "reverted row must join the global scope"
        );
        let fk_check: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fk_check, 0, "no FK violations after SET NULL reversion");

        // Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
    }

    #[test]
    fn store_scoped_update_never_mutates_other_store_or_null_rows() {
        // DB-04 UPDATE-path audit. Migration 117's FK guards writes as
        // well as reads: a store-scoped UPDATE (`WHERE store_id = 'x'`)
        // must touch exactly store x's rows, and SQLite's three-valued
        // logic (`NULL = 'x'` is never TRUE) structurally excludes the
        // NULL-sentinel rows — so unscoped/global data is write-protected
        // from scoped writers exactly as it is from scoped readers.
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Seed the shared cross-store fixture (store-a / store-b / NULL
        // rows on all four ADR #4 scoped tables).
        seed_cross_store_fixture(&conn);

        // Sweep all four tables: a store-a scoped UPDATE must affect
        // exactly one row (the store-a row) and leave the store-b row and
        // the NULL-sentinel row byte-identical.
        for (table, mutcol, a_id, b_id, null_id, _a_old, b_old, null_old, new_val) in [
            (
                "products",
                "name",
                "p-a",
                "p-b",
                "p-null",
                "A",
                "B",
                "Global",
                "Renamed-A",
            ),
            (
                "customers",
                "name",
                "c-a",
                "c-b",
                "c-null",
                "Cust A",
                "Cust B",
                "Cust Global",
                "Renamed-A",
            ),
            (
                "sales",
                "payment_method",
                "s-a",
                "s-b",
                "s-null",
                "cash",
                "cash",
                "cash",
                "card",
            ),
            (
                "sale_lines",
                "course",
                "sl-a",
                "sl-b",
                "sl-null",
                "starter",
                "starter",
                "starter",
                "main",
            ),
        ] {
            let affected = conn
                .execute(
                    &format!("UPDATE {table} SET {mutcol} = ?1 WHERE store_id = 'store-a'"),
                    rusqlite::params![new_val],
                )
                .unwrap();
            assert_eq!(
                affected, 1,
                "{table} store-a scoped UPDATE must affect exactly the store-a row"
            );
            let cell = |id: &str| -> String {
                conn.query_row(
                    &format!("SELECT {mutcol} FROM {table} WHERE id = ?1"),
                    rusqlite::params![id],
                    |r| r.get(0),
                )
                .unwrap()
            };
            assert_eq!(cell(a_id), new_val, "{table} store-a row must be updated");
            assert_eq!(
                cell(b_id),
                b_old,
                "{table} store-b row must be untouched by a store-a scoped UPDATE"
            );
            assert_eq!(
                cell(null_id),
                null_old,
                "{table} NULL-sentinel row must be untouched by a store-a scoped UPDATE"
            );
        }

        // The FK guards UPDATE writes too: reassigning a row to a store
        // that does not exist is rejected, while reverting to NULL (the
        // documented global sentinel) stays legal.
        let ghost = conn.execute(
            "UPDATE products SET store_id = 'ghost-store' WHERE id = 'p-a'",
            [],
        );
        assert!(
            ghost.is_err(),
            "reassigning a row to a missing store_profile must fail the 117 FK"
        );
        conn.execute("UPDATE products SET store_id = NULL WHERE id = 'p-a'", [])
            .unwrap();
        let sid: Option<String> = conn
            .query_row("SELECT store_id FROM products WHERE id = 'p-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(
            sid.is_none(),
            "reverting a row to the NULL sentinel is legal"
        );

        // Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
    }

    #[test]
    fn store_scoped_upsert_never_hijacks_other_store_or_null_rows() {
        // DB-04 upsert-path audit. An `INSERT ... ON CONFLICT(id) DO
        // UPDATE` is the standard idempotent write (cart/offline/sync all
        // use it), but without a scope guard it would silently mutate a
        // row owned by ANOTHER store on conflict — the row is matched by
        // primary key, not by ownership. This test pins the guarded form:
        // `DO UPDATE ... WHERE {table}.store_id = 'store-a'` turns a
        // cross-store conflict into a no-op (affected = 0) instead of a
        // hijack. The NULL-sentinel row is protected the same way, and a
        // fresh insert still lands in the writer's own store.
        let mut conn = fresh();
        run(&mut conn).unwrap();

        seed_cross_store_fixture(&conn);

        // 1. A store-a scoped upsert that CONFLICTS with a store-b row must
        //    NOT overwrite it — the WHERE guard evaluates false and the
        //    statement becomes a no-op, leaving store-b's row intact.
        let hijack = conn
            .execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
                 VALUES ('p-b', 'SKU-B', 'Hijacked', 100, 'USD', 'retail', 'store-a')
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name
                 WHERE products.store_id = 'store-a'",
                [],
            )
            .unwrap();
        assert_eq!(
            hijack, 0,
            "scoped upsert conflicting with a store-b row must be a no-op, not a hijack"
        );
        let name_b: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            name_b, "B",
            "store-b row must be untouched by a store-a scoped upsert"
        );
        let sid_b: String = conn
            .query_row("SELECT store_id FROM products WHERE id = 'p-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            sid_b, "store-b",
            "store-b row must keep its ownership after a conflicting scoped upsert"
        );

        // 2. Same guard protects the NULL-sentinel row from a scoped upsert.
        let null_hijack = conn
            .execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
                 VALUES ('p-null', 'SKU-N', 'Hijacked', 100, 'USD', 'retail', 'store-a')
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name
                 WHERE products.store_id = 'store-a'",
                [],
            )
            .unwrap();
        assert_eq!(
            null_hijack, 0,
            "scoped upsert conflicting with the NULL-sentinel row must be a no-op"
        );
        let name_null: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-null'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            name_null, "Global",
            "NULL-sentinel row must be untouched by a store-a scoped upsert"
        );
        let sid_null: Option<String> = conn
            .query_row(
                "SELECT store_id FROM products WHERE id = 'p-null'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            sid_null.is_none(),
            "NULL-sentinel row must keep store_id NULL"
        );

        // 3. A store-a scoped upsert that conflicts with the writer's OWN
        //    store-a row DOES update it — the guard is satisfied and the
        //    legitimate idempotent-write path still works.
        let mine = conn
            .execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
                 VALUES ('p-a', 'SKU-A', 'Updated-A', 100, 'USD', 'retail', 'store-a')
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name
                 WHERE products.store_id = 'store-a'",
                [],
            )
            .unwrap();
        assert_eq!(
            mine, 1,
            "scoped upsert on the writer's own store-a row must update it"
        );
        let name_a: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            name_a, "Updated-A",
            "store-a row must receive its own scoped upsert"
        );

        // 4. A store-a scoped upsert that is a fresh insert (no conflict)
        //    creates the new row owned by store-a.
        let fresh = conn
            .execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
                 VALUES ('p-new', 'SKU-NEW', 'New A', 100, 'USD', 'retail', 'store-a')
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name
                 WHERE products.store_id = 'store-a'",
                [],
            )
            .unwrap();
        assert_eq!(
            fresh, 1,
            "fresh scoped upsert must insert the new store-a row"
        );
        let new_sid: String = conn
            .query_row(
                "SELECT store_id FROM products WHERE id = 'p-new'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            new_sid, "store-a",
            "fresh upsert row must be owned by store-a"
        );

        // 5. The 117 FK still guards the upsert insert path: a scoped
        //    upsert cannot create a row owned by a non-existent store.
        let ghost = conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type, store_id)
             VALUES ('p-ghost', 'SKU-GHOST', 'Ghost', 100, 'USD', 'retail', 'ghost-store')
             ON CONFLICT(id) DO UPDATE SET name = excluded.name
             WHERE products.store_id = 'store-a'",
            [],
        );
        assert!(
            ghost.is_err(),
            "upsert referencing a missing store_profile must fail the 117 FK"
        );

        // Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
    }

    #[test]
    fn cross_store_transaction_mixed_writes_stay_scoped_and_atomic() {
        // DB-04 transaction audit. Multi-statement transactions are the
        // real write path (products.rs / sales.rs use
        // `unchecked_transaction()` everywhere), so the audit must prove:
        //
        //   (a) a committed transaction that mixes store-a, store-b, and
        //       explicit-global writes keeps every write inside its own
        //       ownership class — a store-a scoped statement can never
        //       mutate store-b rows or the NULL sentinel even when both
        //       run in the same transaction, and the NULL row is reachable
        //       only through the explicit `store_id IS NULL` predicate;
        //   (b) atomicity: if any statement fails, the whole transaction
        //       rolls back — a NULL-sentinel row (or any row) is never
        //       left half-mutated by a partially-applied transaction.
        let mut conn = fresh();
        run(&mut conn).unwrap();

        seed_cross_store_fixture(&conn);

        // ── (a) Committed mixed transaction stays in-scope ────────────
        conn.execute("BEGIN", []).unwrap();
        let a = conn
            .execute(
                "UPDATE products SET name = 'Tx-A' WHERE store_id = 'store-a'",
                [],
            )
            .unwrap();
        assert_eq!(
            a, 1,
            "store-a scoped UPDATE inside tx must affect exactly 1 row"
        );
        let b = conn
            .execute(
                "UPDATE products SET name = 'Tx-B' WHERE store_id = 'store-b'",
                [],
            )
            .unwrap();
        assert_eq!(
            b, 1,
            "store-b scoped UPDATE inside tx must affect exactly 1 row"
        );
        // Explicit global write — the ONLY way the NULL sentinel is
        // reachable, and a deliberate opt-in rather than a scoped leak.
        let g = conn
            .execute(
                "UPDATE products SET name = 'Tx-Global' WHERE store_id IS NULL",
                [],
            )
            .unwrap();
        assert_eq!(
            g, 1,
            "explicit global UPDATE must affect exactly the NULL-sentinel row"
        );
        conn.execute("COMMIT", []).unwrap();

        // Post-commit: every row holds exactly its own write.
        let name_a: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name_a, "Tx-A", "store-a row must receive its own tx write");
        let name_b: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name_b, "Tx-B", "store-b row must receive its own tx write");
        let name_null: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-null'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            name_null, "Tx-Global",
            "NULL-sentinel row must receive only the explicit global write"
        );

        // ── (b) Failed transaction rolls back atomically ──────────────
        // A statement that violates the 117 FK fails mid-transaction;
        // ROLLBACK must restore EVERY prior write, so no row — including
        // the NULL sentinel — is left half-mutated.
        conn.execute("BEGIN", []).unwrap();
        conn.execute(
            "UPDATE products SET name = 'ShouldRollBack-A' WHERE store_id = 'store-a'",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE products SET name = 'ShouldRollBack-Null' WHERE store_id IS NULL",
            [],
        )
        .unwrap();
        let fail = conn.execute(
            "UPDATE products SET store_id = 'ghost-store' WHERE id = 'p-a'",
            [],
        );
        assert!(
            fail.is_err(),
            "FK-violating statement must fail inside the transaction"
        );
        conn.execute("ROLLBACK", []).unwrap();

        // After rollback the DB is byte-identical to the pre-(b) state:
        // the store-a row and the NULL-sentinel row both revert to their
        // committed (a) values, and the FK surface is clean.
        let rb_a: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            rb_a, "Tx-A",
            "store-a write must be rolled back — no half-mutated state"
        );
        let rb_null: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-null'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            rb_null, "Tx-Global",
            "NULL-sentinel write must be rolled back — never left half-mutated"
        );
        let rb_b: String = conn
            .query_row("SELECT name FROM products WHERE id = 'p-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(rb_b, "Tx-B", "store-b write must survive untouched");
        let fk_check: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fk_check, 0, "no FK violations after rollback");

        // Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
    }

    // ── ADR #35 D5 (migration 128): assignment backfill ────────────
    //
    // A legacy database (an older release, migrations through 127 applied)
    // carries one global `users.role_id` and the six legacy role rows.
    // Migrating to 128 must give every existing user exactly one effective
    // assignment: Owner/Manager/Staff/custom keep global mode, while legacy
    // role-cashier / role-kitchen users resolve to role-staff with the
    // scoped workspace their current permission set implies (retail-pos /
    // kds) so their operational access survives the role retirement that
    // follows in a later migration.
    #[test]
    fn migration_128_backfills_assignments_from_legacy_role_ids() {
        // 1. Build the legacy DB: all migrations through 127 applied, with
        //    the legacy roles seeded and users referencing them.
        let idx = ALL
            .iter()
            .position(|m| m.id == "128_assignments.sql")
            .expect("128_assignments.sql registered");
        let mut conn = fresh();
        platform_core::database::run(&mut conn, &ALL[..idx]).unwrap();
        conn.execute_batch(
            "INSERT INTO roles (id, name, permissions) VALUES
                 ('role-owner', 'owner', '[\"*\"]'),
                 ('role-manager', 'manager', '[\"reports:view\"]'),
                 ('role-cashier', 'cashier', '[\"sales:process\"]'),
                 ('role-kitchen', 'kitchen', '[\"kds:view\", \"kds:update\"]'),
                 ('role-staff', 'staff', '[\"sales:view\"]');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                 ('u-owner', 'owner', 'h', 'Owner', 'role-owner', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'),
                 ('u-manager', 'manager', 'h', 'Manager', 'role-manager', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'),
                 ('u-cashier', 'cashier', 'h', 'Cashier', 'role-cashier', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'),
                 ('u-kitchen', 'kitchen', 'h', 'Kitchen', 'role-kitchen', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'),
                 ('u-staff', 'staff', 'h', 'Staff', 'role-staff', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');",
        )
        .unwrap();

        // 2. Apply the remainder (128+).
        platform_core::database::run(&mut conn, &ALL[idx..]).unwrap();

        // 3. Every user has exactly one effective assignment.
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM assignments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 5, "one assignment per legacy user");
        let dupes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (SELECT user_id FROM assignments GROUP BY user_id HAVING COUNT(*) > 1)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dupes, 0, "one effective assignment per user");

        // 4. Owner / Manager / Staff keep global mode with their role.
        for (user, role) in [
            ("u-owner", "role-owner"),
            ("u-manager", "role-manager"),
            ("u-staff", "role-staff"),
        ] {
            let (got_role, mode): (String, String) = conn
                .query_row(
                    "SELECT role_id, scope_mode FROM assignments WHERE user_id = ?1",
                    rusqlite::params![user],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert_eq!(got_role, role, "{user} keeps its role");
            assert_eq!(mode, "global", "{user} keeps global mode");
        }

        // 5. Cashier resolves to Staff + scoped `retail-pos`; kitchen to
        //    Staff + scoped `kds` — the workspace their grants imply.
        let (cash_role, cash_mode): (String, String) = conn
            .query_row(
                "SELECT role_id, scope_mode FROM assignments WHERE user_id = 'u-cashier'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(cash_role, "role-staff");
        assert_eq!(cash_mode, "scoped");
        let cash_ws: Vec<String> = conn
            .prepare("SELECT workspace_key FROM assignment_workspaces WHERE assignment_user_id = 'u-cashier'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(
            cash_ws,
            vec!["retail-pos"],
            "cashier -> retail-pos workspace"
        );

        let (kit_role, kit_mode): (String, String) = conn
            .query_row(
                "SELECT role_id, scope_mode FROM assignments WHERE user_id = 'u-kitchen'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(kit_role, "role-staff");
        assert_eq!(kit_mode, "scoped");
        let kit_ws: Vec<String> = conn
            .prepare("SELECT workspace_key FROM assignment_workspaces WHERE assignment_user_id = 'u-kitchen'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(kit_ws, vec!["kds"], "kitchen -> kds workspace");

        // 6. Global assignments have no scope rows.
        let scope_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM assignment_workspaces aw
                 JOIN assignments a ON a.user_id = aw.assignment_user_id
                 WHERE a.scope_mode = 'global'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(scope_rows, 0, "global assignments carry no scope rows");

        // 7. The `retail-pos` workspace was seeded for the cashier remap.
        let retail: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspaces WHERE key = 'retail-pos'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(retail, 1, "retail-pos workspace must be seeded");

        // 8. Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
        let after: i64 = conn
            .query_row("SELECT COUNT(*) FROM assignments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after, 5, "migration 128 must be idempotent");
    }

    // ── ADR #35 D4 (migration 129): retire cashier/kitchen roles ────
    //
    // The five-role taxonomy replaces the legacy cashier/kitchen roles.
    // Migration 128 already folded their assignments into role-staff; 129
    // re-points any remaining users.role_id / assignments.role_id references
    // and removes the role rows so presets and DB agree.
    #[test]
    fn migration_129_retires_cashier_and_kitchen_roles() {
        let idx = ALL
            .iter()
            .position(|m| m.id == "129_retire_cashier_kitchen.sql")
            .expect("129_retire_cashier_kitchen.sql registered");
        let mut conn = fresh();
        platform_core::database::run(&mut conn, &ALL[..idx]).unwrap();

        // Legacy rows referencing the retired roles.
        conn.execute_batch(
            "INSERT INTO roles (id, name, permissions) VALUES
                 ('role-cashier', 'cashier', '[\"sales:process\"]'),
                 ('role-kitchen', 'kitchen', '[\"kds:view\"]'),
                 ('role-staff', 'staff', '[\"sales:view\"]');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                 ('u-cashier', 'cashier', 'h', 'Cashier', 'role-cashier', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'),
                 ('u-kitchen', 'kitchen', 'h', 'Kitchen', 'role-kitchen', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'),
                 ('u-staff', 'staff', 'h', 'Staff', 'role-staff', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
             INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope) VALUES
                 ('u-cashier', 'role-cashier', 'scoped', 'all', 'list'),
                 ('u-staff', 'role-staff', 'global', 'all', 'all');",
        )
        .unwrap();

        // Apply 129 only.
        platform_core::database::run(&mut conn, &ALL[idx..]).unwrap();

        // Roles are gone.
        let retired: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM roles WHERE id IN ('role-cashier', 'role-kitchen')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(retired, 0, "cashier/kitchen role rows must be removed");

        // Users and assignments re-pointed to role-staff.
        let orphans: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role_id IN ('role-cashier', 'role-kitchen')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphans, 0, "no user may keep a retired role_id");
        let role: String = conn
            .query_row(
                "SELECT role_id FROM users WHERE id = 'u-cashier'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(role, "role-staff");
        let arole: String = conn
            .query_row(
                "SELECT role_id FROM assignments WHERE user_id = 'u-cashier'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(arole, "role-staff");

        // Re-running stays idempotent (module convention).
        platform_core::database::run(&mut conn, &ALL[idx..]).unwrap();
    }

    // ── ADR #35 D6 (migration 130): user profile columns ────────────
    //
    // The users table gains the profile contract columns (nullable in SQL —
    // "mandatory" is enforced at creation) plus unique indexes for email and
    // national_id, and NONE of the D6 not-collected fields.
    #[test]
    fn migration_130_adds_profile_columns_and_unique_indexes() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        let cols: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('users')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for col in [
            "date_of_birth",
            "phone",
            "national_id_type",
            "national_id",
            "email",
            "monthly_take_home_minor",
            "emergency_contact_name",
            "emergency_contact_phone",
            "job_title",
            "notes",
            "address",
            "language",
            "avatar",
            "tax_id",
            "national_id_expires_at",
            "emergency_contact_relationship",
            "hire_date",
        ] {
            assert!(
                cols.contains(&col.to_string()),
                "users must gain profile column `{col}`"
            );
        }

        // Unique indexes: email and national_id (SQLite UNIQUE allows
        // multiple NULLs, so "unique when present" holds).
        let idx: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='users' AND name LIKE 'idx_users_%'",
            )
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            idx.contains(&"idx_users_email".into()),
            "email unique index"
        );
        assert!(
            idx.contains(&"idx_users_national_id".into()),
            "national_id unique index"
        );

        // The D6 not-collected fields must NEVER appear in the schema.
        for banned in [
            "gender",
            "religion",
            "marital_status",
            "ethnicity",
            "blood_type",
            "bank_account",
            "shift_availability",
        ] {
            assert!(
                !cols.contains(&banned.to_string()),
                "{banned} is on the D6 not-collected list and must not appear"
            );
        }

        // Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
    }

    // ── ADR #35 D6 (migration 131): national-id uniqueness hash ─────
    //
    // national_id is encrypted at rest (nonce-randomised ciphertext), so the
    // ciphertext column can no longer enforce uniqueness — the deterministic
    // hash column + unique index carry the "unique when present" invariant.
    #[test]
    fn migration_131_adds_national_id_hash_column_and_index() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        let cols: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('users')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            cols.contains(&"national_id_hash".to_string()),
            "users must gain national_id_hash"
        );

        let idx: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='users' AND name LIKE 'idx_users_%'",
            )
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            idx.contains(&"idx_users_national_id_hash".into()),
            "national_id_hash unique index"
        );

        // Re-running migrations stays idempotent (module convention).
        run(&mut conn).unwrap();
    }

    // ── Regression guard: migration tests must not slice ALL by its tail ──
    //
    // 7af6a6b9 fixed migration_135_backfills_cost_snapshot_from_product_cost,
    // which simulated a "pre-135" release with `ALL.len() - 1`. When
    // 136_processed_webhooks was appended, the tail cut silently excluded
    // 136 instead of 135, so the backfill ran before the seed data existed.
    // Tests that apply "every migration except N" must slice at the
    // migration's REGISTERED position (ALL.iter().position(...)), which is
    // robust to migrations being appended or removed.
    #[test]
    fn no_migration_test_slices_all_by_array_tail() {
        let src = include_str!("migrations.rs");
        for line in src.lines() {
            // Built at runtime so this guard's own source never contains the
            // literal needle (which would make it self-match).
            let needle = format!("ALL.len(){}", " -");
            let stripped = line.split("//").next().unwrap_or("");
            if stripped.contains(&needle) {
                panic!(
                    "migration test slices ALL by tail arithmetic — use ALL.iter().position(...) instead:
{line}"
                );
            }
        }
    }
}
