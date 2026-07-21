# Workspace Topology Editor — Implementation Plan

**Status:** Planning
**Date:** 2026-07-21
**Author:** Architecture Team
**Tags:** workspace-editor, node-topology, settings, ui, multi-store, inventory-routing

---

## Table of Contents

1. [Overview](#1-overview)
2. [Current State](#2-current-state)
3. [Implementation Phases](#3-implementation-phases)
   - [Phase 1: Wire to Real Data](#phase-1-wire-to-real-data)
   - [Phase 2: Move to Settings](#phase-2-move-to-settings)
   - [Phase 3: Polish & Edge Cases](#phase-3-polish--edge-cases)
4. [Node Type Specification](#4-node-type-specification)
5. [Wire Connection Specification](#5-wire-connection-specification)
6. [Backend API Reference](#6-backend-api-reference)
7. [Key Architectural Decisions](#7-key-architectural-decisions)
8. [Testing Strategy](#8-testing-strategy)
9. [Files to Change](#9-files-to-change)

---

## 1. Overview

The Node Topology Editor is a visual canvas-based tool (inspired by node graph interfaces in **Blender**, **Grasshopper**, and **Node-RED**) that allows store owners to visually assemble and wire up their enterprise hierarchy using node cards and directional arrow connections.

Currently a **functional prototype** in `ui/src/features/stores/NodeTopologyEditor.tsx`, it uses hardcoded demo data (`PRESET_RETAIL`, `PRESET_RESTAURANT`) and its `onSave` callback is never wired to the backend. The plan is to:

1. **Wire it to real backend data** — replace every hardcoded node/wire with real entities from the database.
2. **Move it into Settings** — give it a dedicated nav item in the Settings sidebar.
3. **Polish for production** — add undo/redo, real telemetry, accessibility, loading states, and error handling.

---

## 2. Current State

### Frontend

| Component | Status | Purpose |
|-----------|--------|---------|
| `ui/src/features/stores/NodeTopologyEditor.tsx` | Prototype | Visual canvas with drag-and-drop nodes, SVG arrow wires, pan/zoom, simulation debugger, license tier enforcement. Uses `PRESET_RETAIL`/`PRESET_RESTAURANT` hardcoded data. |
| `ui/src/features/stores/NodeTopologyEditor.css` | Prototype | Styling matching the prototype. |
| `ui/src/features/stores/MultiStoreDashboardScreen.tsx` | Shipping | Has a `viewMode` toggle between `'cards'` and `'topology'` that renders `NodeTopologyEditor` with `currentTier="standard"` — but the component gets no real data. |

### Backend (Production-Ready)

All required backend APIs exist and are scoped via session tokens (ADR #7):

| Command | Purpose |
|---------|---------|
| `list_workspaces_scoped` | List instances for current user/store |
| `create_workspace_instance_scoped` | Create a new workspace instance |
| `get_workspace_instance_scoped` | Get single instance |
| `list_all_workspaces_scoped` | List all workspace types (for dropdown) |
| `list_workspace_screens_scoped` | List screens for a workspace type |
| `set_user_workspace_instances_scoped` | Assign instances to users |
| `list_stores` | List store profiles |
| `create_store_profile` | Create a new store |
| `list_inventory_locations` | List inventory storage locations |
| `set_workspace_inventory_locations` | Bind locations to workspace with priority |
| `get_workspace_inventory_locations` | Get location bindings |
| `get_workspace_locations_scoped` | Unified resolver for workspace locations |
| `list_terminals_scoped` | List registered terminals |

### Data Flow (Current)

```
NodeTopologyEditor
  ├── mount: uses hardcoded PRESET_RETAIL data
  ├── nodes: TopologyNodeData[] in-memory state
  ├── wires: TopologyWireData[] in-memory state
  └── onSave?: callback prop — NEVER CALLED
```

### Data Flow (Target)

```
NodeTopologyEditor
  ├── mount: listStores() → populate Store nodes
  │          listWorkspacesScoped() → populate Workspace nodes
  │          listInventoryLocations() → populate Warehouse nodes
  │          listTerminalsScoped() → populate Hardware nodes
  │          getWorkspaceInventoryLocations() → populate wires
  ├── nodes: state derived from real entities
  ├── wires: state derived from real bindings
  └── "Apply" → batch commit: create entities, set bindings
```

---

## 3. Implementation Phases

### Phase 1: Wire to Real Data

**Goal:** Replace all hardcoded demo data with real backend calls.

| # | Step | Backend API | Frontend File |
|---|------|-------------|---------------|
| 1 | Add `sessionToken` prop to `NodeTopologyEditor` (passed from parent or from `useAuth()`) | — | `NodeTopologyEditor.tsx` |
| 2 | Load real stores on mount → convert to Store nodes | `listStores()` | `NodeTopologyEditor.tsx` |
| 3 | Load real workspace instances → convert to Workspace nodes | `listWorkspacesScoped(sessionToken)` | `NodeTopologyEditor.tsx` |
| 4 | Load real inventory locations → convert to Warehouse nodes | `listInventoryLocations(sessionToken)` | `NodeTopologyEditor.tsx` |
| 5 | Load real terminals → convert to Hardware nodes | `listTerminalsScoped(sessionToken)` | `NodeTopologyEditor.tsx` |
| 6 | Load existing wires: for each workspace node, fetch its inventory location bindings | `getWorkspaceInventoryLocations(sessionToken, instanceId)` | `NodeTopologyEditor.tsx` |
| 7 | "+ Store Node" → prompt for store name → call `createStore()` → add node | `createStoreProfile()` | `NodeTopologyEditor.tsx` |
| 8 | "+ Workspace Node" → prompt for name + type + colour → call `createWorkspaceInstanceScoped()` → add node with real ID | `createWorkspaceInstanceScoped()` | `NodeTopologyEditor.tsx` |
| 9 | "+ Warehouse Node" → create inventory location → add node | inventory location API | `NodeTopologyEditor.tsx` |
| 10 | "+ Hardware Node" → register terminal → add node | `registerTerminalScoped()` | `NodeTopologyEditor.tsx` |
| 11 | Wire: Store→Workspace → sets `workspace_instances.store_id` (automatic when created in-store) | (implicit) | `NodeTopologyEditor.tsx` |
| 12 | Wire: Workspace→Warehouse → calls `setWorkspaceInventoryLocations()` with priority | `setWorkspaceInventoryLocations()` | `NodeTopologyEditor.tsx` |
| 13 | Wire: Workspace→Hardware → binds terminal to workspace instance | terminal bind API | `NodeTopologyEditor.tsx` |
| 14 | "Apply Topology Changes" → serializes all pending changes and commits | batch of create/set calls | `NodeTopologyEditor.tsx` |
| 15 | Node Inspector drawer → show real metadata (store address, workspace type, warehouse stock, terminal status) | various get/status calls | `NodeTopologyEditor.tsx` |
| 16 | Simulation debugger → use real `getWorkspaceLocations()` to trace a test sale | `getWorkspaceLocationsScoped()` | `NodeTopologyEditor.tsx` |

**Estimated effort:** 3–5 days
**Risk:** Medium — many API calls to wire, but all endpoints exist.

### Phase 2: Move to Settings

**Goal:** Give the topology editor its own dedicated home in the Settings sidebar.

| # | Step | Files |
|---|------|-------|
| 1 | Add `topology` nav item to `NAV_ITEMS` in `SettingsPage.tsx` with an appropriate icon (e.g., connected nodes SVG) | `SettingsPage.tsx` |
| 2 | Add `topology` to a category in `CATEGORIES` — recommend **Management** alongside `stores` | `SettingsPage.tsx` |
| 3 | Add `'topology': 'settings-nav-topology'` to `NAV_L10N_KEYS` | `SettingsPage.tsx` |
| 4 | Add `case 'topology':` to `renderSection()` that renders `<NodeTopologyEditor sessionToken={...} />` | `SettingsPage.tsx` |
| 5 | Remove the topology tab/toggle from `MultiStoreDashboardScreen.tsx` (keep only card view) | `MultiStoreDashboardScreen.tsx` |
| 6 | Add Fluent l10n keys for the new nav item in all supported locales | `*.ftl` files |
| 7 | Adjust CSS so the editor fits properly inside the settings content area (the editor currently assumes full canvas width) | `NodeTopologyEditor.css` |

**Settings page nav hierarchy after change:**

```
Management
  ├── Staff
  ├── Terminals
  ├── Stores          ← card view only, no topology toggle
  ├── Topology        ← NEW — the node editor
  ├── Audit Log
  ├── Offline Queue
  ├── Shifts
  ├── Tax Rates
  ├── Exchange Rates
  └── Promotions
```

**Estimated effort:** 1 day
**Risk:** Low — purely UI restructuring.

### Phase 3: Polish & Edge Cases

**Goal:** Make the editor production-quality.

| # | Step | Details |
|---|------|---------|
| 1 | **Zoom-to-fit** | Auto-scale canvas on initial load and after add/remove to show all nodes |
| 2 | **Real telemetry** | Show live terminal online/offline status from `listTerminalsScoped()` with polling |
| 3 | **Node filter/search** | Search bar to filter nodes by name/type |
| 4 | **Undo/redo** | State history stack (Ctrl+Z / Ctrl+Y) for canvas operations |
| 5 | **Export/import** | Save topology as JSON, import to restore |
| 6 | **Error handling** | Toast notifications for failed backend operations per node/wire |
| 7 | **Loading states** | Skeleton/spinner while initial data loads |
| 8 | **Accessibility** | ARIA labels for node cards, port sockets, wires; keyboard navigation for adding nodes and drawing wires |
| 9 | **Empty state** | When no nodes exist, show a welcome prompt with preset buttons |
| 10 | **Node inline editing** | Double-click a node title to rename it directly on the canvas |

**Estimated effort:** 3–5 days (spread across multiple PRs)
**Risk:** Low-Medium — mostly UI work, no schema changes.

---

## 4. Node Type Specification

### Node → Entity Mapping

| Node Type | Backend Entity | Creation API | Identifier |
|-----------|---------------|--------------|------------|
| 🏢 **Store** | `store_profiles` row | `create_store_profile()` | `store.id` |
| 🛒 **Workspace** | `workspace_instances` row | `create_workspace_instance_scoped()` | `workspace.instance_id` |
| 📦 **Warehouse** | `inventory_locations` row | inventory location creation | `location.id` |
| 🖨️ **Hardware** | `terminals` row | `register_terminal_scoped()` | `terminal.id` |

### Node Card Data (from real entities)

```typescript
interface TopologyNodeData {
  id: string;                     // Backend entity ID
  type: 'store' | 'workspace' | 'warehouse' | 'hardware';
  name: string;                   // Entity name
  subtitle?: string;              // Type label or location description
  x: number;                      // Canvas X position (persisted locally or in metadata)
  y: number;                      // Canvas Y position
  entityId: string;               // The real backend ID
  telemetryBadge?: string;        // Live status (e.g. "Online (2 POS)", "1,250 items")
  telemetryStatus?: 'online' | 'warning' | 'offline';
  metadata?: Record<string, string>;  // Extra entity fields (address, colour, stock count, etc.)
}
```

### Canvas Position Persistence

Canvas node positions (x, y) are **not** stored in the backend entity tables. Options:

| Option | Approach | Pros | Cons |
|--------|----------|------|------|
| A | Store positions in a local `topology_layouts` JSON blob per store (new DB table) | Survives reinstall; shared across devices | New migration; sync complexity |
| B | Store in `localStorage` keyed by store ID | Simple; no backend changes | Lost on browser data clear; per-device |
| C | Store in `topology_editor_metadata` JSON column on a settings table | Survives data export | Minor schema change |

**Recommendation:** **Option A** — add a `topology_layouts` table keyed by `(store_id, topology_id)` with a JSON column for node positions. This keeps layout separate from entity data and allows multi-store layouts.

---

## 5. Wire Connection Specification

### Wire → Backend Mapping

| Wire (From → To) | Backend Action | API |
|-----------------|----------------|-----|
| 🏢 Store → 🛒 Workspace | Set `workspace_instances.store_id = store.id` | (implicit — done at instance creation) |
| 🛒 Workspace → 📦 Warehouse | Insert/Update `workspace_inventory_locations` row with priority | `set_workspace_inventory_locations()` |
| 🛒 Workspace → 🖨️ Hardware | Set `terminals.bound_workspace_instance_id = instance.id` | Terminal update API |
| 🏢 Store → 🖨️ Hardware | Set `terminals.bound_store_id = store.id` | Terminal update API |
| 📦 Warehouse ↔ 📦 Warehouse | 2-way transfer capability (UI-only for now — future inventory transfer) | (Phase 3) |

### Wire Priority Semantics

When a Workspace node has multiple wires to Warehouse nodes, each wire automatically gets a priority label:

| Wire Count | Label |
|------------|-------|
| 1st wire | `Stock Deduct (P1)` |
| 2nd wire | `Fallback (P2)` |
| 3rd+ wire | `Fallback (P3+)` |

Priorities are stored in `workspace_inventory_locations.sort_order`.

---

## 6. Backend API Reference

### Workspace APIs (`@/api/workspaces.ts`)

```typescript
listWorkspacesScoped(sessionToken): WorkspaceDto[]
createWorkspaceInstanceScoped(sessionToken, req): WorkspaceDto
getWorkspaceInstanceScoped(sessionToken, instanceId): WorkspaceDto
listAllWorkspacesScoped(sessionToken): WorkspaceTypeDto[]
listWorkspaceScreensScoped(sessionToken, typeKey): WorkspaceScreenDto[]
```

### Store APIs (`@/api/stores.ts`)

```typescript
listStores(): StoreProfile[]
createStore(args): StoreProfile
updateStore(args): StoreProfile
deleteStore(id): void
```

### Inventory APIs (`@/api/inventory.ts`)

```typescript
listInventoryLocations(sessionToken): InventoryLocation[]
setWorkspaceInventoryLocations(sessionToken, instanceId, locations): void
getWorkspaceInventoryLocations(sessionToken, instanceId): WorkspaceInventoryLocation[]
getWorkspaceLocations(sessionToken, instanceId, typeKey): WorkspaceLocationBinding[]
```

### Terminal APIs (`@/api/terminals.ts`)

```typescript
listTerminalsScoped(sessionToken): TerminalDto[]
registerTerminalScoped(sessionToken, args): TerminalDto
```

---

## 7. Key Architectural Decisions

### Decision 1: Live Edit vs Draft-and-Apply

| Approach | Description | Pros | Cons |
|----------|-------------|------|------|
| **Live** | Every node add/delete immediately calls backend | Simple UI; no "unsaved changes"; data always consistent | Each action is a separate DB write; slower for bulk ops |
| **Draft-and-Apply** | Edit in-memory, click "Apply" to batch commit | Batched transaction; undo/redo simpler; can preview diff | "Unsaved changes" complexity; risk of data loss on page close |
| **Hybrid** (recommended) | Create/delete node actions are live; wire edits and property changes are draft and batch-applied | Best of both: entities are created immediately, but topology structure is committed atomically | Two-tier save logic is slightly more complex |

**Decision: Hybrid.** Entity creation/deletion is immediate. Wire connections, property changes, and node position changes are draft state until "Apply Topology Changes" is clicked.

### Decision 2: Where in Settings?

**Decision: Dedicated "Topology" nav item in the Management category.** This gives the editor room to breathe and doesn't conflate it with store profile management. The Settings sidebar will list it as "Topology" between "Stores" and "Audit Log".

### Decision 3: Canvas Position Persistence

**Decision: New `topology_layouts` table** in the per-store database:

```sql
CREATE TABLE topology_layouts (
    id          TEXT PRIMARY KEY,
    store_id    TEXT NOT NULL,
    layout_json TEXT NOT NULL,  -- JSON: { nodes: [{id, type, x, y, ...}], wires: [{...}] }
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_topology_layouts_store ON topology_layouts(store_id);
```

This keeps layout data separate from entity data and survives reinstallation.

### Decision 4: Session Token Access

The `NodeTopologyEditor` will accept `sessionToken` as a prop. The parent (`SettingsPage`) obtains it from `WorkspaceContext`:

```typescript
const { sessionToken } = useWorkspaceContext();
// ...
<NodeTopologyEditor sessionToken={sessionToken} />
```

---

## 8. Testing Strategy

### Unit Tests (Rust)

| Test | What it covers |
|------|---------------|
| `create_workspace_instance_scoped` creates a real node | Workspace creation via scoped command |
| `set_workspace_inventory_locations` creates/updates wires | Wire binding persistence |
| Topology serializer: canvas JSON → SQLite commands | Diff-to-apply correctness |

### Component Tests (Vitest)

| Test | What it covers |
|------|---------------|
| `NodeTopologyEditor` mounts and loads real data | API calls on mount |
| Adding a Store node calls `createStoreProfile` | Node creation wiring |
| Drawing a wire calls `setWorkspaceInventoryLocations` | Wire creation wiring |
| Undo/redo state stack | Canvas UX |

### Integration Tests

| Test | What it covers |
|------|---------------|
| Open Settings → Topology → see real nodes | Full front-to-back flow |
| Add workspace → see it on canvas → refresh → it persists | CRUD persistence |
| Draw warehouse wire → run simulation → see debugger trace | Location resolver integration |

---

## 9. Files to Change

### Phase 1 — Wire to Real Data

| File | Change |
|------|--------|
| `ui/src/features/stores/NodeTopologyEditor.tsx` | Major rewrite: replace hardcoded data with API calls, add sessionToken prop, wire every node/wire action to backend |
| `ui/src/features/stores/NodeTopologyEditor.css` | Minor adjustments for loading states, error display, empty state |

### Phase 2 — Move to Settings

| File | Change |
|------|--------|
| `ui/src/features/settings/SettingsPage.tsx` | Add `topology` nav item, category mapping, l10n key, render case |
| `ui/src/features/stores/MultiStoreDashboardScreen.tsx` | Remove topology toggle; keep only card view |
| `ui/src/locales/*/settings.ftl` | Add `settings-nav-topology` fluent key |
| `ui/src/locales/*/settings.ftl` | Add `topology-builder-title` fluent key (if not already present) |

### Phase 3 — Polish

| File | Change |
|------|--------|
| `crates/oz-core/migrations/XXX_topology_layouts.sql` | New migration for position persistence |
| `crates/oz-core/src/db/topology.rs` | New module for topology CRUD |
| `apps/desktop-client/src/commands/topology.rs` | New Tauri commands for save/load layout |
| `ui/src/features/stores/NodeTopologyEditor.tsx` | Undo/redo, accessibility, search, export/import, zoom-to-fit |
| `ui/src/features/stores/NodeTopologyEditor.css` | All polish styling |

### Rust Backend (if topology layout table is added)

| File | Change |
|------|--------|
| `crates/oz-core/Cargo.toml` | (no change — serde_json already a dep) |
| `crates/oz-core/src/lib.rs` | Add `pub mod topology;` |
| `crates/oz-core/src/db/mod.rs` | Add `pub mod topology;` |
| `apps/desktop-client/src/commands/mod.rs` | Add `pub mod topology;` |

---

## Appendix: ADR Cross-References

| ADR | Relation |
|-----|----------|
| [ADR #4](decisions/2026-07-10-workspace-type-instance-design.md) | Workspace types/instances — the entities the topology editor manages |
| [ADR #19](decisions/2026-07-19-sale-deduction-multi-location.md) | Workspace→Location wire semantics (priority, fallback) |
| [ADR #22](decisions/2026-07-20-node-based-store-topology-builder.md) | Original node topology editor proposal document |
| [ADR #7](decisions/2026-07-10-workspace-instance-analysis.md) | Session token scoping — how APIs are called |

---

> **Next step:** Toggle to ACT MODE to begin implementation of Phase 1.
