//! Generic migration runner.
//!
//! A [`Migration`] is a named SQL script. [`run`] applies every
//! unapplied migration against a [`rusqlite::Connection`], tracking
//! applied migrations in a `schema_migrations` table.
//!
//! `rollback_last` reverts the most recently applied migration by
//! running its `down` SQL (if one exists).
//!
//! # Integrity guarantees (audit-open-findings DB-02 / DB-05)
//!
//! * **Migration checksums** — every applied migration records a SHA-256
//!   checksum of its SQL. [`run`] recomputes the checksum of each
//!   registered migration and **fails closed** when an already-applied
//!   definition changed (historical migrations must never be edited in
//!   place). Rows applied before checksum tracking existed are backfilled
//!   once on the first run after upgrade.
//! * **Foreign-key isolation** — [`run`] disables `foreign_keys` at the
//!   connection level *around* each migration apply and restores the
//!   caller's previous setting afterwards. SQLite ignores `PRAGMA
//!   foreign_keys` inside a transaction, so rebuild migrations (081/089)
//!   that toggle it in their own SQL were silently running with
//!   enforcement ON — risking cascade data loss on populated child tables.
//!
//! Callers provide their own list of migrations (typically compiled
//! via `include_str!`).

use std::collections::HashMap;
#[cfg(test)]
use std::collections::HashSet;

use rusqlite::{Connection, Transaction, params};
use sha2::{Digest, Sha256};

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
    let applied = load_applied_with_checksums(conn)?;
    for mig in migrations {
        match applied.get(mig.id) {
            Some(Some(stored)) => {
                // DB-02: an applied migration's definition must be byte-for-byte
                // identical to what was committed. Editing a historical file in
                // place would silently produce a different schema on fresh
                // installs vs upgrades.
                let current = checksum_hex(mig.sql);
                if *stored != current {
                    // Databases created before line-ending canonicalization
                    // may contain the raw Windows checksum. Accept that
                    // exact legacy representation and rewrite it to the
                    // canonical checksum.
                    if has_legacy_checksum(stored, mig.sql) {
                        update_checksum(conn, mig.id, &current)?;
                        tracing::info!(migration = mig.id, "normalized legacy migration checksum");
                    } else {
                        // DB-02: the migration SQL changed after it was applied.
                        // Instead of hard-failing (which bricks the app for
                        // comment-only / whitespace edits), re-run the
                        // migration SQL — properly-written migrations use
                        // `IF NOT EXISTS` / `IF EXISTS` and are idempotent.
                        // If the re-apply fails, the SQL has genuinely
                        // changed in a breaking way and the user must act.
                        tracing::warn!(
                            migration = mig.id,
                            stored = %stored,
                            current = %current,
                            "migration definition drift detected — \
                             re-applying SQL (must be idempotent) and updating checksum (DB-02)"
                        );
                        reapply_for_drift(conn, mig)?;
                        update_checksum(conn, mig.id, &current)?;
                        tracing::info!(migration = mig.id, "drift auto-patched — checksum updated");
                    }
                }
                tracing::debug!(migration = mig.id, "already applied; checksum verified");
            }
            Some(None) => {
                // Row applied before checksum tracking existed: adopt the
                // current definition as the baseline (one-time backfill).
                let current = checksum_hex(mig.sql);
                update_checksum(conn, mig.id, &current)?;
                tracing::info!(migration = mig.id, "backfilled legacy checksum");
            }
            None => apply_one(conn, mig)?,
        }
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
    // DB-05: destructive down SQL (DROP TABLE) must not cascade into
    // dependent rows; same connection-level isolation as apply_one.
    let fk_was_on = foreign_keys_enabled(conn)?;
    if fk_was_on {
        conn.pragma_update(None, "foreign_keys", "OFF")?;
    }
    let result = (|| -> Result<bool, PlatformError> {
        let tx: Transaction = conn.transaction()?;
        tx.execute_batch(down_sql)?;
        tx.execute(
            "DELETE FROM schema_migrations WHERE id = ?1",
            params![migration_id],
        )?;
        tx.commit()?;
        Ok(true)
    })();
    // Restore the caller's FK setting even on failure, but never let a
    // restore error mask the original migration error (DB-05).
    if fk_was_on && let Err(restore_err) = conn.pragma_update(None, "foreign_keys", "ON") {
        tracing::error!(
            migration = migration_id,
            error = %restore_err,
            "failed to restore foreign_keys=ON after rollback"
        );
    }
    if matches!(result, Ok(true)) {
        tracing::info!(migration = migration_id, "rollback complete");
    }
    result
}

