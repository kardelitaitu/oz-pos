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
];

/// Apply every unapplied migration. Convenience wrapper around
/// [`platform_core::database::run`].
pub fn run(conn: &mut rusqlite::Connection) -> Result<(), crate::CoreError> {
    Ok(platform_core::database::run(conn, ALL)?)
}

/// Create a fresh in-memory database with all migrations already applied.
///
/// Concatenates all migration SQLs into a single batch the first time
/// and caches the generated SQL in a `OnceLock<String>`. Subsequent calls
/// just run `execute_batch` on the cached string — no per-test migration
/// overhead.
///
/// # Panics
///
/// Panics if the database cannot be created.
#[doc(hidden)]
pub fn fresh_db() -> rusqlite::Connection {
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
    conn
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
            "user_store_access",
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
