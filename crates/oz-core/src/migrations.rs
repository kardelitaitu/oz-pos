//! Migration definitions for OZ-POS.
//!
//! Migrations are `.sql` files under `crates/oz-core/migrations/`. They are
//! embedded at compile time via [`include_str!`] and run in the
//! compile-time array order of [`ALL`] on first startup by the generic
//! runner in `platform-core`. The array order is canonical — not
//! lexicographic filename order — and the registry↔filesystem parity test
//! [`migration_registry_matches_filesystem`] ensures every `.sql` file has
//! exactly one registry entry.
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
pub const ALL: &[Migration] = &[Migration {
    id: "20260813_init.sql",
    sql: include_str!("../migrations/20260813_init.sql"),
}];

/// Postgres DDL for the full schema, parallel to the SQLite `init.sql`.
///
/// Generated from `20260813_init.sql` by
/// `scripts/generate-pg-migration.py` (types mapped, foreign-key table
/// order topologically sorted, SQLite triggers rewritten as plpgsql). The
/// cloud server's `DbPool::connect_postgres` applies this instead of the
/// SQLite registry; it is idempotent (`IF NOT EXISTS`, `ON CONFLICT DO
/// NOTHING`, `CREATE OR REPLACE`).
pub const PG_INIT: &str = include_str!("../migrations/20260813_init.pg.sql");

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

        // Upgrade path: apply a prefix of the registry (a plausible older
        // release), then the remainder through the same runner. The
        // consolidated registry holds one migration, so the prefix is empty
        // — mirroring a pre-reset database whose old IDs are no longer
        // tracked — but the split generalizes as migrations are added again.
        let split = ALL.len() / 2;
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

    /// Count rows matching an arbitrary scalar SQL query.
    fn row_count(conn: &rusqlite::Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap()
    }

    /// Pin the bootstrap seed rows a fresh install depends on. The reset
    /// collapsed 131 migrations into one `init.sql`; the original failure mode
    /// was that the schema dumped fine but the seed INSERTs were dropped, so
    /// domain FK targets such as `workspaces.key = 'retail-pos'` had nothing
    /// to reference. This fails if any essential lookup row is removed or renamed.
    #[test]
    fn seed_data_bootstraps_essential_rows() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // Currencies (ISO-4217 lookups).
        for code in ["USD", "IDR"] {
            assert_eq!(
                row_count(
                    &conn,
                    &format!("SELECT COUNT(*) FROM currencies WHERE code = '{code}'"),
                ),
                1,
                "missing currency seed {code}"
            );
        }

        // Default store profile — the FK target for store-scoped rows and the
        // canonical `workspace_instances` store.
        assert_eq!(
            row_count(
                &conn,
                "SELECT COUNT(*) FROM store_profiles WHERE id = 'default'",
            ),
            1,
            "missing default store profile"
        );

        // Loyalty tiers.
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM loyalty_tiers"),
            4,
            "loyalty tier seeds must survive"
        );

        // Workspaces — `retail-pos` is the legacy cashier workspace that the
        // assignment tests reference by FK.
        for key in [
            "restaurant-pos",
            "store-pos",
            "warehouse",
            "admin",
            "kds",
            "retail-pos",
        ] {
            assert_eq!(
                row_count(
                    &conn,
                    &format!("SELECT COUNT(*) FROM workspaces WHERE key = '{key}'"),
                ),
                1,
                "missing workspace seed {key}"
            );
        }

        // Workspace types and the canonical default instances.
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM workspace_types"),
            6,
            "workspace type seeds must survive"
        );
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM workspace_instances"),
            5,
            "default workspace instance seeds must survive"
        );

        // Navigation screens (workspace + type).
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM workspace_screens"),
            30,
            "workspace screen seeds must survive"
        );
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM workspace_type_screens"),
            36,
            "workspace type screen seeds must survive"
        );

        // Tenant subscription and inventory locations.
        assert_eq!(
            row_count(
                &conn,
                "SELECT COUNT(*) FROM tenant_subscription WHERE tenant_id = 'default'",
            ),
            1,
            "missing default tenant subscription"
        );
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM inventory_locations"),
            2,
            "inventory location seeds must survive"
        );
    }

    /// Pin the consolidated schema surface: 92 tables, 121 indexes, 4
    /// triggers. (The generated `*.pg.sql` Postgres port is excluded — see
    /// [`pg_init_declares_same_table_surface_as_sqlite`].) A count assertion catches a table/index/trigger silently
    /// dropping out of `init.sql` — something a name-list check misses when a
    /// name changes.
    #[test]
    fn init_sql_creates_complete_schema_surface() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // The runner adds `schema_migrations` on top of the 92 init tables.
        assert_eq!(
            row_count(
                &conn,
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'",
            ),
            92,
            "table surface drifted"
        );
        assert_eq!(
            row_count(
                &conn,
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%'",
            ),
            122,
            "index surface drifted"
        );
        assert_eq!(
            row_count(
                &conn,
                "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger'"
            ),
            4,
            "trigger surface drifted"
        );
    }

    /// Simulate the documented existing-dev-DB upgrade path. A pre-reset
    /// database carries legacy `schema_migrations` rows (now absent from the
    /// registry) and has already been seeded. Running the consolidated init
    /// must ignore the old IDs, leave existing schema + seed rows untouched
    /// (`IF NOT EXISTS` / `INSERT OR IGNORE`), and record only the init row.
    #[test]
    fn existing_db_with_legacy_rows_upgrades_idempotently() {
        let mut conn = fresh();
        run(&mut conn).unwrap();

        // A pre-reset DB would have a legacy tracking row the new registry no
        // longer lists — the runner must ignore it, not error.
        conn.execute(
            "INSERT INTO schema_migrations (id, checksum) VALUES ('001_sales.sql', NULL)",
            [],
        )
        .unwrap();

        // User data that must survive the upgrade untouched.
        conn.execute(
            "INSERT INTO store_profiles (id, name) VALUES ('store-x', 'Store X')",
            [],
        )
        .unwrap();

        let tiers_before = row_count(&conn, "SELECT COUNT(*) FROM loyalty_tiers");
        let screens_before = row_count(&conn, "SELECT COUNT(*) FROM workspace_screens");

        // Boot the new code against the existing DB.
        run(&mut conn).unwrap();

        // The legacy row is ignored (still present) and the init is recorded
        // exactly once — the two rows coexist.
        let ids: Vec<String> = conn
            .prepare("SELECT id FROM schema_migrations ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(
            ids,
            vec!["001_sales.sql".to_string(), "20260813_init.sql".to_string()]
        );

        // INSERT OR IGNORE means the re-run did not duplicate seed rows.
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM loyalty_tiers"),
            tiers_before,
            "seed rows must not be duplicated on upgrade"
        );
        assert_eq!(
            row_count(&conn, "SELECT COUNT(*) FROM workspace_screens"),
            screens_before,
            "screen seed rows must not be duplicated on upgrade"
        );

        // User data survived.
        assert_eq!(
            row_count(
                &conn,
                "SELECT COUNT(*) FROM store_profiles WHERE id = 'store-x'"
            ),
            1,
            "user data must survive the upgrade"
        );

        // Schema surface is unchanged after the no-op re-run.
        assert_eq!(
            row_count(
                &conn,
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'"
            ),
            92,
            "table surface must be unchanged after upgrade"
        );
    }

    // ── Store-scoped isolation (DB-04 end-state) ───────────────────
    //
    // The consolidated schema carries a store_id FK on products, customers,
    // sales and sale_lines (ON DELETE SET NULL). These tests audit the
    // end-state contract: a store-scoped read or write must never leak across
    // stores and must never touch the NULL global sentinel.

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

    #[test]
    fn pg_init_declares_same_table_surface_as_sqlite() {
        // The Postgres port must cover every table in the SQLite init and
        // must not leak SQLite-only dialect through the generator.
        fn table_count(sql: &str) -> usize {
            sql.matches("CREATE TABLE IF NOT EXISTS").count()
        }
        let sqlite = include_str!("../migrations/20260813_init.sql");
        assert_eq!(
            table_count(PG_INIT),
            table_count(sqlite),
            "Postgres DDL table count drifted from the SQLite init — regenerate scripts/generate-pg-migration.py"
        );
        for leftover in [
            "strftime",
            "AUTOINCREMENT",
            "PRAGMA",
            "INSERT OR IGNORE",
            ") STRICT",
        ] {
            assert!(
                !PG_INIT.contains(leftover),
                "Postgres DDL still contains SQLite dialect: {leftover:?}"
            );
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
        //
        // `*.pg.sql` files are the generated Postgres ports of the SQLite
        // registry and are applied separately by the cloud server, so they
        // are not registry entries and are excluded here.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .expect("migrations directory must exist")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".sql") && !n.ends_with(".pg.sql"))
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
}
