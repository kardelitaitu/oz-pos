# Delete the Old Settings — Definitive Plan

## 1. What are the "2 different settings"?

| Surface | What it is | User's label | Status |
|---|---|---|---|
| The **clean hub** at `#/settings` (11 genuine settings tabs) | The SettingsPage shown when you click "Settings" on the home screen | **NEW settings** | ✅ KEEP |
| The **deep-link pattern** `#/settings/staff`, `#/settings/audit`, etc. | The legacy behavior where management screens opened INSIDE the settings hub as a tab | **OLD settings** | ❌ DELETE |

**The core confusion:** Both were the **same page** (SettingsPage component). The only difference was whether you reached it via the home screen "Settings" tile (clean hub, new) or via a management tile like "Staff Management" (deep-linked to a management tab, old). The user saw the same page and couldn't tell why it showed different things.

## 2. What is "old settings" exactly?

The OLD settings = the **SettingsPage hub (route `settings`) when used as a tabbed container for management screens**. This was the legacy pattern where:
- Staff, Audit, Terminals, Stores, Shifts, Tax, Exchange, Promotions, Offline, Features, Data, and KDS all lived as **tabs inside the settings page**
- Clicking "Staff Management" on the home screen navigated to `#/settings/staff` (the settings page with the staff tab selected)
- The settings page had 23 tabs total — 12 management screens + 11 genuine settings

**This is confusing because:** the same page hosted management screens AND settings, making it impossible to tell which "settings" you were in.

## 3. What is "new settings"?

The NEW settings = **the SettingsPage hub (route `settings`) containing ONLY genuine settings** (11 tabs) + **standalone pages for each management screen** (staff, audit, etc., each with its own route).

When you click "Settings" on the home screen → `#/settings` → clean hub (11 tabs under Business/Operations/System).
When you click "Staff Management" on the home screen → `#/staff` → standalone StaffManagementScreen.

## 4. What is already deleted (commits 9196fb69 + a4265229)

| Item | Change |
|---|---|
| **12 management tabs** removed from SettingsPage | cases for `features`, `data`, `staff`, `terminals`, `stores`, `audit`, `offline`, `shifts`, `tax`, `exchange`, `promotions`, `kds` deleted |
| **21 import lines** removed | unused imports for the removed components |
| **`management` sidebar section** deleted | `SectionName` type, `SECTION_LABELS`, `SECTION_ORDER`, `groupBySection` fallback (now `'settings'`), FTL keys |
| **8 nav items re-homed** from `management` to `settings` section | audit, offline, features, data, shifts, staff, stores, terminals — each now routes to its standalone page |
| **Deep-link hash reader whitelisted** | `#/settings/<section>` only resolves to the 11 kept tabs; stale links to removed tabs fall back to the default view |
| **SettingsNavTree categories** | Management category removed; topology folded into System |
| **Orphaned FTL keys** removed | 12 `settings-nav-*` keys + `nav-section-management` + `settings-category-management` from both EN and ID bundles |
| **Tests updated** | `SettingsNavTree.test.tsx`, `groupBySection.test.ts` |
| **Plan document created** | this file |

## 5. Verification: is the old settings fully gone?

After the two commits above, the old settings pattern (deep-linking into the hub on a management tab) is **impossible**:

- `#/settings/staff` → ignored (staff not in KEPT_SECTIONS whitelist) → falls back to the `general` tab (the default hub view)
- `#/settings/audit` → same — ignored
- All 12 management routes → navigate to their standalone pages via the home screen tiles and sidebar
- The home screen "Staff Management" tile → `#/staff` → standalone StaffManagementScreen
- The home screen "Audit" tile → `#/audit-log` → standalone AuditLogScreen

**To verify this is working in your running build:**
1. Open the app → click "Settings" on the home screen → should show the hub with 11 tabs (General, Appearance, etc.)
2. Click "Staff Management" on the home screen → should go to the standalone Staff page (NOT the settings hub)
3. Click "Audit Log" on the home screen → should go to the standalone Audit Log page
4. Manually navigate to `#/settings/staff` in the URL bar → should show the settings hub on the default (General) tab, NOT an empty body

## 6. Remaining items (optional)

| Item | Decision | Rationale |
|---|---|---|
| **Topology** — keep in the hub? | ✅ **Keep in hub** under System category | Topology has no standalone route. It's a config editor, not a management screen. The "Add Workspace" card on the home screen deep-links `settings/topology` — this is fine since topology is a real kept tab |
| **Workspace card duplication** — hub (full-page) vs F10 modal | ✅ **Keep both** | Intentional: Tier 1 (hub, admin) vs Tier 2 (modal, in-workspace context). ADR #22 |
| **Remove the hash reader entirely** (not just whitelist it) | ❌ **Keep it** | `settings/topology` deep-link is used by the Add Workspace card. The whitelist ensures only real tabs respond |

## 7. Definition of done

The old settings is properly deleted when:
- [ ] No management screen can be reached as a tab inside the settings hub
- [ ] All 12 management screens have standalone routes (already true)
- [ ] The home screen tiles route to standalone pages (already true)
- [ ] The deep-link hash `#/settings/<section>` only works for the 11 real tabs (whitelist done)
- [ ] The sidebar "settings" section correctly lists all re-homed items (done)
- [ ] `npm run typecheck` passes (verified)
- [ ] All affected tests pass (verified)

**The old settings is already deleted in code.** If you're still seeing the old behavior, the app needs a rebuild to pick up the new commits.