# ADR #22: Unified 2-Tier Workspace Settings Architecture — Centralized Hub & In-Workspace Contextual Controls

**Status:** Proposed  
**Date:** 2026-07-23  
**Author:** Architecture Team & OZ-POS Contributors  
**Tags:** settings, workspace, architecture, rbac, ui-components, design-system, i18n, a11y, multi-location, hal, crdt, event-bus, node-topology  

---

## Context

Settings management in OZ-POS is currently fragmented across **5 distinct components and UI paradigms**:

1. **`SettingsPage.tsx`** (`ui/src/features/settings/SettingsPage.tsx`): Full-screen Settings Hub with a 13-section left sidebar tree (`SettingsNavTree.tsx`), accessible from the Admin workspace (`#/settings`). Uses modern design tokens and dither cards.
2. **`RetailOptionsScreen.tsx`** (`ui/src/features/retail/RetailOptionsScreen.tsx`): 8-tab modal overlay used when pressing `F10` / `Options` inside Store POS (`RetailPosScreen.tsx`). Uses legacy modal styles, hardcoded colors, and raw inputs.
3. **`SettingsSubScreen`** (`ui/src/features/sales/PosScreen.tsx`): 4-tab inline screen used when clicking the gear icon inside Restaurant POS (`PosScreen.tsx`).
4. **`SettingsPopup.tsx`** (`ui/src/frontend/shared/SettingsPopup.tsx`): 4-tab quick settings modal popup used in various management screens (Tax, Categories, Customers, Staff, Terminals, Suppliers).
5. **`KdsSettingsPanel.tsx`** (`ui/src/features/kds/KdsSettingsPanel.tsx`): Popover panel specifically for KDS screen preferences.

### Operational & Technical Challenges

1. **Inconsistent User Experience**: Depending on where a user opens settings, they see different tab layouts, styling tokens, and form controls.
2. **Duplicate Code & Maintenance Overhead**: Backend IPC commands (e.g. `get_store_settings`, `get_receipt_settings`) are duplicated across `RetailOptionsScreen`, `SettingsSubScreen`, and `SettingsPage`.
3. **Owner / Manager Needs vs. Cashier Scoping**:
   - **Owners & Managers** need a **Centralized Hub** where they can configure *all* workspace settings (Store POS, Restaurant POS, KDS, Inventory, Admin) from a single location without entering each workspace.
   - **In-Workspace Context**: When working inside a specific workspace (e.g. Store POS), an Owner or Manager needs quick access to *that workspace's settings* without wading through unrelated global options.
   - **Cashier / Staff Roles**: Staff working in a POS workspace should only have access to local terminal preferences (e.g. sound volume, dark mode toggle, scale zeroing), while administrative store settings must be protected.
