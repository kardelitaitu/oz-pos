//! Role assignments with explicit-all scopes (ADR #35 D5 / spec 0048).
//!
//! A user's single effective assignment pairs a role with a `scope_mode`:
//! `global` (org-level roles — Owner, Admin, Auditor; branch/workspace scope
//! is ignored) or `scoped` (each of the branch and workspace dimensions is an
//! explicit `all` or a `list` — empty lists never mean "all", per the ADR).
//!
//! The evaluation rule is fail-closed: a scoped assignment grants only when
//! every requested dimension is either explicit `all` or contains the
//! requested id; a missing request context on a `list` dimension denies; an
//! unparsable `scope_mode` row is treated as no assignment at all.

use rusqlite::{OptionalExtension, params};

use crate::error::CoreError;

use super::Store;

/// Assignment scope mode (ADR #35 D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeMode {
    /// Org-level role — branch and workspace scope are ignored.
    Global,
    /// Branch and workspace dimensions are evaluated.
    Scoped,
}

impl ScopeMode {
    /// The SQL value for this mode.
    pub fn as_str(self) -> &'static str {
        match self {
            ScopeMode::Global => "global",
            ScopeMode::Scoped => "scoped",
        }
    }

    /// Parse the SQL value; `None` for anything else (fail closed).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "global" => Some(ScopeMode::Global),
            "scoped" => Some(ScopeMode::Scoped),
            _ => None,
        }
    }
}

/// A user's single effective role assignment (ADR #35 D5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// The owning user (also the assignments primary key).
    pub user_id: String,
    /// The role this user resolves to.
    pub role_id: String,
    /// `global` or `scoped`.
    pub scope_mode: ScopeMode,
    /// Branch dimension is explicit `all` (ignored for `Global`).
    pub branches_all: bool,
    /// Branch ids in scope when `branches_all` is false.
    pub branches: Vec<String>,
    /// Workspace dimension is explicit `all` (ignored for `Global`).
    pub workspaces_all: bool,
    /// Workspace keys in scope when `workspaces_all` is false.
    pub workspaces: Vec<String>,
}

impl Assignment {
    /// Whether this assignment grants access to the given `(branch, workspace)`
    /// request context.
    ///
    /// - `Global` mode ignores both dimensions.
    /// - `Scoped` mode requires each dimension to be explicit `all` or contain
    ///   the requested id; `None` context on a `list` dimension denies
    ///   (fail closed), and an empty list never means "all".
    pub fn matches_scope(&self, branch: Option<&str>, workspace: Option<&str>) -> bool {
        match self.scope_mode {
            // Org-level roles ignore both dimensions.
            ScopeMode::Global => true,
            ScopeMode::Scoped => {
                let branch_ok = self.branches_all
                    || branch.is_some_and(|b| self.branches.iter().any(|x| x == b));
                let workspace_ok = self.workspaces_all
                    || workspace.is_some_and(|w| self.workspaces.iter().any(|x| x == w));
                branch_ok && workspace_ok
            }
        }
    }
}

