//! Terminal Management — register, list, update, ping, delete terminals.

use rusqlite::params;

use crate::Terminal;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// List all registered terminals.
    pub fn list_terminals(&self) -> Result<Vec<Terminal>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, device_id, terminal_secret, is_active,
                    last_seen_at, metadata, created_at, updated_at
             FROM terminals ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_terminal)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get a terminal by id.
    pub fn get_terminal(&self, id: &str) -> Result<Option<Terminal>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, device_id, terminal_secret, is_active,
                    last_seen_at, metadata, created_at, updated_at
             FROM terminals WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], Self::row_to_terminal);
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a terminal by device_id.
    pub fn get_terminal_by_device_id(
        &self,
        device_id: &str,
    ) -> Result<Option<Terminal>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, device_id, terminal_secret, is_active,
                    last_seen_at, metadata, created_at, updated_at
             FROM terminals WHERE device_id = ?1",
        )?;
        let result = stmt.query_row(params![device_id], Self::row_to_terminal);
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Register a new terminal.
    pub fn create_terminal(&self, terminal: &Terminal) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO terminals (id, name, device_id, terminal_secret, is_active,
                                    last_seen_at, metadata, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                terminal.id,
                terminal.name,
                terminal.device_id,
                terminal.terminal_secret,
                terminal.is_active as i64,
                terminal.last_seen_at,
                terminal.metadata,
                terminal.created_at,
                terminal.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Update an existing terminal.
    pub fn update_terminal(&self, terminal: &Terminal) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE terminals SET name = ?1, device_id = ?2, terminal_secret = ?3,
                                   is_active = ?4, last_seen_at = ?5, metadata = ?6,
                                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?7",
            params![
                terminal.name,
                terminal.device_id,
                terminal.terminal_secret,
                terminal.is_active as i64,
                terminal.last_seen_at,
                terminal.metadata,
                terminal.id,
            ],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "terminal",
                id: terminal.id.clone(),
            });
        }
        Ok(())
    }

    /// Update a terminal's last_seen_at timestamp.
    pub fn ping_terminal(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE terminals SET last_seen_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "terminal",
                id: id.to_owned(),
            });
        }
        Ok(())
    }

    /// Delete a terminal by id.
    pub fn delete_terminal(&self, id: &str) -> Result<(), CoreError> {
        let affected = self
            .conn
            .execute("DELETE FROM terminals WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "terminal",
                id: id.to_owned(),
            });
        }
        Ok(())
    }

    fn row_to_terminal(row: &rusqlite::Row) -> rusqlite::Result<Terminal> {
        Ok(Terminal {
            id: row.get("id")?,
            name: row.get("name")?,
            device_id: row.get("device_id")?,
            terminal_secret: row.get("terminal_secret")?,
            is_active: row.get::<_, i64>("is_active")? != 0,
            last_seen_at: row.get("last_seen_at")?,
            metadata: row.get("metadata")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn make_terminal(id: &str, name: &str, device_id: &str) -> Terminal {
        Terminal {
            id: id.to_owned(),
            name: name.to_owned(),
            device_id: device_id.to_owned(),
            terminal_secret: Some("secret-ABC".to_string()),
            is_active: true,
            last_seen_at: None,
            metadata: Some("{}".to_string()),
            created_at: "2025-01-01T00:00:00.000Z".to_string(),
            updated_at: "2025-01-01T00:00:00.000Z".to_string(),
        }
    }

    fn seed_terminals(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO terminals (id, name, device_id, terminal_secret, is_active, metadata, created_at, updated_at) VALUES
                ('term-1', 'Front Register', 'dev-001', 'secret-1', 1, '{}', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('term-2', 'Back Office',    'dev-002', 'secret-2', 1, '{}', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('term-3', 'Kiosk',          'dev-003', 'secret-3', 0, '{\"model\":\"kiosk-v2\"}', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    // ── List ────────────────────────────────────────────────────────

    #[test]
    fn list_terminals_empty_db() {
        let conn = fresh();
        let terminals = store(&conn).list_terminals().unwrap();
        assert!(terminals.is_empty());
    }

    #[test]
    fn list_terminals_returns_all_ordered_by_name() {
        let conn = fresh();
        seed_terminals(&conn);
        let terminals = store(&conn).list_terminals().unwrap();
        assert_eq!(terminals.len(), 3);
        assert_eq!(terminals[0].name, "Back Office");
        assert_eq!(terminals[1].name, "Front Register");
        assert_eq!(terminals[2].name, "Kiosk");
    }

    #[test]
    fn list_terminals_includes_inactive() {
        let conn = fresh();
        seed_terminals(&conn);
        let terminals = store(&conn).list_terminals().unwrap();
        let kiosk = terminals.iter().find(|t| t.id == "term-3").unwrap();
        assert!(!kiosk.is_active);
        assert_eq!(kiosk.metadata.as_deref(), Some("{\"model\":\"kiosk-v2\"}"));
    }

    // ── Get ─────────────────────────────────────────────────────────

    #[test]
    fn get_terminal_found() {
        let conn = fresh();
        seed_terminals(&conn);
        let t = store(&conn).get_terminal("term-1").unwrap().unwrap();
        assert_eq!(t.name, "Front Register");
        assert_eq!(t.device_id, "dev-001");
        assert_eq!(t.terminal_secret.as_deref(), Some("secret-1"));
        assert!(t.is_active);
    }

    #[test]
    fn get_terminal_not_found() {
        let conn = fresh();
        let t = store(&conn).get_terminal("nonexistent").unwrap();
        assert!(t.is_none());
    }

    #[test]
    fn get_terminal_by_device_id_found() {
        let conn = fresh();
        seed_terminals(&conn);
        let t = store(&conn)
            .get_terminal_by_device_id("dev-002")
            .unwrap()
            .unwrap();
        assert_eq!(t.name, "Back Office");
        assert_eq!(t.id, "term-2");
    }

    #[test]
    fn get_terminal_by_device_id_not_found() {
        let conn = fresh();
        let t = store(&conn)
            .get_terminal_by_device_id("unknown-device")
            .unwrap();
        assert!(t.is_none());
    }

    // ── Create ──────────────────────────────────────────────────────

    #[test]
    fn create_terminal_persists() {
        let conn = fresh();
        let t = make_terminal("term-new", "New Register", "dev-999");
        store(&conn).create_terminal(&t).unwrap();

        let loaded = store(&conn).get_terminal("term-new").unwrap().unwrap();
        assert_eq!(loaded.name, "New Register");
        assert_eq!(loaded.device_id, "dev-999");
        assert_eq!(loaded.terminal_secret.as_deref(), Some("secret-ABC"));
        assert!(loaded.is_active);
    }

    #[test]
    fn create_terminal_with_metadata() {
        let conn = fresh();
        let t = Terminal {
            id: "term-meta".to_string(),
            name: "Meta Terminal".to_string(),
            device_id: "dev-meta".to_string(),
            terminal_secret: Some("sec-meta".to_string()),
            is_active: false,
            last_seen_at: None,
            metadata: Some("{\"location\":\"warehouse\"}".to_string()),
            created_at: "2025-01-01T00:00:00.000Z".to_string(),
            updated_at: "2025-01-01T00:00:00.000Z".to_string(),
        };
        store(&conn).create_terminal(&t).unwrap();

        let loaded = store(&conn).get_terminal("term-meta").unwrap().unwrap();
        assert!(!loaded.is_active);
        assert_eq!(
            loaded.metadata.as_deref(),
            Some("{\"location\":\"warehouse\"}")
        );
    }

    #[test]
    fn create_duplicate_terminal_id_fails() {
        let conn = fresh();
        let t = make_terminal("term-dup", "First", "dev-1");
        store(&conn).create_terminal(&t).unwrap();
        let dup = make_terminal("term-dup", "Second", "dev-2");
        let result = store(&conn).create_terminal(&dup);
        assert!(result.is_err());
    }

    // ── Update ──────────────────────────────────────────────────────

    #[test]
    fn update_terminal_basic() {
        let conn = fresh();
        seed_terminals(&conn);

        let updated = Terminal {
            id: "term-1".to_string(),
            name: "Front Register v2".to_string(),
            device_id: "dev-001-new".to_string(),
            terminal_secret: Some("new-secret".to_string()),
            is_active: true,
            last_seen_at: None,
            metadata: Some("{\"version\":2}".to_string()),
            created_at: String::new(),
            updated_at: String::new(),
        };
        store(&conn).update_terminal(&updated).unwrap();

        let loaded = store(&conn).get_terminal("term-1").unwrap().unwrap();
        assert_eq!(loaded.name, "Front Register v2");
        assert_eq!(loaded.device_id, "dev-001-new");
        assert_eq!(loaded.terminal_secret.as_deref(), Some("new-secret"));
        assert_eq!(loaded.metadata.as_deref(), Some("{\"version\":2}"));
        assert!(loaded.updated_at.as_str() > "2025-01-01");
    }

    #[test]
    fn update_terminal_not_found() {
        let conn = fresh();
        let t = make_terminal("nope", "X", "dev-x");
        let err = store(&conn).update_terminal(&t).unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "terminal"));
    }

    #[test]
    fn update_terminal_deactivate() {
        let conn = fresh();
        seed_terminals(&conn);

        let updated = Terminal {
            id: "term-1".to_string(),
            name: "Front Register".to_string(),
            device_id: "dev-001".to_string(),
            terminal_secret: Some("secret-1".to_string()),
            is_active: false,
            last_seen_at: None,
            metadata: Some("{}".to_string()),
            created_at: String::new(),
            updated_at: String::new(),
        };
        store(&conn).update_terminal(&updated).unwrap();

        let loaded = store(&conn).get_terminal("term-1").unwrap().unwrap();
        assert!(!loaded.is_active);
    }

    // ── Ping ────────────────────────────────────────────────────────

    #[test]
    fn ping_terminal_updates_timestamps() {
        let conn = fresh();
        seed_terminals(&conn);

        store(&conn).ping_terminal("term-1").unwrap();

        let loaded = store(&conn).get_terminal("term-1").unwrap().unwrap();
        assert!(loaded.last_seen_at.is_some(), "last_seen_at should be set");
        assert!(!loaded.updated_at.is_empty(), "updated_at should be set");
    }

    #[test]
    fn ping_terminal_not_found() {
        let conn = fresh();
        let err = store(&conn).ping_terminal("nonexistent").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "terminal"));
    }

    // ── Delete ──────────────────────────────────────────────────────

    #[test]
    fn delete_terminal_removes() {
        let conn = fresh();
        seed_terminals(&conn);
        store(&conn).delete_terminal("term-3").unwrap();
        let t = store(&conn).get_terminal("term-3").unwrap();
        assert!(t.is_none());
    }

    #[test]
    fn delete_terminal_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_terminal("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }
}
