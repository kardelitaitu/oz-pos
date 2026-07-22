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
5. **Clear Separation of Hardware Config**: Thermal printers, cash drawers, customer displays, and barcode scanners are peripherals tied directly to a **workspace instance**. Hardware peripheral configurations (IP address, paper width, driver, test prints) belong strictly inside workspace settings, keeping the topology builder focused on high-level enterprise node routing (Stores, Workspaces, Warehouses).

---

## Decision

We propose a **Bulletproof Unified 2-Tier Workspace Settings Architecture** built around **shared workspace settings components, real-time event-bus reactivity, offline CRDT delta sync, workspace-bound hardware peripheral controls, and visual node topology integration**.

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
 ║ • License & Cloud Sync  ║    ║ • Renders Shared Card   ║    ║ • Warehouse Nodes 📦    ║
 ║ • WORKSPACES SECTION:   ║    ║ • Workspace Hardware    ║    ║ • Stock Deduct Wires    ║
 ║   ├── Store POS Card    ║    ║   (Printers, Scanners)  ║    ║ • Click Node -> Opens   ║
 ║   ├── Resto POS Card    ║    ║ • "Admin Settings ↗"    ║    ║   Shared Settings Card  ║
 ║   ├── KDS Display Card  ║    ╚═════════════════════════╝    ╚═════════════════════════╝
 ║   └── Inventory Card    ║
 ╚═════════════════════════╝
```

---

### 2. Bulletproof Architectural Pillars

#### Pillar A: Shared Card Modularization
All workspace configuration logic (including printer, scanner, and scale hardware options) lives in modular, reusable card components:

```
ui/src/features/settings/
├── workspace-cards/
│   ├── WorkspaceStorePosSettings.tsx     <-- Store POS: Receipt Layout, Printers, Scanners, Scale, Presets
│   ├── WorkspaceRestaurantPosSettings.tsx <-- Restaurant POS: Kitchen Printers, Tables, Course Rules
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

#### Pillar B: Workspace-Bound Hardware Peripheral Configuration
Peripherals belong strictly to workspace cards, with strict separation between Store-wide and Device-Local settings:
- **Store-Wide Receipt Rules**: Store Logo, Footer Text, Tax display, Margins (mm) save to SQLite `receipt_settings`.
- **Register-Local Hardware Bindings**: Printer IP address/USB path, Serial Scale COM Port, and Barcode Handler save to the local **`terminal_profile.json`** / `localStorage` per terminal ID. This prevents Register #2 from overwriting Register #1's hardware connections!

#### Pillar C: Real-Time Event Bus Reactivity & Deduplicated Context Refetching
- When settings change, backend emits `settings_updated` via `crates/oz-bus`.
- **Deduplication**: `SettingsContext` hosts the single listener. When `settings_updated` fires, `SettingsContext` performs a single, debounced refetch and updates shared React state. Individual UI components subscribe to `SettingsContext` rather than initiating independent concurrent IPC calls.

#### Pillar D: Offline-First CRDT Delta Sync & Transaction Isolation
- All database mutations run inside **rusqlite transactions** (`conn.transaction()`).
- Settings changes write a CRDT delta ledger record (`setting_updated`) to sync seamlessly across multi-terminal offline clusters.

#### Pillar E: Visual Topology Canvas Integration (`NodeTopologyEditor.tsx`)
- **Inspector Drawer Integration**: Selecting any Workspace, Store, or Warehouse node on the Visual Topology Canvas (`NodeTopologyEditor.tsx`) opens the right Inspector Drawer, embedding the node's shared settings card.
- **Live Status Badges**: Topology nodes display live settings telemetry badges on the canvas (Receipt Configured `✓`, KDS SLA `5m`, Low Stock `12`).
- **Sidebar Integration**: `SettingsNavTree.tsx` features a **Topology Map** menu item that renders the interactive node diagram directly within `SettingsPage.tsx`.

#### Pillar F: Resilient Error Boundaries & Form Draft Isolation
- **Local Form Draft Isolation**: Form fields inside shared cards operate on local draft state (`useState`). Unsaved edits are isolated; closing the modal via `Esc` discards uncommitted drafts without polluting global application state or active cart calculations.
- **Error Boundaries**: Each shared card is wrapped in a localized `<ErrorBoundary>`. IPC errors render `<ErrorState retry={refetch} />` without crashing the parent view.

---

### 3. Edge Case Mitigations & Prevention Matrix

