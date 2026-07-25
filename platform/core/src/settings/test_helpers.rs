//! Shared test helpers for the `settings` module.
//!
//! Only compiled when running tests.

use rusqlite::Connection;

/// Create an in-memory SQLite connection with the `settings` table
/// initialized and foreign keys enabled. This helper is used by both
/// the main settings test suite and the split-sanity tests.
pub fn fresh() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        )",
    )
    .unwrap();
    conn
}
