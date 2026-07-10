# ADR #6: CRDT Delta Ledger & Offline Sync

**Status:** In Progress (Phase 1 Complete 2026-07-10)
**Date:** 2026-07-10
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** crdt, offline, sync, inventory, uuidv7, ulid, concurrency

---

## Context

ADR #4 establishes the **Store-First Tenancy & Workspace Type/Instance Architecture**, enabling multiple POS registers per store and multiple stores per tenant. In multi-register environments, offline connectivity must not cause ID collisions, inventory race conditions, or cross-register data overwrites. This ADR defines the offline-safe data layer that supports concurrent registers operating without a live cloud connection.

This ADR is intentionally separate from ADR #4 because the CRDT delta ledger is a major data-model change that deserves its own design and implementation cycle.

### Relationship to Store-Scoped Databases

ADR #4 stores each store's data in a separate SQLite file. Within a single store, all registers share the same database file — so **within-store** conflicts are handled by SQLite's own locking. **Cross-store** sync (e.g., inventory transfer between Store A and Store B, or chain-wide reporting) goes through the cloud sync layer (`platform/sync/`). The CRDT delta ledger model applies to both scopes:

- **Within a store:** Multiple registers writing to the same `stock_movements` table in the same SQLite file — SQLite's WAL mode handles concurrent writers. UUIDv7 primary keys prevent ID collisions.
- **Across stores:** Each store's `stock_movements` deltas are synced to the cloud and merged deterministically without `last-write-wins` data loss.

---

## Decision

### 1. Time-Ordered UUIDv7 / ULID Primary Keys

All entity primary keys (`orders`, `order_lines`, `payments`, `stock_transfers`, `stock_movements`) use time-ordered **UUIDv7 / ULID** keys (`01F8MECH...`), eliminating ID collisions when multiple registers operate offline locally. This applies both within a single store database (multiple terminals) and across stores (globally unique IDs for sync).

### 2. CRDT Delta Ledger for Inventory

Offline registers never overwrite absolute `quantity` columns directly (`last-write-wins`). They only insert immutable delta ledger rows into `stock_movements` (`+5` or `-2`). When registers reconnect and sync, deltas sum up deterministically with zero race conditions or missing stock.

```sql
CREATE TABLE stock_movements (
    id                  TEXT PRIMARY KEY,  -- UUIDv7
    store_id            TEXT NOT NULL,     -- Boot-time validation: must match the database's owning store.
                                           -- Redundant in store-scoped DBs (the DB file IS the scope),
                                           -- but serves as a sanity check and sync routing key.
    item_id             TEXT NOT NULL,
    delta               INTEGER NOT NULL,  -- +N or -N
    reason              TEXT,
    created_at          TEXT NOT NULL,
    source_terminal_id  TEXT NOT NULL
);
```

Current stock quantity is computed as:

```sql
-- Within a store database, store_id filtering is implicit (all rows belong to this store)
SELECT SUM(delta) FROM stock_movements WHERE item_id = ?;
```

For performance, maintain a materialized `stock_summary` table or in-memory cache that is invalidated on each sync and rebuilt from the delta ledger. This avoids aggregating the full history on every inventory lookup.

**Cross-store sync:** When deltas are synced to the cloud, the `store_id` column serves as a routing key — the cloud knows which store's inventory the delta belongs to. During cross-store transfers (e.g., Store A ships 10 units to Store B), Store A inserts a `-10` delta, Store B receives a `+10` delta via sync.

### 3. Terminal Keyring Binding

Every POS hardware device holds a cryptographic profile (`terminal_id` bound via OS keyring, `oz-security::Keyring`). Sessions require the full scope validation:

```
(user_id + terminal_id + store_id + instance_id)
```

The `store_id` is included per ADR #4's `SessionContext` — it is resolved during session creation and never changes for the lifetime of the session. Terminal binding ensures that sync deltas (`source_terminal_id`) are traceable to a specific device at a specific store.

ADR #4's Security Architecture adds an HMAC signature on device bindings. The terminal keyring binding described here works in tandem: the OS keyring stores the terminal's identity, while the HMAC signature prevents tampering with the binding in the global database.

### 4. Shared Touchscreen Fast-Switching (`FastPINOverlay.tsx`)

To prevent session hijacking on shared touchscreens without slowing service down, the frontend overlays a **Quick Staff PIN Pad (`FastPINOverlay.tsx`)**. Upon PIN verification, the backend issues a **new session token** with the new user's resolved scope, while keeping `terminal_id` and `instance_id` invariant (the device doesn't change, only the operator does).

This aligns with ADR #4's Security Architecture (Section 4): user switching always triggers token invalidation and re-resolution. The old session token is removed from the backend's `SessionStore`, and a new opaque token is issued. This ensures:

