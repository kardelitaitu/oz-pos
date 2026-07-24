//! Terminal Repository — database persistence layer for POS terminals.

use crate::models::Terminal;
use rusqlite::{Connection, params};

/// Database access repository for terminal records.
pub struct TerminalRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TerminalRepository<'a> {
    /// Create a new `TerminalRepository`.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve a terminal by ID.
    pub fn get_terminal(&self, id: &str) -> Result<Option<Terminal>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, device_id, terminal_secret, is_active, last_seen_at, metadata, created_at, updated_at
             FROM terminals WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        let row = match rows.next()? {
            Some(r) => r,
            None => return Ok(None),
        };

        Ok(Some(Terminal {
            id: row.get(0)?,
            name: row.get(1)?,
            device_id: row.get(2)?,
            terminal_secret: row.get(3)?,
            is_active: row.get::<_, i64>(4)? != 0,
            last_seen_at: row.get(5)?,
            metadata: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        }))
    }
}
