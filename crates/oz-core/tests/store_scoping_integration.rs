//! Integration tests for store-scoped repository queries.
//!
//! The raw-SQL cross-store audit lives in the migrations unit module
//! (`migrations::tests::store_scoped_*` and friends). This suite proves
//! the SAME isolation guarantee through the real repository API — the
//! store-scoped workspace-instance calls in `db::workspaces`
//! (`list_all_instances`, `count_active_instances`) filter
//! `WHERE store_id = ?1`, and no cross-store row can surface through
//! them.
//!
//! Why the workspace-instance layer and not `list_products`? The product
//! catalog is deliberately GLOBAL in the per-store-database model — each
//! store opens its own `store-<id>.sqlite` file (see
//! docs/decisions/2026-07-10-workspace-type-instance-design.md), so
//! `store_id` on products is the soft-scoping layer for shared/cloud
//! databases only and `list_products` intentionally has no store filter.
//! The genuinely store-scoped repository API is the workspace-instance
//! layer exercised here. Note `workspace_instances.store_id` is NOT NULL
//! with an FK to `store_profiles(id)` (created in migration 060, rebuilt
//! with ON DELETE RESTRICT in migration 066) — so unlike the domain
//! tables there is NO NULL global-sentinel ambiguity at all: every
//! instance belongs to exactly one store, and a scoped listing can only
//! ever return that store's rows.

use oz_core::{Store, StoreProfile, migrations};
use rusqlite::Connection;

// ── Helpers ───────────────────────────────────────────────────────────

/// Open an in-memory database with the full migration set applied.
fn setup() -> Connection {
    migrations::fresh_db()
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

fn make_profile(id: &str, name: &str) -> StoreProfile {
    StoreProfile {
        id: id.to_owned(),
        name: name.to_owned(),
        address: String::new(),
        tax_id: String::new(),
        currency: "USD".to_owned(),
        timezone: "UTC".to_owned(),
        is_primary: false,
        created_at: "2026-07-01T10:00:00Z".to_owned(),
        updated_at: "2026-07-01T10:00:00Z".to_owned(),
    }
}

/// Seed store-a / store-b profiles plus a workspace instance in each.
///
/// `workspace_instances.store_id` is NOT NULL with an FK to
/// `store_profiles(id)`, so every instance created here is owned by
/// exactly one store.
fn seed_two_stores(conn: &Connection) {
    let s = store(conn);
    s.create_store_profile(&make_profile("store-a", "Store A"))
        .unwrap();
    s.create_store_profile(&make_profile("store-b", "Store B"))
        .unwrap();
    s.create_workspace_instance("ws-a-1", "store-pos", "store-a", "A POS", "", None)
        .unwrap();
    s.create_workspace_instance("ws-a-2", "warehouse", "store-a", "A WH", "", None)
        .unwrap();
    s.create_workspace_instance("ws-b-1", "admin", "store-b", "B Admin", "", None)
        .unwrap();
}

// ── Store-scoped listing ──────────────────────────────────────────────

/// The core API-layer audit: `list_all_instances(store_id)` must return
/// exactly that store's instances — never another store's row. Both
/// directions are asserted so isolation holds symmetrically.
#[test]
fn list_all_instances_returns_only_that_stores_instances() {
    let conn = setup();
    seed_two_stores(&conn);
    let s = store(&conn);

    let a = s.list_all_instances("store-a").unwrap();
    let a_ids: Vec<&str> = a.iter().map(|r| r.id.as_str()).collect();
    // ORDER BY name: "A POS" < "A WH"
    assert_eq!(a_ids, vec!["ws-a-1", "ws-a-2"]);
    assert!(
        a.iter().all(|r| r.store_id == "store-a"),
        "every returned instance must belong to store-a"
    );
    assert!(
        a.iter().all(|r| r.id != "ws-b-1"),
        "store-b's instance must never surface through a store-a scoped listing"
    );

    let b = s.list_all_instances("store-b").unwrap();
    let b_ids: Vec<&str> = b.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(b_ids, vec!["ws-b-1"]);
    assert!(
        b.iter().all(|r| r.store_id == "store-b"),
        "every returned instance must belong to store-b"
    );
    assert!(
        b.iter().all(|r| r.id != "ws-a-1" && r.id != "ws-a-2"),
        "store-a's instances must never surface through a store-b scoped listing"
    );
}

/// `count_active_instances(store_id)` must count only that store's
/// instances — a cross-store count would inflate (or leak) another
/// store's rows.
#[test]
fn count_active_instances_is_store_scoped() {
    let conn = setup();
    seed_two_stores(&conn);
    let s = store(&conn);

    assert_eq!(s.count_active_instances("store-a").unwrap(), 2);
    assert_eq!(s.count_active_instances("store-b").unwrap(), 1);
}

/// An empty store scopes to zero instances — a scoped call never falls
/// back to other stores' rows.
#[test]
fn list_all_instances_empty_for_store_without_instances() {
    let conn = setup();
    seed_two_stores(&conn);
    let s = store(&conn);

    s.create_store_profile(&make_profile("store-c", "Store C"))
        .unwrap();
    let c = s.list_all_instances("store-c").unwrap();
    assert!(
        c.is_empty(),
        "store-c has no instances; scoped list must be empty"
    );
    assert_eq!(s.count_active_instances("store-c").unwrap(), 0);
}

// ── Ownership enforcement through the API ─────────────────────────────

/// The 066 FK (`workspace_instances.store_id` → `store_profiles(id)`,
/// ON DELETE RESTRICT) is enforced on the write path: a caller cannot
/// create an instance for a store that does not exist. The failure is
/// pinned to the FK mechanism itself (a constraint violation, not some
/// unrelated error) by asserting the message names the foreign key.
#[test]
fn create_workspace_instance_for_missing_store_rejected() {
    let conn = setup();
    let s = store(&conn);

    let err =
        s.create_workspace_instance("ws-ghost", "store-pos", "ghost-store", "Ghost", "", None);
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("FOREIGN KEY"),
        "creating an instance for a missing store_profile must fail the FK, got: {msg}"
    );
}
