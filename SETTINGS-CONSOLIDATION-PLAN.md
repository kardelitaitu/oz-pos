# Settings Consolidation Plan — "Old Settings" → "New Settings"

> Status: **ACTIVE** · Owner: agent + maintainer review · Target: only the
> new settings remain; every legacy settings entry point is deleted or
> re-routed.

## 1. The problem

Users and code have two mental models of "settings":

| Surface | What it is | Status |
|---|---|---|
| **New settings** | The clean settings hub at route `settings` (SettingsPage) — **only** real settings tabs (general, appearance, receipt, sync, email, about, license, topology, store-pos, restaurant-pos, inventory) | ✅ Keep |
| **Old settings** | The legacy tabbed hub that hosted **management screens as tabs** (staff, audit, terminals, stores, shifts, tax, exchange, promotions, offline, features, data, kds) reachable by deep-links like `#/settings/staff` | ❌ Delete |

The confusion: clicking "Staff Management" on the home screen used to land on
the settings page with the *staff tab* open (the old settings), while clicking
"Settings" landed on the same page in its default state (the new settings).
Both were the same component — the only difference was the deep-linked tab.

## 2. Target architecture (definition of done)

1. **One settings hub** — route `settings` (SettingsPage) contains only the 11
   genuine settings tabs. No management screens as tabs.
2. **Every management screen is its own page** with its own route, reachable
   from the home screen / sidebar by its real name:
   - `audit-log` (Audit Log)
   - `features` (Features)
   - `data-management` (Data)
   - `staff` (Staff)
   - `terminals` (Terminals)
   - `stores` (Stores)
   - `offline-queue` (Offline Queue)
   - `shifts` (Shifts)
   - `tax-config` (Tax Rates)
   - `exchange-rates` (Exchange Rates)
   - `promotions` (Promotions)
   - `kds` (Kitchen Display)
3. **No deep-links** into the settings hub on a removed tab (`#/settings/staff`,
   `#/settings/audit`, …). The only valid deep-link target left is
   `#/settings/topology` (topology is a real, kept tab).
4. **Sidebar section**: the `management` section is gone; the 8 re-homed items
   live in the `settings` section (already done).

## 3. What is already done

Committed as `9196fb69`:

- **12 legacy tabs removed** from SettingsPage.tsx (features, data, staff,
  terminals, stores, audit, offline, shifts, tax, exchange, promotions, kds).
  Each already had a standalone route, so no functionality was lost.
- **Nav tree reduced** — SettingsNavTree.tsx `NAV_ITEMS`, `CATEGORIES`, and
  `NAV_L10N_KEYS` only list the 11 kept tabs.
- **`management` section deleted** from `SectionName`, `SECTION_LABELS`,
  `SECTION_ORDER`; `groupBySection` fallback now `'settings'`.
- **8 register files re-homed** to `section: 'settings'` with their standalone
  routes: audit, offline, staff, terminals, stores, shifts (in
  `features/*/register.tsx`) and features + data-management
  (in `features/settings/register.tsx`).
- **Orphaned FTL keys removed** from `shared.ftl` / `shared.id.ftl`
  (`nav-section-management`) and `settings.ftl` / `settings.id.ftl`
  (the 12 removed `settings-nav-*` keys).
- **Tests updated**: `groupBySection.test.ts`, `SettingsNavTree.test.tsx`.
- **Typecheck + affected tests green** (SettingsNavTree 41, groupBySection,
  pageRegistry, AuditLogScreen, FeatureToggleScreen, LicenseSettings).
- Backend: added `Store::audit_summary` in `crates/oz-core/src/db/audit.rs`
  (KPI aggregates for the standalone audit page).

## 4. Remaining work

### 4.1 Remove the legacy deep-link hash reader (SettingsPage)

`ui/src/features/settings/SettingsPage.tsx` lines ~312–323 still parse
`#/settings/<section>` on mount and force an active tab. All removed tabs
render `null` now, so a stale deep-link (e.g. `#/settings/staff`) opens the
hub with an empty body — a dead "old settings" entry point.

- [ ] Replace the hash reader with a **whitelist** of the 11 kept sections
      (or drop it entirely and let the nav tree own active section state).
- [ ] Decide: keep `#/settings/topology` deep-link support (used by the home
      screen "Add Workspace" card) — recommendation: **keep it**, it targets a
      real tab of the new hub.

### 4.2 Home screen "Add Workspace" card deep-link

`ui/src/features/workspaces/WorkspaceHome.tsx` lines 728 + 886 call
`handleShortcutNav('settings/topology')`. This is a legitimate deep-link into
a kept tab, so it can stay. Verify it still resolves after 4.1.

- [ ] Verify `settings/topology` deep-link works with the new whitelist.

### 4.3 Topology placement (open question)

`TopologyScreen` has **no standalone route** — it is only reachable via the
settings hub tab. Options:

- **A (recommended): keep it in the hub.** Topology is a settings-style
  configuration tool (it configures workspaces), so it belongs with the
  settings. The hub's "Management" category currently holds only topology —
  rename the category to something meaningful or fold it into another.
- **B: give it a standalone route** (`topology`) like stores/audit. More
  consistent with the "every management screen is a page" rule, but topology
  is a config editor, not a management screen, so it fits the hub better.

### 4.4 Workspace settings duplication (no change needed)

`WorkspaceStorePosSettings` / `WorkspaceRestaurantPosSettings` /
`WorkspaceInventorySettings` are intentionally rendered in two contexts:

1. **Tier 1 hub** (SettingsPage, `variant="full-page"`) — admin, cross-workspace.
2. **Tier 2 modal** (WorkspaceSettingsModal, `variant="modal"`, F10 in a
   workspace, ADR #22) — in-context, scoped to the active workspace.

Both are the same shared card components; the duplication is a *rendering
context*, not a code fork. **No change required.** (The KDS card remains in the
modal + topology inspector only — the hub tab was removed.)

## 5. Verification checklist

- [ ] `#/settings` opens the hub with 11 tabs, no empty sections.
- [ ] `#/settings/staff` (or any removed tab) does **not** deep-link; it falls
      back to the default hub state (or a safe section).
- [ ] Home screen: Settings → `#/settings`; Staff → `#/staff`; Audit →
      `#/audit-log`; Add Workspace → `#/settings/topology`.
- [ ] Sidebar "settings" section lists the 8 re-homed items + Settings, each
      navigating to its own route.
- [ ] `npm run typecheck` and affected vitest suites pass in `ui/`.
- [ ] i18n gates pass (no orphaned keys, bundle parity).

## 6. Out of scope

- Building the standalone audit page KPI strip (backend `audit_summary` is
  staged, front-end not yet built) — separate task.
- Renaming/restructuring the `settings` sidebar section.
- The Tier 2 WorkspaceSettingsModal flow (ADR #22) — already the modern path.
