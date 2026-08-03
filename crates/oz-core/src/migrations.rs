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

        // warehouse_id columns exist on inventory and stock_counts.
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
            assert_eq!(count, 1, "{table} missing warehouse_id column");
        }
    }

    #[test]
    fn migration_069_creates_scoping_indexes() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        let expected_indexes = [
            "idx_sales_store_status",
            "idx_sale_lines_store_sale",
            "idx_products_store_category",
            "idx_inventory_warehouse_product",
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
}