4. **Lack of Real-Time Reactivity**: Changing a setting (e.g. tender presets or KDS SLA timers) in one screen currently requires a full manual reload for active UI components to reflect the update.
5. **Topology Disconnect**: The Visual Store Topology Builder (`NodeTopologyEditor.tsx` / ADR #20) renders store nodes, workspace registers, and hardware peripherals, but lacks direct visual integration with workspace setting configurations.

---

## Decision

We propose a **Bulletproof Unified 2-Tier Workspace Settings Architecture** built around **shared workspace settings components, real-time event-bus reactivity, offline CRDT delta sync, hardware driver bindings, and visual node topology integration**.

### 1. Conceptual Model — 2-Tier Dual-Access & Topology Model

```
                    ┌─────────────────────────────────────────┐
                    │       OWNER / MANAGER AUTHENTICATED     │
                    └────────────────────┬────────────────────┘
                                         │
        ┌────────────────────────────────┼────────────────────────────────┐
        ▼                                ▼                                ▼
 ╔═════════════════════════╗    ╔═════════════════════════╗    ╔═════════════════════════╗
 ║  TIER 1: CENTRAL HUB    ║    ║ TIER 2: IN-WORKSPACE    ║    ║ TIER 3: TOPOLOGY CANVAS ║
 ║ (Admin / #/settings)    ║    ║  (Quick Modal / F10)    ║    ║  (Visual Node Diagram)  ║
 ╠═════════════════════════╣    ╠═════════════════════════╣    ╠═════════════════════════╣
 ║ • Global Store Config   ║    ║ • Auto-scoped to active ║    ║ • Store Branch Nodes 🏢 ║
 ║ • Staff & Security      ║    ║   workspace             ║    ║ • Workspace Nodes 🛒/🍽️ ║
 ║ • License & Cloud Sync  ║    ║ • Renders Shared Card   ║    ║ • Hardware Nodes 🖨️      ║
 ║ • WORKSPACES SECTION:   ║    ║ • Quick Hardware Tweak  ║    ║ • Live Status Badges    ║
 ║   ├── Store POS Card    ║    ║ • "Admin Settings ↗"    ║    ║ • Click Node -> Opens   ║
 ║   ├── Resto POS Card    ║    ╚═════════════════════════╝    ║   Shared Settings Card  ║
 ║   ├── KDS Display Card  ║                                   ╚═════════════════════════╝
 ║   └── Inventory Card    ║
 ╚═════════════════════════╝
```

---

### 2. Bulletproof Architectural Pillars

#### Pillar A: Shared Card Modularization
All workspace configuration logic lives in modular, reusable card components:

```
ui/src/features/settings/
├── workspace-cards/
│   ├── WorkspaceStorePosSettings.tsx     <-- Store POS: Receipt, Scanner, Scale, Tender Presets
│   ├── WorkspaceRestaurantPosSettings.tsx <-- Restaurant POS: Table Layout, Course Rules, Kitchen Printer
│   ├── WorkspaceKdsSettings.tsx           <-- KDS: Layout, SLA Timers, Ticket Colors, Audio Chimes
│   ├── WorkspaceInventorySettings.tsx     <-- Inventory: Low Stock Thresholds, Deduction Location
│   └── TerminalPreferencesCard.tsx        <-- Local Terminal: Theme, Sound, Scale Zeroing
├── SettingsNavTree.tsx                    <-- Nav tree with 'Workspace Configurations' & 'Topology'
├── SettingsPage.tsx                       <-- Tier 1 Central Hub (Renders Shared Cards & Canvas)
└── WorkspaceSettingsModal.tsx             <-- Tier 2 Contextual Modal (Renders Active Shared Card)
```

```tsx
export interface WorkspaceCardProps {
  /** Session token for scoped IPC calls (ADR #7) */
  sessionToken?: string;
  /** Active location ID for multi-location scoping (ADR #18) */
  locationId?: string;
  /** Whether the component is rendered inside Tier 2 modal or Tier 1 full page */
  variant?: 'full-page' | 'modal' | 'inspector-drawer';
  /** Callback fired after settings are successfully saved */
  onSaved?: () => void;
}
```

#### Pillar B: Real-Time Event Bus Reactivity (`SETTINGS_UPDATED`)
When any setting is saved in Tier 1, Tier 2, or the Topology Inspector, the Rust backend emits a `settings_updated` event via `crates/oz-bus`.
The frontend `SettingsContext` listens to this event and invalidates stale state immediately across open screens.

#### Pillar C: Offline-First CRDT Delta Sync & Transaction Isolation
- All database mutations run inside **rusqlite transactions** (`conn.transaction()`).
- Settings changes write a CRDT delta ledger record (`setting_updated`) to sync seamlessly across multi-terminal offline clusters.

#### Pillar D: Hardware Abstraction Layer (HAL) Peripheral Testing
Workspace settings cards integrate directly with `crates/oz-hal` drivers:
- **Receipt & Printer**: Embedded **"Test Print"** button sending ESC/POS test packets.
- **Weight Scale**: Real-time **"Scale Diagnostics & Zeroing"** widget reading HAL serial data.
- **Barcode Scanner**: Live **"Test Barcode Scan"** verification input.

#### Pillar E: Visual Topology Canvas Integration (`NodeTopologyEditor.tsx`)
- **Inspector Drawer Integration**: Selecting any Workspace, Store, Warehouse, or Hardware node on the Visual Topology Canvas (`NodeTopologyEditor.tsx`) opens the right Inspector Drawer, embedding the node's shared settings card.
- **Live Status Badges**: Topology nodes display live settings telemetry badges on the canvas (Receipt Configured `✓`, Printer `192.168.1.100 OK`, KDS SLA `5m`, Low Stock `12`).
- **Sidebar Integration**: `SettingsNavTree.tsx` features a **Topology Map** menu item that renders the interactive node diagram directly within `SettingsPage.tsx`.

#### Pillar F: Resilient Error Boundaries & Fallbacks
Each shared card is wrapped in a dedicated `<ErrorBoundary>`:
- IPC network failures or schema validation errors render a localized `<ErrorState retry={refetch} />` without crashing the parent canvas or POS session.

---

### 3. Storage Layer & Data Persistence Mapping

| Setting Scope | Target Persistence Layer | Backend IPC / Storage Key | Example Properties |
| :--- | :--- | :--- | :--- |
| **Global Store Settings** | SQLite DB (`store_settings`) | `set_store_settings_scoped` | Store Name, Address, Tax ID, Currency |
| **Receipt & Print Settings** | SQLite DB (`receipt_settings`) | `set_receipt_settings_scoped` | Paper width (58mm/80mm), Footer, Margins |
| **Topology Connections** | SQLite DB (`workspace_instances`) | `save_topology_diagram` | Wire connections, node positions, port routing |
| **User Display Preferences** | SQLite DB (`user_preferences`) | `set_user_preference` | KDS layout mode (`kanban`/`focus`), Table toggles |
| **Local Terminal Preferences** | `localStorage` / Device File | `localStorage.setItem(...)` | Sound volume, Dark/Light theme, Scale Zeroing |

---

### 4. Accessibility (A11y) & Keyboard Navigation Standards

- **Keyboard Hotkey**: Pressing `F10` toggles `WorkspaceSettingsModal`.
- **Keyboard Dismissal**: Pressing `Esc` closes the modal using `useExitAnimation`.
- **Focus Management**: Uses `useFocusTrap` to trap tab focus within `WorkspaceSettingsModal` while open.
- **ARIA Semantics**:
  - Container uses `role="dialog"` and `aria-modal="true"`.
  - Heading linked via `aria-labelledby="workspace-settings-title"`.

---

### 5. Internationalization (i18n) & Fluent FTL Contract

In accordance with project standards and githooks bundle parity verification (`scripts/verify-bundle-parity.py`):
- All user-visible text uses `@fluent/react` (`<Localized id="...">`).
- New Fluent keys added symmetrically to `ui/src/locales/settings.ftl` and `ui/src/locales/settings.id.ftl`:
  - `settings-workspace-category-title`
  - `settings-workspace-store-pos`
  - `settings-workspace-restaurant-pos`
  - `settings-workspace-kds`
  - `settings-workspace-inventory`
  - `settings-workspace-topology-map`
  - `settings-workspace-admin-shortcut`

---

### 6. Role-Based Access Control (RBAC) Matrix

| Role | Access Level | Canvas & Modal Behavior |
| :--- | :--- | :--- |
| **Owner** | Full Access | Full node drag/wire editing + Inspector settings cards + `Admin Settings ↗` button. |
| **Manager** | Store & Workspace Level | Node inspector viewing/editing + `Admin Settings ↗` button. |
| **Cashier / Staff** | Local Terminal Only | Topology canvas hidden. In-workspace settings limited to `TerminalPreferencesCard`. |

---

## Migration & Implementation Plan

| Phase | Description | Key Files Created/Modified |
| :--- | :--- | :--- |
| **Phase 1: Shared Cards** | Extract settings logic into `WorkspaceStorePosSettings`, `WorkspaceKdsSettings`, and `TerminalPreferencesCard` with HAL test actions. | `ui/src/features/settings/workspace-cards/*` |
| **Phase 2: Topology Integration** | Wire shared settings cards into `NodeTopologyEditor.tsx` right Inspector Drawer. | `NodeTopologyEditor.tsx`, `TopologyScreen.tsx` |
| **Phase 3: Tier 1 Integration** | Update `SettingsNavTree.tsx` & `SettingsPage.tsx` to include **Workspace Configurations** & **Topology Map**. | `SettingsNavTree.tsx`, `SettingsPage.tsx` |
| **Phase 4: Tier 2 Modal** | Implement `WorkspaceSettingsModal.tsx` with Event Bus subscription, role checking, and `Admin Settings ↗` header shortcut. | `WorkspaceSettingsModal.tsx` |
| **Phase 5: Workspace Wiring** | Replace `RetailOptionsScreen` in `RetailPosScreen.tsx` and `SettingsSubScreen` in `PosScreen.tsx` with `WorkspaceSettingsModal`. | `RetailPosScreen.tsx`, `PosScreen.tsx` |
| **Phase 6: Deprecation** | Delete obsolete `RetailOptionsScreen.tsx` and clean up legacy CSS rules. | `RetailOptionsScreen.tsx`, `RetailPosScreen.css` |
