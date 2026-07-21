//! Generic migration runner.
//!
//! A [`Migration`] is a named SQL script. [`run`] applies every
//! unapplied migration against a [`rusqlite::Connection`], tracking
//! applied migrations in a `schema_migrations` table.
//!
//! [`rollback_last`] reverts the most recently applied migration by
//! running its `down` SQL (if one exists).
//!
//! This runner is deliberately dependency-free beyond `rusqlite`.
//! Callers provide their own list of migrations (typically compiled
//! via `include_str!`).

use std::collections::HashSet;

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

/// Roll back the most recently applied migration by ID.
///
/// `down_sql` is the SQL to revert the migration (e.g. `DROP TABLE IF EXISTS x`).
/// Returns `Ok(false)` if no migrations have been applied or the given
/// migration ID is not the last applied one.
///
/// Only the last migration (by `applied_at` order) can be rolled back.
/// This prevents out-of-order reverts.
pub fn rollback(
    conn: &mut Connection,
    migration_id: &str,
    down_sql: &str,
) -> Result<bool, PlatformError> {
    // Ensure the tracking table exists before reading from it.
    ensure_schema_migrations_table(conn)?;
    let applied = load_applied_ordered(conn)?;
    let Some(last) = applied.last() else {
        return Ok(false); // No migrations applied
    };

    if last != migration_id {
        return Ok(false); // Can only rollback the last applied migration
    }

    tracing::info!(migration = migration_id, "rolling back migration");
    let tx: Transaction = conn.transaction()?;
    tx.execute_batch(down_sql)?;
    tx.execute(
        "DELETE FROM schema_migrations WHERE id = ?1",
        params![migration_id],
    )?;
    tx.commit()?;
    tracing::info!(migration = migration_id, "rollback complete");
    Ok(true)
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

fn load_applied(conn: &Connection) -> Result<HashSet<String>, PlatformError> {
    let mut stmt = conn.prepare("SELECT id FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = HashSet::new();
    for id in rows {
        set.insert(id?);
    }
    Ok(set)
}

/// Load applied migration IDs in application order (oldest first).
fn load_applied_ordered(conn: &Connection) -> Result<Vec<String>, PlatformError> {
    let mut stmt = conn.prepare("SELECT id FROM schema_migrations ORDER BY applied_at ASC")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut ids = Vec::new();
    for id in rows {
        ids.push(id?);
    }
    Ok(ids)
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

    const TWO_MIGRATIONS: &[Migration] = &[
        Migration {
            id: "001_first.sql",
            sql: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
        },
        Migration {
            id: "002_second.sql",
            sql: "ALTER TABLE test_table ADD COLUMN name TEXT",
        },
    ];

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

    // ── Rollback tests ─────────────────────────────────────────────

    #[test]
    fn rollback_reverts_last_migration_and_removes_tracking() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();

        // Verify table exists before rollback.
        let exists_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists_before, 1);

        // Rollback using the `rollback()` function with explicit down SQL.
        let rolled_back =
            rollback(&mut conn, "001_test.sql", "DROP TABLE IF EXISTS test_table").unwrap();
        assert!(rolled_back, "rollback should succeed");

        // Verify table was dropped.
        let exists_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists_after, 0, "table should be dropped after rollback");

        // Verify tracking row removed.
        let applied = load_applied(&conn).unwrap();
        assert!(
            !applied.contains("001_test.sql"),
            "tracking row should be removed"
        );
    }

    #[test]
    fn rollback_empty_db_returns_false() {
        let mut conn = fresh();
        let result = rollback(&mut conn, "001_test.sql", "DROP TABLE test_table").unwrap();
        assert!(!result, "rollback on empty DB should return false");
    }

    #[test]
    fn rollback_wrong_id_returns_false() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();

        // Try rolling back with a non-matching ID.
        let result = rollback(&mut conn, "999_wrong.sql", "DROP TABLE test_table").unwrap();
        assert!(!result, "rollback with wrong ID should return false");

        // Table should still exist.
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "table should survive failed rollback");
    }

    #[test]
    fn rollback_only_reverts_last_migration() {
        let mut conn = fresh();
        run(&mut conn, TWO_MIGRATIONS).unwrap();

        // Try rolling back the first migration while second is on top — should fail.
        let result = rollback(
            &mut conn,
            "001_first.sql",
            "DROP TABLE IF EXISTS test_table",
        )
        .unwrap();
        assert!(
            !result,
            "rollback of non-last migration should return false"
        );

        // Both still exist.
        let applied = load_applied(&conn).unwrap();
        assert_eq!(applied.len(), 2);

        // Rollback the last one instead.
        let result = rollback(
            &mut conn,
            "002_second.sql",
            "ALTER TABLE test_table DROP COLUMN name",
        )
        .unwrap();
        assert!(result, "rollback of last migration should succeed");

        let applied = load_applied(&conn).unwrap();
        assert_eq!(applied.len(), 1);
        assert!(applied.contains("001_first.sql"));
    }

    // ── Edge case tests ─────────────────────────────────────────────

    #[test]
    fn duplicate_migration_id_does_not_reapply() {
        let mut conn = fresh();
        // Run once.
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        // Run with a duplicate in the list (same id, different sql).
        let with_dup = &[Migration {
            id: "001_test.sql",
            sql: "CREATE TABLE other_table (id INTEGER PRIMARY KEY)",
        }];
        run(&mut conn, with_dup).unwrap();

        // The duplicate should NOT have been applied (idempotent).
        let exists_other: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='other_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists_other, 0, "duplicate ID should not trigger re-apply");
    }

    #[test]
    fn partial_crash_with_if_not_exists_recovers() {
        // Simulate a crash that occurs AFTER the SQL executes but BEFORE
        // the tracking INSERT is committed.
        //
        // In this scenario, `run()` will see NO tracking row and attempt
        // to re-apply the SQL. If the SQL uses `IF NOT EXISTS`, it succeeds
        // idempotently. If it uses plain `CREATE TABLE`, the re-apply will
        // fail — which is correct: the migration author must use idempotent
        // SQL patterns.
        //
        // This test verifies that re-running with idempotent SQL works.
        let mut conn = fresh();

        // Create the table manually (simulating the SQL that executed before crash).
        conn.execute_batch("CREATE TABLE test_table (id INTEGER PRIMARY KEY)")
            .unwrap();
        // No tracking row — simulating the missing INSERT + commit.

        // Now run with a migration that uses IF NOT EXISTS (recommended pattern).
        let idempotent_migration = &[Migration {
            id: "001_test.sql",
            sql: "CREATE TABLE IF NOT EXISTS test_table (id INTEGER PRIMARY KEY)",
        }];
        run(&mut conn, idempotent_migration).unwrap();

        // Tracking row should now exist.
        let applied = load_applied(&conn).unwrap();
        assert!(
            applied.contains("001_test.sql"),
            "tracking row should be added on recovery"
        );

        // Table should still exist.
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "table should survive recovery re-run");
    }

    #[test]
    fn migration_table_created_outside_of_runner() {
        // Verify that a table created manually (e.g. by a concurrent process)
        // is detected as already-applied via its tracking row.
        let mut conn = fresh();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                id TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO schema_migrations (id) VALUES (?1)",
            params!["001_test.sql"],
        )
        .unwrap();
        // Create the actual table too (simulating another process that did both).
        conn.execute_batch("CREATE TABLE test_table (id INTEGER PRIMARY KEY)")
            .unwrap();

        // Running migrations should be a no-op.
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        let applied = load_applied(&conn).unwrap();
        assert_eq!(applied.len(), 1);
    }

    #[test]
    fn load_applied_ordered_returns_correct_order() {
        let mut conn = fresh();
        run(&mut conn, TWO_MIGRATIONS).unwrap();

        let ordered = load_applied_ordered(&conn).unwrap();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0], "001_first.sql");
        assert_eq!(ordered[1], "002_second.sql");
    }

    #[test]
    fn rollback_then_rereapply_works() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();

        // Rollback.
        rollback(&mut conn, "001_test.sql", "DROP TABLE IF EXISTS test_table").unwrap();

        // Re-apply.
        run(&mut conn, TEST_MIGRATIONS).unwrap();

        // Table should exist again.
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "table should be re-created after re-apply");

        let applied = load_applied(&conn).unwrap();
        assert!(applied.contains("001_test.sql"));
    }
}
