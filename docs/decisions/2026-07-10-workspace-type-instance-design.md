# ADR #4: Workspace Type/Instance Architecture

**Status:** Accepted
**Date:** 2026-07-10
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** architecture, workspaces, multi-store, data-scoping, multi-terminal

---

## Implementation Checklist & Phased Rollout Tracker

Use this master checklist to track execution progress across backend crates (`crates/oz-core`), security layers, Tauri IPC bridges, and frontend React components (`ui/src/`).

### Phase 1: Database Schema & Core Backend Rust Models (`crates/oz-core`)
- [ ] **Step 1.1**: Create SQL migration script (`crates/oz-core/migrations/004_workspace_instances.sql`) defining tables (`workspace_types`, `workspace_type_screens`, `workspace_instances`, `user_workspace_instances`, `role_workspace_types`) with explicit `FOREIGN KEY (store_id) REFERENCES store_profiles(id) ON DELETE RESTRICT ON UPDATE CASCADE` constraints to prevent orphaned register crashes.
- [ ] **Step 1.2**: Add SQL seed & backward-compatibility migration script to insert base templates (`restaurant-pos`, `store-pos`, `kds`, `inventory`, `admin`), auto-generate `default-<key>` instances (`store_id = NULL`), and migrate existing `user_workspaces` rows to `user_workspace_instances`.
- [ ] **Step 1.3**: Define Rust DTOs and structs (`WorkspaceTypeRow`, `WorkspaceInstanceRow`, `WorkspaceDto`) in `crates/oz-core/src/db/workspaces.rs` with fields for `instance_id`, `type_key`, `name`, `description`, `icon`, `colour`, `render_mode`, and `store_id`.
- [ ] **Step 1.4**: Implement core backend queries in `crates/oz-core/src/db/workspaces.rs`:
  - `list_workspaces(&self, role_id: &str, user_id: Option<&str>) -> Result<Vec<WorkspaceDto>>` with exact precedence (`role-owner` bypass → assigned `user_workspace_instances` → fallback to `role_workspace_types`).
  - `get_workspace_instance(&self, instance_id: &str) -> Result<WorkspaceDto>`
  - `create_workspace_instance(&self, req: &CreateInstanceRequest) -> Result<WorkspaceDto>`
- [ ] **Step 1.V1 (Verification - Unit Test)**: Write comprehensive unit test `test_workspace_instance_migration_and_resolution` in `crates/oz-core/src/db/workspaces.rs` using an in-memory SQLite database (`Store::open_in_memory()`) verifying migration execution, `default-<key>` creation, and `list_workspaces()` access precedence.
- [ ] **Step 1.V2 (Verification - Quality Check)**: Run `cargo clippy -p oz-core -- -D warnings` and `cargo test -p oz-core db::workspaces` ensuring zero compilation warnings and 100% test pass rate.

### Phase 2: Data Scoping Layer, Security Guard & Indexing (`crates/oz-core` / `oz-security`)
- [ ] **Step 2.1**: Add data scoping columns via schema migration: `store_id` (TEXT NULL) on `products`, `orders`, `order_lines`, and `customers`; `warehouse_id` (TEXT NULL) on `stock` and `stock_counts`.
- [ ] **Step 2.2**: Create prefix compound B-Tree indexes: `idx_orders_store_status (store_id, status, created_at DESC)`, `idx_order_lines_store_order (store_id, order_id)`, `idx_products_store_active (store_id, is_active, category_id)`, and `idx_stock_warehouse_item (warehouse_id, item_id)`.
- [ ] **Step 2.3**: Implement type-safe `ScopeGuard` struct in `crates/oz-core/src/scope.rs` utilizing `ScopeMode` (`SingleStore`, `MultiStore`, `ChainGlobal`) and requiring cryptographic session `(user_id + terminal_id + instance_id)` validation via `oz-security`.
- [ ] **Step 2.4**: Refactor domain `Store` CRUD methods (`get_products`, `list_orders`, `get_stock`, `create_order`) across all db modules (`sales.rs`, `products.rs`, `stock_counts.rs`) to mandate `&ScopeGuard` and dynamically expand query scoping (e.g. `WHERE store_id IN (...)` or bypass for `ChainGlobal`).
- [ ] **Step 2.5**: Integrate time-ordered `UUIDv7` / `ULID` ID generation, optimistic concurrency fields (`version INTEGER`, `updated_at TEXT`), and the **CRDT Delta Ledger pattern** (`stock_movements` delta inserts instead of absolute quantity overwrites) to eliminate offline inventory race conditions.
- [ ] **Step 2.V1 (Verification - Scope & Index Test)**: Write automated test `test_scope_guard_enforcement_and_indexing` verifying that cross-store access attempts throw `CoreError::UnauthorizedScope`, chain-wide aggregation succeeds for `ChainGlobal`, and confirm B-Tree index hits via `EXPLAIN QUERY PLAN`.
- [ ] **Step 2.V2 (Verification - Quality Check)**: Run `cargo clippy -p oz-core -- -D warnings` and `cargo test -p oz-core` across all modified database modules.

