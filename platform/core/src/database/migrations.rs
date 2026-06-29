//! Generic migration runner.
//!
//! A [`Migration`] is a named SQL script. [`run`] applies every
//! unapplied migration against a [`rusqlite::Connection`], tracking
//! applied migrations in a `schema_migrations` table.
//!
//! This runner is deliberately dependency-free beyond `rusqlite`.
//! Callers provide their own list of migrations (typically compiled
//! via `include_str!`).

use rusqlite::{Connection, Transaction, params};

use crate::error::PlatformError;

/// One embedded migration.
pub struct Migration {
    /// Filename, e.g. `"001_sales.sql"`. Also used as the primary key in
    /// `schema_migrations`.
    pub id: &'static str,
    /// Raw SQL contents.
    pub sql: &'static str,
}

/// Apply every unapplied migration. Idempotent: running twice is a no-op
/// after the first call.
///
/// Requires `&mut Connection` because [`Connection::transaction`] does.
pub fn run(conn: &mut Connection, migrations: &[Migration]) -> Result<(), PlatformError> {
    ensure_schema_migrations_table(conn)?;
    let applied = load_applied(conn)?;
    for mig in migrations {
        if applied.contains(mig.id) {
            tracing::debug!(migration = mig.id, "already applied; skipping");
            continue;
        }
        apply_one(conn, mig)?;
    }
    Ok(())
}

fn ensure_schema_migrations_table(conn: &Connection) -> Result<(), PlatformError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            id         TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        )",
    )?;
    Ok(())
}

fn load_applied(conn: &Connection) -> Result<std::collections::HashSet<String>, PlatformError> {
    let mut stmt = conn.prepare("SELECT id FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = std::collections::HashSet::new();
    for id in rows {
        set.insert(id?);
    }
    Ok(set)
}

fn apply_one(conn: &mut Connection, mig: &Migration) -> Result<(), PlatformError> {
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

    const TEST_MIGRATIONS: &[Migration] = &[Migration {
        id: "001_test.sql",
        sql: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
    }];

    #[test]
    fn first_run_applies_all_migrations() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        let applied = load_applied(&conn).unwrap();
        for mig in TEST_MIGRATIONS {
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
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        let applied = load_applied(&conn).unwrap();
        assert_eq!(applied.len(), TEST_MIGRATIONS.len());
    }

    #[test]
    fn migration_creates_table() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "expected `test_table` after migration");
    }

    #[test]
    fn run_with_empty_list_does_nothing() {
        let mut conn = fresh();
        run(&mut conn, &[]).unwrap();
        let applied = load_applied(&conn).unwrap();
        assert!(applied.is_empty());
    }
}
