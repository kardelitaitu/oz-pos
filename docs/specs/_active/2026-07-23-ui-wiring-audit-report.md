# UI Wiring & Element Audit Report — Settings (OZ-POS Desktop App)

- **Audit ID:** 2026-07-23-ui-wiring-audit
- **Status:** Report first — findings only, no fixes applied
- **Scope:** `ui/src/features/settings/` (desktop client only; tablet client excluded)
- **Method:** Static code audit (no test runs)

---

## 1. Files audited

| File | Purpose |
|------|---------|
| `SettingsPage.tsx` | Main settings hub, sync, updates, store info |
| `AppearanceSettings.tsx` | Branding, colour, logo, store name |
| `FeatureToggleScreen.tsx` | Feature flag management |
| `DataManagementScreen.tsx` | Export, import, backup |
| `EmailReportSettings.tsx` | SMTP and email schedule |
| `LicenseSettings.tsx` | License info and server status |
| `SettingsNavTree.tsx` | Sidebar navigation |
| `SettingsSelect.tsx` | Custom dropdown component |

---

## 2. Summary of findings

| Severity | Count |
|----------|-------|
| Critical | 0 |
| High | 1 |
| Medium | 5 |
| Low | 2 |
| **Total** | **8** |

### High

#### H-1 — Native `confirm()` used for destructive reset in `AppearanceSettings.tsx`

- **Location:** `ui/src/features/settings/AppearanceSettings.tsx:147`
- **Current code:** `if (!window.confirm(l10n.getString('appearance-reset-all-confirm'))) return;`
- **Issue:** Uses browser native `confirm()` instead of the project's `ConfirmDialog`. This bypasses the Fluent localisation pipeline, is inconsistent with the rest of the app, and is harder to style/test.
- **Recommendation:** Replace with `<ConfirmDialog>` wired to `handleResetAll`.

### Medium

#### M-1 — Save action in `SettingsPage` is not tied to a form submit

- **Location:** `ui/src/features/settings/SettingsPage.tsx:1703`
- **Issue:** `handleSave` is bound to a `<Button onClick={handleSave}>`. There is no `<form onSubmit={handleSave}>`, so pressing Enter in fields does not save. This is a wiring gap for keyboard users.
- **Recommendation:** Wrap the relevant settings fields in a `<form>` and bind `handleSave` to `onSubmit`; keep the button `type="submit"`.

#### M-2 — Some raw `<button>` elements missing explicit `type="button"`

- **Files:** `AppearanceSettings.tsx`, `EmailReportSettings.tsx`, `DataManagementScreen.tsx`, `FeatureToggleScreen.tsx`
- **Issue:** Most raw buttons have `type="button"`, but a few rely on the implicit default. If these are ever moved inside a form, they will submit unexpectedly.
- **Recommendation:** Add explicit `type="button"` to every raw `<button>` in settings.

#### M-3 — `DataManagementScreen` import/confirm wiring unclear without deeper inspection

- **Location:** `ui/src/features/settings/DataManagementScreen.tsx`
- **Issue:** The screen has multiple async buttons (export, import, backup). From the static search it is unclear whether all destructive actions disable appropriately and whether `ConfirmDialog` is used before overwrite operations.
- **Recommendation:** Manually trace the export/import handlers and add explicit confirmation dialogs for destructive operations.

#### M-4 — `EmailReportSettings` schedule inputs lack obvious validation wiring

- **Location:** `ui/src/features/settings/EmailReportSettings.tsx`
- **Issue:** Schedule fields (cadence, time, timezone, lookback days) have `onChange` handlers, but no visible validation or disabled-save state is apparent from the static scan.
- **Recommendation:** Verify that invalid schedule values disable the Save button and surface errors.

#### M-5 — `LicenseSettings` refresh button state is local only

- **Location:** `ui/src/features/settings/LicenseSettings.tsx:386`
- **Issue:** `loading={checkingServer}` is wired, but there is no obvious error recovery if the server check fails (no retry or toast feedback visible in the search results).
- **Recommendation:** Confirm error feedback path; add a retry button or toast on failure.

### Low

#### L-1 — Mixed `<Button>` and raw `<button>` usage

- **Observation:** `SettingsPage` uses the design-system `<Button>` for top-level actions, while sub-screens use raw `<button>` elements for custom controls (colour picker, size toggles, password reveal).
- **Recommendation:** Standardise on `<Button>` where possible; document exceptions.

#### L-2 — `SettingsSelect` trigger is a raw button but is keyboard accessible

- **Location:** `ui/src/features/settings/SettingsSelect.tsx`
- **Observation:** The custom dropdown uses a `<button>` trigger with `onClick`, `onKeyDown`, and `aria-label` wiring. No obvious gap, but it is a custom control that should be covered by tests.
- **Recommendation:** Add unit tests for keyboard navigation and selection.

---

## 3. Per-file element inventory (high-level)