- Perfect audit logs per operator (every order, stock movement, and payment is tagged with the correct `user_id`)
- Cash drawer accountability (the session's `user_id` is bound to cash operations)
- No stale scope leakage (the new token carries the new user's `user_store_access` and `user_workspace_instances`)

The `terminal_id` and `instance_id` remain invariant through the switch because the physical device and its assigned workspace don't change — only the operator does.

### 5. Immutable Transaction Clock

Every record tracks `version INTEGER` and `updated_at TEXT` so when offline registers reconnect to each other or `cloud-server`, optimistic concurrency checks prevent ghost overwrites.

### 6. Orphan Prevention

All foreign keys to `store_profiles` in the **global database** explicitly enforce `ON DELETE RESTRICT ON UPDATE CASCADE`, making it impossible to delete a location while it has active registers, open shifts, or historical transactions. In per-store databases, `store_id` columns are logical references (no FK constraint — see ADR #4's schema), validated at boot time rather than enforced by the database engine. The global database is the authoritative source for store existence; per-store databases trust it.

---

## Consequences

### Positive

- No ID collisions across offline registers (UUIDv7/ULID).
- Inventory deltas merge deterministically without last-write-wins data loss.
- Terminal binding prevents session spoofing.
- Optimistic concurrency prevents ghost overwrites.
- Cross-store inventory transfers are mediated by the CRDT model — no double-counting.

### Negative

- Requires migrating existing auto-increment IDs to UUIDv7/ULID.
- Inventory queries must aggregate `stock_movements` instead of reading a single `quantity` column.
- Stock corrections and adjustments must be modeled as delta rows (a correction is a compensating delta, e.g., `+3` if the actual count is 3 higher than the computed sum).
- Sync protocol between registers and cloud-server must handle both within-store (same DB, SQLite WAL) and cross-store (different DBs, cloud-mediated) sync correctly.
- The materialized `stock_summary` cache must be invalidated on every sync and rebuilt — this is a performance consideration for high-volume stores.

---

## Open Questions

1. Should we use UUIDv7 or ULID for primary keys? (UUIDv7 is natively supported in some databases; ULID is more human-readable.)
2. How do we handle stock corrections that need to override a previous incorrect delta? (A compensating delta is the CRDT-consistent approach; an admin override with audit trail is the practical approach.)
3. What is the sync protocol for merging deltas between registers and the cloud? How does it handle the within-store (shared DB) vs cross-store (cloud-mediated) distinction?
4. How do we prune old `stock_movements` rows without losing audit history? (Snapshot + archive approach: periodically materialize the current sum, archive the contributing deltas.)

---

## Implementation Checklist

### Phase 1: Stock Movements Delta Ledger (Done 2026-07-10)

- [x] Create `stock_movements` delta ledger table and `stock_summary` materialized cache (migration 063).
- [x] Register migration in `crates/oz-core/src/migrations.rs`.
- [x] Add `StockMovement` Rust struct with id, item_id, delta, reason, source_terminal_id, source_user_id, created_at.
- [x] Refactor `adjust_stock` → `adjust_stock_with_reason(sku, delta, reason)`: writes delta row to ledger + updates `inventory` + `stock_summary` in one transaction.
- [x] `create_product` writes initial stock to ledger (`reason: 'initial-stock'`).
- [x] `get_stock_from_ledger(product_id)` — computes `SUM(delta)` from ledger; falls back to `inventory.qty` if no rows.
- [x] `list_stock_movements(product_id, limit, offset)` — paginated ledger query for audit/sync.
- [x] 7 new tests: table existence, ledger writes, SUM computation, pagination, stock_summary tracking, creation ledger.
- [x] `cargo check` ✅, `cargo test -p oz-core -- products` ✅ (67/67), `cargo fmt` ✅.

### Phase 2 (Future)

- [x] Choose and adopt UUIDv7 for all entity primary keys — replaced all 158 `Uuid::new_v4()` call sites with `Uuid::now_v7()` across the entire workspace. Added `v7` feature to workspace `uuid` dependency. Added `oz_core::new_id()` helper for future entity ID generation.
- [x] Implement materialized `stock_summary` cache rebuild from deltas — `rebuild_stock_summary()` method recomputes both `stock_summary` and `inventory` tables from `SUM(delta)` in a single transaction. Sync integration point ready but not yet wired. Added 2 new tests.
- [x] Populate `source_terminal_id` and `source_user_id` from session context — `adjust_stock_with_reason` now accepts optional audit params. Scoped `adjust_stock_scoped` Tauri command passes `session.terminal_id` and `session.user_id`. Backward-compat `adjust_stock` passes `None`. Added 2 new tests verifying audit field persistence.
- [x] Add `version` and `updated_at` optimistic concurrency fields to synced entities — migration 065 adds `version INTEGER NOT NULL DEFAULT 1` to both `products` and `sales`. `update_product` accepts `Option<i64>` for gradual adoption. `update_sale_status` and `void_sale` increment version on UPDATE. All struct literals and row mappers updated across the workspace.
- [ ] Implement `FastPINOverlay.tsx` for shared touchscreen user switching.
- [x] Enforce `ON DELETE RESTRICT` on `store_profiles` foreign keys in the global database — migration 066 adds FK to `workspace_instances.store_id` and changes `user_store_access.store_id` from CASCADE to RESTRICT.
- [ ] Cross-store delta routing via `platform/sync/`.

---

## Related

- ADR #4 — Store-First Tenancy & Workspace Type/Instance Architecture
- ADR #5 — Subscription Tier & Entitlement
- `crates/oz-core/src/db/stock.rs`
- `crates/oz-security/src/terminal.rs`
- `platform/sync/` — Cross-store sync layer
- `ui/src/components/FastPINOverlay.tsx` (planned)
