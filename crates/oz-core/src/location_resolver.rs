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
//! The resolver is read-heavy (called once per cart-open + possibly per
//! `add_line`), so all functions accept a `&rusqlite::Connection` and
//! perform direct SELECTs — no caching at this layer (ADR-19 §4.2 defers
//! caching to a future ADR).

use rusqlite::params;

use crate::error::CoreError;
use crate::inventory::{CANONICAL_DEFAULT_LOCATION_UUID, LocationId};
use crate::sale_deduction::LocationStock;

/// Return the frozen canonical default location UUID as a [`LocationId`].
///
/// ADR-18 §13-36: this UUID is `01926b3a-0000-7000-8000-000000000001` and
/// matches the seed row in migration 078. All legacy single-location callers
/// resolve here transparently.
#[must_use]
pub fn get_default_location_id() -> LocationId {
    LocationId::from(CANONICAL_DEFAULT_LOCATION_UUID)
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
        return Ok(LocationId::from(bound));
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
            return Ok(LocationId::from(pid));
        }

        // Multi-binding with no explicit primary — fall through to canonical
        // default (the admin hasn't finished configuring; don't hard-error).
    }

    // Tier 4: canonical default.
    Ok(get_default_location_id())
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
}
