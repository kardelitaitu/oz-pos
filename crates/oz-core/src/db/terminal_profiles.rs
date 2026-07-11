//! Terminal profile CRUD — kiosk/kds lockdown per terminal.
//!
//! Each terminal can have a profile type that controls which UI is
//! rendered and whether navigation is restricted. See the `050_terminal_profiles`
//! migration for schema details.

use rusqlite::params;

use crate::error::CoreError;
use crate::terminal_profile::TerminalProfile;

use super::Store;

impl Store<'_> {
    /// Get the profile for a terminal, if one exists.
    pub fn get_terminal_profile(
        &self,
        terminal_id: &str,
    ) -> Result<Option<TerminalProfile>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT terminal_id, profile_type, locked_screen, updated_at
             FROM terminal_profiles WHERE terminal_id = ?1",
        )?;
        let result = stmt.query_row(params![terminal_id], |row| {
            Ok(TerminalProfile {
                terminal_id: row.get("terminal_id")?,
                profile_type: row.get("profile_type")?,
                locked_screen: row.get("locked_screen")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set (upsert) the profile for a terminal.
    ///
    /// If a profile already exists, it is replaced; otherwise a new
    /// row is inserted.
    pub fn set_terminal_profile(
        &self,
        terminal_id: &str,
        profile_type: &str,
        locked_screen: Option<&str>,
    ) -> Result<(), CoreError> {
        // Verify the terminal exists.
        let exists: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM terminals WHERE id = ?1",
                params![terminal_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !exists {
            return Err(CoreError::NotFound {
                entity: "terminal",
                id: terminal_id.to_owned(),
            });
        }

        self.conn.execute(
            "INSERT INTO terminal_profiles (terminal_id, profile_type, locked_screen, updated_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(terminal_id) DO UPDATE SET
                profile_type = excluded.profile_type,
                locked_screen = excluded.locked_screen,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![terminal_id, profile_type, locked_screen],
        )?;
        Ok(())
    }

    /// Delete a terminal's profile row.
    pub fn delete_terminal_profile(&self, terminal_id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "DELETE FROM terminal_profiles WHERE terminal_id = ?1",
            params![terminal_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "terminal_profile",
                id: terminal_id.to_owned(),
            });
        }
        Ok(())
    }

    /// List all terminal profiles.
    pub fn list_terminal_profiles(&self) -> Result<Vec<TerminalProfile>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT terminal_id, profile_type, locked_screen, updated_at
             FROM terminal_profiles ORDER BY terminal_id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TerminalProfile {
                terminal_id: row.get("terminal_id")?,
                profile_type: row.get("profile_type")?,
                locked_screen: row.get("locked_screen")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }
}

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

    fn seed_terminal(conn: &Connection, id: &str, name: &str, device_id: &str) {
        conn.execute(
            "INSERT INTO terminals (id, name, device_id, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1,
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![id, name, device_id],
        )
        .unwrap();
    }

    // ── Get ─────────────────────────────────────────────────────────

    #[test]
    fn get_profile_when_not_set_returns_none() {
        let conn = fresh();
        seed_terminal(&conn, "t1", "Front Counter", "dev-1");
        let profile = store(&conn).get_terminal_profile("t1").unwrap();
        assert!(profile.is_none());
    }

    #[test]
    fn get_profile_after_set() {
        let conn = fresh();
        seed_terminal(&conn, "t1", "Front Counter", "dev-1");
        store(&conn)
            .set_terminal_profile("t1", "kds_kiosk", None)
            .unwrap();

        let profile = store(&conn).get_terminal_profile("t1").unwrap().unwrap();
        assert_eq!(profile.terminal_id, "t1");
        assert_eq!(profile.profile_type, "kds_kiosk");
        assert!(profile.locked_screen.is_none());
    }

    #[test]
    fn get_profile_not_found_when_terminal_does_not_exist() {
        let conn = fresh();
        let profile = store(&conn).get_terminal_profile("nonexistent").unwrap();
        assert!(profile.is_none());
    }

    // ── Set ─────────────────────────────────────────────────────────

    #[test]
    fn set_profile_upserts() {
        let conn = fresh();
        seed_terminal(&conn, "t1", "Front Counter", "dev-1");
        store(&conn)
            .set_terminal_profile("t1", "kds_kiosk", None)
            .unwrap();

        // Update existing
        store(&conn)
            .set_terminal_profile("t1", "counter_pos", Some("products"))
            .unwrap();

        let profile = store(&conn).get_terminal_profile("t1").unwrap().unwrap();
        assert_eq!(profile.profile_type, "counter_pos");
        assert_eq!(profile.locked_screen.as_deref(), Some("products"));
    }

    #[test]
    fn set_profile_with_locked_screen() {
        let conn = fresh();
        seed_terminal(&conn, "t2", "KDS Tablet", "dev-2");
        store(&conn)
            .set_terminal_profile("t2", "kds_kiosk", Some("kds"))
            .unwrap();

        let profile = store(&conn).get_terminal_profile("t2").unwrap().unwrap();
        assert_eq!(profile.profile_type, "kds_kiosk");
        assert_eq!(profile.locked_screen.as_deref(), Some("kds"));
    }

    #[test]
    fn set_profile_fails_for_nonexistent_terminal() {
        let conn = fresh();
        let err = store(&conn)
            .set_terminal_profile("nonexistent", "kds_kiosk", None)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "terminal"));
    }

    // ── Delete ──────────────────────────────────────────────────────

    #[test]
    fn delete_profile_removes_row() {
        let conn = fresh();
        seed_terminal(&conn, "t1", "Front Counter", "dev-1");
        store(&conn)
            .set_terminal_profile("t1", "kds_kiosk", None)
            .unwrap();
        store(&conn).delete_terminal_profile("t1").unwrap();

        let profile = store(&conn).get_terminal_profile("t1").unwrap();
        assert!(profile.is_none());
    }

    #[test]
    fn delete_profile_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .delete_terminal_profile("nonexistent")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "terminal_profile"));
    }

    // ── List ────────────────────────────────────────────────────────

    #[test]
    fn list_profiles_empty_db() {
        let conn = fresh();
        let profiles = store(&conn).list_terminal_profiles().unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn list_profiles_after_seeding() {
        let conn = fresh();
        seed_terminal(&conn, "t1", "Front Counter", "dev-1");
        seed_terminal(&conn, "t2", "KDS Tablet", "dev-2");
        store(&conn)
            .set_terminal_profile("t1", "counter_pos", None)
            .unwrap();
        store(&conn)
            .set_terminal_profile("t2", "kds_kiosk", Some("kds"))
            .unwrap();

        let profiles = store(&conn).list_terminal_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].terminal_id, "t1");
        assert_eq!(profiles[1].profile_type, "kds_kiosk");
    }
}
