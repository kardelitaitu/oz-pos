# ADR #22: Unified 2-Tier Workspace Settings Architecture — Centralized Hub & In-Workspace Contextual Controls

**Status:** Proposed  
**Date:** 2026-07-23  
**Author:** Architecture Team & OZ-POS Contributors  
**Tags:** settings, workspace, architecture, rbac, ui-components, design-system, i18n, a11y  

---

## Context

Settings management in OZ-POS is currently fragmented across **5 distinct components and UI paradigms**:

1. **`SettingsPage.tsx`** (`ui/src/features/settings/SettingsPage.tsx`): Full-screen Settings Hub with a 13-section left sidebar tree (`SettingsNavTree.tsx`), accessible from the Admin workspace (`#/settings`). Uses modern design tokens and dither cards.
2. **`RetailOptionsScreen.tsx`** (`ui/src/features/retail/RetailOptionsScreen.tsx`): 8-tab modal overlay used when pressing `F10` / `Options` inside Store POS (`RetailPosScreen.tsx`). Uses legacy modal styles, hardcoded colors, and raw inputs.
3. **`SettingsSubScreen`** (`ui/src/features/sales/PosScreen.tsx`): 4-tab inline screen used when clicking the gear icon inside Restaurant POS (`PosScreen.tsx`).
4. **`SettingsPopup.tsx`** (`ui/src/frontend/shared/SettingsPopup.tsx`): 4-tab quick settings modal popup used in various management screens (Tax, Categories, Customers, Staff, Terminals, Suppliers).
5. **`KdsSettingsPanel.tsx`** (`ui/src/features/kds/KdsSettingsPanel.tsx`): Popover panel specifically for KDS screen preferences.

### Operational Challenges

1. **Inconsistent User Experience**: Depending on where a user opens settings, they see different tab layouts, styling tokens, and form controls.
2. **Duplicate Code & Maintenance Overhead**: Backend IPC commands (e.g. `get_store_settings`, `get_receipt_settings`) are duplicated across `RetailOptionsScreen`, `SettingsSubScreen`, and `SettingsPage`.
3. **Owner / Manager Needs vs. Cashier Scoping**:
   - **Owners & Managers** need a **Centralized Hub** where they can configure *all* workspace settings (Store POS, Restaurant POS, KDS, Inventory, Admin) from a single location without entering each workspace.
   - **In-Workspace Context**: When working inside a specific workspace (e.g. Store POS), an Owner or Manager needs quick access to *that workspace's settings* without wading through unrelated global options.
   - **Cashier / Staff Roles**: Staff working in a POS workspace should only have access to local terminal preferences (e.g. sound volume, dark mode toggle, scale zeroing), while administrative store settings must be protected.

---

## Decision

We propose a **Unified 2-Tier Workspace Settings Architecture** built around **shared workspace settings components**.

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

### 2. Architecture & Shared Component Strategy

To eliminate component duplication between Tier 1 and Tier 2, all workspace settings forms will be extracted into **reusable workspace card components** under `ui/src/features/settings/workspace-cards/`:

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

#### Reusable Card Interface

Each workspace settings card exposes a clean, standardized component API:

```tsx
export interface WorkspaceCardProps {
  /** Session token for scoped IPC calls (ADR #7) */
  sessionToken?: string;
  /** Whether the component is rendered inside Tier 2 modal or Tier 1 full page */
  variant?: 'full-page' | 'modal';
  /** Callback fired after settings are successfully saved */
  onSaved?: () => void;
}
```

---

### 3. Tier 1: Centralized Settings Hub Layout (`SettingsPage.tsx`)

In `SettingsNavTree.tsx`, a dedicated **Workspace Configurations** category will be added to the left sidebar tree:

```
⚙️ Settings
├── 🌐 Global System
│   ├── General & Store Info
│   ├── Appearance & Themes
│   ├── Tax & Currency
│   ├── Staff & Roles
│   ├── Cloud Sync & License
│   └── Data Management
│
└── 🖥️ Workspace Configurations
    ├── Store POS
    ├── Restaurant POS
    ├── Kitchen Display (KDS)
    └── Inventory Management
```

Selecting any workspace under **Workspace Configurations** renders the corresponding shared card (`WorkspaceStorePosSettings`, `WorkspaceKdsSettings`, etc.) within the main dithered card container.

---

### 4. Tier 2: In-Workspace Contextual Modal (`WorkspaceSettingsModal.tsx`)

When an Owner or Manager opens settings inside an active workspace (e.g. pressing `F10` in Store POS or clicking the Gear icon in Restaurant POS/KDS), `WorkspaceSettingsModal.tsx` opens as a modern slide-over/dialog modal.

#### Modal Header Features:
- **Title**: Shows active workspace name (e.g., `Store POS Settings`).
- **Scope Indicator**: Displays active workspace badge.
- **Admin Shortcut Button**: `Admin Settings ↗` button. Clicking this closes the modal and navigates directly to `#/settings` (Tier 1) for full system configuration.
- **Close Button**: Symmetric exit animation via `useExitAnimation`.

#### Modal Body:
- Renders the workspace's dedicated shared card (`WorkspaceStorePosSettings`).
- Renders `TerminalPreferencesCard` for hardware/display tweaks specific to the local device.

