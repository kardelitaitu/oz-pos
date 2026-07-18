//! Migration definitions for OZ-POS.
//!
//! Migrations are `.sql` files under `crates/oz-core/migrations/`. They are
//! embedded at compile time via [`include_str!`] and run in lexicographic
//! order on first startup by the generic runner in `platform-core`.

use platform_core::database::Migration;

/// All migrations in the order they should be applied.
///
/// The list is exhaustive at compile time; adding a new migration means
/// adding a new entry here AND a new file in `crates/oz-core/migrations/`.
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
        id: "047_stock_transfers.sql",
        sql: include_str!("../migrations/047_stock_transfers.sql"),
    },
    Migration {
        id: "046_track_serial.sql",
        sql: include_str!("../migrations/046_track_serial.sql"),
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
];

/// Apply every unapplied migration. Convenience wrapper around
/// [`platform_core::database::run`].
pub fn run(conn: &mut rusqlite::Connection) -> Result<(), crate::CoreError> {
    Ok(platform_core::database::run(conn, ALL)?)
}

/// Create a fresh in-memory database with all migrations already applied.
///
/// Uses a [`std::sync::LazyLock`]ed pre-migrated snapshot connection.
/// The first call runs all 75 migrations once; subsequent calls clone the
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
                     applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))\n\
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

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(cached_sql()).unwrap();
        Mutex::new(conn)
    });

    let mut fresh = rusqlite::Connection::open_in_memory().unwrap();
    {
        let snapshot = SNAPSHOT.lock().unwrap();
        let backup = rusqlite::backup::Backup::new(&snapshot, &mut fresh).unwrap();
        backup
            .run_to_completion(100, std::time::Duration::from_millis(0), None)
            .unwrap();
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
}
