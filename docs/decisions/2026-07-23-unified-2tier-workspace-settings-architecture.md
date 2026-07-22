# ADR #22: Unified 2-Tier Workspace Settings Architecture — Centralized Hub & In-Workspace Contextual Controls

**Status:** Proposed  
**Date:** 2026-07-23  
**Author:** Architecture Team & OZ-POS Contributors  
**Tags:** settings, workspace, architecture, rbac, ui-components, design-system, i18n, a11y, multi-location, hal, crdt, event-bus  

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
5. **Multi-Location & Hardware Isolation**: Workspace settings must respect location-scoped overrides (ADR #18 / ADR #19) and bind cleanly to HAL peripheral drivers (`crates/oz-hal`).

---

## Decision

We propose a **Bulletproof Unified 2-Tier Workspace Settings Architecture** built around **shared workspace settings components, real-time event-bus reactivity, offline CRDT delta sync, and hardware driver bindings**.

### 1. Conceptual Model — 2-Tier Dual-Access Architecture

```
                    ┌─────────────────────────────────────────┐
                    │       OWNER / MANAGER AUTHENTICATED     │
                    └────────────────────┬────────────────────┘
                                         │
        ┌────────────────────────────────┴────────────────────────────────┐
        ▼                                                                 ▼
 ╔═══════════════════════════════════════╗               ╔═══════════════════════════════════════╗
 ║        TIER 1: CENTRAL HUB            ║               ║      TIER 2: CONTEXTUAL IN-WORKSPACE   ║
 ║     (Admin Workspace / #/settings)    ║               ║       (Unified Quick Settings Modal)  ║
 ╠═══════════════════════════════════════╣               ╠═══════════════════════════════════════╣
 ║ • Store Info & Tax Configuration      ║               ║ • Auto-scoped to active workspace     ║
 ║ • Staff & Security Management         ║               ║ • Renders Shared Workspace Card       ║
 ║ • License & Cloud Sync                ║               ║ • Quick Hardware & Preset Controls    ║
 ║ • WORKSPACES SECTION (All Workspaces):║               ║ • "Admin Settings ↗" Header Shortcut  ║
 ║   ├── Store POS Settings Card         ║               ╚═══════════════════════════════════════╝
 ║   ├── Restaurant POS Settings Card    ║
 ║   ├── KDS Display Settings Card       ║
 ║   └── Inventory & Audit Settings Card ║
 ╚═══════════════════════════════════════╝
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
├── SettingsNavTree.tsx                    <-- Nav tree with 'Workspace Configurations' section
├── SettingsPage.tsx                       <-- Tier 1 Central Hub (Renders Shared Cards)
└── WorkspaceSettingsModal.tsx             <-- Tier 2 Contextual Modal (Renders Active Shared Card)
```

```tsx
export interface WorkspaceCardProps {
  /** Session token for scoped IPC calls (ADR #7) */
  sessionToken?: string;
  /** Active location ID for multi-location scoping (ADR #18) */
  locationId?: string;
  /** Whether the component is rendered inside Tier 2 modal or Tier 1 full page */
  variant?: 'full-page' | 'modal';
  /** Callback fired after settings are successfully saved */
  onSaved?: () => void;
}
```

#### Pillar B: Real-Time Event Bus Reactivity (`SETTINGS_UPDATED`)
When any setting is saved in Tier 1 or Tier 2, the Rust backend emits a `settings_updated` scoped event via the Event Bus (`crates/oz-bus`).
The frontend `SettingsContext` listens to this event and invalidates stale state immediately:
- Updating receipt layout in Settings instantly updates the preview in POS cart without F5.
- Changing KDS SLA thresholds immediately updates open KDS ticket timers.

```tsx
useScopedEvent('settings_updated', (payload) => {
  if (payload.scope === activeWorkspace) {
    refetchSettings();
  }
});
```

#### Pillar C: Offline-First CRDT Delta Sync & Transaction Isolation
- All database mutations run inside **rusqlite transactions** (`conn.transaction()`).
- Settings changes write a CRDT delta ledger record (`setting_updated`) to sync seamlessly across multi-terminal offline clusters.
- Conflict resolution uses `last-write-wins` with wall-clock vector timestamps.

#### Pillar D: Hardware Abstraction Layer (HAL) Peripheral Testing
Workspace settings cards integrate directly with `crates/oz-hal` drivers:
- **Receipt & Printer**: Embedded **"Test Print"** button that sends raw ESC/POS test packets to the configured printer driver.
- **Weight Scale**: Real-time **"Scale Diagnostics & Zeroing"** widget reading live HAL serial data.
- **Barcode Scanner**: Live **"Test Barcode Scan"** input box to verify HAL scanner handler input.

#### Pillar E: Entitlements & Subscription Tier Guarding
Features gated by subscription tier (`Basic`, `Pro`, `Enterprise`) render graceful entitlement badges:
- If a store is on `Basic`, advanced multi-location inventory deduction settings show a locked `Pro Feature 🔒` badge with an upgrade callout.

#### Pillar F: Resilient Error Boundaries & Fallback
Each shared card is wrapped in a dedicated `<ErrorBoundary>`:
- If an IPC network call fails or schema validation errors occur, the card renders a localized `<ErrorState retry={refetch} />` without crashing the parent Settings page or POS session.

---

### 3. Storage Layer & Data Persistence Mapping

Settings belong to 3 distinct persistence categories:

| Setting Scope | Target Persistence Layer | Backend IPC / Storage Key | Example Properties |
| :--- | :--- | :--- | :--- |
| **Global Store Settings** | SQLite DB (`store_settings`) | `set_store_settings_scoped` | Store Name, Address, Tax ID, Currency |
| **Receipt & Print Settings** | SQLite DB (`receipt_settings`) | `set_receipt_settings_scoped` | Paper width (58mm/80mm), Footer, Margins |
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
  - Close button has explicit `aria-label`.

---

### 5. Internationalization (i18n) & Fluent FTL Contract

In accordance with project standards and githooks bundle parity verification (`scripts/verify-bundle-parity.py`):
- All user-visible text uses `@fluent/react` (`<Localized id="...">`).
- New Fluent keys are added symmetrically to both `ui/src/locales/settings.ftl` and `ui/src/locales/settings.id.ftl`:
  - `settings-workspace-category-title`
  - `settings-workspace-store-pos`
  - `settings-workspace-restaurant-pos`
  - `settings-workspace-kds`
  - `settings-workspace-inventory`
  - `settings-workspace-admin-shortcut`

---

### 6. Role-Based Access Control (RBAC) Matrix

| Role | Access Level | In-Workspace Behavior |
| :--- | :--- | :--- |
| **Owner** | Full Access | Renders full `WorkspaceSettingsModal` with active workspace card + `Admin Settings ↗` button. |
| **Manager** | Store & Workspace Level | Renders `WorkspaceSettingsModal` with active workspace card + `Admin Settings ↗` button. |
| **Cashier / Staff** | Local Terminal Only | Renders restricted `TerminalPreferencesCard` (Theme, Sound, Scale Zeroing). Admin settings and shortcut link are hidden. |

---

## Consequences

### Positive
- **Single Source of Truth**: Setting forms exist in one shared file per workspace.
- **Real-Time Responsiveness**: Setting changes propagate instantly across open UI screens via Event Bus.
- **Hardware Diagnostic Confidence**: Integrated HAL test buttons eliminate guesswork during hardware setup.
- **Clean Deprecation**: Obsoletes `RetailOptionsScreen.tsx`, `SettingsSubScreen`, and `SettingsPopup.tsx`.

---

## Verification & Testing Strategy

1. **Unit Tests**:
   - `WorkspaceSettingsModal.test.tsx`: Test modal render, focus trapping, `Esc` closing, and `Admin Settings ↗` navigation.
   - `WorkspaceStorePosSettings.test.tsx`: Test form load, HAL test button triggers, and save routines.
2. **Noise-Dither Test**: Add `.workspace-settings-modal` to `KNOWN_NOISE_SELECTORS` in `ui/src/__tests__/noiseDitherCompliance.test.ts`.
3. **E2E Playwright**: Update `ui/e2e/new-flows.spec.ts` (E2E-27) to verify opening settings via `F10` in Store POS.

---

## Migration & Implementation Plan

| Phase | Description | Key Files Created/Modified |
| :--- | :--- | :--- |
| **Phase 1: Shared Cards** | Extract settings logic into `WorkspaceStorePosSettings`, `WorkspaceKdsSettings`, and `TerminalPreferencesCard` with HAL test actions. | `ui/src/features/settings/workspace-cards/*` |
| **Phase 2: Tier 1 Integration** | Update `SettingsNavTree.tsx` & `SettingsPage.tsx` to include the **Workspace Configurations** tree group. | `SettingsNavTree.tsx`, `SettingsPage.tsx` |
| **Phase 3: Tier 2 Modal** | Implement `WorkspaceSettingsModal.tsx` with Event Bus subscription, role checking, and `Admin Settings ↗` header shortcut. | `WorkspaceSettingsModal.tsx` |
| **Phase 4: Workspace Wiring** | Replace `RetailOptionsScreen` in `RetailPosScreen.tsx` and `SettingsSubScreen` in `PosScreen.tsx` with `WorkspaceSettingsModal`. | `RetailPosScreen.tsx`, `PosScreen.tsx` |
| **Phase 5: Deprecation** | Delete obsolete `RetailOptionsScreen.tsx` and clean up legacy CSS rules. | `RetailOptionsScreen.tsx`, `RetailPosScreen.css` |