---

### 5. Storage Layer & Data Persistence Mapping

Settings across OZ-POS belong to 3 distinct persistence categories. Shared workspace cards map their state according to this structure:

| Setting Scope | Target Persistence Layer | Backend IPC / Storage Key | Example Properties |
| :--- | :--- | :--- | :--- |
| **Global Store Settings** | SQLite DB (`store_settings`) | `set_store_settings_scoped` | Store Name, Address, Tax ID, Currency |
| **Receipt & Print Settings** | SQLite DB (`receipt_settings`) | `set_receipt_settings_scoped` | Paper width (58mm/80mm), Footer, Margins |
| **User Display Preferences** | SQLite DB (`user_preferences`) | `set_user_preference` | KDS layout mode (`kanban`/`focus`), Table toggles |
| **Local Terminal Preferences** | `localStorage` / Device File | `localStorage.setItem(...)` | Sound volume, Dark/Light theme, Scale Zeroing |

---

### 6. Accessibility (A11y) & Keyboard Navigation Standards

To maintain POS keyboard usability and accessibility compliance:

- **Keyboard Hotkey**: Pressing `F10` toggles `WorkspaceSettingsModal`.
- **Keyboard Dismissal**: Pressing `Esc` closes the modal using `useExitAnimation` for smooth entry/exit.
- **Focus Management**: Uses `useFocusTrap` to trap tab focus within `WorkspaceSettingsModal` while open.
- **ARIA Semantics**:
  - Container uses `role="dialog"` and `aria-modal="true"`.
  - Heading linked via `aria-labelledby="workspace-settings-title"`.
  - Close button has explicit `aria-label`.

---

### 7. Internationalization (i18n) & Fluent FTL Contract

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

### 8. Role-Based Access Control (RBAC) Matrix

| Role | Access Level | In-Workspace Behavior |
| :--- | :--- | :--- |
| **Owner** | Full Access | Renders full `WorkspaceSettingsModal` with active workspace card + `Admin Settings ↗` button. |
| **Manager** | Store & Workspace Level | Renders `WorkspaceSettingsModal` with active workspace card + `Admin Settings ↗` button. |
| **Cashier / Staff** | Local Terminal Only | Renders restricted `TerminalPreferencesCard` (Theme, Sound, Scale Zeroing). Admin settings and shortcut link are hidden. |

---

## Consequences

### Positive
- **Single Source of Truth**: Setting forms (inputs, validation, IPC calls) exist in one shared file per workspace.
- **Visual Consistency**: All settings UI uses modern design system tokens, dither cards (`.noise-dither`), and `@fluent/react` localization strings.
- **Owner Efficiency**: Owners/Managers can adjust any workspace from the Admin settings hub, or quickly tweak the active workspace in-context.
- **Clean Deprecation**: Obsoletes `RetailOptionsScreen.tsx`, `SettingsSubScreen`, and `SettingsPopup.tsx`.

### Risks & Mitigations
- **IPC State Parity**: Shared workspace cards must maintain state synchronization across both Tier 1 and Tier 2 views.  
  *Mitigation*: Use existing IPC hooks (`useStoreSettings`, `useReceiptSettings`) and invalidate cache on save.

---

## Verification & Testing Strategy

1. **Unit Tests**:
   - `WorkspaceSettingsModal.test.tsx`: Test modal render, focus trapping, `Esc` closing, and `Admin Settings ↗` navigation.
   - `WorkspaceStorePosSettings.test.tsx`: Test form load and save routines for Store POS settings.
2. **Noise-Dither Test**: Add `.workspace-settings-modal` to `KNOWN_NOISE_SELECTORS` in `ui/src/__tests__/noiseDitherCompliance.test.ts`.
3. **E2E Playwright**: Update `ui/e2e/new-flows.spec.ts` (E2E-27) to verify opening settings via `F10` in Store POS.

---

## Migration & Implementation Plan

| Phase | Description | Key Files Created/Modified |
| :--- | :--- | :--- |
| **Phase 1: Shared Cards** | Extract settings logic into `WorkspaceStorePosSettings`, `WorkspaceKdsSettings`, and `TerminalPreferencesCard`. | `ui/src/features/settings/workspace-cards/*` |
| **Phase 2: Tier 1 Integration** | Update `SettingsNavTree.tsx` & `SettingsPage.tsx` to include the **Workspace Configurations** tree group. | `SettingsNavTree.tsx`, `SettingsPage.tsx` |
| **Phase 3: Tier 2 Modal** | Implement `WorkspaceSettingsModal.tsx` with role checking and `Admin Settings ↗` header shortcut. | `WorkspaceSettingsModal.tsx` |
| **Phase 4: Workspace Wiring** | Replace `RetailOptionsScreen` in `RetailPosScreen.tsx` and `SettingsSubScreen` in `PosScreen.tsx` with `WorkspaceSettingsModal`. | `RetailPosScreen.tsx`, `PosScreen.tsx` |
| **Phase 5: Deprecation** | Delete obsolete `RetailOptionsScreen.tsx` and clean up legacy CSS rules. | `RetailOptionsScreen.tsx`, `RetailPosScreen.css` |
