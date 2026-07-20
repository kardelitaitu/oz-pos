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
use crate::subscription::{QuotaError, SubscriptionTier};

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
            || role_id == "manager"
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

    /// Legacy: replace workspace assignments for a user (old tables).
    pub fn set_user_workspaces_legacy<'b>(
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

    /// Legacy: get workspace keys assigned to a user (old table).
    pub fn get_user_workspace_keys_legacy(&self, user_id: &str) -> Result<Vec<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT ws_key FROM user_workspaces WHERE user_id = ?1 ORDER BY ws_key")?;
        let rows = stmt.query_map(params![user_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }
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
    /// Same resolution as [`list_workspaces`] but additionally filters
    /// out instances whose `type_key` is not allowed by the given tier.
    pub fn list_workspaces_with_entitlement(
        &self,
        role_id: &str,
        user_id: Option<&str>,
        store_id: &str,
        tier: &SubscriptionTier,
    ) -> Result<Vec<WorkspaceDto>, CoreError> {
        let mut results = self.list_workspaces_inner(role_id, user_id, store_id)?;
        results.retain(|dto| tier.allows_workspace_type(&dto.type_key));
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
        // TODO(ADR #4 Phase 2): Check user_store_access before returning all instances.
        // In multi-store mode, role-owner with user_store_access rows should only see
        // instances from assigned stores (see ADR #4 Security Architecture §3).
        if role_id == "role-owner"
            || role_id == "role-admin"
            || role_id == "admin"
            || role_id == "role-manager"
            || role_id == "manager"
        {
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
            name: row.get(4)?,
            description: row.get(5)?,
            icon: row.get(6)?,
            layout_mode: row.get(7)?,
            colour: row.get(8)?,
            is_default: row.get::<_, i32>(9)? != 0,
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
                name: row.get(4)?,
                description: row.get(5)?,
                icon: row.get(6)?,
                layout_mode: row.get(7)?,
                colour: row.get(8)?,
                is_default: row.get::<_, i32>(9)? != 0,
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
    /// 1. Tier allows this workspace type
    /// 2. Per-store register count is within tier limit
    ///
    /// Called by Tauri commands before delegating to `create_workspace_instance`.
    pub fn enforce_instance_quota(
        &self,
        tier: &SubscriptionTier,
        type_key: &str,
        store_id: &str,
    ) -> Result<(), CoreError> {
        // 1. Workspace type must be allowed by this tier.
        if !tier.allows_workspace_type(type_key) {
            return Err(QuotaError::TypeNotAllowed {
                tier: tier.name().into(),
                type_key: type_key.into(),
            }
            .into());
        }

        // 2. Per-store register limit.
        if let Some(limit) = tier.max_pos_instances() {
            let current = self.count_active_instances(store_id)?;
            if current >= limit {
                return Err(QuotaError::RegisterLimit {
                    tier: tier.name().into(),
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
    pub fn create_workspace_instance(
        &self,
        id: &str,
        type_key: &str,
        store_id: &str,
        name: &str,
        description: &str,
        colour: Option<&str>,
    ) -> Result<WorkspaceInstanceRow, CoreError> {
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
            "INSERT INTO workspace_instances (id, type_key, store_id, name, description, colour, status, last_accessed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![id, type_key, store_id, name, description, colour],
        )?;

        tx.commit()?;

        let row: WorkspaceInstanceRow = self.conn.query_row(
            "SELECT id, type_key, store_id, name, description, colour, status, created_at, updated_at
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
                    status: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
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

    /// List all workspace instances in a store (admin use, no access control).
    pub fn list_all_instances(
        &self,
        store_id: &str,
    ) -> Result<Vec<WorkspaceInstanceRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type_key, store_id, name, description, colour, status, created_at, updated_at
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
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
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

    // ── Legacy tests (backward compatible) ────────────────────────────

    #[test]
    fn list_all_workspace_types_returns_seeded() {
        let (store, _) = fresh();
        let ws = store.list_all_workspace_types().unwrap();
        assert_eq!(ws.len(), 5);
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
        let kds = ws.iter().find(|w| w.key == "kds").unwrap();
        assert_eq!(kds.name, "Kitchen Display");
        assert_eq!(kds.icon, "kds");
    }

    #[test]
    fn list_workspaces_legacy_owner_returns_all() {
        let (store, _) = fresh();
        let ws = store.list_workspaces_legacy("role-owner", None).unwrap();
        assert_eq!(ws.len(), 5);
    }

    #[test]
    fn set_user_workspaces_legacy_replaces_previous() {
        let (store, user_id) = fresh();
        // Post ADR-18 §13 finding 37 (migration 091): workspace_types.key is
        // 'warehouse', not 'inventory' — the user_workspaces.ws_key FK
        // column references `workspaces.key` and the literal 'inventory'
        // would FK-violate against the post-rename workspaces row.
        store
            .set_user_workspaces_legacy(&user_id, ["restaurant-pos", "warehouse"])
            .unwrap();
        let keys = store.get_user_workspace_keys_legacy(&user_id).unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"warehouse".into()));

        store
            .set_user_workspaces_legacy(&user_id, ["admin"])
            .unwrap();
        let keys = store.get_user_workspace_keys_legacy(&user_id).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "admin");
    }

    #[test]
    fn set_user_workspaces_legacy_empty_clears() {
        let (store, user_id) = fresh();
        store
            .set_user_workspaces_legacy(&user_id, ["admin"])
            .unwrap();
        store.set_user_workspaces_legacy(&user_id, []).unwrap();
        let keys = store.get_user_workspace_keys_legacy(&user_id).unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn list_workspaces_legacy_with_user_override() {
        let (store, user_id) = fresh();
        let before = store
            .list_workspaces_legacy("role-test", Some(&user_id))
            .unwrap();
        assert!(before.is_empty(), "role-test has no role_workspaces");

        store
            .set_user_workspaces_legacy(&user_id, ["admin"])
            .unwrap();
        let after = store
            .list_workspaces_legacy("role-test", Some(&user_id))
            .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].key, "admin");
    }

    #[test]
    fn get_user_workspace_keys_legacy_empty_when_no_override() {
        let (store, user_id) = fresh();
        let keys = store.get_user_workspace_keys_legacy(&user_id).unwrap();
        assert!(keys.is_empty());
    }

    // ── New tests (ADR #4 Phase 1) ────────────────────────────────────

    #[test]
    fn list_workspace_types_returns_all() {
        let (store, _) = fresh();
        let types = store.list_workspace_types().unwrap();
        assert_eq!(types.len(), 5);
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
        let free_tier = SubscriptionTier::Free;
        let dto = store
            .list_workspaces_with_entitlement("role-owner", None, "default", &free_tier)
            .unwrap();
        // KDS and inventory should be filtered out
        assert!(
            dto.iter()
                .all(|w| free_tier.allows_workspace_type(&w.type_key))
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
        let premium = SubscriptionTier::Premium;
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
        let enterprise = SubscriptionTier::Enterprise;
        let dto = store
            .list_workspaces_with_entitlement("role-owner", None, "default", &enterprise)
            .unwrap();
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
        // Archive one instance.
        // TODO(ADR #5): Add a public archive_instance() method to Store
        // for proper encapsulation.
        store
            .conn
            .execute(
                "UPDATE workspace_instances SET status = 'archived' WHERE id = 'default-kds'",
                [],
            )
            .unwrap();
        let after = store.count_active_instances("default").unwrap();
        assert_eq!(after, 4);
    }

    #[test]
    fn enforce_instance_quota_rejects_disallowed_type() {
        let (store, _) = fresh();
        let free = SubscriptionTier::Free;
        let result = store.enforce_instance_quota(&free, "kds", "default");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("kds"));
        assert!(err.contains("Free"));
    }

    #[test]
    fn enforce_instance_quota_allows_type_but_fails_on_count() {
        let (store, _) = fresh();
        let free = SubscriptionTier::Free;
        // Free tier allows restaurant-pos but we have 5 active instances.
        // Free tier allows 1 max, so this should fail on count, not type.
        let result = store.enforce_instance_quota(&free, "restaurant-pos", "default");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("1 registers"));
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

        // Recover with Standard (2 limit)
        let std = SubscriptionTier::Standard;
        let restored = store.auto_recover_instances("default", &std).unwrap();
        assert_eq!(restored, 2);
        assert_eq!(store.count_active_instances("default").unwrap(), 2);

        // Downgrade to Free (1 limit) — should suspend 1
        let free = SubscriptionTier::Free;
        let suspended = store.suspend_surplus_instances("default", &free).unwrap();
        assert_eq!(suspended, 1);
        assert_eq!(store.count_active_instances("default").unwrap(), 1);
    }
}
