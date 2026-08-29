//! Workspace CRUD — workspace types, instances, navigation screens,
//! per-user instance assignments, role-to-type access, and session resolution.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice B: workspaces deep read)
crate: oz-core | status: SAFE | lint: CLEAN
findings: ADR #4 resolution chain well built (role-owner bypass -> explicit user instances -> role types), quota enforcement checks the signed entitlement's allowed_types (C3.2), no-nesting caveat documented per RUST-08 with a pinned test; dynamic SQL interpolates only internal param markers (injection-safe); COR-30 LOW: four .unwrap_or(false) sites on access-resolution queries (385/395/1169/1188) fail toward the MORE PERMISSIVE tier on DB error — access guards should propagate or deny (same family as COR-11/25, rare under single-connection mutex); hardcoded role-id allowlist (8 variants) is fragile if presets change
next: propagate access-check errors or fail closed (COR-30) | perf: indexed resolution queries
*/
//!
//! ADR #4 Phase 1: Type/Instance Separation
//!
//! A user's effective workspace set is resolved as:
//! 1. `role-owner` with empty `user_store_access` → all instances in store
//! 2. `user_workspace_instances` rows exist → return ONLY those instances
//! 3. Otherwise → fall back to `role_workspace_types` → instances of allowed types

use rusqlite::params;
use serde::Serialize;

use crate::error::CoreError;
use crate::subscription::{QuotaError, SubscriptionTier, TenantSubscription};

use super::Store;

// ── Legacy DTOs (backward compatible) ────────────────────────────────────

/// DTO for a workspace type row — matches the old `workspaces` table.
/// Kept for backward compatibility during Phase 1 transition.
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

// ── New DTOs (ADR #4) ────────────────────────────────────────────────────

/// DTO for a workspace type (UI template).
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceTypeRow {
    /// Unique key — 'restaurant-pos', 'store-pos', 'kds', 'inventory', 'admin'.
    pub key: String,
    /// Human-readable display name.
    pub name: String,
    /// Short description.
    pub description: String,
    /// Layout hint — 'fullscreen' or 'sidebar'.
    pub layout_mode: String,
    /// Icon identifier.
    pub icon: String,
    /// Display order.
    pub sort_order: i32,
    /// Default accent colour (overridable per instance).
    pub accent_colour: String,
}

/// DTO for a workspace instance row.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInstanceRow {
    /// Instance ID — 'default-restaurant-pos', 'ws-dt-cashier-1', etc.
    pub id: String,
    /// FK to workspace_types.key.
    pub type_key: String,
    /// The store this instance belongs to.
    pub store_id: String,
    /// Display name — 'Downtown - Cashier 1'.
    pub name: String,
    /// Description.
    pub description: String,
    /// Optional per-instance accent colour override.
    pub colour: Option<String>,
    /// Controlled business purpose independent from type and display label.
    pub purpose_key: String,
    /// Instance status — 'active', 'quota_suspended', 'archived'.
    pub status: String,
    /// ISO timestamp.
    pub created_at: String,
    /// ISO timestamp.
    pub updated_at: String,
}

/// Comprehensive workspace DTO sent to the frontend.
///
/// Contains the full resolution chain: store → instance → type.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceDto {
    /// Instance ID.
    pub instance_id: String,
    /// Workspace type key (determines UI component).
    pub type_key: String,
    /// Store ID for data scoping.
    pub store_id: String,
    /// Store display name (from store_profiles).
    pub store_name: String,
    /// Controlled business purpose, independent from type, label, and access policy.
    pub purpose_key: String,
    /// Instance display name.
    pub name: String,
    /// Description (from the type).
    pub description: String,
    /// Icon identifier (from the type).
    pub icon: String,
    /// Layout hint — 'fullscreen' or 'sidebar'.
    pub layout_mode: String,
    /// Accent colour (instance override or type default).
    pub colour: Option<String>,
    /// Whether this is the user's default instance.
    pub is_default: bool,
}

