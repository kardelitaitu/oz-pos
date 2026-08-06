# Location Management Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** LocationPicker / LocationManagement — inventory-location CRUD, workspace bindings, stock safety, selection UX, authorization, and tests  
> **Status:** ✅ REMEDIATED — all 10 findings closed via commits `fb1842ce` (LOC-02/03), `e943095b` (LOC-04), `451747b6` (LOC-05), `4b227e4f` (LOC-06), `5ee88824` (LOC-07), `688dca10` (LOC-08), `815bd1aa` (LOC-09); LOC-01 landed earlier as part of audit/07 (`3caddf6e`).  
> **Production code changed:** Yes — see the finding-by-finding commit references below.

## Scope

This audit evaluates the location-management surface against the universal checklist in `audit/AUDIT_JULY_2026.md`: functionality and state management, loading/error/empty states, accessibility and localization, theming, performance, security and authorization, inventory integrity, and quality assurance.

Inspected areas:

- `ui/src/features/inventory/LocationPicker.tsx`
- `ui/src/features/inventory/LocationPicker.css`
- `ui/src/api/inventory.ts`
- `apps/desktop-client/src/commands/inventory.rs`
- `crates/oz-core/src/db/inventory.rs`
- Inventory location, stock-summary, transfer, and workspace-binding migrations
- `ui/src/__tests__/LocationPicker.test.tsx`
- `ui/src/__tests__/PosScreenDeductionLocation.test.tsx`
- Inventory locale bundles and related location consumers

## Architecture summary

The inspected UI surface is a reusable `LocationPicker`; no standalone `LocationManagementScreen.tsx` was found under the inventory feature. The picker loads inventory locations with a session token, filters inactive locations in the renderer, and exposes a custom listbox-like dropdown. It is used by inventory/product and POS flows to select a location or display the current deduction location.

The API exposes session-token-based location CRUD and workspace-binding commands. Desktop handlers resolve the session to a store, open the store database, and enforce `SALES_PROCESS` permission before invoking core methods. Core location CRUD validates names and the five allowed types. Deactivation blocks positive stock and pending transfers, then sets `is_active = 0`; the picker silently disappears when loading fails or when no active locations are returned.

## Findings

### LOC-01 — Location picker silently disappears on load failure or empty active set

**Evidence:** `LocationPicker.load()` catches `listInventoryLocations()` errors with an empty catch and only sets `loading` false. Rendering returns `null` when `loading || locations.length === 0`. The component therefore provides no error, retry, disabled state, or indication that location selection is unavailable.

**Impact:** A permission, session, database, or IPC failure looks identical to a store with no active locations. A POS or inventory screen can lose its location control without explaining which location will be used or whether a stock operation is safe.

**Recommendation:** Track a load error separately from an empty successful response. Render a localized unavailable/error state with Retry, and make the selected/default location explicit when the picker cannot load. Add tests for rejection, retry, and an empty active-location response.