### `SettingsPage.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| Retry button (load error) | `load()` | Yes | Resets error and reloads |
| Store name input | `setStore`, `validateField` | Yes | Validates on blur |
| Address input | `setStore` | Yes | No validation |
| Tax ID input | `setStore`, `validateField` | Yes | Pattern validation |
| Currency `<SettingsSelect>` | `setDefaultCurrencyState` | Yes | Disabled while loading |
| Card size +/- buttons | `setDisplayCardSize` | Yes | Disabled at bounds |
| Font size +/- buttons | `setDisplayFontSize` | Yes | Disabled at bounds |
| Font smoothing `<SettingsSelect>` | `setDisplayFontSmoothing` | Yes | — |
| Receipt toggles | `setReceipt` | Yes | role="switch" |
| Receipt footer textarea | `setReceipt` | Yes | — |
| Sync server URL input | `setSyncServerUrl` | Yes | — |
| Sync API key input | `setSyncApiKey` | Yes | Password reveal wired |
| Request token button | inline async | Yes | `loading={requesting}` |
| Sync enabled toggle | `setSync` | Yes | — |
| Test connection button | inline async | Yes | `loading={testing}` |
| Sync now button | inline async | Yes | `loading={syncing}` |
| Pull from server button | inline async | Yes | `loading={pulling}` |
| Check for updates button | `handleCheckUpdates` | Yes | `loading`/`disabled` wired |
| Install update button | `handleInstallUpdate` | Yes | — |
| Mobile sidebar toggle | `setMobileSidebarOpen` | Yes | — |
| Search input | `setSearchQuery` | Yes | Clear search wired |
| Revert changes button | `handleRevert` | Yes | — |
| Save button | `handleSave` | Yes | `loading={saving}` |
| Theme toggle button | `toggleTheme` | Yes | — |

### `AppearanceSettings.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| Primary colour picker | `updateColour` | Yes | — |
| Colour hex input | `updateColour` | Yes | — |
| Reset colour button | `updateColour(DEFAULT_COLOUR)` | Yes | — |
| Choose logo button | `handlePickLogo` | Yes | `<Button>` |
| Display store name input | `updateStoreName` | Yes | — |
| Interface zoom `<SettingsSelect>` | `setZoomLevel` | Yes | — |
| HW accel toggle | `setHwAccelEnabled` | Yes | role="switch" |
| Reset all button | `handleResetAll` | Yes | Uses native `confirm()` — see H-1 |
| Save button | `save` | Yes | `disabled={saving}` |

### `FeatureToggleScreen.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| Retry button | `load` | Yes | — |
| Search input | `setSearchQuery` | Yes | Clear search wired |
| Bulk enable button | `toggleGroup(..., true)` | Yes | `disabled={togglingBatch === group}` |
| Bulk disable button | `toggleGroup(..., false)` | Yes | `disabled={togglingBatch === group}` |
| Feature toggles | `handleToggle` | Yes | `disabled={toggling === feat.key}` |

### `DataManagementScreen.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| Tab buttons | `setActiveTab` | Yes | — |
| Select-all checkbox | `toggleAll` | Yes | — |
| Type checkboxes | `toggleType` | Yes | — |
| Date from/to inputs | `setExportState` | Yes | — |
| Export password inputs | `setExportState` | Yes | Show/hide password wired |
| Export buttons | `startExport`, `confirmExport`, `resetExport` | Yes | — |
| Import file select button | `handleFileSelect` | Yes | — |
| Import password input | `setImportState` | Yes | — |
| Analyse / import buttons | `handleAnalyse`, `startImport` | Yes | `loading`/`disabled` wired |
| Backup now button | `handleBackup` | Yes | `loading={backup.backingUp}` |

### `EmailReportSettings.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| SMTP host/port/username/password/from | `updateField` | Yes | Password reveal wired |
| Use TLS toggle | `updateField('use_tls')` | Yes | role="switch" |
| Save config button | `saveConfig` | Yes | `disabled={saving}` |
| Send test email button | `handleSendTest` | Yes | `disabled={sending || !config.host.trim()}` |
| Schedule enabled toggle | `updateSchedField('enabled')` | Yes | role="switch" |
| Cadence/time/timezone/lookback inputs | `updateSchedField` | Yes | See M-4 |
| Recipients list | `updateSchedField('recipients')` | Yes | Add/remove wired |
| Save schedule button | `saveSchedule` | Yes | `disabled={scheduleSaving}` |

### `LicenseSettings.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| Retry button | `load` | Yes | — |
| Refresh server status button | `handleRefresh` | Yes | `loading={checkingServer}` |

### `SettingsNavTree.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| Mobile close button | `onMobileClose` | Yes | — |
| Collapse all button | `setExpandedCategories` | Yes | — |
| Keyboard shortcuts button | `setShowShortcuts` | Yes | — |
| Sidebar collapse button | `setSidebarCollapsed` | Yes | — |
| Nav item buttons | `onNavigate` | Yes | — |
| Pin/unpin buttons | `togglePin` | Yes | — |
| Search input | `onSearchChange` | Yes | Clear search wired |
| Category accordion buttons | `toggleCategory` | Yes | — |
| Resize handle | `onKeyDown` | Yes | — |

### `SettingsSelect.tsx`

| Element | Handler | Wired? | Notes |
|---------|---------|--------|-------|
| Native select | `onChange` | Yes | Hidden visually |
| Trigger button | `handleTriggerClick`, `handleKeyDown` | Yes | — |
| Option buttons | `selectOption` | Yes | Keyboard wired |

---

## 4. Recommendations summary

1. **H-1:** Replace native `confirm()` in `AppearanceSettings.tsx` with `<ConfirmDialog>`.
2. **M-1:** Wrap `SettingsPage` fields in a form and bind Save to `onSubmit`.
3. **M-2:** Add explicit `type="button"` to every raw `<button>` in settings.
4. **M-3:** Trace `DataManagementScreen` destructive flows and add confirmations.
5. **M-4:** Verify `EmailReportSettings` schedule validation and disabled-save state.
6. **M-5:** Verify `LicenseSettings` error feedback for server-status refresh.
7. **L-1 / L-2:** Add unit tests for custom controls and standardise on `<Button>`.

---

*End of report.*
