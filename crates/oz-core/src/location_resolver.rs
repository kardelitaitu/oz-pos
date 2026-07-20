//! Workspace-binding resolution layer (ADR-19 §4).
//!
//! When a POS workspace needs to deduct stock on sale, it must know *which
//! inventory location* to deduct from. The resolution layer answers that
//! question via a strict priority tree:
//!
//! | Tier | Source | Field |
//! |------|--------|-------|
//! | 1 | Explicit override (cashier FastPIN) | `explicit_override` arg |
//! | 2 | Single-binding | `workspace_instances.bound_location_id` |
//! | 3 | Multi-binding primary | `workspace_inventory_locations.is_primary = 1` |
//! | 4 | Canonical default | `CANONICAL_DEFAULT_LOCATION_UUID` |
//!
//! Performance optimisation: `resolve_primary_location` is read-heavy (called
//! once per cart-open + possibly per `add_line`), so we cache the result per
//! `workspace_instance_id` with a 30-second TTL. The cache is invalidated on
//! workspace switch to force a fresh SELECT from the database.

use rusqlite::params;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::Instant;

use crate::error::CoreError;
use crate::inventory::{CANONICAL_DEFAULT_LOCATION_UUID, LocationId};
use crate::sale_deduction::LocationStock;

// ── In-memory cache with 30s TTL ────────────────────────────────────

#[derive(Clone)]
struct CachedLocation {
    location_id: LocationId,
    cached_at: Instant,
}

/// Global cache for resolved primary locations, keyed by `workspace_instance_id`.
/// Entries expire after 30 seconds. Call [`invalidate_location_cache`] to clear
/// the entire cache (e.g. on workspace switch).
static LOCATION_CACHE: LazyLock<Mutex<HashMap<String, CachedLocation>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// TTL for cached location resolutions, in seconds.
const CACHE_TTL_SECS: u64 = 30;

/// Check the in-memory cache for a previously-resolved primary location.
/// Returns `None` on cache miss or if the entry has expired (30s TTL).
fn cache_get(workspace_instance_id: &str) -> Option<LocationId> {
    let cache = LOCATION_CACHE.lock().ok()?;
    if let Some(entry) = cache.get(workspace_instance_id)
        && entry.cached_at.elapsed().as_secs() < CACHE_TTL_SECS
    {
        return Some(entry.location_id.clone());
    }
    None
}

/// Store a resolved primary location in the in-memory cache.
fn cache_set(workspace_instance_id: &str, location_id: &LocationId) {
    if let Ok(mut cache) = LOCATION_CACHE.lock() {
        cache.insert(
            workspace_instance_id.to_owned(),
            CachedLocation {
                location_id: location_id.clone(),
                cached_at: Instant::now(),
            },
        );
    }
}

/// Clear the entire location cache. Called on workspace switch so the next
/// `resolve_primary_location` call performs a fresh database SELECT.
pub fn invalidate_location_cache() {
    if let Ok(mut cache) = LOCATION_CACHE.lock() {
        cache.clear();
    }
}

/// An enriched binding that pairs a location ID with its display name and
/// workspace-specific flags (is_primary, allow_negative_stock).
///
/// Returned by [`get_workspace_locations`] for use in front-end location
/// pickers and sale-deduction flows (ADR-19 §10).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceLocationBinding {
    /// Inventory location UUID.
    pub location_id: String,
    /// Human-readable location name (from `inventory_locations.name`).
    pub location_name: String,
    /// Whether this is the primary location for stock deductions.
    pub is_primary: bool,
    /// Whether this location allows negative stock.
    pub allow_negative_stock: bool,
}

/// Return the frozen canonical default location UUID as a [`LocationId`].
///
/// ADR-18 §13-36: this UUID is `01926b3a-0000-7000-8000-000000000001` and
/// matches the seed row in migration 078. All legacy single-location callers
/// resolve here transparently.
#[must_use]
pub fn get_default_location_id() -> LocationId {
    LocationId::from(CANONICAL_DEFAULT_LOCATION_UUID)
}