| Identified Edge Case / Potential Bug | Impact | Architectural Mitigation |
| :--- | :--- | :--- |
| **1. Hardware Overwrite across Registers** | Register #2 overwrites Register #1's printer IP. | Split Store-wide rules (SQLite) from Register-local hardware bindings (`terminal_profile.json`). |
| **2. POS Hotkey Leakage in Modal** | Pressing `F1` (Pay) while typing in settings input triggers checkout. | Enforce `if (document.querySelector('[aria-modal="true"]')) return;` guard on all POS key listeners. |
| **3. IPC Event Storm on Refetch** | Multiple subscribers fire duplicate `get_store_settings` IPC calls. | Centralize `SETTINGS_UPDATED` event listener inside `SettingsContext` with debounced refetches. |
| **4. Unsaved Draft Pollution** | Closing modal without saving leaves dirty state in cart/app. | Hold all form inputs in local draft state; only commit to context/IPC upon explicit user submission. |
| **5. Session Role Swap / Timeout** | Manager session times out or swaps to Cashier while modal is open. | Re-evaluate `useAuth()` session reactively inside `WorkspaceSettingsModal`; fallback to `TerminalPreferencesCard`. |
| **6. Visual Snap on Dismissal** | Modal snaps shut without exit animation. | Gate unmounting through `useExitAnimation` / `useAnimatedModal` duration. |

---

### 4. Storage Layer & Data Persistence Mapping

| Setting Scope | Target Persistence Layer | Backend IPC / Storage Key | Example Properties |
| :--- | :--- | :--- | :--- |
| **Global Store Settings** | SQLite DB (`store_settings`) | `set_store_settings_scoped` | Store Name, Address, Tax ID, Currency |
| **Receipt & Print Settings** | SQLite DB (`receipt_settings`) | `set_receipt_settings_scoped` | Paper width (58mm/80mm), Footer, Margins |
| **Terminal Hardware Bindings** | Device File / `localStorage` | `terminal_profile.json` | Printer IP/USB path, Scale COM port |
| **Topology Connections** | SQLite DB (`workspace_instances`) | `save_topology_diagram` | Node positions, stock deduction priority wires |
| **User Display Preferences** | SQLite DB (`user_preferences`) | `set_user_preference` | KDS layout mode (`kanban`/`focus`), Table toggles |
| **Local Terminal Preferences** | `localStorage` | `localStorage.setItem(...)` | Sound volume, Dark/Light theme, Scale Zeroing |

---

### 5. Accessibility (A11y) & Keyboard Navigation Standards

- **Keyboard Hotkey**: Pressing `F10` toggles `WorkspaceSettingsModal`.
- **Keyboard Dismissal**: Pressing `Esc` closes the modal using `useExitAnimation`.
- **Focus Management**: Uses `useFocusTrap` to trap tab focus within `WorkspaceSettingsModal` while open.
- **ARIA Semantics**:
  - Container uses `role="dialog"` and `aria-modal="true"`.
  - Heading linked via `aria-labelledby="workspace-settings-title"`.

---

### 6. Internationalization (i18n) & Fluent FTL Contract

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

### 7. Role-Based Access Control (RBAC) Matrix

| Role | Access Level | Canvas & Modal Behavior |
| :--- | :--- | :--- |
| **Owner** | Full Access | Full node drag/wire editing + Inspector settings cards + `Admin Settings ↗` button. |
| **Manager** | Store & Workspace Level | Node inspector viewing/editing + `Admin Settings ↗` button. |
| **Cashier / Staff** | Local Terminal Only | Topology canvas hidden. In-workspace settings limited to `TerminalPreferencesCard`. |

---

## Migration & Implementation Plan

| Phase | Description | Key Files Created/Modified |
| :--- | :--- | :--- |
| **Phase 1: Shared Cards & Local Profile** | Extract settings logic (with terminal profile separation for local hardware) into `WorkspaceStorePosSettings`, `WorkspaceRestaurantPosSettings`, `WorkspaceKdsSettings`, and `TerminalPreferencesCard`. | `ui/src/features/settings/workspace-cards/*`, `hooks/useTerminalProfile.ts` |
| **Phase 2: Topology Integration** | Wire shared workspace cards into `NodeTopologyEditor.tsx` right Inspector Drawer. | `NodeTopologyEditor.tsx`, `TopologyScreen.tsx` |
| **Phase 3: Tier 1 Integration** | Update `SettingsNavTree.tsx` & `SettingsPage.tsx` to include **Workspace Configurations** & **Topology Map**. | `SettingsNavTree.tsx`, `SettingsPage.tsx` |
| **Phase 4: Tier 2 Modal** | Implement `WorkspaceSettingsModal.tsx` with Event Bus subscription, role checking, hotkey isolation, and `Admin Settings ↗` header shortcut. | `WorkspaceSettingsModal.tsx` |
| **Phase 5: Workspace Wiring** | Replace `RetailOptionsScreen` in `RetailPosScreen.tsx` and `SettingsSubScreen` in `PosScreen.tsx` with `WorkspaceSettingsModal`. Add `aria-modal="true"` guards to POS hotkey handlers. | `RetailPosScreen.tsx`, `PosScreen.tsx` |
| **Phase 6: Deprecation** | Delete obsolete `RetailOptionsScreen.tsx` and clean up legacy CSS rules. | `RetailOptionsScreen.tsx`, `RetailPosScreen.css` |
