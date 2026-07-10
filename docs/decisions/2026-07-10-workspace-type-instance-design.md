# ADR #4: Workspace Type/Instance Architecture

**Status:** Accepted
**Date:** 2026-07-10
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** architecture, workspaces, multi-store, data-scoping, multi-terminal

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

### 6. Workspace Picker Visual Design

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

### 7. Default Instance + Backward Compatibility

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
