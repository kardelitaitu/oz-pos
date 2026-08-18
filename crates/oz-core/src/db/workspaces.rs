//! Workspace CRUD — workspace types, instances, navigation screens,
//! per-user instance assignments, role-to-type access, and session resolution.
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

    /// A subscription with the EMPTY quota block — workspace-type
    /// entitlement falls back to the tier's static defaults, which is what
    /// the pre-bundle entitlement tests exercised via a bare `SubscriptionTier`.
    fn sub_for_tier(tier: SubscriptionTier) -> TenantSubscription {
        TenantSubscription {
            tenant_id: "default".into(),
            tier,
            status: "active".into(),
            expires_at: None,
            max_stores: 1,
            max_pos_instances: 1,
            allowed_types_json: "[]".into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        }
    }

    /// A Plus + restaurant_starter bundle subscription — the signed payload's
    /// `allowed_types` lists `kds` even though the Plus TIER statically
    /// excludes it (C3.2).
    fn plus_bundle_sub() -> TenantSubscription {
        TenantSubscription {
            tenant_id: "default".into(),
            tier: SubscriptionTier::Plus,
            status: "active".into(),
            expires_at: None,
            max_stores: 1,
            max_pos_instances: 2,
            allowed_types_json:
                r#"["store-pos","restaurant-pos","admin","warehouse","inventory","kds"]"#.into(),
            signature: "BOOTSTRAP_FREE".into(),
            signed_payload: String::new(),
            api_key: String::new(),
            updated_at: String::new(),
        }
    }

    // ── Legacy tests (backward compatible) ────────────────────────────

    #[test]
    fn list_all_workspace_types_returns_seeded() {
        let (store, _) = fresh();
        let ws = store.list_all_workspace_types().unwrap();
        assert_eq!(ws.len(), 6);
        assert!(ws.iter().any(|w| w.key == "restaurant-pos"));
        assert!(ws.iter().any(|w| w.key == "kds"));
        assert!(ws.iter().any(|w| w.key == "store-pos"));
        // ADR-18 §3 + §13 finding 37 (migration 091): workspace_types.key
        // rename cascade renames 'inventory' -> 'warehouse' across all FK
        // references including the legacy `workspaces` table. This fixture
        // asserts the post-rename state — the user-facing workspace type
        // for stock-keeping is 'warehouse', not 'inventory'.
        assert!(ws.iter().any(|w| w.key == "warehouse"));
        assert!(ws.iter().any(|w| w.key == "admin"));
        // ADR #35 D5 (migration 128): 'retail-pos' is the legacy cashier
        // workspace that role-cashier users fold into as Staff assignments.
        assert!(ws.iter().any(|w| w.key == "retail-pos"));
        let kds = ws.iter().find(|w| w.key == "kds").unwrap();
        assert_eq!(kds.name, "Kitchen Display");
        assert_eq!(kds.icon, "kds");
    }

    #[test]
    fn list_workspaces_legacy_owner_returns_all() {
        let (store, _) = fresh();
        let ws = store.list_workspaces_legacy("role-owner", None).unwrap();
        assert_eq!(ws.len(), 6);
    }

    #[test]
    fn list_workspaces_legacy_with_user_override() {
        let (store, user_id) = fresh();
        let before = store
            .list_workspaces_legacy("role-test", Some(&user_id))
            .unwrap();
        assert!(before.is_empty(), "role-test has no role_workspaces");

        // The user_workspaces write path is retired (assignment model
        // supersedes it); seed the legacy row directly to keep pinning the
        // legacy listing's replace-mode read.
        store
            .conn
            .execute(
                "INSERT INTO user_workspaces (user_id, ws_key) VALUES (?1, ?2)",
                params![user_id, "admin"],
            )
            .unwrap();
        let after = store
            .list_workspaces_legacy("role-test", Some(&user_id))
            .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].key, "admin");
    }

    // ── New tests (ADR #4 Phase 1) ────────────────────────────────────

    #[test]
    fn list_workspace_types_returns_all() {
        let (store, _) = fresh();
        let types = store.list_workspace_types().unwrap();
        assert_eq!(types.len(), 6);
        assert!(types.iter().any(|t| t.layout_mode == "fullscreen"));
        assert!(types.iter().any(|t| t.layout_mode == "sidebar"));
    }

    #[test]
    fn list_workspaces_owner_returns_instances_in_store() {
        let (store, _) = fresh();
        // Primary store has default instances seeded by migration.
        let dto = store
            .list_workspaces("role-owner", None, "default")
            .unwrap();
        assert_eq!(dto.len(), 5);
        assert!(dto.iter().any(|w| w.type_key == "kds"));
        assert!(dto.iter().any(|w| w.type_key == "restaurant-pos"));
        // All should have instance_id, store_id, etc.
        for w in &dto {
            assert!(!w.instance_id.is_empty());
            assert!(!w.store_id.is_empty());
            assert!(!w.name.is_empty());
            assert!(!w.layout_mode.is_empty());
        }
    }

    #[test]
    fn list_workspaces_auditor_returns_instances_in_store() {
        // Auditor is a global read-only role per the five-role taxonomy — it
        // must resolve the same workspace instances as the management roles
        // so it can reach its read-only screens (audit log, reports,
        // inventory) through the workspace picker.
        let (store, _) = fresh();
        let dto = store
            .list_workspaces("role-auditor", None, "default")
            .unwrap();
        assert_eq!(dto.len(), 5);
        assert!(dto.iter().any(|w| w.type_key == "kds"));
        assert!(dto.iter().any(|w| w.type_key == "restaurant-pos"));
        assert!(dto.iter().any(|w| w.type_key == "admin"));
    }

    #[test]
    fn get_workspace_instance_returns_correct_dto() {
        let (store, user_id) = fresh();
        let dto = store
            .get_workspace_instance("default-restaurant-pos", Some(&user_id))
            .unwrap();
        assert_eq!(dto.instance_id, "default-restaurant-pos");
        assert_eq!(dto.type_key, "restaurant-pos");
        assert_eq!(dto.store_id, "default");
        assert_eq!(dto.layout_mode, "fullscreen");
    }

    #[test]
    fn create_workspace_instance_basic() {
        let (store, _) = fresh();
        let row = store
            .create_workspace_instance(
                "test-cashier-1",
                "restaurant-pos",
                "default",
                "Test Cashier 1",
                "A test instance",
                Some("#FF0000"),
            )
            .unwrap();
        assert_eq!(row.id, "test-cashier-1");
        assert_eq!(row.type_key, "restaurant-pos");
        assert_eq!(row.colour, Some("#FF0000".into()));
        assert_eq!(row.status, "active");

        // Verify it appears in owner's list.
        let dto = store
            .list_workspaces("role-owner", None, "default")
            .unwrap();
        assert_eq!(dto.len(), 6);
        assert!(dto.iter().any(|w| w.instance_id == "test-cashier-1"));
    }

    #[test]
    fn purpose_key_is_independent_from_type_and_name() {
        let (store, _) = fresh();
        store
            .create_workspace_instance_with_purpose(CreateWorkspaceInstanceArgs {
                id: "ws-checkout".into(),
                type_key: "store-pos".into(),
                store_id: "default".into(),
                name: "Front Counter".into(),
                description: String::new(),
                colour: None,
                purpose_key: "checkout".into(),
            })
            .unwrap();
        store
            .create_workspace_instance_with_purpose(CreateWorkspaceInstanceArgs {
                id: "ws-returns".into(),
                type_key: "store-pos".into(),
                store_id: "default".into(),
                name: "Returns Counter".into(),
                description: String::new(),
                colour: None,
                purpose_key: "returns".into(),
            })
            .unwrap();

        let rows = store.list_all_instances("default").unwrap();
        let checkout = rows.iter().find(|row| row.id == "ws-checkout").unwrap();
        let returns = rows.iter().find(|row| row.id == "ws-returns").unwrap();
        assert_eq!(checkout.type_key, returns.type_key);
        assert_eq!(checkout.purpose_key, "checkout");
        assert_eq!(returns.purpose_key, "returns");
        assert_ne!(checkout.name, returns.name);
    }

    #[test]
    fn create_workspace_instance_duplicate_fails() {
        let (store, _) = fresh();
        let result = store.create_workspace_instance(
            "default-restaurant-pos",
            "restaurant-pos",
            "default",
            "Dup",
            "",
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn list_workspaces_with_user_override_instances() {
        let (store, user_id) = fresh();

        // No user override → falls back to role_workspace_types.
        let before = store
            .list_workspaces("role-test", Some(&user_id), "default")
            .unwrap();
        assert!(before.is_empty(), "role-test has no role_workspace_types");

        // Set explicit instances for user.
        store
            .set_user_workspace_instances(&user_id, ["default-admin"], Some("default-admin"))
            .unwrap();

        let after = store
            .list_workspaces("role-test", Some(&user_id), "default")
            .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].type_key, "admin");
        assert!(after[0].is_default);
    }

    #[test]
    fn set_user_workspace_instances_empty_clears() {
        let (store, user_id) = fresh();
        store
            .set_user_workspace_instances(&user_id, ["default-admin"], None)
            .unwrap();
        let ids = store.get_user_workspace_instance_ids(&user_id).unwrap();
        assert_eq!(ids.len(), 1);

        store
            .set_user_workspace_instances(&user_id, [], None)
            .unwrap();
        let ids = store.get_user_workspace_instance_ids(&user_id).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn list_workspaces_owner_without_store_access_sees_all() {
        let (store, _) = fresh();
        // role-owner with no user_store_access (Phase 1 single-store mode)
        let dto = store
            .list_workspaces("role-owner", None, "default")
            .unwrap();
        assert_eq!(dto.len(), 5);
    }

    #[test]
    fn list_all_instances_returns_all_in_store() {
        let (store, _) = fresh();
        let instances = store.list_all_instances("default").unwrap();
        assert_eq!(instances.len(), 5);
        assert!(instances.iter().any(|i| i.id == "default-kds"));
    }

    // ── Entitlement tests (ADR #5) ───────────────────────────────

    #[test]
    fn list_workspaces_with_entitlement_filters_by_tier() {
        let (store, _) = fresh();
        // Free tier only allows restaurant-pos, store-pos, admin
        let free = sub_for_tier(SubscriptionTier::Free);
        let dto = store
            .list_workspaces_with_entitlement("role-owner", None, "default", &free)
            .unwrap();
        // KDS and inventory should be filtered out
        assert!(
            dto.iter()
                .all(|w| SubscriptionTier::Free.allows_workspace_type(&w.type_key))
        );
        assert!(!dto.iter().any(|w| w.type_key == "kds"));
        assert!(!dto.iter().any(|w| w.type_key == "inventory"));
        // restaurant-pos, store-pos, admin should remain
        assert!(dto.iter().any(|w| w.type_key == "restaurant-pos"));
        assert!(dto.iter().any(|w| w.type_key == "store-pos"));
        assert!(dto.iter().any(|w| w.type_key == "admin"));
    }

    #[test]
    fn list_workspaces_with_entitlement_premium_sees_kds() {
        let (store, _) = fresh();
        // Premium tier includes KDS. Post ADR-18 §13-37 migration 091
        // renamed `workspace_types.key = 'inventory'` -> `'warehouse'`,
        // so the entitlement query checks 'warehouse' as the user-facing
        // stock-keeping workspace type (internal crate is still
        // `modules/inventory/` per §3 multi-crate carve-out rationale).
        let premium = sub_for_tier(SubscriptionTier::Premium);
        let dto = store
            .list_workspaces_with_entitlement("role-owner", None, "default", &premium)
            .unwrap();
        assert!(dto.iter().any(|w| w.type_key == "kds"));
        assert!(dto.iter().any(|w| w.type_key == "warehouse"));
        // All 5 types should be present
        assert_eq!(dto.len(), 5);
    }

    #[test]
    fn list_workspaces_with_entitlement_enterprise_sees_all() {
        let (store, _) = fresh();
        let enterprise = sub_for_tier(SubscriptionTier::Enterprise);
        let dto = store
            .list_workspaces_with_entitlement("role-owner", None, "default", &enterprise)
            .unwrap();
        assert_eq!(dto.len(), 5);
    }

    #[test]
    fn list_workspaces_with_entitlement_bundle_plus_sees_kds() {
        let (store, _) = fresh();
        // A Plus + restaurant_starter bundle subscriber's signed payload
        // lists kds — the listing must show the KDS workspace even though
        // the Plus TIER statically excludes it (C3.2).
        let sub = plus_bundle_sub();
        let dto = store
            .list_workspaces_with_entitlement("role-owner", None, "default", &sub)
            .unwrap();
        assert!(
            dto.iter().any(|w| w.type_key == "kds"),
            "bundle subscriber must see the KDS workspace, got {dto:?}"
        );
        assert_eq!(dto.len(), 5);
    }

    #[test]
    fn list_workspaces_without_entitlement_sees_all() {
        let (store, _) = fresh();
        // Original list_workspaces without tier filtering should return all 5
        let dto = store
            .list_workspaces("role-owner", None, "default")
            .unwrap();
        assert_eq!(dto.len(), 5);
        assert!(dto.iter().any(|w| w.type_key == "kds"));
    }
    #[test]
    fn count_active_instances_excludes_suspended() {
        let (store, _) = fresh();
        let initial = store.count_active_instances("default").unwrap();
        assert_eq!(initial, 5);
        // Archive one instance using the public wrapper.
        store.archive_instance("default-kds").unwrap();
        let after = store.count_active_instances("default").unwrap();
        assert_eq!(after, 4);
    }

    #[test]
    fn update_workspace_instance_changes_editable_fields() {
        let (store, _) = fresh();
        // Seed a fresh instance to mutate.
        store
            .create_workspace_instance(
                "ws-edit",
                "store-pos",
                "default",
                "Old Name",
                "Old desc",
                Some("#111111"),
            )
            .unwrap();

        store
            .update_workspace_instance("ws-edit", "New Name", Some("New desc"), Some("#222222"))
            .unwrap();

        let row = store
            .list_all_instances("default")
            .unwrap()
            .into_iter()
            .find(|r| r.id == "ws-edit")
            .unwrap();
        assert_eq!(row.name, "New Name");
        assert_eq!(row.description, "New desc");
        assert_eq!(row.colour.as_deref(), Some("#222222"));
    }

    #[test]
    fn update_workspace_instance_none_preserves_existing_fields() {
        let (store, _) = fresh();
        store
            .create_workspace_instance(
                "ws-preserve",
                "store-pos",
                "default",
                "Name",
                "keep me",
                Some("#abcdef"),
            )
            .unwrap();

        // Rename only — description and colour must be preserved (COALESCE).
        store
            .update_workspace_instance("ws-preserve", "Renamed", None, None)
            .unwrap();

        let row = store
            .list_all_instances("default")
            .unwrap()
            .into_iter()
            .find(|r| r.id == "ws-preserve")
            .unwrap();
        assert_eq!(row.name, "Renamed");
        assert_eq!(row.description, "keep me");
        assert_eq!(row.colour.as_deref(), Some("#abcdef"));
    }

    #[test]
    fn update_workspace_instance_missing_returns_not_found() {
        let (store, _) = fresh();
        let err = store
            .update_workspace_instance("does-not-exist", "X", Some("Y"), None)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn owner_with_user_store_access_filtered_by_assigned_stores() {
        let (store, user_id) = fresh();
        // Create a second store profile so we have multiple stores.
        store
            .conn
            .execute(
                "INSERT INTO store_profiles (id, name, address, currency, timezone)
                 VALUES ('store-b', 'Store B', '456 Elm', 'IDR', 'Asia/Jakarta')",
                [],
            )
            .unwrap();
        // Seed a workspace instance in store-b so we can detect cross-store leakage.
        store
            .create_workspace_instance(
                "store-b-restaurant-pos",
                "restaurant-pos",
                "store-b",
                "Store B POS",
                "",
                None,
            )
            .unwrap();

        // Seed user_store_access — user-1 only has access to "default", not "store-b".
        store
            .conn
            .execute(
                "INSERT INTO user_store_access (user_id, store_id, access_level)
                 VALUES (?1, 'default', 'manager')",
                params![user_id],
            )
            .unwrap();

        // User can see instances in "default" store.
        let dto_default = store
            .list_workspaces("role-owner", Some(&user_id), "default")
            .unwrap();
        assert!(
            !dto_default.is_empty(),
            "should see default store instances"
        );

        // User CANNOT see instances in "store-b" — empty result.
        let dto_store_b = store
            .list_workspaces("role-owner", Some(&user_id), "store-b")
            .unwrap();
        assert!(
            dto_store_b.is_empty(),
            "owner with user_store_access should not see unassigned store"
        );
    }

    #[test]
    fn enforce_instance_quota_rejects_disallowed_type() {
        let (store, _) = fresh();
        let free = sub_for_tier(SubscriptionTier::Free);
        let result = store.enforce_instance_quota(&free, "kds", "default");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("kds"));
        assert!(err.contains("Free"));
    }

    #[test]
    fn enforce_instance_quota_allows_type_but_fails_on_count() {
        let (store, _) = fresh();
        let free = sub_for_tier(SubscriptionTier::Free);
        // Free tier allows restaurant-pos but we have 5 active instances.
        // Free tier allows 1 max, so this should fail on count, not type.
        let result = store.enforce_instance_quota(&free, "restaurant-pos", "default");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("1 registers"));
    }

    #[test]
    fn enforce_instance_quota_bundle_plus_allows_kds() {
        let (store, _) = fresh();
        // A fresh store id has zero active instances, so the type check is
        // the only gate — kds must pass for the bundle even at Plus tier.
        let sub = plus_bundle_sub();
        assert!(
            store
                .enforce_instance_quota(&sub, "kds", "fresh-store")
                .is_ok(),
            "Plus + restaurant_starter must be able to create a kds workspace"
        );
        // The same type stays rejected for plain Plus (empty block → tier
        // defaults), proving the payload is what widened the entitlement.
        let plain = sub_for_tier(SubscriptionTier::Plus);
        let result = store.enforce_instance_quota(&plain, "kds", "fresh-store");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("kds"));
        assert!(err.contains("Plus"));
    }

    // ── Auto-Recovery & Suspension tests (ADR #5 Phase 3b/3c) ───────

    #[test]
    fn auto_recover_restores_suspended_to_limit() {
        let (store, _) = fresh();
        // Suspend two instances manually. Post ADR-18 §13-37 migration 091
        // renamed workspace_instances.id 'default-inventory' -> 'default-warehouse'
        // (the matched-pair workaround for the workspace_types.key -> id rename
        // cascade — see the migration_060 seed-row derivation cited inline in
        // migration 091).
        store.conn.execute(
            "UPDATE workspace_instances SET status = 'quota_suspended' WHERE id IN ('default-kds', 'default-warehouse')",
            [],
        ).unwrap();
        // Now: 3 active, 2 suspended.
        assert_eq!(store.count_active_instances("default").unwrap(), 3);

        // Premium tier allows 10 per store — recover should restore both.
        let premium = SubscriptionTier::Premium;
        let restored = store.auto_recover_instances("default", &premium).unwrap();
        assert_eq!(restored, 2);
        assert_eq!(store.count_active_instances("default").unwrap(), 5);
    }

    #[test]
    fn auto_recover_respects_tier_limit() {
        let (store, _) = fresh();
        // Suspend one instance.
        store.conn.execute(
            "UPDATE workspace_instances SET status = 'quota_suspended' WHERE id = 'default-kds'",
            [],
        ).unwrap();
        // Now: 4 active, 1 suspended.

        // Free tier allows 1 per store — no slots, nothing to recover.
        let free = SubscriptionTier::Free;
        let restored = store.auto_recover_instances("default", &free).unwrap();
        assert_eq!(restored, 0);
        assert_eq!(store.count_active_instances("default").unwrap(), 4);
    }

    #[test]
    fn auto_recover_unlimited_restores_all() {
        let (store, _) = fresh();
        store
            .conn
            .execute(
                "UPDATE workspace_instances SET status = 'quota_suspended'",
                [],
            )
            .unwrap();
        assert_eq!(store.count_active_instances("default").unwrap(), 0);

        let enterprise = SubscriptionTier::Enterprise;
        let restored = store
            .auto_recover_instances("default", &enterprise)
            .unwrap();
        assert_eq!(restored, 5);
        assert_eq!(store.count_active_instances("default").unwrap(), 5);
    }

    #[test]
    fn suspend_surplus_transitions_excess_to_suspended() {
        let (store, _) = fresh();
        // 5 active instances. Free tier allows 1. Surplus = 4.
        let free = SubscriptionTier::Free;
        let suspended = store.suspend_surplus_instances("default", &free).unwrap();
        assert_eq!(suspended, 4);
        assert_eq!(store.count_active_instances("default").unwrap(), 1);
    }

    #[test]
    fn suspend_surplus_no_op_when_under_limit() {
        let (store, _) = fresh();
        // Premium allows 10, we only have 5 — nothing to suspend.
        let premium = SubscriptionTier::Premium;
        let suspended = store
            .suspend_surplus_instances("default", &premium)
            .unwrap();
        assert_eq!(suspended, 0);
        assert_eq!(store.count_active_instances("default").unwrap(), 5);
    }

    #[test]
    fn suspend_surplus_unlimited_tier_no_op() {
        let (store, _) = fresh();
        let enterprise = SubscriptionTier::Enterprise;
        let suspended = store
            .suspend_surplus_instances("default", &enterprise)
            .unwrap();
        assert_eq!(suspended, 0);
    }

    #[test]
    fn auto_recover_then_suspend_roundtrip() {
        let (store, _) = fresh();
        // Suspend all
        store
            .conn
            .execute(
                "UPDATE workspace_instances SET status = 'quota_suspended'",
                [],
            )
            .unwrap();

        // Recover with Plus (2 limit)
        let plus = SubscriptionTier::Plus;
        let restored = store.auto_recover_instances("default", &plus).unwrap();
        assert_eq!(restored, 2);
        assert_eq!(store.count_active_instances("default").unwrap(), 2);

        // Downgrade to Free (1 limit) — should suspend 1
        let free = SubscriptionTier::Free;
        let suspended = store.suspend_surplus_instances("default", &free).unwrap();
        assert_eq!(suspended, 1);
        assert_eq!(store.count_active_instances("default").unwrap(), 1);
    }

    // ── TOPOLOGY_AUDIT follow-up tests ───────────────────────────────
    //
    // Cover audit #1 (type_key / store_id immutability) and audit #4
    // (atomicity of the create + update + archive diff that
    // `apply_topology_diff` runs in one SQLite transaction).

    /// Helper: fetch a single instance row by id, panicking if absent.
    fn fetch_instance(store: &Store<'_>, id: &str) -> WorkspaceInstanceRow {
        store
            .list_all_instances("default")
            .unwrap()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap_or_else(|| panic!("instance {id} not found"))
    }

    // ── #4 regression: create_workspace_instance CANNOT be called from
    //    inside an open transaction (it uses unchecked_transaction, which
    //    issues a raw BEGIN that SQLite rejects when a tx is active).
    //
    // `apply_topology_diff` opens an outer transaction and then runs the
    // create INSERT SQL directly on it (NOT via this method) for exactly
    // this reason. This test documents the constraint so it is not
    // accidentally regressed.

    #[test]
    fn create_workspace_instance_cannot_nest_in_open_transaction() {
        let (store, _) = fresh();
        let conn = store.conn;
        let outer = conn.unchecked_transaction().unwrap();
        let tx_store = Store::new(&outer);

        let result = tx_store.create_workspace_instance(
            "nested-should-fail",
            "restaurant-pos",
            "default",
            "Nested",
            "",
            None,
        );
        assert!(
            result.is_err(),
            "create_workspace_instance must not nest inside an open transaction; \
             apply_topology_diff must run the SQL directly on its own tx instead"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("cannot start a transaction within a transaction"),
            "expected the SQLite nesting error, got: {err}"
        );
        drop(outer);
        // Nothing was created.
        assert!(
            store
                .list_all_instances("default")
                .unwrap()
                .iter()
                .all(|r| r.id != "nested-should-fail")
        );
    }

    // ── #4: the correct pattern — run SQL directly on an outer tx ──

    #[test]
    fn direct_insert_on_outer_tx_persists_on_commit() {
        // The pattern apply_topology_diff uses: open one tx, run the
        // INSERT directly, commit once.
        let (store, _) = fresh();
        let conn = store.conn;
        let tx = conn.unchecked_transaction().unwrap();

        tx.execute(
            "INSERT INTO workspace_instances \
             (id, type_key, store_id, name, description, colour, status, last_accessed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'active', \
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params!["direct-1", "restaurant-pos", "default", "Direct", ""],
        )
        .unwrap();
        tx.commit().unwrap();

        let row = fetch_instance(&store, "direct-1");
        assert_eq!(row.type_key, "restaurant-pos");
        assert_eq!(row.status, "active");
    }

    #[test]
    fn direct_insert_on_outer_tx_rolls_back_on_drop() {
        // Dropping the outer tx without commit rolls everything back —
        // the atomicity guarantee apply_topology_diff relies on.
        let (store, _) = fresh();
        let conn = store.conn;
        {
            let tx = conn.unchecked_transaction().unwrap();
            tx.execute(
                "INSERT INTO workspace_instances \
                 (id, type_key, store_id, name, description, colour, status, last_accessed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'active', \
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params!["rollback-1", "restaurant-pos", "default", "Roll", ""],
            )
            .unwrap();
            // Drop without commit → rollback.
        }
        assert!(
            store
                .list_all_instances("default")
                .unwrap()
                .iter()
                .all(|r| r.id != "rollback-1")
        );
    }

    #[test]
    fn mixed_create_update_archive_on_one_tx_commits_atomically() {
        // Audit #4 happy path: create + update + archive in one tx all
        // succeed and commit together (direct SQL, no nested tx).
        let (store, _) = fresh();
        let conn = store.conn;
        let tx = conn.unchecked_transaction().unwrap();

        // Create two.
        for (id, name) in [("diff-a", "A"), ("diff-b", "B")] {
            tx.execute(
                "INSERT INTO workspace_instances \
                 (id, type_key, store_id, name, description, colour, status, last_accessed_at) \
                 VALUES (?1, 'store-pos', 'default', ?2, '', NULL, 'active', \
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![id, name],
            )
            .unwrap();
        }
        // Update A's name.
        tx.execute(
            "UPDATE workspace_instances SET name = ?2, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = ?1",
            params!["diff-a", "A Renamed"],
        )
        .unwrap();
        // Archive B.
        tx.execute(
            "UPDATE workspace_instances SET status = 'archived', \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = ?1",
            params!["diff-b"],
        )
        .unwrap();
        tx.commit().unwrap();

        let instances = store.list_all_instances("default").unwrap();
        let a = instances.iter().find(|r| r.id == "diff-a").unwrap();
        assert_eq!(a.name, "A Renamed");
        assert_eq!(a.status, "active");
        let b = instances.iter().find(|r| r.id == "diff-b").unwrap();
        assert_eq!(b.status, "archived");
    }

    #[test]
    fn failed_step_rolls_back_entire_diff_tx() {
        // Audit #4: if a later step fails, prior creates/updates must
        // roll back — no partial persistence.
        let (store, _) = fresh();
        let conn = store.conn;
        let tx = conn.unchecked_transaction().unwrap();

        // Create.
        tx.execute(
            "INSERT INTO workspace_instances \
             (id, type_key, store_id, name, description, colour, status, last_accessed_at) \
             VALUES (?1, 'store-pos', 'default', 'Will Roll Back', '', NULL, 'active', \
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params!["diff-rollback"],
        )
        .unwrap();
        // Archive a non-existent id → 0 rows affected (failure signal).
        let archived = tx
            .execute(
                "UPDATE workspace_instances SET status = 'archived' WHERE id = ?1",
                params!["ghost-id"],
            )
            .unwrap();
        assert_eq!(archived, 0, "ghost archive affects 0 rows");
        // Roll back (apply_topology_diff returns the error, drops the tx).
        drop(tx);

        assert!(
            store
                .list_all_instances("default")
                .unwrap()
                .iter()
                .all(|r| r.id != "diff-rollback")
        );
    }

    // ── #1: type_key and store_id are immutable via update ──────────

    #[test]
    fn update_does_not_change_type_key() {
        // Audit #1: a rename must not silently change the type. The
        // update path has no type_key parameter, so the type stays.
        let (store, _) = fresh();
        store
            .create_workspace_instance(
                "imm-type",
                "restaurant-pos",
                "default",
                "Original",
                "",
                None,
            )
            .unwrap();

        store
            .update_workspace_instance("imm-type", "Renamed", None, None)
            .unwrap();

        let row = fetch_instance(&store, "imm-type");
        assert_eq!(row.name, "Renamed");
        assert_eq!(
            row.type_key, "restaurant-pos",
            "type_key must be immutable across an update"
        );
    }

    #[test]
    fn update_does_not_change_store_id() {
        let (store, _) = fresh();
        store
            .create_workspace_instance("imm-store", "store-pos", "default", "Original", "", None)
            .unwrap();

        store
            .update_workspace_instance("imm-store", "Renamed", None, None)
            .unwrap();

        let row = fetch_instance(&store, "imm-store");
        assert_eq!(row.name, "Renamed");
        assert_eq!(
            row.store_id, "default",
            "store_id must be immutable across an update"
        );
    }

    #[test]
    fn update_preserves_type_and_store_when_changing_other_fields() {
        let (store, _) = fresh();
        store
            .create_workspace_instance(
                "imm-full",
                "kds",
                "default",
                "Kitchen",
                "old desc",
                Some("#aaaaaa"),
            )
            .unwrap();

        store
            .update_workspace_instance(
                "imm-full",
                "Kitchen Renamed",
                Some("new desc"),
                Some("#bbbbbb"),
            )
            .unwrap();

        let row = fetch_instance(&store, "imm-full");
        assert_eq!(row.name, "Kitchen Renamed");
        assert_eq!(row.description, "new desc");
        assert_eq!(row.colour.as_deref(), Some("#bbbbbb"));
        // Immutable fields untouched.
        assert_eq!(row.type_key, "kds");
        assert_eq!(row.store_id, "default");
    }

    #[test]
    fn update_cannot_move_instance_to_another_store() {
        // Even when a second store exists, update has no store_id param.
        let (store, _) = fresh();
        store
            .conn
            .execute(
                "INSERT INTO store_profiles (id, name, address, currency, timezone)
                 VALUES ('store-b', 'Store B', '456 Elm', 'IDR', 'Asia/Jakarta')",
                [],
            )
            .unwrap();
        store
            .create_workspace_instance("stay", "store-pos", "default", "A", "", None)
            .unwrap();

        store
            .update_workspace_instance("stay", "Renamed", None, None)
            .unwrap();

        let row = fetch_instance(&store, "stay");
        assert_eq!(row.store_id, "default");
        let store_b = store.list_all_instances("store-b").unwrap();
        assert!(
            !store_b.iter().any(|r| r.id == "stay"),
            "instance must not leak across stores on update"
        );
    }

    #[test]
    fn update_coalesces_unchanged_fields_preserving_type_and_store() {
        // COALESCE contract: None for description/colour keeps existing
        // values — the mechanism that makes partial updates safe and
        // never clobbers type/store.
        let (store, _) = fresh();
        store
            .create_workspace_instance(
                "coalesce",
                "store-pos",
                "default",
                "Name",
                "keep me",
                Some("#abcdef"),
            )
            .unwrap();

        store
            .update_workspace_instance("coalesce", "Renamed", None, None)
            .unwrap();

        let row = fetch_instance(&store, "coalesce");
        assert_eq!(row.name, "Renamed");
        assert_eq!(row.description, "keep me");
        assert_eq!(row.colour.as_deref(), Some("#abcdef"));
        assert_eq!(row.type_key, "store-pos");
        assert_eq!(row.store_id, "default");
    }

    // ── Input validation ────────────────────────────────────────────────

    #[test]
    fn create_workspace_instance_rejects_empty_id() {
        let (store, _) = fresh();
        let err = store
            .create_workspace_instance("", "store-pos", "default", "Name", "desc", None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: "id", .. }));
    }

    #[test]
    fn create_workspace_instance_rejects_empty_type_key() {
        let (store, _) = fresh();
        let err = store
            .create_workspace_instance("ws-1", "", "default", "Name", "desc", None)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "type_key",
                ..
            }
        ));
    }

    #[test]
    fn create_workspace_instance_rejects_empty_store_id() {
        let (store, _) = fresh();
        let err = store
            .create_workspace_instance("ws-1", "store-pos", "", "Name", "desc", None)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "store_id",
                ..
            }
        ));
    }

    #[test]
    fn create_workspace_instance_rejects_empty_name() {
        let (store, _) = fresh();
        let err = store
            .create_workspace_instance("ws-1", "store-pos", "default", "", "desc", None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: "name", .. }));
    }

    #[test]
    fn update_workspace_instance_rejects_empty_name() {
        let (store, _) = fresh();
        store
            .create_workspace_instance("ws-1", "store-pos", "default", "Name", "desc", None)
            .unwrap();
        let err = store
            .update_workspace_instance("ws-1", "", None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: "name", .. }));
    }

    // ── Session-mint authorization gate (audit/06 residual) ────────────
    //
    // `verify_instance_access` is the server-side gate `create_session`
    // calls in both desktop and tablet clients (ADR #4 / ADR #7). TDD red:
    // the gate must FAIL CLOSED when the caller identity cannot be trusted
    // — unknown user, inactive user, or a claimed `role_id` that does not
    // match the user's actual database role. The previous implementation
    // trusted the caller-supplied `role_id` for the owner/manager bypass
    // and never resolved the user, so any IPC caller who knew a user id
    // could mint a session AS that user (privilege escalation) in ANY
    // store's active instance (cross-store session minting) — the residual
    // recorded in audit/06.

    /// Seed the built-in roles plus an owner user (role-owner carries `*`).
    fn seed_owner_user(conn: &rusqlite::Connection) {
        let store = Store::new(conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
    }

    #[test]
    fn verify_instance_access_denies_unknown_user() {
        let (store, _) = fresh();
        // A ghost user id with the owner claim previously passed the owner
        // bypass (no `user_store_access` rows → single-store mode) and
        // would have minted a session for an identity that does not exist.
        let ok = store
            .verify_instance_access(
                "role-owner",
                "ghost-user",
                "default-restaurant-pos",
                "default",
            )
            .unwrap();
        assert!(!ok, "unknown user must not be able to open a session");
    }

    #[test]
    fn verify_instance_access_rejects_forged_owner_role() {
        let (store, user_id) = fresh();
        // user-1's ACTUAL role is role-test. Claiming role-owner must be
        // rejected even though the instance exists and is active.
        let ok = store
            .verify_instance_access("role-owner", &user_id, "default-restaurant-pos", "default")
            .unwrap();
        assert!(
            !ok,
            "a claimed role differing from the user's real role must be denied"
        );
    }

    #[test]
    fn verify_instance_access_denies_inactive_user() {
        let (store, user_id) = fresh();
        // Claim the user's REAL role AND grant an explicit instance
        // assignment: without the `is_active` guard, branch 2 would return
        // Ok(true), so this test uniquely pins the inactive check rather
        // than being denied by a role mismatch.
        store
            .set_user_workspace_instances(&user_id, ["default-admin"], None)
            .unwrap();
        store
            .conn
            .execute(
                "UPDATE users SET is_active = 0 WHERE id = ?1",
                params![user_id],
            )
            .unwrap();
        let ok = store
            .verify_instance_access("role-test", &user_id, "default-admin", "default")
            .unwrap();
        assert!(!ok, "deactivated users must not be able to open a session");
    }

    #[test]
    fn verify_instance_access_allows_real_owner() {
        let (store, _) = fresh();
        seed_owner_user(store.conn);
        let ok = store
            .verify_instance_access(
                "role-owner",
                "user-owner",
                "default-restaurant-pos",
                "default",
            )
            .unwrap();
        assert!(
            ok,
            "a real owner with the matching role keeps instance access"
        );
    }

    #[test]
    fn verify_instance_access_allows_auditor() {
        // Auditor is a global read-only role — the session-open gate must
        // admit it into any active instance so it can reach its read-only
        // screens (the plan's "Auditor is global" claim).
        let (store, _) = fresh();
        seed_owner_user(store.conn);
        store
            .conn
            .execute(
                "INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at)
                 VALUES ('user-auditor', 'auditor', 'hash', 'Auditor', 'role-auditor', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
                [],
            )
            .unwrap();
        let ok = store
            .verify_instance_access(
                "role-auditor",
                "user-auditor",
                "default-restaurant-pos",
                "default",
            )
            .unwrap();
        assert!(
            ok,
            "a real auditor with the matching role keeps instance access"
        );
    }

    #[test]
    fn verify_instance_access_allows_explicit_assignment_with_real_role() {
        let (store, user_id) = fresh();
        store
            .set_user_workspace_instances(&user_id, ["default-admin"], None)
            .unwrap();
        let ok = store
            .verify_instance_access("role-test", &user_id, "default-admin", "default")
            .unwrap();
        assert!(
            ok,
            "explicit instance assignment with the real role stays allowed"
        );
    }

    #[test]
    fn verify_instance_access_multi_store_owner_limited_to_assigned_stores() {
        let (store, _) = fresh();
        seed_owner_user(store.conn);
        store
            .conn
            .execute(
                "INSERT INTO store_profiles (id, name, address, currency, timezone)
                 VALUES ('store-b', 'Store B', '456 Elm', 'IDR', 'Asia/Jakarta')",
                [],
            )
            .unwrap();
        store
            .create_workspace_instance(
                "store-b-restaurant-pos",
                "restaurant-pos",
                "store-b",
                "Store B POS",
                "",
                None,
            )
            .unwrap();
        store
            .conn
            .execute(
                "INSERT INTO user_store_access (user_id, store_id, access_level)
                 VALUES ('user-owner', 'default', 'manager')",
                [],
            )
            .unwrap();

        let ok_default = store
            .verify_instance_access(
                "role-owner",
                "user-owner",
                "default-restaurant-pos",
                "default",
            )
            .unwrap();
        let ok_store_b = store
            .verify_instance_access(
                "role-owner",
                "user-owner",
                "store-b-restaurant-pos",
                "store-b",
            )
            .unwrap();
        assert!(
            ok_default,
            "owner with store access keeps their assigned store"
        );
        assert!(
            !ok_store_b,
            "multi-store owner must not open a session in an unassigned store"
        );
    }
}