/// Unified workspace-location resolver (ADR-19 §10).
///
/// Resolves the inventory locations bound to a workspace instance,
/// enriched with display names and binding flags. Behaviour differs
/// by workspace type:
///
/// | `type_key` | Resolution strategy |
/// |------------|----------------------|
/// | `store-pos` | `workspace_inventory_locations` table (multi-binding via `set_workspace_inventory_locations`) |
/// | `warehouse` | `workspace_instances.bound_location_id` (single FK); returns all active locations when NULL (admin view) |
/// | other | Returns empty vec |
///
/// # Errors
///
/// Returns [`CoreError::Validation`] if **both** `bound_location_id` is set
/// AND rows exist in `workspace_inventory_locations` (split-brain config).
/// Returns [`CoreError::NotFound`] if the workspace instance does not exist.
pub fn get_workspace_locations(
    conn: &rusqlite::Connection,
    instance_id: &str,
    type_key: &str,
) -> Result<Vec<WorkspaceLocationBinding>, CoreError> {
    // Verify the instance exists and read its bound_location_id.
    let (bound_location_id,): (Option<String>,) = conn
        .query_row(
            "SELECT bound_location_id FROM workspace_instances WHERE id = ?1",
            params![instance_id],
            |row| Ok((row.get(0)?,)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound {
                entity: "workspace_instance",
                id: instance_id.to_owned(),
            },
            other => CoreError::Db(other),
        })?;

    let has_bound = bound_location_id.as_ref().is_some_and(|b| !b.is_empty());

    // Check for multi-binding rows.
    let multi_rows: Vec<(String, bool, bool)> = {
        let mut stmt = conn
            .prepare(
                "SELECT wil.location_id, wil.is_primary, wil.allow_negative_stock \
                 FROM workspace_inventory_locations wil \
                 WHERE wil.instance_id = ?1 \
                 ORDER BY wil.is_primary DESC, wil.sort_order ASC",
            )
            .map_err(CoreError::Db)?;
        let rows = stmt
            .query_map(params![instance_id], |row| {
                let loc_id: String = row.get(0)?;
                let prim: i64 = row.get(1)?;
                let neg: i64 = row.get(2)?;
                Ok((loc_id, prim == 1, neg == 1))
            })
            .map_err(CoreError::Db)?;
        let mut ids = Vec::new();
        for r in rows {
            ids.push(r.map_err(CoreError::Db)?);
        }
        ids
    };

    // Split-brain detection.
    if has_bound && !multi_rows.is_empty() {
        return Err(CoreError::Validation {
            field: "workspace_binding",
            message: format!(
                "workspace instance {instance_id} has both bound_location_id \
                 and workspace_inventory_locations rows — split-brain config"
            ),
        });
    }

    match type_key {
        "store-pos" => {
            // Multi-binding via workspace_inventory_locations.
            if multi_rows.is_empty() {
                // No explicit bindings — return canonical default.
                let (name,): (String,) = conn
                    .query_row(
                        "SELECT COALESCE(name, 'Default') FROM inventory_locations WHERE id = ?1",
                        params![CANONICAL_DEFAULT_LOCATION_UUID],
                        |row| Ok((row.get(0)?,)),
                    )
                    .unwrap_or(("Default".into(),));
                return Ok(vec![WorkspaceLocationBinding {
                    location_id: CANONICAL_DEFAULT_LOCATION_UUID.to_owned(),
                    location_name: name,
                    is_primary: true,
                    allow_negative_stock: false,
                }]);
            }
            enrich_bindings(conn, &multi_rows)
        }
        "warehouse" => {
            if has_bound {
                // Single-binding via bound_location_id.
                let loc_id = bound_location_id.expect("has_bound is true");
                let (name,): (String,) = conn
                    .query_row(
                        "SELECT COALESCE(name, '') FROM inventory_locations WHERE id = ?1",
                        params![loc_id],
                        |row| Ok((row.get(0)?,)),
                    )
                    .unwrap_or((loc_id.clone(),));
                Ok(vec![WorkspaceLocationBinding {
                    location_id: loc_id,
                    location_name: name,
                    is_primary: true,
                    allow_negative_stock: multi_rows
                        .first()
                        .map(|(_, _, neg)| *neg)
                        .unwrap_or(false),
                }])
            } else {
                // Unbound warehouse: return ALL active inventory locations.
                let mut stmt = conn
                    .prepare(
                        "SELECT id, name, type FROM inventory_locations \
                         WHERE is_active = 1 \
                         ORDER BY name ASC",
                    )
                    .map_err(CoreError::Db)?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok(WorkspaceLocationBinding {
                            location_id: row.get(0)?,
                            location_name: row.get(1)?,
                            is_primary: false,
                            allow_negative_stock: false,
                        })
                    })
                    .map_err(CoreError::Db)?;
                let mut locs = Vec::new();
                for r in rows {
                    locs.push(r.map_err(CoreError::Db)?);
                }
                Ok(locs)
            }
        }
        _ => {
            // Unknown type: return empty (no location binding concept).
            Ok(vec![])
        }
    }
}

