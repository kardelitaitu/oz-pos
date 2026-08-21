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

use rusqlite::{Connection, OptionalExtension, params};

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

/// A scoped assignment write (ADR #35 D5 / spec 0048): the scope mode plus
/// the per-dimension explicit-all flag and list. Empty lists never mean
/// "all" — the `*_all` flags are the explicit marker, so `list` with no
/// rows is a deny, not an implicit "all".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentSpec {
    /// `global` or `scoped`.
    pub scope_mode: ScopeMode,
    /// Branch dimension is explicit `all`.
    pub branches_all: bool,
    /// Branch ids in scope when `branches_all` is false.
    pub branches: Vec<String>,
    /// Workspace dimension is explicit `all`.
    pub workspaces_all: bool,
    /// Workspace keys in scope when `workspaces_all` is false.
    pub workspaces: Vec<String>,
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

    /// Write a user's assignment scope (ADR #35 D5 / spec 0048) inside an
    /// open transaction: upserts the `assignments` row and replaces the
    /// dimension rows. Safe to call inside an existing transaction — the
    /// statements join it (no nested BEGIN). Standalone callers should use
    /// [`Store::set_assignment`], which wraps this in one transaction.
    pub fn write_assignment_scope(
        &self,
        user_id: &str,
        role_id: &str,
        spec: &AssignmentSpec,
    ) -> Result<(), CoreError> {
        Self::write_assignment_scope_on(self.conn, user_id, role_id, spec)
    }

    /// Write a user's single effective assignment (ADR #35 D5 / spec 0048),
    /// atomic in its own transaction.
    ///
    /// Upserts the `assignments` row and replaces the scoped dimension rows
    /// to match: `branches_all` / `workspaces_all` set that dimension's `all`
    /// flag and clear its rows, so toggling `list` → `all` never leaves stale
    /// grants (and a `list` dimension re-inserts exactly the given ids — an
    /// empty list is a deny, never an implicit "all"). The `role_id` is kept
    /// in sync with `users.role_id` by `update_user` / `create_user`; this
    /// write preserves it and only replaces the scope.
    pub fn set_assignment(
        &self,
        user_id: &str,
        role_id: &str,
        spec: &AssignmentSpec,
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        Self::write_assignment_scope_on(&tx, user_id, role_id, spec)?;
        tx.commit()?;
        Ok(())
    }

    /// The upsert + dimension-replacement statements, runnable on any
    /// connection (joins an open transaction when one exists).
    fn write_assignment_scope_on(
        conn: &Connection,
        user_id: &str,
        role_id: &str,
        spec: &AssignmentSpec,
    ) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let branch_scope = if spec.branches_all { "all" } else { "list" };
        let workspace_scope = if spec.workspaces_all { "all" } else { "list" };

        conn.execute(
            "INSERT INTO assignments
                 (user_id, role_id, scope_mode, branch_scope, workspace_scope, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(user_id) DO UPDATE SET
                 role_id = excluded.role_id,
                 scope_mode = excluded.scope_mode,
                 branch_scope = excluded.branch_scope,
                 workspace_scope = excluded.workspace_scope,
                 updated_at = excluded.updated_at",
            params![user_id, role_id, spec.scope_mode.as_str(), branch_scope, workspace_scope, now],
        )?;

        // Replace the branch dimension rows: a stale row must never survive a
        // scope change, and `all` always means every branch (ADR #35 D5).
        conn.execute(
            "DELETE FROM assignment_branches WHERE assignment_user_id = ?1",
            params![user_id],
        )?;
        for branch in &spec.branches {
            conn.execute(
                "INSERT INTO assignment_branches (assignment_user_id, branch_id) VALUES (?1, ?2)",
                params![user_id, branch],
            )?;
        }

        // Same replacement semantics for the workspace dimension.
        conn.execute(
            "DELETE FROM assignment_workspaces WHERE assignment_user_id = ?1",
            params![user_id],
        )?;
        for workspace in &spec.workspaces {
            conn.execute(
                "INSERT INTO assignment_workspaces (assignment_user_id, workspace_key) VALUES (?1, ?2)",
                params![user_id, workspace],
            )?;
        }

        Ok(())
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
#[path = "assignments_tests.rs"]
mod tests;
