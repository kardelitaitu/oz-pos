# ADR #22: Visual Node-Based Store & Workspace Topology Builder

**Status:** Proposed (2026-07-20)  
**Date:** 2026-07-20  
**Author:** Architecture Team & OZ-POS Contributors  
**Tags:** store-topology, node-editor, multi-store, workspaces, inventory-routing, ux  

---

## Context

Managing multi-store retail and food & beverage enterprises involves complex relationships between store branches, checkout registers (workspaces), inventory storage locations (warehouses), and physical peripherals (printers, KDS displays). 

Historically, managing these entity relationships required navigating separate administrative forms and database tables:
- Stores registered in `store_profiles`
- Workspaces registered in `workspace_instances`
- Inventory locations registered in `inventory_locations`
- Deduction priorities configured via junction tables

This traditional form-based approach lacks visual clarity, making it difficult for merchants to understand their enterprise topology or configure multi-warehouse stock deduction fallbacks.

To solve this, this decision introduces a **Visual Node-Based Store Topology Builder** (inspired by node graph interfaces in **Blender**, **Grasshopper**, and **Node-RED**) in **Settings → Stores**.

---

## Decision

We will implement an interactive, canvas-based **Node Topology Editor** allowing store owners to visually assemble and wire up their enterprise hierarchy using node cards and directional **1-Way (`→`)** or **2-Way (`↔`)** arrow connections.

### 1. Node Types & Capabilities

| Node Type | Icon | Output Ports | Input Ports | Description |
|---|:---:|---|---|---|
| **Store Node** | 🏢 | Store Out (`→`) | — | Central store branch profile. Displays online terminal count telemetry. |
| **Workspace Node** | 🛒 / 🍽️ | Stock Out (`→`), Print Out (`→`) | Store In (`→`) | POS checkout register / KDS instance. |
| **Warehouse Node** | 📦 / 🥬 | Transfer (`↔`) | Stock In (`→`), Transfer (`↔`) | Physical inventory storage location. Displays live stock & low-stock alerts. |
| **Hardware Node** | 🖨️ | — | Print In (`→`) | Thermal receipt printers, cash drawers, or hardware displays. |

---

### 2. Arrow Connection Specification

Connections between nodes on the SVG canvas are represented by clean directional wires:

#### 1-Way Arrow Connections (`─>`)
- **Store ─> Workspace**: Binds a workspace instance (`workspace_instances.store_id`) to a store location.
- **Workspace ─> Warehouse**: Directs stock deduction routing (`inventory_locations.bound_workspace_id`). Multiple wires from a single POS workspace automatically assign **Priority 1, Priority 2 (Fallback)** tags.
- **Workspace ─> Printer / KDS**: Routes receipts or kitchen tickets to target peripherals.

#### 2-Way Arrow Connections (`<─>`)
- **Warehouse A <─> Warehouse B**: Configures bi-directional inventory transfer capabilities between storage locations.
- **Store <─> Cloud Daemon**: Indicates bi-directional cloud data synchronization.

---

### 3. Architecture Overview

```
MultiStoreDashboardScreen (parent tab view)
  ├─ [ Card View ]
  └─ [ Node Topology Editor ] (Canvas View)
       ├─ NodeToolRack (Left Sidebar: Drag & Drop palette + Presets)
       ├─ NodeCanvas (SVG bezier arrow rendering + pan/zoom + grid)
       │    ├─ StoreNode (telemetry badges + ports)
       │    ├─ WorkspaceNode (accent colors + ports)
       │    ├─ WarehouseNode (stock alert indicators + ports)
       │    └─ HardwareNode (connection status + test print)
       ├─ NodeInspectorDrawer (Right Slide-over: node properties + per-node role overrides)
       ├─ SimulationDebugger (Animated energy pulse packets along arrow paths)
       └─ TopologyDiffModal (Summary of SQLite changes before atomic commit)
```

---

### 4. Enterprise Subsystems & UX Features

1. **⚡ 1-Click Topology Presets**:
   - *Simple Retail Preset*: 1 Store ─> 1 Retail POS ─> 1 Main Warehouse.
   - *Restaurant Preset*: 1 Store ─> 1 Resto POS ─> 1 Kitchen KDS ─> 1 Kitchen Pantry.
   - *Franchise Preset*: 1 Central Store ─> 3 POS Workspaces ─> 2 Warehouses.

2. **🎛️ Node Properties Inspector Drawer**:
   - Contextual right-side drawer to inspect and edit Store details, Workspace colors/roles, Warehouse low-stock alert thresholds, and Printer IP addresses.

3. **🧪 Live Order Simulation Debugger**:
   - A `[ Test Order Simulation ]` button triggers an animated test transaction where glowing energy pulse packets travel along arrow wires from POS to Warehouse and KDS/Printer nodes.

4. **🔲 Zone Group Containers**:
   - Collapsible regional containers (e.g. *"Jakarta Region"*, *"Logistics Hub"*) to group and drag multi-store clusters together.

5. **🔍 Topology Change Diff Preview**:
   - An `[ Apply Topology Changes ]` button opens a clean diff modal displaying precise SQLite record insertions, updates, and wire deletions before committing.

6. **⌨️ Canvas UX & Shortcuts**:
   - Pan/Zoom (0.2x – 2.0x), `Delete`/`Backspace` to remove nodes/wires, `Ctrl+Z`/`Ctrl+Y` undo/redo, and single-click wire mode toggle (`→` ↔ `↔`).

---

## Consequences

### Positive
- **Visual Clarity**: Eliminates administrative friction when configuring complex multi-store, multi-warehouse enterprises.
- **Immediate Validation**: Order simulation debugger prevents misconfigured stock deduction rules before going live.
- **Full Backend Alignment**: Maps cleanly to existing SQLite schemas (`store_profiles`, `workspace_instances`, `inventory_locations`) without requiring breaking schema migrations.

### Negative
- Requires maintaining SVG arrow connection rendering and node canvas state logic in React/TypeScript.

---

## Verification Plan

### Automated Tests
- Unit tests for topology serializer: canvas graph JSON → SQLite relational commands.
- Vitest React component tests for node dragging, wire connections, and undo/redo state stack.

### Manual Verification
- Open **Settings → Stores** → **Node Topology Editor**.
- Drag preset or custom nodes onto canvas and draw 1-Way and 2-Way arrow wires.
- Run **Test Order Simulation** and verify pulse animation travels along connected arrow paths.
- Click **Apply Topology Changes** and confirm SQLite updates atomically.