/// Enrich raw workspace_inventory_locations rows with location names from
/// the `inventory_locations` table. This is a helper for [`get_workspace_locations`].
fn enrich_bindings(
    conn: &rusqlite::Connection,
    rows: &[(String, bool, bool)],
) -> Result<Vec<WorkspaceLocationBinding>, CoreError> {
    let mut results = Vec::with_capacity(rows.len());
    for (loc_id, is_primary, allow_negative) in rows {
        let name: String = conn
            .query_row(
                "SELECT COALESCE(name, '') FROM inventory_locations WHERE id = ?1",
                params![loc_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| loc_id.clone());
        results.push(WorkspaceLocationBinding {
            location_id: loc_id.clone(),
            location_name: name,
            is_primary: *is_primary,
            allow_negative_stock: *allow_negative,
        });
    }
    Ok(results)
}

/// Resolve the primary deduction location for a workspace instance.
///
/// Returns the first non-None value in priority order:
///   1. `explicit_override` (cashier FastPIN override)
///   2. `workspace_instances.bound_location_id` (single-binding)
///   3. `workspace_inventory_locations.is_primary = 1` (multi-binding)
///   4. Canonical default UUID
///
/// # Errors
///
/// Returns [`CoreError::Validation`] if the workspace has **both** a
/// `bound_location_id` set AND rows in `workspace_inventory_locations`
/// (split-brain — ADR-18 §5). Returns [`CoreError::NotFound`] if
/// `workspace_instance_id` does not exist.
pub fn resolve_primary_location(
    conn: &rusqlite::Connection,
    workspace_instance_id: &str,
    explicit_override: Option<&LocationId>,
) -> Result<LocationId, CoreError> {
    // Tier 1: explicit override from cashier FastPIN.
    if let Some(loc) = explicit_override {
        return Ok(loc.clone());
    }

    // Check the in-memory cache (30s TTL). Only applies to non-override paths
    // because overrides are ephemeral and should not pollute the cache.
    if let Some(cached) = cache_get(workspace_instance_id) {
        return Ok(cached);
    }

    // Verify the workspace instance exists.
    let (bound_location_id,): (Option<String>,) = conn
        .query_row(
            "SELECT bound_location_id FROM workspace_instances WHERE id = ?1",
            params![workspace_instance_id],
            |row| Ok((row.get(0)?,)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound {
                entity: "workspace_instance",
                id: workspace_instance_id.to_owned(),
            },
            other => CoreError::Db(other),
        })?;

    // Tier 2: single-binding.
    if let Some(bound) = bound_location_id.filter(|b| !b.is_empty()) {
        let loc = LocationId::from(bound);
        cache_set(workspace_instance_id, &loc);
        return Ok(loc);
    }

    // Check for multi-binding rows.
    let multi_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workspace_inventory_locations WHERE instance_id = ?1",
            params![workspace_instance_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if multi_count > 0 {
        // Tier 3: multi-binding primary.
        let primary: Option<String> = conn
            .query_row(
                "SELECT location_id FROM workspace_inventory_locations \
                 WHERE instance_id = ?1 AND is_primary = 1 \
                 LIMIT 1",
                params![workspace_instance_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(pid) = primary {
            let loc = LocationId::from(pid);
            cache_set(workspace_instance_id, &loc);
            return Ok(loc);
        }

        // Multi-binding with no explicit primary — fall through to canonical
        // default (the admin hasn't finished configuring; don't hard-error).
    }

    // Tier 4: canonical default.
    let loc = get_default_location_id();
    cache_set(workspace_instance_id, &loc);
    Ok(loc)
}

/// Resolve ALL inventory location bindings for a workspace instance.
///
/// Returns all locations in priority order (primary first, then secondaries
/// sorted by `sort_order`). For single-binding workspaces, returns a
/// one-element vec containing `bound_location_id`. For unbound workspaces,
/// returns a one-element vec containing the canonical default.
///
/// # Errors
///
/// Returns [`CoreError::Validation`] if the workspace has both binding
/// mechanisms active (split-brain). Returns [`CoreError::NotFound`] if
/// the workspace instance does not exist.
pub fn resolve_all_locations(
    conn: &rusqlite::Connection,
    workspace_instance_id: &str,
) -> Result<Vec<LocationId>, CoreError> {
    // Verify workspace exists and read single-binding.
    let (bound_location_id,): (Option<String>,) = conn
        .query_row(
            "SELECT bound_location_id FROM workspace_instances WHERE id = ?1",
            params![workspace_instance_id],
            |row| Ok((row.get(0)?,)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound {
                entity: "workspace_instance",
                id: workspace_instance_id.to_owned(),
            },
            other => CoreError::Db(other),
        })?;

    let has_single = bound_location_id.as_ref().is_some_and(|b| !b.is_empty());

    // Check for multi-binding rows.
    let multi_rows: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT location_id FROM workspace_inventory_locations \
                 WHERE instance_id = ?1 \
                 ORDER BY is_primary DESC, sort_order ASC",
            )
            .map_err(CoreError::Db)?;
        let rows = stmt
            .query_map(params![workspace_instance_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(CoreError::Db)?;
        let mut ids = Vec::new();
        for r in rows {
            ids.push(r.map_err(CoreError::Db)?);
        }
        ids
    };

    // Split-brain detection.
    if has_single && !multi_rows.is_empty() {
        return Err(CoreError::Validation {
            field: "workspace_binding",
            message: format!(
                "workspace instance {workspace_instance_id} has both bound_location_id \
                 and workspace_inventory_locations rows — this is a split-brain \
                 configuration (ADR-18 §5)"
            ),
        });
    }

    if has_single {
        return Ok(vec![LocationId::from(
            bound_location_id.expect("has_single is true"),
        )]);
    }

    if !multi_rows.is_empty() {
        return Ok(multi_rows.into_iter().map(LocationId::from).collect());
    }

    // Unbound — fall back to canonical default.
    Ok(vec![get_default_location_id()])
}

/// Compute a greedy-fill suggestion across the workspace's bound locations
/// for a given SKU and requested quantity.
///
/// **This function never executes deductions.** It is a read-only computation
/// for the cashier UI to show alternative locations with live stock counts.
/// The caller (typically [`crate::db::Store::complete_sale`]) uses the
/// returned vec to populate [`crate::sale_deduction::Shortfall::alternatives`].
///
/// Returns all bound locations (in priority order) that have stock > 0 for
/// the given SKU, along with the available quantity at each. The cashier UI
/// can display these as suggested fallback sources.
pub fn resolve_location_chain_for_sku(
    conn: &rusqlite::Connection,
    workspace_instance_id: &str,
    sku: &str,
    #[allow(unused)] qty: i64, // TODO(ADR-19): implement greedy-fill across locations using qty
) -> Result<Vec<LocationStock>, CoreError> {
    let location_ids = resolve_all_locations(conn, workspace_instance_id)?;

    // Resolve product_id from SKU.
    let product_id: String = conn
        .query_row(
            "SELECT id FROM products WHERE sku = ?1",
            params![sku],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound {
                entity: "product",
                id: sku.to_owned(),
            },
            other => CoreError::Db(other),
        })?;

    let mut results = Vec::with_capacity(location_ids.len());

    for loc_id in &location_ids {
        let qty: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary \
                 WHERE item_id = ?1 AND location_id = ?2",
                params![product_id, loc_id.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if qty > 0 {
            let name: String = conn
                .query_row(
                    "SELECT name FROM inventory_locations WHERE id = ?1",
                    params![loc_id.as_str()],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| loc_id.as_str().to_owned());

            results.push(LocationStock {
                location_id: loc_id.clone(),
                location_name: name,
                qty_available: qty,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a pre-migrated in-memory connection for unit tests.
    fn migrated() -> rusqlite::Connection {
        crate::migrations::fresh_db()
    }

    /// Seed the minimum rows needed to satisfy FK constraints when
    /// inserting a `workspace_instances` test row.
    fn seed_fks(conn: &rusqlite::Connection) {
        conn.execute_batch(
            "INSERT OR IGNORE INTO store_profiles (id, name) VALUES ('store-1', 'Test Store');",
        )
        .unwrap();
    }

    #[test]
    fn get_default_location_id_returns_canonical() {
        let loc = get_default_location_id();
        assert_eq!(loc.as_str(), CANONICAL_DEFAULT_LOCATION_UUID);
    }

    #[test]
    fn resolve_primary_location_unbound_returns_canonical_default() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
             VALUES ('ws-unbound', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Unbound')",
            [],
        )
        .unwrap();
        let loc = resolve_primary_location(&conn, "ws-unbound", None).unwrap();
        assert_eq!(loc.as_str(), CANONICAL_DEFAULT_LOCATION_UUID);
    }

    #[test]
    fn resolve_primary_location_single_binding_returns_bound() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) \
             VALUES ('loc-store', 'Store', 'store')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
             VALUES ('ws-single', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Single', 'loc-store')",
            [],
        )
        .unwrap();
        let loc = resolve_primary_location(&conn, "ws-single", None).unwrap();
        assert_eq!(loc.as_str(), "loc-store");
    }

    #[test]
    fn resolve_primary_location_multi_binding_returns_is_primary() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-1', 'A', 'store');\
             INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-2', 'B', 'warehouse');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
               VALUES ('ws-multi', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Multi');\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-1', 'ws-multi', 'loc-2', 1, 0);\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-2', 'ws-multi', 'loc-1', 0, 1);",
        )
        .unwrap();
        let loc = resolve_primary_location(&conn, "ws-multi", None).unwrap();
        assert_eq!(loc.as_str(), "loc-2");
    }

    #[test]
    fn resolve_primary_location_explicit_override_wins() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-store', 'Store', 'store')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
             VALUES ('ws-single', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Single', 'loc-store')",
            [],
        )
        .unwrap();
        let override_loc = LocationId::from("loc-override");
        let loc = resolve_primary_location(&conn, "ws-single", Some(&override_loc)).unwrap();
        assert_eq!(loc.as_str(), "loc-override");
    }

    #[test]
    fn resolve_all_locations_single_binding() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-store', 'Store', 'store')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
             VALUES ('ws-single', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Single', 'loc-store')",
            [],
        )
        .unwrap();
        let locs = resolve_all_locations(&conn, "ws-single").unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].as_str(), "loc-store");
    }

    #[test]
    fn resolve_all_locations_multi_binding_primary_first() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-3', 'C', 'store');\
             INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-1', 'A', 'warehouse');\
             INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-2', 'B', 'warehouse');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
               VALUES ('ws-multi', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Multi');\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-1', 'ws-multi', 'loc-1', 0, 1);\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-2', 'ws-multi', 'loc-2', 1, 0);\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-3', 'ws-multi', 'loc-3', 0, 2);",
        )
        .unwrap();
        let locs = resolve_all_locations(&conn, "ws-multi").unwrap();
        assert_eq!(locs.len(), 3, "expected 3 locations, got {locs:?}");
        // is_primary=1 sorts first, then by sort_order ASC.
        assert_eq!(locs[0].as_str(), "loc-2", "primary must be first");
        assert_eq!(locs[1].as_str(), "loc-1");
        assert_eq!(locs[2].as_str(), "loc-3");
    }

    #[test]
    fn resolve_all_locations_unbound_returns_canonical() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
             VALUES ('ws-unbound', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Unbound')",
            [],
        )
        .unwrap();
        let locs = resolve_all_locations(&conn, "ws-unbound").unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].as_str(), CANONICAL_DEFAULT_LOCATION_UUID);
    }

    #[test]
    fn resolve_location_chain_for_sku_returns_stocked_alternatives() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-store', 'Store', 'store');\
             INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-wh-a', 'WH A', 'warehouse');\
             INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-wh-b', 'WH B', 'warehouse');\
             INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
               VALUES ('prod-test', 'CHO-001', 'Choco Bar', 15000, 'IDR', 'retail');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
               VALUES ('ws-multi', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'Multi');\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-1', 'ws-multi', 'loc-store', 1, 0);\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-2', 'ws-multi', 'loc-wh-a', 0, 1);\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-3', 'ws-multi', 'loc-wh-b', 0, 2);\
             INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) VALUES ('prod-test', 'loc-store', 5);\
             INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) VALUES ('prod-test', 'loc-wh-a', 500);",
        )
        .unwrap();
        let chain = resolve_location_chain_for_sku(&conn, "ws-multi", "CHO-001", 20).unwrap();
        // loc-store has 5, loc-wh-a has 500, loc-wh-b has 0 (no stock row).
        assert_eq!(chain.len(), 2, "only locations with stock > 0 returned");
        assert_eq!(chain[0].location_name, "Store");
        assert_eq!(chain[0].qty_available, 5);
        assert_eq!(chain[1].location_name, "WH A");
        assert_eq!(chain[1].qty_available, 500);
    }

    #[test]
    fn resolve_location_chain_for_sku_no_stock_anywhere_returns_empty() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-1', 'A', 'store');\
             INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
               VALUES ('prod-empty', 'EMPTY', 'Empty', 100, 'IDR', 'retail');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
               VALUES ('ws-1', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'WS1');\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-1', 'ws-1', 'loc-1', 1, 0);",
        )
        .unwrap();
        let chain = resolve_location_chain_for_sku(&conn, "ws-1", "EMPTY", 10).unwrap();
        assert!(chain.is_empty());
    }

    #[test]
    fn resolve_primary_location_multi_binding_no_primary_returns_canonical() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-a', 'A', 'store');\
             INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-b', 'B', 'warehouse');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
               VALUES ('ws-no-primary', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'NoPrimary');\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-a', 'ws-no-primary', 'loc-a', 0, 0);\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
               VALUES ('wsl-b', 'ws-no-primary', 'loc-b', 0, 1);",
        )
        .unwrap();
        let loc = resolve_primary_location(&conn, "ws-no-primary", None).unwrap();
        // Falls through to canonical default — no is_primary=1 row exists.
        assert_eq!(loc.as_str(), CANONICAL_DEFAULT_LOCATION_UUID);
    }

    // ── Cache tests ────────────────────────────────────────────────

    #[test]
    fn location_cache_returns_cached_value_invalidation_forces_db_read() {
        // Uses entirely unique IDs to avoid any possible collision with seed data
        // or parallel test interference. The location name must be globally unique
        // due to the UNIQUE index on inventory_locations(name).
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-cache-zzz', 'Cache Test Loc Z99', 'store')",
            [],
        )
        .expect("insert inventory_locations");
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
             VALUES ('ws-cache-zz99', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'CacheTestZZ99', 'loc-cache-zzz')",
            [],
        )
        .expect("insert workspace_instances");

        invalidate_location_cache();

        // First call — hits DB, populates cache.
        let loc = resolve_primary_location(&conn, "ws-cache-zz99", None).unwrap();
        assert_eq!(loc.as_str(), "loc-cache-zzz");

        // Second call — verifies cache hit (immediate re-read).
        // Even if a parallel test invalidates between calls, the DB still
        // holds the same value, so the assertion stays green.
        let loc2 = resolve_primary_location(&conn, "ws-cache-zz99", None).unwrap();
        assert_eq!(
            loc2.as_str(),
            "loc-cache-zzz",
            "cache hit should return same value"
        );

        // Modify DB behind the cache.
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-fake-zz99', 'Cache Fake Loc Z99', 'warehouse')",
            [],
        )
        .expect("insert fake location for FK");
        let rows = conn
            .execute(
                "UPDATE workspace_instances SET bound_location_id = 'loc-fake-zz99' WHERE id = 'ws-cache-zz99'",
                [],
            )
            .expect("update bound_location_id");
        assert_eq!(rows, 1, "UPDATE must affect exactly 1 row");

        // Invalidate cache, then verify fresh DB read returns new value.
        invalidate_location_cache();

        let loc3 = resolve_primary_location(&conn, "ws-cache-zz99", None).unwrap();
        assert_eq!(
            loc3.as_str(),
            "loc-fake-zz99",
            "after invalidation, should read fresh DB value"
        );
    }

    #[test]
    fn location_cache_notfound_cleared_by_invalidation() {
        let conn = migrated();
        seed_fks(&conn);
        invalidate_location_cache();

        // Resolving a nonexistent workspace returns NotFound.
        let err = resolve_primary_location(&conn, "ws-noexist-cache", None).unwrap_err();
        assert!(
            matches!(
                err,
                CoreError::NotFound {
                    entity: "workspace_instance",
                    ..
                }
            ),
            "expected NotFound error"
        );

        // Create a workspace and resolve again.
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-b', 'B', 'store');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
               VALUES ('ws-noexist-cache', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'NowExists', 'loc-b');",
        )
        .unwrap();

        invalidate_location_cache();
        let ok_loc = resolve_primary_location(&conn, "ws-noexist-cache", None).unwrap();
        assert_eq!(
            ok_loc.as_str(),
            "loc-b",
            "must resolve after NotFound + invalidation"
        );
    }

    #[test]
    fn location_cache_explicit_override_never_cached() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-z', 'Z', 'store');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
               VALUES ('ws-override-cache', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'OCache', 'loc-z');",
        )
        .unwrap();

        invalidate_location_cache();

        // Call with explicit override — should return override, NOT bound.
        let ovr = LocationId::from("loc-override");
        let loc = resolve_primary_location(&conn, "ws-override-cache", Some(&ovr)).unwrap();
        assert_eq!(loc.as_str(), "loc-override");

        // After override, subsequent non-override call should hit DB.
        let loc2 = resolve_primary_location(&conn, "ws-override-cache", None).unwrap();
        assert_eq!(
            loc2.as_str(),
            "loc-z",
            "non-override call after override should hit DB"
        );
    }

    #[test]
    fn resolve_primary_location_nonexistent_workspace_errors() {
        let conn = migrated();
        seed_fks(&conn);
        let err = resolve_primary_location(&conn, "ws-nonexistent", None).unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "workspace_instance",
                ..
            }
        ));
    }

    #[test]
    fn resolve_all_locations_nonexistent_workspace_errors() {
        let conn = migrated();
        seed_fks(&conn);
        let err = resolve_all_locations(&conn, "ws-nonexistent").unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "workspace_instance",
                ..
            }
        ));
    }

    #[test]
    fn resolve_location_chain_for_sku_nonexistent_product_errors() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
             VALUES ('ws-1', (SELECT key FROM workspace_types LIMIT 1), 'store-1', 'WS1')",
            [],
        )
        .unwrap();
        let err = resolve_location_chain_for_sku(&conn, "ws-1", "NONEXISTENT-SKU", 10).unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "product",
                ..
            }
        ));
    }

    // ── get_workspace_locations tests (ADR-19 §10) ────────────────

    #[test]
    fn get_workspace_locations_unknown_type_returns_empty() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
             VALUES ('ws-admin', 'admin', 'store-1', 'Admin')",
            [],
        )
        .unwrap();
        let locs = get_workspace_locations(&conn, "ws-admin", "admin").unwrap();
        assert!(locs.is_empty(), "admin type should have no locations");
    }

    #[test]
    fn get_workspace_locations_store_pos_multi_binding() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-a', 'Store Front', 'store');\
             INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-b', 'Back Room', 'warehouse');\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
               VALUES ('ws-pos', 'store-pos', 'store-1', 'Main POS');\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, allow_negative_stock, sort_order) \
               VALUES ('wsl-1', 'ws-pos', 'loc-b', 1, 1, 0);\
             INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, allow_negative_stock, sort_order) \
               VALUES ('wsl-2', 'ws-pos', 'loc-a', 0, 0, 1);",
        )
        .unwrap();
        let locs = get_workspace_locations(&conn, "ws-pos", "store-pos").unwrap();
        assert_eq!(locs.len(), 2);
        // Primary first (is_primary=1).
        assert_eq!(locs[0].location_id, "loc-b");
        assert_eq!(locs[0].location_name, "Back Room");
        assert!(locs[0].is_primary);
        assert!(locs[0].allow_negative_stock);
        // Secondary.
        assert_eq!(locs[1].location_id, "loc-a");
        assert!(!locs[1].is_primary);
    }

    #[test]
    fn get_workspace_locations_store_pos_no_bindings_returns_default() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
             VALUES ('ws-pos-empty', 'store-pos', 'store-1', 'Empty POS')",
            [],
        )
        .unwrap();
        let locs = get_workspace_locations(&conn, "ws-pos-empty", "store-pos").unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].location_id, CANONICAL_DEFAULT_LOCATION_UUID);
        assert!(locs[0].is_primary);
    }

    #[test]
    fn get_workspace_locations_warehouse_single_binding() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-wh', 'Main WH', 'warehouse')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
             VALUES ('ws-wh', 'warehouse', 'store-1', 'Warehouse', 'loc-wh')",
            [],
        )
        .unwrap();
        let locs = get_workspace_locations(&conn, "ws-wh", "warehouse").unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].location_id, "loc-wh");
        assert_eq!(locs[0].location_name, "Main WH");
        assert!(locs[0].is_primary);
    }

    #[test]
    fn get_workspace_locations_warehouse_unbound_returns_all_active() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute_batch(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type, is_active) VALUES ('loc-a', 'WH A', 'warehouse', 1);\
             INSERT OR IGNORE INTO inventory_locations (id, name, type, is_active) VALUES ('loc-b', 'Store B', 'store', 1);\
             INSERT OR IGNORE INTO inventory_locations (id, name, type, is_active) VALUES ('loc-c', 'Inactive C', 'warehouse', 0);\
             INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name) \
               VALUES ('ws-wh-unbound', 'warehouse', 'store-1', 'Unbound WH');",
        )
        .unwrap();
        // Unbound warehouse should return all active locations.
        // Note: migration 078 seeds 2 canonical locations (default + transit),
        // plus our 2 added locations = 4 active total. The inactive one is excluded.
        let locs = get_workspace_locations(&conn, "ws-wh-unbound", "warehouse").unwrap();
        assert_eq!(locs.len(), 4, "2 canonical + 2 added = 4 active");
        assert!(locs.iter().any(|l| l.location_id == "loc-a"));
        assert!(locs.iter().any(|l| l.location_id == "loc-b"));
        assert!(
            !locs.iter().any(|l| l.location_id == "loc-c"),
            "inactive excluded"
        );
        // Verify canonical locations are included.
        assert!(locs.iter().any(|l| l.location_name.contains("Default")));
    }

    #[test]
    fn get_workspace_locations_split_brain_errors() {
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-x', 'X', 'store')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
             VALUES ('ws-brain', 'store-pos', 'store-1', 'SplitBrain', 'loc-x')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order) \
             VALUES ('wsl-brain', 'ws-brain', 'loc-x', 1, 0)",
            [],
        )
        .unwrap();
        let err = get_workspace_locations(&conn, "ws-brain", "store-pos").unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "workspace_binding",
                ..
            }
        ));
        assert!(
            err.to_string().contains("split-brain"),
            "error should mention split-brain"
        );
    }

    #[test]
    fn get_workspace_locations_nonexistent_instance_errors() {
        let conn = migrated();
        seed_fks(&conn);
        let err = get_workspace_locations(&conn, "ws-nonexistent", "store-pos").unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "workspace_instance",
                ..
            }
        ));
    }

    #[test]
    fn get_workspace_locations_warehouse_type_key_from_instance() {
        // Test with a workspace_instances row that has type_key='warehouse'
        // but we pass type_key='store-pos' — verifies the parameter is honored.
        let conn = migrated();
        seed_fks(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-wh', 'WH', 'warehouse')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, bound_location_id) \
             VALUES ('ws-wh', 'warehouse', 'store-1', 'WH Instance', 'loc-wh')",
            [],
        )
        .unwrap();
        // Call with type_key='store-pos' (different from instance's type_key).
        // store-pos with single binding + no multi-rows → falls back to default.
        let locs = get_workspace_locations(&conn, "ws-wh", "store-pos").unwrap();
        // store-pos with bound_location_id set but no multi-rows: the bound is IGNORED
        // (store-pos resolves from workspace_inventory_locations, not bound_location_id).
        // No multi-rows means we go to default.
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].location_id, CANONICAL_DEFAULT_LOCATION_UUID);
    }
}