### Phase 3: Subscription Tier & Entitlement Enforcement (`crates/oz-core`)
- [ ] **Step 3.1**: Create `tenant_subscription` table schema (`tenant_id`, `tier_key`, `status`, `expires_at`, `max_stores`, `max_pos_instances`, `allowed_types_json`, `signature`) and `SubscriptionTier` Rust struct in `crates/oz-core/src/subscription.rs` with RSA/HMAC anti-tamper signature validation.
- [ ] **Step 3.2**: Integrate runtime quantity validation inside `create_workspace_instance()`, querying active instances by type and returning `CoreError::SubscriptionLimitExceeded` when tier register quota is reached.
- [ ] **Step 3.3**: Implement `tier.allows_workspace_type(&instance.type_key)` entitlement checks at workspace boot time, returning `CoreError::SubscriptionUpgradeRequired` if specialized templates (`kds`, `analytics-pro`) are locked.
- [ ] **Step 3.4**: Implement 14-day offline grace window with **Monotonic Ledger Clock Check** (`MAX(orders.created_at, audit_logs.created_at)`) to detect OS clock rollbacks (`CoreError::SystemClockTampered`), and safe downgrade handling (`is_active = false` without data loss).
- [ ] **Step 3.V1 (Verification - Tier & Signature Test)**: Write automated test `test_tenant_subscription_anti_tamper_and_quotas` in `crates/oz-core/src/db/workspaces.rs` testing creation quotas across tiers, verifying clock drift detection (`SystemClockTampered`), and asserting `CoreError::InvalidSubscriptionSignature` when local SQLite data is manually tampered.
- [ ] **Step 3.V2 (Verification - Quality Check)**: Run `cargo test -p oz-core test_subscription` and verify clean formatting across error types.

