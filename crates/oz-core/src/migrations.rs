//! Migration runner.
//!
//! Migrations are `.sql` files under `crates/oz-core/migrations/`. They are
//! embedded at compile time via [`include_str!`] (in `lib.rs`) and run in
//! lexicographic order on first startup. Applied migrations are tracked in
//! a `schema_migrations` table so subsequent runs are no-ops.
//!
//! This is a deliberately small, dependency-free runner. Larger projects
//! may want `refinery` or `sqlx-migrate`, but the OZ-POS migration count
//! is small (a handful of files) and the surface area of "open conn, run
//! .sql inside a transaction, mark applied" is short enough to keep here.

use rusqlite::{Connection, Transaction, params};

use crate::error::CoreError;

/// One embedded migration.
pub struct Migration {
    /// Filename, e.g. `"001_sales.sql"`. Also used as the primary key in
    /// `schema_migrations`.
    pub id: &'static str,
    /// Raw SQL contents.
    pub sql: &'static str,
}

/// All migrations in the order they should be applied.
///
/// The list is exhaustive at compile time; adding a new migration means
/// adding a new `Migration` entry here AND a new file in
/// `crates/oz-core/migrations/`.
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
];

/// Apply every unapplied migration. Idempotent: running twice is a no-op
/// after the first call.
///
/// Requires `&mut Connection` because [`Connection::transaction`] does.
/// The `ensure_schema_migrations_table` and `load_applied` helpers take
/// `&Connection` (read-only access) and are safe to call from any
/// context.
pub fn run(conn: &mut Connection) -> Result<(), CoreError> {
    ensure_schema_migrations_table(conn)?;
    let applied = load_applied(conn)?;
    for mig in ALL {
        if applied.contains(mig.id) {
            tracing::debug!(migration = mig.id, "already applied; skipping");
            continue;
        }
        apply_one(conn, mig)?;
    }
    Ok(())
}

fn ensure_schema_migrations_table(conn: &Connection) -> Result<(), CoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            id         TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        )",
    )?;
    Ok(())
}

fn load_applied(conn: &Connection) -> Result<std::collections::HashSet<String>, CoreError> {
    let mut stmt = conn.prepare("SELECT id FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = std::collections::HashSet::new();
    for id in rows {
        set.insert(id?);
    }
    Ok(set)
}

fn apply_one(conn: &mut Connection, mig: &Migration) -> Result<(), CoreError> {
    tracing::info!(migration = mig.id, "applying migration");
    let tx: Transaction = conn.transaction()?;
    tx.execute_batch(mig.sql)?;
    tx.execute(
        "INSERT INTO schema_migrations (id) VALUES (?1)",
        params![mig.id],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn
    }

    #[test]
    fn first_run_applies_all_migrations() {
        let mut conn = fresh();
        run(&mut conn).unwrap();
        let applied = load_applied(&conn).unwrap();
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
        let applied = load_applied(&conn).unwrap();
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
}