/// Input parameters for [`Store::create_workspace_instance_with_purpose`].
///
/// Bundled into a struct so the creator's signature stays under clippy's
/// `too_many_arguments` threshold as it grows.
#[derive(Debug, Clone)]
pub struct CreateWorkspaceInstanceArgs {
    /// Unique workspace instance id.
    pub id: String,
    /// Technical instance type key (e.g. `store-pos`).
    pub type_key: String,
    /// Owning store profile id.
    pub store_id: String,
    /// Human-readable instance name.
    pub name: String,
    /// Optional free-text description.
    pub description: String,
    /// Optional accent colour.
    pub colour: Option<String>,
    /// Controlled business purpose key, independent from the technical
    /// type and the label (`general` is the neutral default).
    pub purpose_key: String,
}

// ── Legacy Queries (backward compatible) ────────────────────────────────

impl Store<'_> {
    /// List all workspace types (the old `list_all_workspaces`).
    /// Maps old `workspaces` table rows to `WorkspaceRow`.
    pub fn list_all_workspace_types(&self) -> Result<Vec<WorkspaceRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT key, name, description, icon
             FROM workspaces
             ORDER BY name",
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

    /// Legacy: list workspaces from old tables.
    /// Resolution order:
    /// 1. `role-owner` → all workspaces
    /// 2. If `user_id` is provided and `user_workspaces` has rows
    ///    → return ONLY those workspaces (replace mode)
    /// 3. Otherwise → fall back to `role_workspaces`
    pub fn list_workspaces_legacy(
        &self,
        role_id: &str,
        user_id: Option<&str>,
    ) -> Result<Vec<WorkspaceRow>, CoreError> {
        if role_id == "role-owner"
            || role_id == "role-admin"
            || role_id == "admin"
            || role_id == "role-manager"
            || role_id == "role-staff"
            || role_id == "role-auditor"
            || role_id == "manager"
            || role_id == "auditor"
        {
            return self.list_all_workspace_types();
        }

        if let Some(uid) = user_id {
            let user_keys: Vec<String> = self
                .conn
                .prepare("SELECT ws_key FROM user_workspaces WHERE user_id = ?1")?
                .query_map(params![uid], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();

            if !user_keys.is_empty() {
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

        self.list_role_workspaces_legacy(role_id)
    }

    fn list_role_workspaces_legacy(&self, role_id: &str) -> Result<Vec<WorkspaceRow>, CoreError> {
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

    /// Legacy: get screens for a workspace key (old table).
    pub fn list_workspace_screens_legacy(
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

    // Retired (0048 follow-up): the `set_user_workspaces_legacy` /
    // `get_user_workspace_keys_legacy` write path is gone — the assignment
    // model (ADR #35 D5 / spec 0048) supersedes it, and staff CRUD writes
    // `assignments` + dimension rows in the global identity DB. The old
    // `user_workspaces` table is still READ by the legacy listing above and
    // kept for data compatibility; nothing writes it anymore.
}

// ── New Type Queries (ADR #4) ────────────────────────────────────────────

impl Store<'_> {
    /// List all workspace types from the `workspace_types` table.
    pub fn list_workspace_types(&self) -> Result<Vec<WorkspaceTypeRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT key, name, description, layout_mode, icon, sort_order, accent_colour
             FROM workspace_types
             ORDER BY sort_order",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WorkspaceTypeRow {
                key: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                layout_mode: row.get(3)?,
                icon: row.get(4)?,
                sort_order: row.get(5)?,
                accent_colour: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Get screens for a workspace type.
    pub fn list_workspace_type_screens(
        &self,
        type_key: &str,
    ) -> Result<Vec<WorkspaceScreenRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT screen_key, sort_order
             FROM workspace_type_screens
             WHERE type_key = ?1
             ORDER BY sort_order",
        )?;
        let rows = stmt.query_map([type_key], |row| {
            Ok(WorkspaceScreenRow {
                screen_key: row.get(0)?,
                sort_order: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }
}

// ── New Instance Queries (ADR #4) ────────────────────────────────────────

impl Store<'_> {
    /// List workspace instances accessible to a given role and user
    /// within a specific store.
    ///
    /// Resolution order (ADR #4 Phase 1):
    /// 1. `role-owner` → all active instances in this store
    /// 2. If `user_id` has `user_workspace_instances` rows → only those
    /// 3. Otherwise → fall back to `role_workspace_types` → instances of
    ///    allowed types in this store
    ///
    /// When `tier` is provided (ADR #5), results are additionally filtered
    /// to only include instances whose `type_key` is allowed by the
    /// subscription tier.
    pub fn list_workspaces(
        &self,
        role_id: &str,
        user_id: Option<&str>,
        store_id: &str,
    ) -> Result<Vec<WorkspaceDto>, CoreError> {
        let results = self.list_workspaces_inner(role_id, user_id, store_id)?;
        Ok(results)
    }

    /// List workspace instances with subscription tier entitlement
    /// filtering (ADR #5).
    ///
    /// Same resolution as `list_workspaces` but additionally filters
    /// out instances whose `type_key` is not allowed by the subscription's
    /// entitlement — the signed payload's `allowed_types_json`, falling
    /// back to the tier's static defaults (C3.2: a Plus + restaurant_starter
    /// bundle lists `kds`, so a bundle subscriber sees its KDS workspace).
    pub fn list_workspaces_with_entitlement(
        &self,
        role_id: &str,
        user_id: Option<&str>,
        store_id: &str,
        sub: &TenantSubscription,
    ) -> Result<Vec<WorkspaceDto>, CoreError> {
        let mut results = self.list_workspaces_inner(role_id, user_id, store_id)?;
        results.retain(|dto| sub.allows_workspace_type(&dto.type_key));
        Ok(results)
    }

    /// Inner resolution without entitlement filtering.
    fn list_workspaces_inner(
        &self,
        role_id: &str,
        user_id: Option<&str>,
        store_id: &str,
    ) -> Result<Vec<WorkspaceDto>, CoreError> {
        // 1. Owner bypass — all active instances in store.
        //
        // ADR #4 Phase 2: If the user has explicit `user_store_access` rows,
        // even owner/admin roles are limited to their assigned stores.
        // This enables multi-store mode where an owner may only manage a
        // subset of stores. When no `user_store_access` rows exist, the
        // legacy single-store bypass applies unchanged.
        if role_id == "role-owner"
            || role_id == "role-admin"
            || role_id == "admin"
            || role_id == "role-manager"
            || role_id == "role-staff"
            || role_id == "role-auditor"
            || role_id == "manager"
            || role_id == "auditor"
        {
            // Phase 2: check user_store_access for multi-store enforcement.
            if let Some(uid) = user_id {
                let has_store_access_rows: bool = self
                    .conn
                    .query_row(
                        "SELECT COUNT(*) > 0 FROM user_store_access WHERE user_id = ?1",
                        params![uid],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);

                if has_store_access_rows {
                    let store_accessible: bool = self
                        .conn
                        .query_row(
                            "SELECT COUNT(*) > 0 FROM user_store_access WHERE user_id = ?1 AND store_id = ?2",
                            params![uid, store_id],
                            |row| row.get(0),
                        )
                        .unwrap_or(false);

                    if !store_accessible {
                        return Ok(vec![]); // User has no access to this store
                    }
                }
            }

            return self.list_store_instances(store_id, user_id);
        }

        // 2. Check for explicit user-level instance assignment.
        if let Some(uid) = user_id {
            let instance_ids: Vec<String> = self
                .conn
                .prepare(
                    "SELECT instance_id
                     FROM user_workspace_instances
                     WHERE user_id = ?1",
                )?
                .query_map(params![uid], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();

            if !instance_ids.is_empty() {
                return self.list_instances_by_ids(&instance_ids, store_id, uid);
            }
        }

        // 3. Fall back to role-based type access.
        self.list_store_instances_by_role(role_id, store_id, user_id)
    }

    /// Build the base SELECT/FROM/JOIN for workspace instance DTO queries.
    ///
    /// The returned SQL includes a `LEFT JOIN user_workspace_instances uwi`
    /// with `uwi.user_id = {user_id_param}` — the caller provides the
    /// correct parameter placeholder (e.g. `"?1"`, `"?2"`) based on where
    /// the user ID sits in their parameter array.
    fn instance_dto_sql(user_id_param: &str) -> String {
        format!(
            "SELECT wi.id, wi.type_key, wi.store_id,
                    COALESCE(sp.name, wi.store_id) AS store_name,
                    wi.purpose_key,
                    wi.name, wt.description, wt.icon, wt.layout_mode,
                    COALESCE(wi.colour, wt.accent_colour) AS colour,
                    COALESCE(uwi.is_default, 0) AS is_default
             FROM workspace_instances wi
             JOIN workspace_types wt ON wi.type_key = wt.key
             LEFT JOIN store_profiles sp ON wi.store_id = sp.id
             LEFT JOIN user_workspace_instances uwi
               ON uwi.instance_id = wi.id AND uwi.user_id = {user_id_param}"
        )
    }

    /// Map a row to a WorkspaceDto.
    fn map_instance_dto(row: &rusqlite::Row) -> rusqlite::Result<WorkspaceDto> {
        Ok(WorkspaceDto {
            instance_id: row.get(0)?,
            type_key: row.get(1)?,
            store_id: row.get(2)?,
            store_name: row.get(3)?,
            purpose_key: row.get(4)?,
            name: row.get(5)?,
            description: row.get(6)?,
            icon: row.get(7)?,
            layout_mode: row.get(8)?,
            colour: row.get(9)?,
            is_default: row.get::<_, i32>(10)? != 0,
        })
    }

    /// Get all active instances in a store.
    fn list_store_instances(
        &self,
        store_id: &str,
        user_id: Option<&str>,
    ) -> Result<Vec<WorkspaceDto>, CoreError> {
        let uid = user_id.unwrap_or("");
        let sql = format!(
            "{} WHERE wi.store_id = ?1 AND wi.status = 'active' ORDER BY wt.sort_order, wi.name",
            Self::instance_dto_sql("?2")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![store_id, uid], Self::map_instance_dto)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Get specific instances by IDs, scoped to a store.
    fn list_instances_by_ids(
        &self,
        instance_ids: &[String],
        store_id: &str,
        user_id: &str,
    ) -> Result<Vec<WorkspaceDto>, CoreError> {
        let placeholders: Vec<String> = instance_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 3))
            .collect();
        // Params: ?1 = user_id, ?2 = store_id, ?3.. = instance_ids
        let sql = format!(
            "{} WHERE wi.id IN ({}) AND wi.store_id = ?2 AND wi.status = 'active' ORDER BY wt.sort_order, wi.name",
            Self::instance_dto_sql("?1"),
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(user_id.to_string()));
        param_values.push(Box::new(store_id.to_string()));
        for id in instance_ids {
            param_values.push(Box::new(id.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::map_instance_dto)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Get instances via role_workspace_types for a given store.
    fn list_store_instances_by_role(
        &self,
        role_id: &str,
        store_id: &str,
        user_id: Option<&str>,
    ) -> Result<Vec<WorkspaceDto>, CoreError> {
        let uid = user_id.unwrap_or("");
        let sql = format!(
            "{} JOIN role_workspace_types rwt ON wt.key = rwt.type_key
             WHERE wi.store_id = ?1 AND rwt.role_id = ?2 AND wi.status = 'active'
             ORDER BY wt.sort_order, wi.name",
            Self::instance_dto_sql("?3")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![store_id, role_id, uid], Self::map_instance_dto)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Get a single workspace instance by ID.
    ///
    /// When `user_id` is provided, `is_default` is computed from
    /// `user_workspace_instances`. Otherwise it is always `false`.
    pub fn get_workspace_instance(
        &self,
        instance_id: &str,
        user_id: Option<&str>,
    ) -> Result<WorkspaceDto, CoreError> {
        let uid = user_id.unwrap_or("");
        let mut stmt = self.conn.prepare(
            "SELECT wi.id, wi.type_key, wi.store_id,
                    COALESCE(sp.name, wi.store_id) AS store_name,
                    wi.purpose_key,
                    wi.name, wt.description, wt.icon, wt.layout_mode,
                    COALESCE(wi.colour, wt.accent_colour) AS colour,
                    COALESCE((SELECT is_default FROM user_workspace_instances
                              WHERE user_id = ?2 AND instance_id = wi.id), 0) AS is_default
             FROM workspace_instances wi
             JOIN workspace_types wt ON wi.type_key = wt.key
             LEFT JOIN store_profiles sp ON wi.store_id = sp.id
             WHERE wi.id = ?1
               AND wi.status = 'active'",
        )?;
        stmt.query_row(params![instance_id, uid], |row| {
            Ok(WorkspaceDto {
                instance_id: row.get(0)?,
                type_key: row.get(1)?,
                store_id: row.get(2)?,
                store_name: row.get(3)?,
                purpose_key: row.get(4)?,
                name: row.get(5)?,
                description: row.get(6)?,
                icon: row.get(7)?,
                layout_mode: row.get(8)?,
                colour: row.get(9)?,
                is_default: row.get::<_, i32>(10)? != 0,
            })
        })
        .map_err(CoreError::from)
    }

    /// Count active (non-archived, non-suspended) workspace instances
    /// in the given store.
    pub fn count_active_instances(&self, store_id: &str) -> Result<i64, CoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM workspace_instances
             WHERE store_id = ?1 AND status NOT IN ('archived', 'quota_suspended')",
            params![store_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Enforce subscription quota before creating a workspace instance.
    ///
    /// Checks:
    /// 1. Subscription entitlement allows this workspace type — the signed
    ///    payload's `allowed_types_json` (C3.2: a Plus + restaurant_starter
    ///    bundle lists `kds`), falling back to the tier's static defaults
    /// 2. Per-store register count is within the effective tier's limit
    ///
    /// Called by Tauri commands before delegating to `create_workspace_instance`.
    pub fn enforce_instance_quota(
        &self,
        sub: &TenantSubscription,
        type_key: &str,
        store_id: &str,
    ) -> Result<(), CoreError> {
        let effective = sub.effective_tier();
        // 1. Workspace type must be allowed by this subscription's
        //    entitlement (the signed payload's quota block, not just the
        //    static tier defaults).
        if !sub.allows_workspace_type(type_key) {
            return Err(QuotaError::TypeNotAllowed {
                tier: effective.name().into(),
                type_key: type_key.into(),
            }
            .into());
        }

        // 2. Per-store register limit from the effective tier.
        if let Some(limit) = effective.max_pos_instances() {
            let current = self.count_active_instances(store_id)?;
            if current >= limit {
                return Err(QuotaError::RegisterLimit {
                    tier: effective.name().into(),
                    limit,
                    current,
                }
                .into());
            }
        }

        Ok(())
    }

    /// Create a new workspace instance.
    ///
    /// Returns `CoreError::Conflict` if an instance with the given
    /// ID already exists.
    ///
    /// **Note:** Callers must verify subscription quota via
    /// `enforce_instance_quota()` before calling this method.
    ///
    /// # Nesting caveat
    ///
    /// This method opens its own transaction via `unchecked_transaction`,
    /// which issues a raw `BEGIN`. SQLite rejects `BEGIN` when a
    /// transaction is already open ("cannot start a transaction within a
    /// transaction"), so this method **cannot be called from inside an
    /// open transaction**. Callers that need to batch multiple mutations
    /// atomically must run the INSERT SQL directly on their outer
    /// transaction (see `apply_topology_diff` in desktop-client, which
    /// does exactly this rather than delegating here). See the
    /// `create_workspace_instance_cannot_nest_in_open_transaction` test.
    pub fn create_workspace_instance(
        &self,
        id: &str,
        type_key: &str,
        store_id: &str,
        name: &str,
        description: &str,
        colour: Option<&str>,
    ) -> Result<WorkspaceInstanceRow, CoreError> {
        self.create_workspace_instance_with_purpose(CreateWorkspaceInstanceArgs {
            id: id.to_string(),
            type_key: type_key.to_string(),
            store_id: store_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            colour: colour.map(str::to_string),
            purpose_key: "general".to_string(),
        })
    }

    /// Create a workspace instance with an explicit controlled business purpose.
    ///
    /// `purpose_key` is independent from the technical `type_key`, editable
    /// instance name, and authorization assignments. The legacy creator above
    /// delegates to this method with the neutral `general` purpose.
    pub fn create_workspace_instance_with_purpose(
        &self,
        args: CreateWorkspaceInstanceArgs,
    ) -> Result<WorkspaceInstanceRow, CoreError> {
        let CreateWorkspaceInstanceArgs {
            id,
            type_key,
            store_id,
            name,
            description,
            colour,
            purpose_key,
        } = args;

        if id.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "id",
                message: "workspace instance id must not be empty".into(),
            });
        }
        if type_key.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "type_key",
                message: "type_key must not be empty".into(),
            });
        }
        if store_id.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "store_id",
                message: "store_id must not be empty".into(),
            });
        }
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "workspace instance name must not be empty".into(),
            });
        }
        if purpose_key.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "purpose_key",
                message: "workspace instance purpose_key must not be empty".into(),
            });
        }

        let tx = self.conn.unchecked_transaction()?;

        let exists: bool = tx
            .query_row(
                "SELECT COUNT(*) > 0 FROM workspace_instances WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Err(CoreError::Conflict {
                entity: "workspace instance",
                field: "id",
            });
        }

        tx.execute(
            "INSERT INTO workspace_instances (id, type_key, store_id, name, description, colour, purpose_key, status, last_accessed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![id, type_key, store_id, name, description, colour, purpose_key],
        )?;

        tx.commit()?;

        let row: WorkspaceInstanceRow = self.conn.query_row(
            "SELECT id, type_key, store_id, name, description, colour, purpose_key, status, created_at, updated_at
             FROM workspace_instances WHERE id = ?1",
            params![id],
            |row| {
                Ok(WorkspaceInstanceRow {
                    id: row.get(0)?,
                    type_key: row.get(1)?,
                    store_id: row.get(2)?,
                    name: row.get(3)?,
                    description: row.get(4)?,
                    colour: row.get(5)?,
                    purpose_key: row.get(6)?,
                    status: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )?;

        Ok(row)
    }

    /// Touch `last_accessed_at` for a workspace instance (ADR #5).
    ///
    /// Called during session resolution to track most-recently-used
    /// ordering for automatic quota recovery.
    pub fn touch_instance_access(&self, instance_id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE workspace_instances
             SET last_accessed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![instance_id],
        )?;
        Ok(())
    }

    /// Restore `QuotaSuspended` instances to `Active` up to the tier's
    /// per-store limit (ADR #5 Phase 3b).
    ///
    /// Called when a tier is upgraded — the new tier allows more
    /// registers per store. Instances are restored in most-recently-used
    /// order (`last_accessed_at DESC`). Already-`Active` instances count
    /// toward the limit. Returns the count of restored instances.
    ///
    /// Wrapped in a transaction to prevent race conditions between the
    /// SELECT count and UPDATE.
    pub fn auto_recover_instances(
        &self,
        store_id: &str,
        tier: &SubscriptionTier,
    ) -> Result<usize, CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        let limit = match tier.max_pos_instances() {
            Some(n) => n,
            None => {
                // Unlimited — restore ALL QuotaSuspended instances.
                let updated = tx.execute(
                    "UPDATE workspace_instances
                     SET status = 'active',
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE store_id = ?1 AND status = 'quota_suspended'",
                    params![store_id],
                )?;
                tx.commit()?;
                if updated > 0 {
                    tracing::info!(
                        store_id = %store_id,
                        restored = %updated,
                        "unlimited tier — all suspended instances restored"
                    );
                }
                return Ok(updated);
            }
        };

        // Count already-active instances (they count toward the limit).
        let active_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM workspace_instances
             WHERE store_id = ?1 AND status = 'active'",
            params![store_id],
            |row| row.get(0),
        )?;

        let slots_available = (limit - active_count).max(0);
        if slots_available == 0 {
            tx.commit()?;
            return Ok(0);
        }

        // Restore the most-recently-used suspended instances.
        let updated = tx.execute(
            "UPDATE workspace_instances
             SET status = 'active',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id IN (
                 SELECT id FROM workspace_instances
                 WHERE store_id = ?1 AND status = 'quota_suspended'
                 ORDER BY last_accessed_at DESC
                 LIMIT ?2
             )",
            params![store_id, slots_available],
        )?;

        tx.commit()?;

        if updated > 0 {
            tracing::info!(
                store_id = %store_id,
                restored = %updated,
                active = %active_count,
                limit = %limit,
                "suspended instances restored after tier upgrade"
            );
        }

        Ok(updated)
    }

    /// Suspend surplus `Active` instances when a tier is downgraded
    /// (ADR #5 Phase 3c).
    ///
    /// If the store has more active instances than the tier allows,
    /// the least-recently-used instances are transitioned to
    /// `QuotaSuspended`. Returns the count of suspended instances.
    ///
    /// Wrapped in a transaction to prevent race conditions between the
    /// SELECT count and UPDATE.
    pub fn suspend_surplus_instances(
        &self,
        store_id: &str,
        tier: &SubscriptionTier,
    ) -> Result<usize, CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        let limit = match tier.max_pos_instances() {
            Some(n) => n,
            None => {
                tx.commit()?;
                return Ok(0); // Unlimited — nothing to suspend
            }
        };

        let active_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM workspace_instances
             WHERE store_id = ?1 AND status = 'active'",
            params![store_id],
            |row| row.get(0),
        )?;

        let surplus = (active_count - limit).max(0);
        if surplus == 0 {
            tx.commit()?;
            return Ok(0);
        }

        // Suspend the least-recently-used active instances.
        let updated = tx.execute(
            "UPDATE workspace_instances
             SET status = 'quota_suspended',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id IN (
                 SELECT id FROM workspace_instances
                 WHERE store_id = ?1 AND status = 'active'
                 ORDER BY last_accessed_at ASC
                 LIMIT ?2
             )",
            params![store_id, surplus],
        )?;

        tx.commit()?;

        if updated > 0 {
            tracing::info!(
                store_id = %store_id,
                suspended = %updated,
                active_before = %active_count,
                limit = %limit,
                "surplus instances suspended after tier downgrade"
            );
        }

        Ok(updated)
    }

    /// Archive a workspace instance by setting its status to `'archived'`.
    ///
    /// Archived instances are excluded from `list_workspaces` and do not
    /// count toward the active instance quota. Returns
    /// `CoreError::NotFound` if the instance does not exist.
    pub fn archive_instance(&self, instance_id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE workspace_instances
             SET status = 'archived',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![instance_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "workspace instance",
                id: instance_id.to_owned(),
            });
        }
        Ok(())
    }

    /// Update the editable fields of a workspace instance.
    ///
    /// Performs a partial update: `name` is always set, while `description`
    /// and `colour` are only updated when `Some(..)` is supplied. Passing
    /// `None` leaves the existing column value unchanged (COALESCE), so a
    /// caller that only knows the new name will not clobber other fields.
    /// The instance `type_key` and `store_id` are immutable and cannot be
    /// changed here. Returns [`CoreError::NotFound`] when no instance with
    /// the given `id` exists.
    ///
    /// Note: to intentionally clear a colour, callers must go through a
    /// dedicated path — `None` here is "leave as-is", not "clear".
    pub fn update_workspace_instance(
        &self,
        instance_id: &str,
        name: &str,
        description: Option<&str>,
        colour: Option<&str>,
    ) -> Result<(), CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "workspace instance name must not be empty".into(),
            });
        }

        let affected = self.conn.execute(
            "UPDATE workspace_instances
             SET name = ?2,
                 description = COALESCE(?3, description),
                 colour = COALESCE(?4, colour),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![instance_id, name, description, colour],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "workspace instance",
                id: instance_id.to_owned(),
            });
        }
        Ok(())
    }

    /// List all workspace instances in a store (admin use, no access control).
    pub fn list_all_instances(
        &self,
        store_id: &str,
    ) -> Result<Vec<WorkspaceInstanceRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type_key, store_id, name, description, colour, purpose_key, status, created_at, updated_at
             FROM workspace_instances
             WHERE store_id = ?1
             ORDER BY name",
        )?;
        let rows = stmt.query_map(params![store_id], |row| {
            Ok(WorkspaceInstanceRow {
                id: row.get(0)?,
                type_key: row.get(1)?,
                store_id: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                colour: row.get(5)?,
                purpose_key: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    // ── User-Instance Assignment ──────────────────────────────────────

    /// Replace all instance assignments for a user.
    ///
    /// Passing an empty `instance_ids` clears all assignments, causing
    /// the user to fall back to role-based type access.
    pub fn set_user_workspace_instances<'b>(
        &self,
        user_id: &str,
        instance_ids: impl IntoIterator<Item = &'b str>,
        default_instance_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "DELETE FROM user_workspace_instances WHERE user_id = ?1",
            params![user_id],
        )?;

        for id in instance_ids {
            let is_default = if Some(id) == default_instance_id {
                1
            } else {
                0
            };
            tx.execute(
                "INSERT OR IGNORE INTO user_workspace_instances
                 (user_id, instance_id, is_default)
                 VALUES (?1, ?2, ?3)",
                params![user_id, id, is_default],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get the explicit instance IDs assigned to a user.
    pub fn get_user_workspace_instance_ids(&self, user_id: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT instance_id
             FROM user_workspace_instances
             WHERE user_id = ?1
             ORDER BY instance_id",
        )?;
        let rows = stmt.query_map(params![user_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Verify that a user has access to a specific workspace instance.
    ///
    /// This is the server-side authorization gate `create_session` calls in
    /// both desktop and tablet clients (ADR #4 / ADR #7). It FAILS CLOSED:
    ///
    /// 0. The caller identity is bound to the database — the user must
    ///    exist, be active, and the claimed `role_id` must equal the user's
    ///    actual role. A claimed role is never trusted for the privilege
    ///    checks below; otherwise any caller who knew an owner's user id
    ///    could mint a session as that owner (privilege escalation) in any
    ///    store's active instance (cross-store session minting).
    /// 1. Owner/admin role keys — instance must exist and be active (with
    ///    `user_store_access` check for multi-store mode)
    /// 2. `user_workspace_instances` — direct assignment for this user
    /// 3. `role_workspace_types` — role grants access to the instance's type
    ///
    /// Returns `Ok(true)` if the user may create a session against this
    /// instance, `Ok(false)` if access is denied. Denials (unknown user,
    /// inactive user, forged role, missing instance) all return `Ok(false)`
    /// so the caller surfaces one uniform "no access" error without
    /// revealing which identity records exist.
    pub fn verify_instance_access(
        &self,
        claimed_role_id: &str,
        user_id: &str,
        instance_id: &str,
        store_id: &str,
    ) -> Result<bool, CoreError> {
        // 0. Bind the caller identity to the database. Every later branch
        // uses the REAL role resolved from `users`, never the claim.
        let Some(user) = self.get_user(user_id)? else {
            return Ok(false); // unknown identity — fail closed
        };
        if !user.is_active {
            return Ok(false); // deactivated account — fail closed
        }
        if user.role_id != claimed_role_id {
            return Ok(false); // forged role claim — fail closed
        }
        let role_id = &user.role_id;

        // 1. Owner/admin bypass — check store access if user_store_access is active.
        if role_id == "role-owner"
            || role_id == "role-admin"
            || role_id == "admin"
            || role_id == "role-manager"
            || role_id == "role-staff"
            || role_id == "role-auditor"
            || role_id == "manager"
            || role_id == "auditor"
        {
            // Check if user has explicit store access rows (multi-store mode, ADR #4 Phase 2).
            let has_store_access: bool = self
                .conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM user_store_access WHERE user_id = ?1",
                    params![user_id],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if has_store_access {
                // Multi-store mode: user must have access to this specific store.
                let store_accessible: bool = self
                    .conn
                    .query_row(
                        "SELECT COUNT(*) > 0 FROM user_store_access WHERE user_id = ?1 AND store_id = ?2",
                        params![user_id, store_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                if !store_accessible {
                    return Ok(false);
                }
            }

            // Instance must exist and be active in this store.
            let exists: bool = self
                .conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM workspace_instances WHERE id = ?1 AND store_id = ?2 AND status = 'active'",
                    params![instance_id, store_id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            return Ok(exists);
        }

        // 2. Check for explicit user-level instance assignment.
        let has_explicit: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM user_workspace_instances WHERE user_id = ?1 AND instance_id = ?2",
                params![user_id, instance_id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if has_explicit {
            return Ok(true);
        }

        // 3. Fall back to role-based type access.
        let has_role_access: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM workspace_instances wi
                 JOIN role_workspace_types rwt ON wi.type_key = rwt.type_key
                 WHERE wi.id = ?1
                   AND wi.store_id = ?2
                   AND wi.status = 'active'
                   AND rwt.role_id = ?3",
                params![instance_id, store_id, role_id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        Ok(has_role_access)
    }
}

#[cfg(test)]
#[path = "workspaces_tests.rs"]
mod tests;
