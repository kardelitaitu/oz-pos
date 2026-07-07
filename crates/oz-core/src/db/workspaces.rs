//! Workspace CRUD — workspace definitions, navigation screens, and
//! per-user workspace access assignments.
//!
//! A user's effective workspace set is resolved as:
//! 1. `role-owner` → all workspaces (admin bypass)
//! 2. `user_workspaces` rows exist → return ONLY those keys
//! 3. Otherwise → fall back to `role_workspaces`

use rusqlite::params;
use serde::Serialize;

use crate::error::CoreError;

use super::Store;

/// DTO for workspace data sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceRow {
    /// Unique key identifying the workspace.
    pub key: String,
    /// Human-readable display name.
    pub name: String,
    /// Short description of the workspace purpose.
    pub description: String,
    /// Icon identifier for the workspace card.
    pub icon: String,
}

/// DTO for workspace screen data.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceScreenRow {
    /// Key identifying the screen within a workspace.
    pub screen_key: String,
    /// Display order (ascending).
    pub sort_order: i32,
}

impl Store<'_> {
    /// List workspaces accessible to a given role, with optional
    /// per-user override.
    ///
    /// Resolution order:
    /// 1. `role-owner` → all workspaces
    /// 2. If `user_id` is provided and `user_workspaces` has rows for
    ///    that user → return ONLY those workspaces (replace mode)
    /// 3. Otherwise → fall back to `role_workspaces`
    pub fn list_workspaces(
        &self,
        role_id: &str,
        user_id: Option<&str>,
    ) -> Result<Vec<WorkspaceRow>, CoreError> {
        // 1. Owner bypass.
        if role_id == "role-owner" {
            return self.list_all_workspaces();
        }

        // 2. Check for explicit user-level workspace assignment.
        if let Some(uid) = user_id {
            let user_keys: Vec<String> = self
                .conn
                .prepare("SELECT ws_key FROM user_workspaces WHERE user_id = ?1")?
                .query_map(params![uid], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();

            if !user_keys.is_empty() {
                // Use parameterised IN clause for safety.
                let placeholders: Vec<String> = user_keys
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 1))
                    .collect();
                let sql = format!(
                    "SELECT w.key, w.name, w.description, w.icon
                     FROM workspaces w
                     WHERE w.key IN ({})
                     ORDER BY w.name",
                    placeholders.join(", ")
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let param_refs: Vec<&dyn rusqlite::types::ToSql> = user_keys
                    .iter()
                    .map(|k| k as &dyn rusqlite::types::ToSql)
                    .collect();
                let rows = stmt.query_map(param_refs.as_slice(), Self::map_workspace_row)?;
                return rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from);
            }
        }

        // 3. Fall back to role-based access.
        self.list_role_workspaces(role_id)
    }

    /// List ALL workspaces in the system (for admin dropdowns).
    ///
    /// No access control — callers must gate on `staff:read` or equivalent.
    pub fn list_all_workspaces(&self) -> Result<Vec<WorkspaceRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT w.key, w.name, w.description, w.icon
             FROM workspaces w
             ORDER BY w.name",
        )?;
        let rows = stmt.query_map([], Self::map_workspace_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    fn map_workspace_row(row: &rusqlite::Row) -> rusqlite::Result<WorkspaceRow> {
        Ok(WorkspaceRow {
            key: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            icon: row.get(3)?,
        })
    }

    /// List workspaces accessible via role_workspaces for a given role.
    fn list_role_workspaces(&self, role_id: &str) -> Result<Vec<WorkspaceRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT w.key, w.name, w.description, w.icon
             FROM workspaces w
             JOIN role_workspaces rw ON w.key = rw.workspace_key
             WHERE rw.role_id = ?1
             ORDER BY w.name",
        )?;
        let rows = stmt.query_map(params![role_id], Self::map_workspace_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Get screens (nav items) for a given workspace.
    pub fn list_workspace_screens(
        &self,
        workspace_key: &str,
    ) -> Result<Vec<WorkspaceScreenRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT screen_key, sort_order
             FROM workspace_screens
             WHERE workspace_key = ?1
             ORDER BY sort_order",
        )?;
        let rows = stmt.query_map([workspace_key], |row| {
            Ok(WorkspaceScreenRow {
                screen_key: row.get(0)?,
                sort_order: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    // ── User-workspace assignment ─────────────────────────────────

    /// Replace all workspace assignments for a user (delete old, insert
    /// new in a single transaction).
    ///
    /// Passing an empty `ws_keys` clears all assignments, causing the
    /// user to fall back to role-based defaults.
    pub fn set_user_workspaces<'b>(
        &self,
        user_id: &str,
        ws_keys: impl IntoIterator<Item = &'b str>,
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "DELETE FROM user_workspaces WHERE user_id = ?1",
            params![user_id],
        )?;

        for key in ws_keys {
            tx.execute(
                "INSERT OR IGNORE INTO user_workspaces (user_id, ws_key) VALUES (?1, ?2)",
                params![user_id, key],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get the explicit workspace keys assigned to a user.
    /// Returns an empty vec when the user has no custom assignments
    /// (i.e. they use role-based defaults).
    pub fn get_user_workspace_keys(&self, user_id: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT ws_key FROM user_workspaces WHERE user_id = ?1 ORDER BY ws_key")?;
        let rows = stmt.query_map(params![user_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn fresh() -> (Store<'static>, String) {
        let conn = migrations::fresh_db();
        let conn: &'static rusqlite::Connection = Box::leak(Box::new(conn));
        let store = Store::new(conn);

        // Seed a role and user for FK compliance.
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at)
             VALUES ('role-test', 'Test', 'Test', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at)
             VALUES ('user-1', 'alice', 'hash', 'Alice', 'role-test', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();

        (store, "user-1".into())
    }

    #[test]
    fn list_all_workspaces_returns_seeded() {
        let (store, _) = fresh();
        let ws = store.list_all_workspaces().unwrap();
        assert!(ws.len() >= 4); // restaurant-pos, store-pos, inventory, admin
        assert!(ws.iter().any(|w| w.key == "restaurant-pos"));
    }

    #[test]
    fn list_workspaces_owner_returns_all() {
        let (store, _) = fresh();
        let ws = store.list_workspaces("role-owner", None).unwrap();
        assert!(ws.len() >= 4);
    }

    #[test]
    fn set_user_workspaces_replaces_previous() {
        let (store, user_id) = fresh();

        store
            .set_user_workspaces(&user_id, ["restaurant-pos", "inventory"])
            .unwrap();
        let keys = store.get_user_workspace_keys(&user_id).unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"inventory".into()));

        // Replace with a different set.
        store.set_user_workspaces(&user_id, ["admin"]).unwrap();
        let keys = store.get_user_workspace_keys(&user_id).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "admin");
    }

    #[test]
    fn set_user_workspaces_empty_clears() {
        let (store, user_id) = fresh();
        store.set_user_workspaces(&user_id, ["admin"]).unwrap();
        store.set_user_workspaces(&user_id, []).unwrap();
        let keys = store.get_user_workspace_keys(&user_id).unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn list_workspaces_with_user_override() {
        let (store, user_id) = fresh();

        // No user override → falls back to role_workspaces.
        let before = store.list_workspaces("role-test", Some(&user_id)).unwrap();
        assert!(before.is_empty(), "role-test has no role_workspaces");

        // Set explicit workspace for user.
        store.set_user_workspaces(&user_id, ["admin"]).unwrap();
        let after = store.list_workspaces("role-test", Some(&user_id)).unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].key, "admin");
    }

    #[test]
    fn list_workspaces_user_override_ignores_role() {
        let (store, user_id) = fresh();
        store.set_user_workspaces(&user_id, ["admin"]).unwrap();

        // Even if role has no role_workspaces, user override still works.
        let ws = store.list_workspaces("role-test", Some(&user_id)).unwrap();
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn get_user_workspace_keys_empty_when_no_override() {
        let (store, user_id) = fresh();
        let keys = store.get_user_workspace_keys(&user_id).unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn list_workspaces_without_user_falls_back_to_role() {
        let (store, _) = fresh();
        // role-test has no role_workspaces.
        let ws = store.list_workspaces("role-test", None).unwrap();
        assert!(ws.is_empty());
    }
}