impl Store<'_> {
    /// Load a user's single effective assignment, or `None` when the user has
    /// none — legacy rows created before 0048, or a corrupt `scope_mode`
    /// (fail closed: no assignment means no grant).
    pub fn assignment_for_user(&self, user_id: &str) -> Result<Option<Assignment>, CoreError> {
        let Some((role_id, scope_mode, branch_scope, workspace_scope)) = self
            .conn
            .query_row(
                "SELECT role_id, scope_mode, branch_scope, workspace_scope
                 FROM assignments WHERE user_id = ?1",
                params![user_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
        else {
            return Ok(None);
        };

        // Fail closed: an unparsable scope_mode is treated as no assignment
        // (no grant). The schema CHECK makes this unreachable via SQL, but
        // defense-in-depth keeps the rule in the model.
        let Some(scope_mode) = ScopeMode::parse(&scope_mode) else {
            return Ok(None);
        };

        let branches = self.branch_ids_for(user_id)?;
        let workspaces = self.workspace_keys_for(user_id)?;

        Ok(Some(Assignment {
            user_id: user_id.to_string(),
            role_id,
            scope_mode,
            branches_all: branch_scope != "list",
            branches,
            workspaces_all: workspace_scope != "list",
            workspaces,
        }))
    }

    /// Branch ids in scope for a user's assignment (empty when `all`).
    fn branch_ids_for(&self, user_id: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT branch_id FROM assignment_branches
             WHERE assignment_user_id = ?1 ORDER BY branch_id",
        )?;
        let rows = stmt.query_map(params![user_id], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Workspace keys in scope for a user's assignment (empty when `all`).
    fn workspace_keys_for(&self, user_id: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT workspace_key FROM assignment_workspaces
             WHERE assignment_user_id = ?1 ORDER BY workspace_key",
        )?;
        let rows = stmt.query_map(params![user_id], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn seed_user(conn: &rusqlite::Connection) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, permissions) VALUES
                 ('role-staff', 'staff', '[\"sales:view\"]');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('u1', 'u1', 'h', 'U1', 'role-staff', 1,
                     '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');",
        )
        .unwrap();
    }

    fn insert_assignment(
        conn: &rusqlite::Connection,
        mode: &str,
        branch_scope: &str,
        workspace_scope: &str,
    ) {
        conn.execute(
            "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
             VALUES ('u1', 'role-staff', ?1, ?2, ?3)",
            params![mode, branch_scope, workspace_scope],
        )
        .unwrap();
    }

    #[test]
    fn assignment_for_user_loads_global_assignment() {
        let conn = migrations::fresh_db();
        seed_user(&conn);
        insert_assignment(&conn, "global", "all", "all");
        let store = Store::new(&conn);

        let a = store
            .assignment_for_user("u1")
            .unwrap()
            .expect("assignment");
        assert_eq!(a.user_id, "u1");
        assert_eq!(a.role_id, "role-staff");
        assert_eq!(a.scope_mode, ScopeMode::Global);
        assert!(a.branches.is_empty() && a.workspaces.is_empty());
    }

    #[test]
    fn assignment_for_user_returns_none_when_absent() {
        let conn = migrations::fresh_db();
        seed_user(&conn);
        let store = Store::new(&conn);

        assert!(store.assignment_for_user("u1").unwrap().is_none());
        assert!(store.assignment_for_user("no-such-user").unwrap().is_none());
    }

    #[test]
    fn assignment_for_user_loads_scoped_lists() {
        let conn = migrations::fresh_db();
        seed_user(&conn);
        insert_assignment(&conn, "scoped", "list", "list");
        conn.execute_batch(
            "INSERT INTO assignment_branches (assignment_user_id, branch_id) VALUES
                 ('u1', 'store-a'), ('u1', 'store-b');
             INSERT INTO assignment_workspaces (assignment_user_id, workspace_key)
             VALUES ('u1', 'retail-pos');",
        )
        .unwrap();
        let store = Store::new(&conn);

        let a = store
            .assignment_for_user("u1")
            .unwrap()
            .expect("assignment");
        assert_eq!(a.scope_mode, ScopeMode::Scoped);
        assert!(!a.branches_all && !a.workspaces_all);
        assert_eq!(a.branches, vec!["store-a", "store-b"]);
        assert_eq!(a.workspaces, vec!["retail-pos"]);
    }

    #[test]
    fn matches_scope_global_ignores_dimensions() {
        let a = Assignment {
            user_id: "u1".into(),
            role_id: "role-staff".into(),
            scope_mode: ScopeMode::Global,
            branches_all: true,
            branches: vec![],
            workspaces_all: true,
            workspaces: vec![],
        };
        assert!(a.matches_scope(None, None));
        assert!(a.matches_scope(Some("store-a"), Some("retail-pos")));
        assert!(a.matches_scope(Some("anything"), Some("anything-else")));
    }

    #[test]
    fn matches_scope_explicit_all_matches_any_context() {
        let a = Assignment {
            user_id: "u1".into(),
            role_id: "role-staff".into(),
            scope_mode: ScopeMode::Scoped,
            branches_all: true,
            branches: vec![],
            workspaces_all: true,
            workspaces: vec![],
        };
        assert!(a.matches_scope(None, None));
        assert!(a.matches_scope(Some("store-z"), Some("kds")));
    }

    #[test]
    fn matches_scope_branch_list_requires_branch_in_scope() {
        let a = Assignment {
            user_id: "u1".into(),
            role_id: "role-staff".into(),
            scope_mode: ScopeMode::Scoped,
            branches_all: false,
            branches: vec!["store-a".into()],
            workspaces_all: true,
            workspaces: vec![],
        };
        assert!(a.matches_scope(Some("store-a"), None));
        assert!(!a.matches_scope(Some("store-b"), None));
        // No branch context on a list dimension denies (fail closed).
        assert!(!a.matches_scope(None, None));
    }

    #[test]
    fn matches_scope_workspace_list_requires_workspace_in_scope() {
        let a = Assignment {
            user_id: "u1".into(),
            role_id: "role-staff".into(),
            scope_mode: ScopeMode::Scoped,
            branches_all: true,
            branches: vec![],
            workspaces_all: false,
            workspaces: vec!["retail-pos".into()],
        };
        assert!(a.matches_scope(None, Some("retail-pos")));
        assert!(!a.matches_scope(None, Some("kds")));
        assert!(!a.matches_scope(None, None));
    }

    #[test]
    fn matches_scope_both_lists_require_combination() {
        let a = Assignment {
            user_id: "u1".into(),
            role_id: "role-staff".into(),
            scope_mode: ScopeMode::Scoped,
            branches_all: false,
            branches: vec!["store-a".into()],
            workspaces_all: false,
            workspaces: vec!["retail-pos".into()],
        };
        assert!(a.matches_scope(Some("store-a"), Some("retail-pos")));
        assert!(!a.matches_scope(Some("store-a"), Some("kds")));
        assert!(!a.matches_scope(Some("store-b"), Some("retail-pos")));
        assert!(!a.matches_scope(Some("store-b"), Some("kds")));
    }

    #[test]
    fn matches_scope_empty_list_is_deny_not_all() {
        let a = Assignment {
            user_id: "u1".into(),
            role_id: "role-staff".into(),
            scope_mode: ScopeMode::Scoped,
            branches_all: false,
            branches: vec![],
            workspaces_all: true,
            workspaces: vec![],
        };
        // An empty list must never mean "all" — it denies everything.
        assert!(!a.matches_scope(Some("store-a"), None));
        assert!(!a.matches_scope(None, None));
    }

    #[test]
    fn scope_mode_parse_roundtrips_and_unknown_is_none() {
        assert_eq!(ScopeMode::parse("global"), Some(ScopeMode::Global));
        assert_eq!(ScopeMode::parse("scoped"), Some(ScopeMode::Scoped));
        assert_eq!(ScopeMode::parse("bogus"), None);
        assert_eq!(ScopeMode::parse(""), None);
        assert_eq!(ScopeMode::Global.as_str(), "global");
        assert_eq!(ScopeMode::Scoped.as_str(), "scoped");
    }
}