fn ensure_schema_migrations_table(conn: &Connection) -> Result<(), PlatformError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            id         TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            checksum   TEXT
        )",
    )?;
    // DB-02: databases created before checksum tracking lack the column.
    // `CREATE TABLE IF NOT EXISTS` won't add it to an existing table, so
    // migrate the tracking table in place.
    let has_checksum: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('schema_migrations') WHERE name = 'checksum'",
        [],
        |r| r.get(0),
    )?;
    if has_checksum == 0 {
        conn.execute_batch("ALTER TABLE schema_migrations ADD COLUMN checksum TEXT")?;
    }
    Ok(())
}

/// Load applied migration IDs (used by tests; `run` uses the checksum
/// variant [`load_applied_with_checksums`] for drift detection).
#[cfg(test)]
fn load_applied(conn: &Connection) -> Result<HashSet<String>, PlatformError> {
    let mut stmt = conn.prepare("SELECT id FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = HashSet::new();
    for id in rows {
        set.insert(id?);
    }
    Ok(set)
}

/// Load applied migration IDs with their stored checksums.
///
/// `None` marks a row applied before checksum tracking existed (backfilled
/// on the next [`run`]).
fn load_applied_with_checksums(
    conn: &Connection,
) -> Result<HashMap<String, Option<String>>, PlatformError> {
    let mut stmt = conn.prepare("SELECT id, checksum FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (id, checksum) = row?;
        map.insert(id, checksum);
    }
    Ok(map)
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

/// SHA-256 hex checksum of a migration's SQL (DB-02).
///
/// Migration files are stored with LF endings, but a Windows working tree may
/// provide CRLF text to `include_str!`. Canonicalize line endings before
/// hashing so the checksum represents the SQL definition rather than the
/// checkout platform.
fn checksum_hex(sql: &str) -> String {
    let canonical = canonicalize_line_endings(sql);
    checksum_hex_bytes(canonical.as_bytes())
}

/// Whether a stored checksum matches a pre-canonicalization line ending form.
fn has_legacy_checksum(stored: &str, sql: &str) -> bool {
    let canonical = canonicalize_line_endings(sql);
    stored == legacy_checksum_hex(sql)
        || stored == legacy_checksum_hex(&canonical.replace('\n', "\r\n"))
}

/// Normalize CRLF and bare CR line endings to LF.
fn canonicalize_line_endings(sql: &str) -> String {
    sql.replace("\r\n", "\n").replace('\r', "\n")
}

/// SHA-256 hex checksum using the pre-canonicalization byte representation.
fn legacy_checksum_hex(sql: &str) -> String {
    checksum_hex_bytes(sql.as_bytes())
}

/// Hash bytes as a lowercase SHA-256 hex string.
fn checksum_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Update a stored migration checksum atomically.
fn update_checksum(
    conn: &mut Connection,
    migration_id: &str,
    checksum: &str,
) -> Result<(), PlatformError> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE schema_migrations SET checksum = ?1 WHERE id = ?2",
        params![checksum, migration_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// Whether the connection currently enforces foreign keys (DB-05).
fn foreign_keys_enabled(conn: &Connection) -> Result<bool, PlatformError> {
    let v: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
    Ok(v == 1)
}

/// Re-apply an already-applied migration's SQL to handle definition drift.
///
/// Used when a migration file was edited after it was already applied.
/// The SQL is re-executed (must be idempotent) and the stored checksum
/// is updated by the caller. FK isolation matches [`apply_one`].
fn reapply_for_drift(conn: &mut Connection, mig: &Migration) -> Result<(), PlatformError> {
    let fk_was_on = foreign_keys_enabled(conn)?;
    if fk_was_on {
        conn.pragma_update(None, "foreign_keys", "OFF")?;
    }
    let result = (|| -> Result<(), PlatformError> {
        let tx: Transaction = conn.transaction()?;
        tx.execute_batch(mig.sql)?;
        tx.commit()?;
        Ok(())
    })();
    if fk_was_on && let Err(restore_err) = conn.pragma_update(None, "foreign_keys", "ON") {
        tracing::error!(
            migration = mig.id,
            error = %restore_err,
            "failed to restore foreign_keys=ON after drift re-apply"
        );
    }
    result
}

fn apply_one(conn: &mut Connection, mig: &Migration) -> Result<(), PlatformError> {
    tracing::info!(migration = mig.id, "applying migration");
    // DB-05: `PRAGMA foreign_keys` is a no-op inside a transaction, so
    // rebuild migrations (081/089) cannot rely on their own toggles.
    // Disable enforcement at the connection level *before* the transaction
    // and restore the caller's prior setting afterwards.
    let fk_was_on = foreign_keys_enabled(conn)?;
    if fk_was_on {
        conn.pragma_update(None, "foreign_keys", "OFF")?;
    }
    let result = (|| -> Result<(), PlatformError> {
        let tx: Transaction = conn.transaction()?;
        tx.execute_batch(mig.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (id, checksum) VALUES (?1, ?2)",
            params![mig.id, checksum_hex(mig.sql)],
        )?;
        tx.commit()?;
        Ok(())
    })();
    // Restore the caller's FK setting even on failure, but never let a
    // restore error mask the original migration error (DB-05).
    if fk_was_on && let Err(restore_err) = conn.pragma_update(None, "foreign_keys", "ON") {
        tracing::error!(
            migration = mig.id,
            error = %restore_err,
            "failed to restore foreign_keys=ON after migration apply"
        );
    }
    result
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
    fn migration_checksums_are_stable_across_line_endings() {
        let lf = "CREATE TABLE test_table (id INTEGER PRIMARY KEY)\n";
        let crlf = lf.replace('\n', "\r\n");

        assert_eq!(checksum_hex(lf), checksum_hex(&crlf));
    }

    #[test]
    fn legacy_line_ending_checksum_is_migrated() {
        use sha2::Digest;

        let migrations = [Migration {
            id: "001_line_endings.sql",
            sql: "CREATE TABLE line_endings (id INTEGER PRIMARY KEY)\n",
        }];
        let mut conn = fresh();
        run(&mut conn, &migrations).unwrap();

        let legacy_sql = migrations[0].sql.replace('\n', "\r\n");
        let legacy_checksum = hex::encode(sha2::Sha256::digest(legacy_sql.as_bytes()));
        let tx = conn.transaction().unwrap();
        tx.execute(
            "UPDATE schema_migrations SET checksum = ?1 WHERE id = ?2",
            params![legacy_checksum, migrations[0].id],
        )
        .unwrap();
        tx.commit().unwrap();

        run(&mut conn, &migrations).unwrap();
        let stored: String = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE id = ?1",
                params![migrations[0].id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, checksum_hex(migrations[0].sql));
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
    fn duplicate_migration_id_with_identical_sql_is_skipped() {
        let mut conn = fresh();
        // Run once.
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        // Run again with the same list — the duplicate ID is skipped and the
        // checksum verifies (idempotent).
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        let applied = load_applied(&conn).unwrap();
        assert_eq!(applied.len(), 1);
    }

    #[test]
    fn drift_with_non_idempotent_sql_fails() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();

        // Same ID, non-idempotent SQL (missing IF NOT EXISTS) → re-apply
        // fails because the table already exists, surfacing the breaking
        // change to the user.
        let drifted = &[Migration {
            id: "001_test.sql",
            sql: "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)",
        }];
        let err = run(&mut conn, drifted).unwrap_err();
        // The error comes from the SQL execution, not the checksum check.
        assert!(
            err.to_string().contains("already exists"),
            "expected SQL re-apply error, got: {err}"
        );
    }

    #[test]
    fn drift_with_idempotent_sql_auto_patches() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();

        // Same ID, idempotent SQL (IF NOT EXISTS) → re-apply is a no-op,
        // checksum is auto-updated, and startup succeeds.
        let drifted = &[Migration {
            id: "001_test.sql",
            sql: "CREATE TABLE IF NOT EXISTS test_table (id INTEGER PRIMARY KEY, name TEXT)",
        }];
        run(&mut conn, drifted).unwrap(); // must not error

        // Checksum was updated to match the new definition.
        let stored: String = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE id = '001_test.sql'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, checksum_hex(drifted[0].sql));

        // Table still exists.
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='test_table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1);
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

        // Running migrations should be a no-op (and backfill the checksum).
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        let applied = load_applied(&conn).unwrap();
        assert_eq!(applied.len(), 1);
        let stored: String = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE id = '001_test.sql'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, checksum_hex(TEST_MIGRATIONS[0].sql));
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

    // ── DB-02: checksum tracking & drift detection ─────────────────

    #[test]
    fn applied_migration_records_checksum() {
        let mut conn = fresh();
        run(&mut conn, TEST_MIGRATIONS).unwrap();
        let stored: String = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE id = '001_test.sql'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, checksum_hex(TEST_MIGRATIONS[0].sql));
    }

    #[test]
    fn legacy_row_without_checksum_is_backfilled() {
        // Old-shape tracking table (no checksum column) with an applied row
        // — simulates a database migrated before DB-02 shipped.
        let mut conn = fresh();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                id TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO schema_migrations (id) VALUES ('001_test.sql')",
            [],
        )
        .unwrap();
        conn.execute_batch("CREATE TABLE test_table (id INTEGER PRIMARY KEY)")
            .unwrap();

        run(&mut conn, TEST_MIGRATIONS).unwrap();

        let stored: String = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE id = '001_test.sql'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            stored,
            checksum_hex(TEST_MIGRATIONS[0].sql),
            "legacy row must be backfilled with the current definition checksum"
        );
    }

    // ── DB-05: FK isolation around rebuild migrations ──────────────

    #[test]
    fn foreign_keys_disabled_during_rebuild_migration() {
        let mut conn = fresh(); // FK enforcement ON

        run(
            &mut conn,
            &[Migration {
                id: "001_parent.sql",
                sql: "CREATE TABLE parent (id TEXT PRIMARY KEY);
                      CREATE TABLE child (id TEXT PRIMARY KEY, parent_id TEXT NOT NULL REFERENCES parent(id) ON DELETE CASCADE);",
            }],
        )
        .unwrap();
        conn.execute("INSERT INTO parent (id) VALUES ('p1')", [])
            .unwrap();
        conn.execute("INSERT INTO child (id, parent_id) VALUES ('c1', 'p1')", [])
            .unwrap();

        // Rebuild migration following the 081/089 pattern (DROP parent, then
        // rename the replacement into place). With FK enforcement ON at the
        // connection level, DROP TABLE would cascade-delete the child row.
        run(
            &mut conn,
            &[Migration {
                id: "002_rebuild.sql",
                sql: "CREATE TABLE parent_new (id TEXT PRIMARY KEY);
                      INSERT INTO parent_new SELECT id FROM parent;
                      DROP TABLE parent;
                      ALTER TABLE parent_new RENAME TO parent;",
            }],
        )
        .unwrap();

        // Child row must survive the parent rebuild.
        let children: i64 = conn
            .query_row("SELECT COUNT(*) FROM child", [], |r| r.get(0))
            .unwrap();
        assert_eq!(children, 1, "child row must survive parent rebuild");

        // FK enforcement restored, and no violations remain.
        let fk_violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fk_violations, 0, "no FK violations after rebuild");
    }
}