### Phase 4: Tauri IPC Bridge & Front-End API (`apps/` & `ui/src/api/`)
- [ ] **Step 4.1**: Update Tauri `get_workspaces` command in `apps/desktop-client/src/commands/workspaces.rs` and `apps/tablet-client/src/commands/workspaces.rs` to return `Vec<WorkspaceDto>` with full instance and scope attributes.
- [ ] **Step 4.2**: Register new admin Tauri commands in `lib.rs`: `create_workspace_instance`, `update_workspace_instance`, `delete_workspace_instance`, and `assign_user_workspace_instance`.
- [ ] **Step 4.3**: Update TypeScript interfaces (`WorkspaceDto`, `WorkspaceScope`, `CreateInstanceRequest`) and implement clean async wrapper functions in `ui/src/api/workspaces.ts`.
- [ ] **Step 4.4**: Implement scoped event emission (`self.event_bus.emit_scoped(scope.store_id(), ...)` across Tauri IPC channels (`StoreEvent::OrderCompleted`, `StoreEvent::TableStatusChanged`) so table updates broadcast strictly by `store_id`.
- [ ] **Step 4.V1 (Verification - API & IPC Test)**: Write integration tests `ui/src/__tests__/api/workspaces.test.ts` verifying `api/workspaces.ts` correctly deserializes `WorkspaceDto` and transmits `WorkspaceScope` headers.
- [ ] **Step 4.V2 (Verification - Quality Check)**: Run `cargo clippy -p desktop-client -p tablet-client -- -D warnings` and verify zero Tauri IPC command registration or serialization errors.

### Phase 5: Front-End State, Routing & UI Components (`ui/src/`)
- [ ] **Step 5.1**: Refactor `WorkspaceContext.tsx` (`WorkspaceProvider`) state from `activeWorkspace: string` (flat key) to `activeInstance: WorkspaceDto | null`, expose `useWorkspaceScope()` (`{ storeId, warehouseId }`), and implement **Fast-Switch Staff PIN Overlay (`FastPINOverlay.tsx`)** + `ScopeGuard.user_id` hot-swapping for shared touchscreens without session hijacking.
- [ ] **Step 5.2**: Update all data-fetching hooks (`useProducts`, `useOrders`, `useStock`) across `ui/src/features/` to extract `storeId` from `useWorkspaceScope()` and pass it to API adapters.
- [ ] **Step 5.3**: Refactor `AppShell.tsx` routing logic to switch on `activeInstance.type_key` (`restaurant-pos` → `<PosScreen />`, `store-pos` → `<RetailPosScreen />`, `kds` → `<KdsScreen />`, fallback → `<AppLayout />`).
- [ ] **Step 5.4**: Update `WorkspaceHome.tsx` (`WorkspaceCard`) to render grouped instances (`Downtown - Cashier 1`, `Mall - Cashier 1`), displaying location badges (`Downtown`, `Mall`, `Airport`), instance colors, and active status.
- [ ] **Step 5.V1 (Verification - UI Component Tests)**: Create/update Vitest tests `ui/src/__tests__/WorkspaceHome.test.tsx` and `ui/src/__tests__/AppShell.test.tsx` verifying instance card rendering, group labels, ARIA compliance, and `type_key` routing.
- [ ] **Step 5.V2 (Verification - Full Pre-Commit & CI Matrix Gate)**: Run `./scripts/check.sh` confirming `cargo fmt --all`, `i18n lint`, `bundle-parity`, `FTL dedupe`, `cargo clippy -- -D warnings`, and the complete front-end test suite pass with zero errors across the entire codebase.

---

## Context

OZ-POS currently defines workspaces as a flat set of unique keys:

```
restaurant-pos  → fullscreen PosScreen with table management
store-pos       → fullscreen RetailPosScreen with product lookup
kds             → fullscreen KdsScreen (kitchen display)
inventory       → AppLayout sidebar with inventory screens
admin           → AppLayout sidebar with admin screens
```

This works for single-location businesses, but breaks down when a business operates multiple venues:

- A restaurant chain needs separate POS workspaces for **Downtown** and **Mall** locations.
- A retail chain needs separate POS workspaces for each store.
- A warehouse operator needs separate inventory workspaces for **Warehouse A** and **Warehouse B**.

The core problem is that each workspace key conflates **type** (what UI to render) with **instance** (which data to scope).

### Current Limitations

1. **No duplicated types** — You cannot have two `restaurant-pos` workspaces because the key must be unique and the frontend routing is key-based.

2. **No data scoping** — All products, orders, customers, and inventory are global. There is no `store_id` or `warehouse_id` filter.

3. **Hardcoded frontend** — `AppShell.tsx`, `WorkspaceHome.tsx`, and `WorkspaceIcon` have hardcoded key → behavior mappings that new workspaces cannot integrate with.

4. **User assignment is type-based** — `user_workspaces` assigns a user to a workspace KEY, not to an instance. A user cannot be assigned to "Downtown Restaurant" specifically.

### Requirements

- A business can have **N workspaces of the same type** (e.g., 3 restaurant POS workspaces).
- Each workspace instance **scopes data** to a specific store/warehouse/location.
- The **workspace picker** shows instance names (e.g., "Downtown Restaurant POS"), not type names.
- **User assignment** is per-instance, not per-type.
- The **rendering system** determines what UI to show from the workspace type, while the **data layer** filters by the instance's scope.

### Real-World Example: Multi-Location Restaurant Chain (3 Stores, 9 Workspaces)

Consider a restaurant group with three locations: **Downtown (`store-downtown`)**, **Mall (`store-mall`)**, and **Airport (`store-airport`)**. Each location requires **2 Cashier POS terminals** and **1 Inventory management workspace**.

Under the old flat architecture, this setup is impossible due to unique key constraints (`restaurant-pos` and `inventory` can only exist once). With Type/Instance separation, exactly **2 templates** govern all **9 instances**:

```
Workspace Types (Templates seeded once):
├── restaurant-pos  → Fullscreen POS layout, table map, order processing
└── inventory       → Sidebar AppLayout with stock counts and ordering

Workspace Instances (Deployments with store_id data scoping):
├── Downtown Store (`store_id: 'store-downtown'`)
│   ├── [ws-dt-cashier-1]   "Downtown - Cashier 1"       (Type: restaurant-pos)
│   ├── [ws-dt-cashier-2]   "Downtown - Cashier 2"       (Type: restaurant-pos)
│   └── [ws-dt-inventory]   "Downtown - Inventory"       (Type: inventory)
│
├── Mall Store (`store_id: 'store-mall'`)
│   ├── [ws-mall-cashier-1] "Mall - Cashier 1"           (Type: restaurant-pos)
│   ├── [ws-mall-cashier-2] "Mall - Cashier 2"           (Type: restaurant-pos)
│   └── [ws-mall-inventory] "Mall - Inventory"           (Type: inventory)
│
└── Airport Store (`store_id: 'store-airport'`)
    ├── [ws-air-cashier-1]  "Airport - Cashier 1"        (Type: restaurant-pos)
    ├── [ws-air-cashier-2]  "Airport - Cashier 2"        (Type: restaurant-pos)
    └── [ws-air-inventory]  "Airport - Inventory"        (Type: inventory)
```

#### User Access & Scoping Behavior:
- **Restaurant Owner (`user-owner`)**: Assigned to all 9 instances (or bypasses via `role-owner`). Their Workspace Picker (`WorkspaceHome.tsx`) groups all 9 instances cleanly across locations. Clicking *Mall - Cashier 1* queries `WHERE store_id = 'store-mall'`, while clicking *Airport - Inventory* queries `WHERE store_id = 'store-airport'`.
- **Downtown Staff Member (`user-cashier-dt`)**: Assigned exclusively to `ws-dt-cashier-1` and `ws-dt-cashier-2`. When they log in, only the two Downtown registers appear. They cannot see Mall or Airport data.
- **Mall Stock Manager (`user-stock-mall`)**: Assigned exclusively to `ws-mall-inventory`. They boot directly into the inventory sidebar layout scoped strictly to the Mall location (`store_id = 'store-mall'`).

---

## Decision

### 1. Separate Workspace Type from Workspace Instance

#### Workspace Type (Template)

A workspace type defines **what the workspace looks like and how it renders**:

```rust
pub struct WorkspaceType {
    pub key: String,           // 'restaurant-pos', 'store-pos', 'inventory', 'admin', 'kds'
    pub name: String,          // 'Restaurant POS'
    pub render_mode: String,   // 'fullscreen' | 'sidebar'
    pub icon: String,          // 'restaurant', 'store', 'inventory', 'admin', 'kds'
    pub sort_order: i32,
}
```

- Types are **seeded at migration time** and rarely change.
- `render_mode` tells the shell whether to render fullscreen (PosScreen, RetailPosScreen, KdsScreen) or inside the sidebar AppLayout (inventory, admin).
- `workspace_type_screens` maps each type to its nav items (same as current `workspace_screens`).

#### Workspace Instance (Deployment)

A workspace instance is a **specific deployment of a type**, with its own name, description, and data scope:

```rust
pub struct WorkspaceInstance {
    pub id: String,             // 'ws-downtown-resto', 'ws-mall-resto', 'ws-wh-a'
    pub type_key: String,       // 'restaurant-pos'
    pub name: String,           // 'Downtown Restaurant POS'
    pub description: String,
    pub store_id: Option<String>, // scopes data; None = global scope
    pub colour: Option<String>,   // optional accent colour override
    pub is_active: bool,
}
```

- Instances are created by administrators and can be named per-venue.
- `store_id` links to the store/location that this workspace manages. When present, all data queries include `WHERE store_id = ?`.

### 2. Database Schema

```sql
-- What a workspace looks like (seeded, rarely changes)
CREATE TABLE workspace_types (
    key         TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    render_mode TEXT NOT NULL DEFAULT 'sidebar',  -- 'fullscreen' | 'sidebar'
    icon        TEXT NOT NULL DEFAULT '',
    sort_order  INTEGER NOT NULL DEFAULT 0
);

-- Which screens/nav items appear in each workspace type
CREATE TABLE workspace_type_screens (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    type_key    TEXT NOT NULL REFERENCES workspace_types(key),
    screen_key  TEXT NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    UNIQUE(type_key, screen_key)
);

-- Actual workspace deployments (created by admins)
CREATE TABLE workspace_instances (
    id          TEXT PRIMARY KEY,
    type_key    TEXT NOT NULL REFERENCES workspace_types(key),
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    store_id    TEXT,                   -- nullable; scopes data queries
    colour      TEXT,                   -- optional accent colour for picker card
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Per-user workspace assignment (pointing at INSTANCES, not types)
CREATE TABLE user_workspace_instances (
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    instance_id TEXT NOT NULL REFERENCES workspace_instances(id) ON DELETE CASCADE,
    is_default  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, instance_id)
);

-- Role-level defaults (which instance types a role can access)
CREATE TABLE role_workspace_types (
    role_id  TEXT NOT NULL REFERENCES roles(id),
    type_key TEXT NOT NULL REFERENCES workspace_types(key),
    UNIQUE(role_id, type_key)
);
```

#### Migration Path from Current Tables

| Current Table | Migrates To |
|---|---|
| `workspaces` | `workspace_types` (data preserved as-is) |
| `workspace_screens` | `workspace_type_screens` (data preserved) |
| `role_workspaces` | `role_workspace_types` (renamed, data preserved) |
| `user_workspaces` | `user_workspace_instances` (migrate each row: create a default instance per type_key, then point user at it) |

A migration script creates one **default instance** per existing workspace key so existing deployments are unaffected:

```sql
INSERT INTO workspace_instances (id, type_key, name, description)
SELECT 'default-' || key, key, name, description FROM workspaces;
```

### 3. Workspace Resolution Algorithm

When resolving a user's accessible workspaces:

```
1. If role = 'role-owner' → return ALL active instances.
2. If user_workspace_instances has rows → return ONLY those instances.
3. Otherwise → find instances whose type_key is in role_workspace_types.
```

The `WorkspaceDto` sent to the frontend includes both instance and type info:

```rust
pub struct WorkspaceDto {
    pub instance_id: String,
    pub type_key: String,
    pub name: String,              // Instance name (e.g., "Downtown Restaurant POS")
    pub description: String,
    pub icon: String,              // From the type
    pub colour: Option<String>,    // Instance-specific colour override
    pub render_mode: String,
    pub store_id: Option<String>,
}
```

### 4. Frontend Routing by Type, Not Key

`AppShell.tsx` switches on `type_key` instead of the workspace key:

```typescript
const workspaceTypeRoute: Record<string, string> = {
  'restaurant-pos': 'sales',
  'store-pos': 'products',
  kds: 'kds',
  inventory: 'inventory',
  admin: 'settings',
};

// Instead of:
//   if (activeWorkspace === 'restaurant-pos') ...
// Use:
const instance = getActiveWorkspaceInstance();
switch (instance.type_key) {
  case 'restaurant-pos':
    return <PosScreen storeId={instance.store_id} onNavigate={...} />;
  case 'store-pos':
    return <RetailPosScreen storeId={instance.store_id} onNavigate={...} />;
  // ...
}
```

Unknown `type_key` values fall through to the generic `AppLayout` sidebar renderer.

### 5. Data Scoping via `store_id`

Every major data query that should be scoped per-location includes a `WHERE store_id = ?` parameter:

```sql
-- Products for "Downtown Restaurant POS":
SELECT * FROM products WHERE store_id = 'store-downtown';

-- Open orders for "Mall Store POS":
SELECT * FROM orders WHERE store_id = 'store-mall' AND status != 'completed';

-- Stock for "Warehouse A" workspace:
SELECT * FROM stock WHERE warehouse_id = 'wh-a';
```

The `store_id` / `warehouse_id` column is added to each relevant table:

| Table | Scope Column |
|---|---|
| `products` | `store_id` |
| `orders` | `store_id` |
| `order_lines` | `store_id` (denormalized for query performance) |
| `stock` | `warehouse_id` |
| `stock_counts` | `warehouse_id` |
| `customers` | `store_id` (or global if shared) |
| `gift_cards` | `store_id` (or global) |

The scope parameter is threaded through the React component tree via a `WorkspaceScope` context:

```typescript
interface WorkspaceScope {
  storeId?: string;
  warehouseId?: string;
}
```

All API calls extract the scope from context rather than hardcoding it:

```typescript
function useProducts() {
  const { storeId } = useWorkspaceScope();
  return useQuery(['products', storeId], () => getProducts(storeId));
}
```

### 6. Security & Hardening Pillars (Enterprise Architecture)

To ensure this multi-instance architecture operates reliably across thousands of registers without data swapping, offline sync collisions, or query lag, the framework enforces four mandatory security and performance boundaries:

#### Pillar 1: Type-Safe Query Scope Guard (`Compile-Time Scope Enforcer`)
To prevent developers from accidentally omitting `WHERE store_id = ?` in backend endpoints, data scoping is enforced by Rust's type system via `ScopeGuard`:
- Every store-scoped data query method (`list_orders`, `get_products`, `update_stock`) requires a validated `&ScopeGuard` parameter.
- A `ScopeGuard` can only be constructed after the backend verifies the caller's session (`user_id`) against `user_workspace_instances` and `terminals` (`terminal_id`).
- If the frontend sends a tampered `store_id` payload, the verification fails (`CoreError::UnauthorizedScope`). If a backend developer forgets to scope a query, the Rust compiler (`rustc`) rejects the method signature.

```rust
pub enum ScopeMode {
    SingleStore(String),       // Regular Cashier / Stock Manager (1 store)
    MultiStore(Vec<String>),   // Area Manager (assigned to specific stores)
    ChainGlobal,               // Owner / Admin (chain-wide consolidated views)
}

pub struct ScopeGuard {
    pub mode: ScopeMode,
    pub warehouse_id: Option<String>,
    pub terminal_id: String,
    pub user_id: String,
}

impl Store<'_> {
    // This will NOT compile or execute without passing a cryptographically verified ScopeGuard
    pub fn list_orders(&self, scope: &ScopeGuard, status: &str) -> Result<Vec<Order>> {
        let (sql_filter, params) = match &scope.mode {
            ScopeMode::SingleStore(store_id) => ("(store_id = ? OR store_id IS NULL)", vec![store_id.clone()]),
            ScopeMode::MultiStore(stores) => {
                let placeholders = vec!["?"; stores.len()].join(", ");
                // Builds `(store_id IN (?, ?) OR store_id IS NULL)`
                // ...
            },
            ScopeMode::ChainGlobal => ("1=1", vec![]), // Owner accesses all stores across the chain
        };
        // ...
    }
}
```

#### Pillar 2: Cryptographic Terminal Binding, CRDT Stock Ledger & Offline UUIDv7 Sync
In multi-register environments, offline connectivity must not cause ID collisions, inventory race conditions, or cross-store data overwrites:
- **Zero Auto-Increment IDs**: All entity primary keys (`orders`, `order_lines`, `payments`, `stock_transfers`) use time-ordered **UUIDv7 / ULID** keys (`01F8MECH...`), eliminating ID conflicts when multiple registers operate offline locally.
- **CRDT Delta Ledger for Inventory (`stock_movements`)**: Offline registers never overwrite absolute `quantity` columns directly (`last-write-wins`). They only insert immutable delta ledger rows into `stock_movements` (`+5` or `-2`). When registers reconnect and sync, deltas sum up deterministically across `(store_id, item_id)` with zero race conditions or missing stock.
- **Terminal Keyring Binding (`oz-security`)**: Every POS hardware device holds a cryptographic profile (`terminal_id` bound via OS keyring). Sessions require `(user_id + terminal_id + instance_id)` validation.
- **Shared Touchscreen Fast-Switching (`ScopeGuard.user_id` Hot-Swapping)**: To prevent session hijacking on shared touchscreens without slowing service down, the frontend overlays a **Quick Staff PIN Pad (`FastPINOverlay.tsx`)**. Upon PIN verification, the backend dynamically hot-swaps `ScopeGuard.user_id` while keeping `terminal_id` and `instance_id` invariant, ensuring perfect audit logs and cash drawer accountability per operator.
- **Immutable Transaction Clock (`version` + LSN)**: Every record tracks `version INTEGER` and `updated_at TEXT` so when offline registers reconnect to each other or `cloud-server`, optimistic concurrency checks prevent ghost overwrites.
- **Orphan Prevention (`ON DELETE RESTRICT`)**: All foreign keys to `store_profiles` explicitly enforce `ON DELETE RESTRICT ON UPDATE CASCADE`, making it impossible to delete a location while it has active registers, open shifts, or historical transactions.

#### Pillar 3: Prefix Compound B-Tree Indexing `(store_id, ...)`
To maintain sub-millisecond query performance across large-scale multi-store databases:
- All store-scoped tables must define compound B-Tree indexes prefixed with the scoping column (`store_id` or `warehouse_id`):
  ```sql
  CREATE INDEX idx_orders_store_status ON orders(store_id, status, created_at DESC);
  CREATE INDEX idx_order_lines_store_order ON order_lines(store_id, order_id);
  CREATE INDEX idx_products_store_active ON products(store_id, is_active, category_id);
  CREATE INDEX idx_stock_warehouse_item ON stock(warehouse_id, item_id);
  ```
- SQLite immediately jumps directly to the target instance's index slice, isolating multi-million row tables by store in `< 1ms`.

#### Pillar 4: Scoped Real-Time Event Bus (`Tauri v2 IPC Channels`)
To ensure cashier screens update instantaneously without receiving unrelated cross-store noise:
- The backend broadcasts domain events (`StoreEvent::OrderCompleted`, `StoreEvent::TableStatusChanged`) via scoped Tauri IPC channels:
  ```rust
  self.event_bus.emit_scoped(
      scope.store_id.as_deref(),
      &StoreEvent::TableStatusChanged { table_id: "tbl-4".into(), status: "occupied".into() }
  )?;
- Only client instances whose active `WorkspaceScope` matches the emitted `store_id` process the event and trigger a re-render.

### 7. Subscription Tier & Entitlement Architecture

Separating `WorkspaceType` from `WorkspaceInstance` unlocks granular, backend-enforced subscription tiering across three monetization dimensions: physical locations (`store_profiles`), concurrent cashier registers (`workspace_instances`), and specialized UI templates (`workspace_types`).

#### 1. Signed Tenant Subscription Schema (Anti-Tamper & Offline Grace)
Because `oz-pos` stores data locally in SQLite (`rusqlite`), subscription limits must be cryptographically signed (`RSA/HMAC`) to prevent users from opening `POS.sqlite` locally and modifying their tier. The active subscription is stored with a signature issued by `apps/cloud-server`:

```sql
CREATE TABLE tenant_subscription (
    tenant_id          TEXT PRIMARY KEY,
    tier_key           TEXT NOT NULL,        -- 'free', 'pro', 'premium', 'enterprise'
    status             TEXT NOT NULL,        -- 'active', 'past_due', 'canceled'
    expires_at         TEXT NULL,            -- ISO timestamp (NULL = lifetime/free)
    max_stores         INTEGER NOT NULL,
    max_pos_instances  INTEGER NOT NULL,
    allowed_types_json TEXT NOT NULL,        -- '["restaurant-pos", "store-pos", "admin"]'
    signature          TEXT NOT NULL,        -- RSA/HMAC signature from apps/cloud-server
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
```

**Security & Offline Rules:**
- **Signature Verification**: On startup and prior to quota checks, the backend verifies `signature` against the public key (`oz-pos-updater.key.pub`). If tampered, the backend raises `CoreError::InvalidSubscriptionSignature`.
- **14-Day Offline Grace & Monotonic Ledger Clock Check**: When offline (`offline.rs`), registers evaluate `expires_at`. To prevent users from rolling back their Windows/iPad OS System Clock to bypass expiration indefinitely, the backend computes:
  $$\text{Effective Time} = \max\Big(\mathtt{Utc::now()},\; \max_{r \in \text{orders}}(\mathtt{r.created\_at}),\; \max_{l \in \text{audit\_logs}}(\mathtt{l.created\_at})\Big)$$
  If `MAX(orders.created_at)` exceeds `Utc::now()`, the system detects clock rollback (`CoreError::SystemClockTampered`) and immediately locks the register until an online cloud sync occurs. Paid tiers (`Pro`, `Premium`, `Enterprise`) continue operating for up to 14 days offline. If 14 days elapse without syncing, the system gracefully reverts to `Free` tier quotas until connectivity returns.

#### 2. Subscription Tier Enforcement Matrix

| Tier | Store Quota (`store_profiles`) | POS Register Quota (`workspace_instances`) | Allowed Workspace Types (`workspace_types`) | Advanced Features & Hardware |
| :--- | :--- | :--- | :--- | :--- |
| **Free** | **1 Store** | **1 POS Register** | `store-pos` (or `restaurant-pos`), `admin` | Basic receipt printing (`oz-hal`), Local SQLite only. |
| **Pro** | **Up to 2 Stores** | **Up to 3 Registers / Store** | `restaurant-pos`, `store-pos`, `inventory`, `admin` | Barcode scanners, Cash drawers, Basic inventory tracking. |
| **Premium** | **Up to 5 Stores** | **Up to 10 Registers / Store** | + `kds` (Kitchen Display System), `analytics-pro` | Multi-store cloud sync (`apps/cloud-server`), Advanced recipe costing. |
| **Enterprise** | **Unlimited (`N`)** | **Unlimited Registers** | + All types + Custom Plugin Workspaces (`oz-plugin`) | Multi-warehouse routing, Custom Lua scripts (`oz-lua`), Dedicated API access. |

#### 3. Runtime Quota Validation (`create_workspace_instance`)
When an administrator attempts to create a new register instance or add a location, the backend evaluates active instance counts against `SubscriptionTier` limits inside a database transaction:

```rust
pub fn create_workspace_instance(&self, req: &CreateInstanceRequest, tier: &SubscriptionTier) -> Result<WorkspaceDto> {
    self.verify_subscription_signature()?; // Ensures local database was not tampered with
    let active_pos_count = self.count_active_instances_by_type(&req.type_key)?;
    if active_pos_count >= tier.max_pos_instances() {
        return Err(CoreError::SubscriptionLimitExceeded(
            format!("Your {} tier allows maximum {} registers. Upgrade to add more.", tier.name, tier.max_pos_instances())
        ));
    }
    // ... proceed with instance creation
}
```

#### 4. Workspace Boot Entitlement Check (`allows_workspace_type`)
When a user attempts to open an advanced workspace (`kds` or `analytics-pro`), the backend verifies template entitlements before issuing session scope credentials:

```rust
if !tier.allows_workspace_type(&instance.type_key) {
    return Err(CoreError::SubscriptionUpgradeRequired(
        "Kitchen Display System (KDS) requires Premium tier or higher."
    ));
}
```

#### 5. Graceful Upgrades & Downgrades
- **Upgrades**: Raising a tier instantly increases `max_pos_instances` and adds allowed `workspace_types`. No database migration or client restart is required.
- **Downgrading (Safe Archiving)**: If a client downgrades below their current register count, excess instances transition to `is_active = false`. Historical audit logs and orders (`WHERE store_id = ...`) are preserved, while only quota-compliant instances remain openable for new sales.

### 8. Workspace Picker Visual Design

The workspace picker (`WorkspaceHome.tsx`) shows workspace **instances**, not types:

```typescript
interface WorkspaceCardDto {
  instanceId: string;
  typeKey: string;
  name: string;
  description: string;
  icon: string;
  colour?: string;
  storeId?: string;
  lastAccessed?: boolean;
}
```

Cards are grouped by type (e.g., all restaurant POS instances together) with a small type label. Each card has:

- A type-based icon (drawn by `WorkspaceIcon` using `typeKey`).
- The instance name prominently displayed.
- A subtle type tag (e.g., "Restaurant POS") below.
- An accent colour derived from the instance's `colour` field, falling back to a `WS_COLORS` mapping by type.

The `WS_COLORS` and `WS_ORDER` maps are migrated from hardcoded objects to database-driven values:

```sql
-- accent_colours stored alongside workspace_types
ALTER TABLE workspace_types ADD COLUMN accent_colour TEXT NOT NULL DEFAULT '';
```

### 9. Default Instance + Backward Compatibility

To ensure zero disruption for existing single-location deployments:

- A migration creates a default instance (e.g., `default-restaurant-pos`, `default-store-pos`) for each existing workspace key.
- Existing `user_workspaces` rows are migrated to point at the corresponding default instance.
- The frontend falls back to the default instance if no other instance is specified.
- Single-location businesses see exactly the same UI and data as before.

---

## Options Considered

### Option A — Type/Instance Separation (Chosen)

Separate workspace types (UI template) from workspace instances (deployment + data scope).

- **Pro:** Clean conceptual model, works for all business types (restaurant chains, retail chains, multi-warehouse).
- **Pro:** Data scoping is explicit and consistent across all queries.
- **Pro:** User assignment is naturally per-instance.
- **Con:** Requires database migration for existing data.
- **Con:** All data queries need a scope parameter added.

### Option B — Tags on Workspace Keys (Rejected)

Keep a flat workspace list but allow duplicate keys differentiated by tags/metadata:

```
restaurant-pos:downtown
restaurant-pos:mall
inventory:warehouse-a
inventory:warehouse-b
```

- **Pro:** Minimal schema change — just add metadata JSON column.
- **Con:** Tag parsing is fragile, rendering logic becomes conditional on tag values.
- **Con:** No clear place to store `store_id` scope — would be buried in JSON metadata.
- **Con:** Type comparison requires string parsing (`key.split(':')[0]`).

### Option C — Separate Store/Location Entity (Deferred)

Make "store" the primary organizational unit, and workspaces a display mode within a store:

```
Store: Downtown
  ├── Restaurant POS (type)
  ├── Inventory (type)
  └── Admin (type)

Store: Mall
  ├── Store POS (type)
  └── Inventory (type)
```

- **Pro:** Clean data model — store is the scoping entity, workspace is just the UI mode.
- **Con:** More radical departure from current architecture. Every screen and API would need store-level awareness.
- **Con:** Many cross-store features (global reporting, chain-wide inventory management) become harder.
- **Decision:** Consider for Phase 6+ if chain-wide operations become a primary use case.

### Option D — Multi-Workspace Per User Session (Deferred)

Allow a user to have multiple workspaces open simultaneously in tabs within the same session.

- **Pro:** Power users can switch between workspaces without losing context.
- **Con:** Major frontend complexity (multi-tab state, conflicting scopes).
- **Decision:** Revisit when the single-workspace-per-session model proves limiting.

---

## Consequences

### Positive

- A business can create N workspaces of any type, each scoped to a specific location.
- The workspace picker shows meaningful instance names ("Downtown Restaurant POS") instead of technical keys.
- User assignment is granular — a manager can access "Downtown Inventory" but not "Mall Inventory".
- Data scoping is consistent — every query filters by store/warehouse, preventing data leaks between locations.
- The type system is forward-compatible with future workspace types (e.g., a self-service kiosk type).
- Existing single-location deployments are unaffected (default instances + migration).

### Negative

- All major data tables need a `store_id` or `warehouse_id` column added.
- All API endpoints and Rust handlers need a scope parameter threaded through.
- The frontend query layer needs `WorkspaceScope` context on every data-fetching hook.
- Migration from existing `user_workspaces` requires creating default instances.
- The `WorkspaceIcon` component must handle type-based rendering.

### Mitigations

- The scope column can be added as a single migration with a default value for existing rows.
- The `WorkspaceScope` context is set once at workspace entry and consumed by hooks — no prop drilling.
- Instance creation is an admin UI feature gated behind `staff:update` permission.
- The `render_mode` field on types makes adding new workspace types a data-only operation.

---

## Phased Implementation & Migration Guide

To execute this transition cleanly without breaking existing single-location deployments or introducing regressions, we divide the implementation into 5 sequential phases:

### Phase 1: Database Schema & Core Backend Rust Models (`crates/oz-core`)
1. **Migration SQL Script (`create_workspace_types_and_instances.sql`)**:
   - Create `workspace_types`, `workspace_type_screens`, `workspace_instances`, `user_workspace_instances`, and `role_workspace_types` tables.
   - Run seed script to copy existing rows from `workspaces` into `workspace_types` (`restaurant-pos`, `store-pos`, `kds`, `inventory`, `admin`).
   - Run auto-generation script to create one **default instance** per existing workspace key (e.g., `default-restaurant-pos` pointing to type `restaurant-pos` with `store_id = NULL`).
   - Migrate existing user assignments from `user_workspaces` to `user_workspace_instances` (`instance_id = 'default-' || key`).
2. **Rust DTOs & Domain Models (`crates/oz-core/src/db/workspaces.rs`)**:
   - Define `WorkspaceTypeRow`, `WorkspaceInstanceRow`, and `WorkspaceDto` (containing `instance_id`, `type_key`, `name`, `description`, `icon`, `colour`, `render_mode`, and `store_id`).
   - Implement `list_workspaces(&self, role_id: &str, user_id: Option<&str>) -> Result<Vec<WorkspaceDto>>`:
     - Query `workspace_instances` joined with `workspace_types`.
     - Maintain resolution precedence: `role-owner` → all active instances; `user_workspace_instances` rows → only assigned instances; otherwise fallback to `role_workspace_types`.

### Phase 2: Data Scoping Layer & Store Filtering (`store_id` / `warehouse_id`)
1. **Schema Scope Columns**:
   - Add `store_id` (TEXT NULL) to `products`, `orders`, `order_lines`, and `customers`.
   - Add `warehouse_id` (TEXT NULL) to `stock` and `stock_counts`.
2. **Query Builder Enhancements**:
   - Update `Store` CRUD operations (`get_products`, `list_orders`, `get_stock`) to accept an optional `scope: Option<&WorkspaceScope>`.
   - When `store_id` is present, append `AND (store_id = ? OR store_id IS NULL)` so global shared items (or unscoped single-store setups) remain accessible while multi-store data is partitioned cleanly.

### Phase 3: Tauri IPC Bridge & Front-End API (`apps/` & `ui/src/api/`)
1. **Tauri Command Registration (`apps/desktop-client/src/commands/workspaces.rs`)**:
   - Update `get_workspaces` command to return `Vec<WorkspaceDto>` matching the new DTO payload.
   - Add admin commands: `create_workspace_instance`, `update_workspace_instance`, `delete_workspace_instance`, and `assign_user_workspace_instance`.
2. **Front-End API Adapter (`ui/src/api/workspaces.ts`)**:
   - Update TypeScript interfaces (`WorkspaceDto`, `WorkspaceInstanceCreateRequest`).
   - Ensure all calls flow through clean `api/workspaces.ts` wrappers without direct `invoke()` usage in components.

### Phase 4: Front-End State & Type-Based Routing (`ui/src/`)
1. **Workspace Context (`ui/src/context/WorkspaceContext.tsx`)**:
   - Update state from `activeWorkspace: string` (flat key) to `activeInstance: WorkspaceDto | null`.
   - Provide `WorkspaceScope` context (`{ storeId: activeInstance?.store_id, typeKey: activeInstance?.type_key }`) across the component tree so data hooks (`useQuery`) can automatically attach `storeId` to requests.
2. **Type-Based Routing (`ui/src/components/AppShell.tsx`)**:
   - Change routing logic to switch on `activeInstance.type_key` instead of the unique instance ID:
     ```typescript
     switch (activeInstance?.type_key) {
       case 'restaurant-pos':
         return <PosScreen storeId={activeInstance.store_id} onNavigate={...} />;
       case 'store-pos':
         return <RetailPosScreen storeId={activeInstance.store_id} onNavigate={...} />;
       case 'kds':
         return <KdsScreen storeId={activeInstance.store_id} onNavigate={...} />;
       default:
         return <AppLayout activeScreen={currentScreen} />;
     }
     ```
3. **Workspace Picker Card UI (`ui/src/components/WorkspaceHome.tsx`)**:
   - Render `WorkspaceCard` items using `instance_id` as key and `name` as primary title (e.g., "Downtown - Cashier 1").
   - Group cards by `type_key` or `store_id` depending on user role.
   - Display a subtle location badge (`Downtown`, `Mall`, `Airport`) when `store_id` is set, and apply instance `colour` overrides.

### Phase 5: Verification & Automated Safety Gates
1. **Automated Unit & Integration Tests**:
   - Verify `rusqlite` migration script creates exact `default-<key>` instances for existing databases.
   - Test `list_workspaces()` under `role-owner`, specific user assignments (`user_workspace_instances`), and role defaults (`role_workspace_types`).
   - Run Vitest/Jest tests for `AppShell` routing ensuring each `type_key` renders the appropriate screen (`PosScreen`, `RetailPosScreen`, `AppLayout`).
2. **Pre-Commit Hook Validation (`core.hooksPath .githooks`)**:
   - Run `scripts/check.sh` and ensure `cargo clippy`, `cargo fmt`, ARIA checks, and Fluent i18n (`bundle-parity` / `FTL dedupe`) pass with zero errors across all modified Rust crates and React components.

---

## Related

- `WorkspaceContext.tsx` — Current workspace state management (needs instance awareness)
- `WorkspaceHome.tsx` — Workspace picker (needs instance rendering)
- `AppShell.tsx` — Workspace routing (needs type-based dispatch)
- `crates/oz-core/src/db/workspaces.rs` — Backend workspace queries
- `apps/desktop-client/src/commands/workspaces.rs` — Tauri workspace commands
- ADR #1 — Module System Design
- ADR #3 — Frontend Restructure (registry-based shell)