**Status:** ✅ Remediated · LOC-01 landed as part of audit/07 INV-08 (`3caddf6e` — durable error/retry states; the picker's retry + empty-state tests verify the behavior).

### LOC-02 — Deactivation allows locations with negative stock balances

**Evidence:** `Store::deactivate_inventory_location()` checks `stock_summary` only with `qty > 0`. A location whose balance is negative is therefore allowed through the stock check. The command documentation says deactivation fails when the location contains stock, but the implementation does not treat negative quantity as a non-zero balance.

**Impact:** Negative inventory can be hidden from active-location workflows by deactivating its location, making reconciliation and future stock correction harder. The location may no longer appear in active pickers while its ledger still contains an unresolved balance.

**Recommendation:** Block deactivation whenever any relevant balance is non-zero (`qty <> 0`), or require an explicit reconciliation/zeroing workflow with an audit trail. Add core tests for positive, zero, and negative balances and align the command documentation with the policy.

**Status:** ✅ Remediated · `fb1842ce` — deactivation now blocks any non-zero balance (`qty <> 0`) and returns `NotFound` on missing/already-inactive IDs; core tests cover positive, zero, and negative balances plus missing IDs.

### LOC-03 — Deactivation and update do not clearly report missing location IDs

**Evidence:** `deactivate_inventory_location()` executes the update and commits without checking affected rows. `update_inventory_location()` does check for zero updated rows and returns `NotFound`, but deactivation does not. A stale or cross-workspace ID can therefore be treated as a successful command even though no location changed.

**Impact:** The UI or caller can show a successful state after a no-op. This is especially dangerous for deactivation because operators may assume a location is no longer active and proceed with reconciliation based on a false result.

**Recommendation:** Check the affected-row count for deactivation and return `NotFound` when it is zero. Return a structured result containing the new active state, and add command/core tests for missing IDs and already-inactive locations.

**Status:** ✅ Remediated · `fb1842ce` — deactivation checks the affected-row count and returns `NotFound` for stale or already-inactive IDs; command/core tests added.

### LOC-04 — Location selection has incomplete listbox keyboard behavior

**Evidence:** The picker assigns `role="listbox"` and `role="option"`, but only implements Escape and click handling. There is no ArrowUp/ArrowDown navigation, Home/End behavior, active-descendant management, or focus movement to the selected option. The options are buttons, but the listbox pattern is not completed.

**Impact:** Keyboard and assistive-technology users cannot navigate the location list using expected listbox controls. They must tab through each option, and the semantic role may promise behavior the component does not provide.

**Recommendation:** Either use a native `<select>` or implement the full combobox/listbox pattern: stable IDs, `aria-controls`, `aria-activedescendant`, roving focus, Arrow/Home/End handling, Enter/Space selection, and focus restoration. Add keyboard interaction and accessibility tests.

**Status:** ✅ Remediated · `e943095b` + `815bd1aa` — full combobox/listbox pattern: ArrowUp/Down (wrap), Home/End, `aria-activedescendant`, Enter/Space select, Escape closes + focus restore, and `aria-controls` on the trigger; three keyboard-interaction tests pin the behavior.

### LOC-05 — Location labels and types are hardcoded and not localized

**Evidence:** `LocationPicker` uses the default label `'Location'`, trigger text `Select inventory location. Current: ...`, listbox label `Inventory locations`, and raw `loc.type` values. The component does not use `useLocalization`, and no type-label mapping is applied for localized output.

**Impact:** Location controls remain in English in localized deployments, while types such as `warehouse`, `transit`, and `damaged` expose machine values. Screen-reader users receive hardcoded English labels and potentially ambiguous concatenated names.

**Recommendation:** Add value-bearing Fluent keys for trigger/list labels and every supported location type in both bundles. Use a typed mapping with a safe localized fallback for unknown future types. Add English/Indonesian accessibility-label tests.

**Status:** ✅ Remediated · `451747b6` — typed `LOCATION_TYPE_KEYS` mapping with a `loc-type-unknown` fallback renders value-bearing Fluent labels in both bundles; English and Indonesian accessibility tests assert raw machine values never leak.

### LOC-06 — Location CRUD uses a broad sales permission rather than a location-specific capability

**Evidence:** All location CRUD, workspace-binding, shift, transaction, threshold, and alert commands inspected require `permissions::SALES_PROCESS`. There is no narrower location-management permission in the command surface. The location APIs do correctly carry a session token and the backend resolves the session store/user.

**Impact:** Any role granted the broad sales-processing capability may be able to create, rename, deactivate, or rebind inventory locations even when the role should only process sales. Conversely, tightening sales permission to protect checkout can unintentionally remove location-management access.

**Recommendation:** Define separate location-view, location-manage, and binding/stock-policy permissions. Keep session/store resolution server-side, apply least privilege to each command, and add role-matrix tests for picker visibility versus management mutations.

**Status:** ✅ Remediated · `4b227e4f` — new `INVENTORY_VIEW` / `INVENTORY_LOCATIONS_MANAGE` permissions split location reads from CRUD/binding mutations (granted to Manager+Staff, not Cashier); all inventory commands now authorize against the global identity DB via `require_inventory_permission` (also fixed a latent store-scoped-DB authz bug); role-matrix preset test + command-level permission tests added.

### LOC-07 — Picker data can become stale after location or binding changes

**Evidence:** The picker loads only when its `token` changes. It does not subscribe to `invalidateLocationCache`, workspace-binding changes, or an application event. A location can be renamed/deactivated or bindings can change in another screen while an already-mounted picker retains its old list. The load also has no cancellation/request-generation guard.

**Impact:** Operators may select a location that has just been deactivated or see an old name after a rename. Concurrent session changes can also allow an earlier request to write results after a newer token has been selected.

**Recommendation:** Introduce a location-store invalidation event/version, refresh on workspace instance changes, and guard asynchronous results by request generation. Ensure consumers revalidate the selected ID before stock mutations. Add tests for external invalidation and out-of-order responses.

**Status:** ✅ Remediated · `5ee88824` — new `refreshKey` prop for external invalidation, reload on workspace instance change, and a `loadSeqRef` request-generation guard; tests prove a refreshKey bump refetches and a stale response resolving after a fresh one is dropped.

### LOC-08 — Location picker selection is not guaranteed to match the stock-operation scope

**Evidence:** `LocationPicker` calls `onChange(location.id, location.name)` but has no knowledge of workspace bindings or `allow_negative_stock`. The API provides a separate `getWorkspaceLocations()` resolver and POS flows separately manage deduction-location overrides. The picker can display all active inventory locations returned by `list_inventory_locations`, rather than only locations bound to the current workspace.

**Impact:** A user can choose a globally active location that is not valid for the current workspace policy. The UI selection can therefore diverge from the location resolver used by stock operations, especially around negative-stock allowances and primary-location rules.

**Recommendation:** Make workspace-bound locations the source for workspace pickers, including binding policy metadata. Validate the chosen location server-side in every stock command, and show the active/primary/negative-stock policy in the selector when relevant. Add cross-workspace selection tests.

**Status:** ✅ Remediated · `688dca10` — when a workspace instance is active, the picker sources from `getWorkspaceLocations` (workspace-bound locations with `is_primary` / `allow_negative_stock` policy), falling back to the full active list for workspaces with no declared bindings; Primary / "Neg. stock" badges surface the policy; cross-workspace scoping + fallback tests added.

### LOC-09 — Dynamic location lists are not bounded or virtualized

**Evidence:** `list_inventory_locations()` returns every location ordered by name, and the picker renders all active locations into one dropdown. There is no count limit, search field, pagination, or virtualization. The dropdown has a max height but still creates every option in the DOM.

**Impact:** Large tenants or operationally fragmented stores can incur unnecessary IPC, memory, and rendering cost. A long list is difficult to use even though it scrolls.

**Recommendation:** Return bounded, workspace-scoped locations and add search for larger sets. Keep a deterministic selected-location-first ordering and measure render behavior at realistic counts.

**Status:** ✅ Remediated · `815bd1aa` — dropdown is ordered selected-first then by name (deterministic), an inline search field appears for sets ≥ 8 and narrows by name or localized type label, and a localized no-results row replaces the empty list; Escape still dismisses the dropdown from the no-results state and Space is never hijacked while typing.

### LOC-10 — Focused tests cover happy-path selection but not safety or recovery semantics

**Evidence:** The focused run contains 12 passing tests across `LocationPicker.test.tsx` and `PosScreenDeductionLocation.test.tsx`. The inspected coverage does not establish tests for load rejection/retry, empty active locations, Arrow-key behavior, localization, stale responses, negative-stock deactivation, missing IDs, role boundaries, workspace-binding mismatch, or cache invalidation.

**Impact:** Regressions in stock safety, scope consistency, accessibility, and error recovery can pass the existing UI suite.

**Recommendation:** Add component tests for picker failure/empty/keyboard/i18n paths, IPC contract tests for all location arguments and errors, and Rust tests for deactivation balances, missing IDs, binding validation, and permission matrices.

**Status:** ✅ Remediated · closed across the chain — picker failure/retry/empty (LOC-01), Arrow-key + focus-restoration (LOC-04), en/id localization (LOC-05), role boundary + permission matrices + two-store isolation (LOC-06), stale responses + invalidation (LOC-07), workspace-binding scope (LOC-08), ordering/search/no-results (LOC-09), plus Rust tests for deactivation balances, missing IDs, and the RBAC split. `LocationPicker.test.tsx` now carries 24 passing tests.

## Positive controls observed

- Location CRUD APIs carry a session token.
- Desktop location commands resolve the session's store and enforce a backend permission before database access.
- Core create/update validate non-empty names and restrict location types to known values.
- Deactivation is transactional and checks positive stock plus pending transfer statuses.
- The picker filters inactive locations before rendering.
- The picker closes on outside click and Escape, and restores focus to its trigger on Escape.
- SQL uses bound parameters and the location dropdown has a bounded visual height.

## Test and validation results

Focused validation completed during this audit:

```text
cd ui
npx vitest run src/__tests__/*Location*.test.tsx src/__tests__/*location*.test.tsx
npm run typecheck
```

Results:

- Focused UI tests: **12 passed, 0 failed** across 2 files
- TypeScript typecheck: **passed with 0 errors**
- Report existence and Markdown trailing-whitespace validation: **passed after report generation**
- No dedicated Rust location test count is claimed; backend source was inspected but no focused Rust test command was run during this audit

## Recommended remediation order

1. **LOC-01 and LOC-08:** Make picker availability and workspace stock scope explicit and safe.
2. **LOC-02 and LOC-03:** Close negative-balance and missing-ID deactivation gaps.
3. **LOC-04 and LOC-05:** Complete listbox behavior and localization.
4. **LOC-06 and LOC-07:** Separate least-privilege permissions and add invalidation/race protection.
5. **LOC-09 and LOC-10:** Bound large location lists and expand safety/accessibility coverage.

## Audit status

All findings are closed. Remediation commits:

| Finding | Commit(s) |
| --- | --- |
| LOC-01 — silent disappearance on load failure / empty set | `3caddf6e` (INV-08) |
| LOC-02 — negative-balance deactivation | `fb1842ce` |
| LOC-03 — missing-ID deactivation silently succeeds | `fb1842ce` |
| LOC-04 — incomplete listbox keyboard behavior | `e943095b`, `815bd1aa` |
| LOC-05 — hardcoded, unlocalized labels/types | `451747b6` |
| LOC-06 — broad sales permission instead of location capability | `4b227e4f` |
| LOC-07 — stale picker data after location/binding changes | `5ee88824` |
| LOC-08 — selection not guaranteed to match stock scope | `688dca10` |
| LOC-09 — unbounded, unordered location lists | `815bd1aa` |
| LOC-10 — missing safety/recovery/scope test coverage | chain above |

**Final validation:** typecheck clean · `LocationPicker.test.tsx` 24/24 · eslint clean · `scripts/lint-i18n.sh` clean · backend `cargo test -p oz-pos-app --lib commands::inventory` and `platform-core` rbac tests green (LOC-06).
