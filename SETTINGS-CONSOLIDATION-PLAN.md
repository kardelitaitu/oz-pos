# Delete the Old Settings — Definitive Plan

## Architectural principle

**Tools** (staff, audit, terminals, stores, shifts, tax, exchange, promotions, offline, features, data, kds, analytics, reports) → home "Tools" area, each with a dedicated full page. Visibility gated by subscription tier + user role.

**Configuration** (general, appearance, receipt, sync, email, about, license, topology, store-pos, restaurant-pos, inventory) → stays on Settings (route `settings`).

## What is already done

| Change | Commit |
|---|---|
| 12 management tabs removed from settings hub | `9196fb69` |
| `management` sidebar section deleted; 8 nav items re-homed to `settings` | `9196fb69` |
| Deep-link hash reader whitelisted (only 11 real tabs respond) | `a4265229` |
| Settings hub categories: Management folded into System | `a4265229` |
| Tests and FTL keys updated | both |

## What remains to do

### Step 1: Expand home "Tools" area

The home screen (`WorkspaceHome.tsx`) has a hardcoded `TOOLS` array with 5 tools. Add all the missing tools:

**Already there:** analytics, reports (→dashboard), staff, settings, audit-log

**Add:** terminals, stores, shifts, tax-config, exchange-rates, promotions, offline-queue, features, data-management, kds

Each needs:
- `route` (already exists — terminals, stores, shifts, etc.)
- `minRole` gating (already exists in their register files)
- Subscription-tier gating (via `useSubscription` — check if the tool's feature is enabled)
- SVG icon, label FTL key, description FTL key

### Step 2: Move tools out of the sidebar `settings` section

Currently the sidebar `settings` section has: General, Features, Data, Audit, Offline, Shifts, Staff, Stores, Terminals.

Per the concept: only configuration (General) belongs in `settings`. The tools should NOT be in the `settings` section.

**Option A (recommended):** Remove the 8 tool nav items from the sidebar entirely. The home "Tools" area is their canonical entry point. The sidebar `settings` section shows only "General" (the settings hub).

**Option B:** Keep them in the sidebar under a different section name.

### Step 3: Verify the Settings hub

The 11 configuration tabs are correct. No changes needed.

### Step 4: Clean up

- Remove the 8 re-homed nav items' `section: 'settings'` → delete them from sidebar (if Option A)
- Remove orphaned sidebar nav keys from FTL bundles
- Verify all tests pass

## Key decision

**Step 2:** Option A removes tools from sidebar entirely (home Tools area is the entry). Option B keeps them in sidebar under a different section. Which one?