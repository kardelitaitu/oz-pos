# 0.0.13 — Release Gate + ADR-20 Payment-Capture Ordering

> **Goal:** Fix release gate (4 failing tests, 2 Rust warnings), implement ADR-20 three-phase sale lifecycle (6 acceptance criteria), and clean up orphaned test code.

**Current state:** 6 / 15 items complete · Updated 2026-07-19

---

## Progress Summary

| Area | Total | Done | Progress |
|------|-------|------|----------|
| 🔴 P0 — Release Gate | 6 | **6** | **███████████████ 100% ✅** |
| 🟢 P1 — ADR-20 Payment-Capture | 7 | **0** | **▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟡 P2 — Codebase Health | 1 | **0** | **▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱ 0%** |
| **Total** | **15** | **6** | **███████████████ 40%** |

---

## 🔴 P0 — Release Gate (must fix before PR)

**Goal:** Fix all 4 failing UI tests (StatusBar, RetailOptionsScreen, screenExtraction, StockAlertPanel) and 2 Rust compiler warnings.

### P0-1: StatusBar version string

**Current:** `expect(screen.getByText(/v0\.0\.11/)).toBeTruthy()` — fails because version is now `0.0.12`
**File:** `ui/src/__tests__/StatusBar.test.tsx`

- [x] Fix: changed regex to `/v0\.0\.\d+/` for forward compatibility
- [x] 8/8 tests pass

### P0-2: RetailOptionsScreen System tab

**Current:** Expects to find version string `0.0.11` in a display element on the System tab
**File:** `ui/src/__tests__/RetailOptionsScreen.test.tsx`

- [x] Fix: changed `'0.0.11'` to `/0\.0\.\d+/` regex
- [x] 21/21 tests pass

### P0-3: screenExtraction dead CSS classes

**Current:** `ProductManagementScreen` has dead CSS classes from StockAlertPanel/LocationPicker integration
**File:** `ui/src/__tests__/screenExtraction.test.ts`

- [x] Fix: added `product-mgmt-alert-badge` and `stock-alert-panel` to `externalClasses`
- [x] Scanner passes

### P0-4: StockAlertPanel empty-state test

**Current:** Mock API returns data but stays in loading state
**File:** `ui/src/__tests__/StockAlertPanel.test.tsx`

- [x] Fix: replaced `renderWithProviders` with plain `render` to avoid BrandProvider interference
- [x] 6/6 tests pass

### P0-5: Rust warnings

**Current:** 2 unused variable warnings in `oz-core` tests: `shift2` (inventory.rs), `po_a` (purchase_orders.rs)
**Files:** `crates/oz-core/src/db/inventory.rs`, `crates/oz-core/src/db/purchase_orders.rs`

- [x] Fix: prefixed with `_` in both files
- [x] `cargo test -p oz-core --lib` — zero warnings

### P0-6: Full validation gate

- [x] `cargo test -p oz-core --lib` — 1441 passed, 0 failed, 0 warnings ✅
- [x] `npx vitest run` — 2785 passed, 0 failed ✅
- [x] `npx eslint .` — 0 errors (1 pre-existing warning) ✅
- [x] `npx tsc --noEmit` — 0 errors ✅
- [x] P0 fixes committed with proper message

---

## 🟢 P1 — ADR-20 Payment-Capture Ordering (Stock Reservation)

**Goal:** Implement the three-phase sale lifecycle (`active → pending → completed/voided`) to prevent the pre-capture race condition where two terminals capture payment against the same stock.

### References

- **ADR-20 spec:** `docs/decisions/2026-07-19-payment-capture-ordering.md`
- **6 acceptance criteria (20-1 through 20-6):** Dedup, serialization, finalize, void, stale-reap, concurrent finalize/void

### P1-1: Migration 095 — Add `pending` to sales.status CHECK

- [ ] Create `migrations/095_pending_sale_status.sql`
- [ ] Register migration in `crates/oz-core/src/migrations.rs`
- [ ] Verify migration applies cleanly

### P1-2: `create_pending_sale` backend

- [ ] Add `PendingSale` struct
- [ ] Add `create_pending_sale` method on Store with `BEGIN IMMEDIATE`
- [ ] Integrate `active_carts.deduction_location_id` check
- [ ] Add dedup via `sale_data_hash`
- [ ] Add `create_pending_sale_with_resolution`
- [ ] Unit test for 20-1 (dedup) and 20-2 (serialization)

### P1-3: `finalize_sale` and `void_pending_sale` backend

- [ ] Add `finalize_sale` — status='completed', record payment details
- [ ] Add `void_pending_sale` — credit stock back via FIFO oldest-credit
- [ ] Unit test for 20-3 and 20-4

### P1-4: Tauri commands

- [ ] Add 3 scoped Tauri commands in `apps/desktop-client/src/commands/sales.rs`
- [ ] Register in `lib.rs`

### P1-5: Frontend API wrappers

- [ ] Add `PendingSale`, `createPendingSale`, `finalizeSale`, `voidPendingSale`

### P1-6: Stale-pending-sale reaper

- [ ] Add `find_stale_pending_sales` and `reap_stale_pending_sales`
- [ ] Register in `platform/startup/src/lib.rs`
- [ ] Unit test for 20-5 (stale-reap) and 20-6 (concurrent finalize/void)

### P1-7: PaymentModal three-phase flow

- [ ] Update PaymentModal for three-phase flow
- [ ] UI tests for new flow

---

## 🟢 P1 — ADR-20 Payment-Capture Ordering (Stock Reservation)

**Goal:** Implement the three-phase sale lifecycle (`active → pending → completed/voided`) to prevent the pre-capture race condition where two terminals capture payment against the same stock.

### References

- **ADR-20 spec:** `docs/decisions/2026-07-19-payment-capture-ordering.md`
- **6 acceptance criteria (20-1 through 20-6):** Dedup, serialization, finalize, void, stale-reap, concurrent finalize/void

### P1-1: Migration 095 — Add `pending` to sales.status CHECK

- [ ] Create `migrations/095_pending_sale_status.sql` — add `'pending'` to CHECK, add `pending_expires_at`, `payment_method`, `payment_reference`, `captured_at` columns via table rebuild
- [ ] Register migration in `crates/oz-core/src/migrations.rs`
- [ ] Verify migration applies cleanly

### P1-2: `create_pending_sale` backend

- [ ] Add `PendingSale` struct (sale_id, receipt_number, deduct_tx_id, total, line_count)
- [ ] Add `create_pending_sale` method on Store with `BEGIN IMMEDIATE` transaction
- [ ] Integrate with ADR-19 `active_carts.deduction_location_id` check → `CartLocationUnbound` error
- [ ] Add dedup logic via `sale_data_hash` collision detection
- [ ] Return `PartialStockResult` on insufficient stock (re-use ADR-19)
- [ ] Add `create_pending_sale_with_resolution` for shortfall resolution
- [ ] Unit test for criterion 20-1 (dedup) and 20-2 (serialization)

### P1-3: `finalize_sale` and `void_pending_sale` backend

- [ ] Add `finalize_sale` — set status to `'completed'`, record payment details, fire `sale.completed` event
- [ ] Add `void_pending_sale` — credit stock back via FIFO oldest-credit from `deduction_locations` JSON, set status to `'voided'`
- [ ] Unit test for criterion 20-3 (finalize) and 20-4 (void)

### P1-4: Tauri commands for ADR-20

- [ ] Add `create_pending_sale_scoped` — `apps/desktop-client/src/commands/sales.rs`
- [ ] Add `finalize_sale_scoped` — `apps/desktop-client/src/commands/sales.rs`
- [ ] Add `void_pending_sale_scoped` — `apps/desktop-client/src/commands/sales.rs`
- [ ] Register all 3 in `lib.rs`

### P1-5: Frontend API wrappers

- [ ] Add `PendingSale` TypeScript interface to `ui/src/api/sales.ts`
- [ ] Add `createPendingSale`, `finalizeSale`, `voidPendingSale` wrappers
- [ ] Add `CartLocationUnbound` error, `PaymentGatewayFailure` error types

### P1-6: Stale-pending-sale reaper

- [ ] Add `find_stale_pending_sales` on Store
- [ ] Add `reap_stale_pending_sales` function (called every 60s by background task)
- [ ] Register in `platform/startup/src/lib.rs`
- [ ] Unit test for criterion 20-5 (stale-reap)
- [ ] Unit test for criterion 20-6 (concurrent finalize/void — one wins)

### P1-7: PaymentModal three-phase flow

- [ ] Update PaymentModal to call `create_pending_sale_scoped` before gateway
- [ ] On success → proceed to payment capture → call `finalize_sale_scoped`
- [ ] On failure → call `void_pending_sale_scoped` with reason
- [ ] Add "Reserving stock…" / "Finalizing payment…" spinner states
- [ ] Add "Pending Payments" sub-tab for retry of failed pending sales
- [ ] UI tests for PaymentModal three-phase flow

---

## 🟡 P2 — Codebase Health

### P2-1: Integrate orphaned tests

- [ ] Review `crates/oz-core/src/db/payments_new_tests.rs` (orphaned — contains 5 valid tests)
- [ ] Merge tests into proper `payments.rs` `#[cfg(test)]` module
- [ ] Delete orphaned file
- [ ] Verify `cargo test -p oz-core` passes with merged tests

---

## 🧭 Dependency Graph

```
🔴 P0 Release Gate (4 failing tests + 2 Rust warnings)
  │
  ├── unblocks → PR creation, merge to main
  │
🟢 P1 ADR-20
  │   ├── P1-1 Migration 095 ────────────────────┐
  │   ├── P1-2 create_pending_sale backend ───────┤
  │   ├── P1-3 finalize_sale + void_pending_sale ─┤──→ ADR-20 Complete
  │   ├── P1-4 Tauri commands ────────────────────┤
  │   ├── P1-5 Frontend API ──────────────────────┤
  │   ├── P1-6 Stale-reaper worker ───────────────┤
  │   └── P1-7 PaymentModal UI ───────────────────┘
  │
🟡 P2 Integrate orphaned tests ──── independent
```

---

# 0.0.12 — ADR-18 Implementation Gaps

> **Goal:** Close all remaining ADR-18 Multi-Location Inventory gaps — unified resolver, alert engine, frontend components, and §13 amendments.

**Current state:** 31 / 31 items complete (100%) · Updated 2026-07-19 🎉

---

## Progress Summary

| Area | Total | Done | Progress |
|------|-------|------|----------|
| 🔴 Backend — Critical | 2 | **2** | **███████████████ 100% 🎉** |
| 🟡 Backend — Medium | 2 | **2** | **███████████████ 100% 🎉** |
| 🧪 Rust Test Coverage | 20 | **20** | **███████████████ 100% 🎉** |
| 🧪 UI Test Coverage | 7 | **7** | **███████████████ 100% 🎉** |
| 🔵 Frontend — Missing | 2 | **2** | **███████████████ 100% 🎉** |
| 🔴 §13 Amendments | 1 | **1** | **███████████████ 100% 🎉** |
| 🟡 §13 Amendments | 1 | **1** | **███████████████ 100% 🎉** |
| ❓ Verification | 1 | **1** | **███████████████ 100% 🎉** |
| 🟡 New ADR | 1 | **1** | **███████████████ 100% 🎉** |
| **Total** | **31** | **31** | **██████████████████████████████████████████████████████████████████ 100% 🎉** |

---

## 🧪 Rust Test Coverage — Low-Coverage Modules

**Goal:** Bring all `oz-core` sub-modules to **20+ tests each**. Current: 27+ modules with <20 tests.

| Module | Current | Target | New tests needed |
|--------|---------|--------|-----------------|
| `recipes.rs` | 4 → **16** | 15+ | ✅ |
| `product_bundles.rs` | 8 → **20** | 15+ | ✅ |
| `promotions.rs` | 9 → **18** | 15+ | ✅ |
| `loyalty.rs` | 10 → **20** | 15+ | ✅ |
| `stock_counts.rs` | 10 → **20** | 20+ | ✅ |
| `tables.rs` | 10 → **18** | 15+ | ✅ |
| `terminal_overrides.rs` | 10 → **16** | 15+ | ✅ |
| `terminal_profiles.rs` | 10 → **16** | 15+ | ✅ |
| `refunds.rs` | 11 → **21** | 20+ | ✅ |
| `cart.rs` | 12 → **21** | 20+ | ✅ |
| `gift_cards.rs` | 12 → **18** | 15+ | ✅ |
| `kds.rs` | 12 → **21** | 20+ | ✅ |
| `customers.rs` | 13 → **16** | 15+ | ✅ |
| `offline.rs` | 14 → **21** | 20+ | ✅ |
| `audit.rs` | 15 → **20** | 20+ | ✅ |
| `cash_payouts.rs` | 15 → **20** | 20+ | ✅ |
| `payments.rs` | 15 → **20** | 20+ | ✅ |
| `purchase_orders.rs` | 15 → **21** | 20+ | ✅ |
| `suppliers.rs` | 15 → **20** | 20+ | ✅ |
| `reports.rs` | 17 → **30** | 25+ | ✅ |
| `settings.rs` | 17 → **27** | 25+ | ✅ |
| `terminals.rs` | 17 → **25** | 25+ | ✅ |
| `stock_transfers.rs` | 18 → **25** | 25+ | ✅ |
| `inventory.rs` | 19 → **30** | 30+ | ✅ |
| `tax.rs` | 19 → **25** | 25+ | ✅ |

**Total new Rust tests needed:** ~160+

### Key test scenarios to add

- **Recipes**: BOM deduction edge cases, fractional ingredient handling, no-recipe product fallback
- **Cart**: Tax re-computation on line change, multi-line discount interactions, empty cart edge cases
- **Offline queue**: Serialization roundtrips, priority ordering, deduplication by sale ID
- **Inventory**: Location-aware stock movements, threshold CRUD edge cases, negative stock guards
- **Stock transfers**: Partial receipt lifecycle, cancelled-draft interactions, transit expiry
- **Reports**: Date-range bounds, empty data periods, multi-currency aggregation

---

## 🧪 UI Test Coverage — Untested Screens

**Goal:** Add dedicated test files for all screens missing test coverage.

### Completely untested screens (7 screens, no test file exists)

| Screen | Feature area | Suggested test count | Key coverage areas |
|--------|--------------|---------------------|-------------------|
| `KdsLayoutFocus` | kds | **8** ✅ | Urgency sorting, status filter pills, active class, empty state, counts |
| `KdsLayoutKanban` | kds | **8** ✅ | Column rendering, per-column counts, column class names, ticket placement, empty state, onAdvance |
| `KdsLayoutMetro` | kds | **8** ✅ | Responsive grid, overdue tile styling, action buttons per tile |
| `KdsLayoutSwitcher` | kds | **13** ✅ | Popover open/close (click, Escape, outside), layout selection with aria-pressed, display toggle callbacks |
| `ShiftBar` | inventory | **8** ✅ | Active shift display, end-shift flow, transaction summary, empty state, start form, location selection, modal close |
| `ThresholdConfigScreen` | inventory | **8** ✅ | Table rendering, add/edit/delete threshold, validation, location filter, dialog, delete |
| `TransitAuditScreen` | inventory | **8** ✅ | Overdue detection, reverse transfer, empty state, line items, confirm/cancel dialog |

### Screens with existing tests but <25 tests (low coverage)

| Screen | Current | Target | New tests needed |
|--------|---------|--------|-----------------|
| `RetailPosScreen` | 24 | 35+ | 11 |
| `PosScreen` | 22 | 35+ | 13 |
| `SalesHistoryScreen` | 14 | 25+ | 11 |
| `InventoryAdjustmentScreen` | 8 | 20+ | 12 |
| `ProductLookupScreen` | 14 | 25+ | 11 |
| `StaffLoginScreen` | 17 | 30+ | 13 |

**Total new UI tests needed:** ~120+

---

## 🔴 Critical Backend Gaps

### 1. `get_workspace_locations` — Unified Resolver (ADR §10)

**Status:** ✅ IMPLEMENTED
**File:** `crates/oz-core/src/location_resolver.rs`

**Acceptance criteria:**
- [x] `pub fn get_workspace_locations(conn, instance_id, type_key) -> Result<Vec<WorkspaceLocationBinding>, CoreError>` — resolves from `workspace_inventory_locations` for `store-pos`, from `bound_location_id` for `warehouse`, returns empty for other types
- [x] Returns `CoreError::Validation` on split-brain (both binding mechanisms active)
- [x] Returns ALL active inventory_locations when `bound_location_id IS NULL` on `warehouse` type
- [x] `WorkspaceLocationBinding` struct with `location_id`, `location_name`, `is_primary`, `allow_negative_stock`
- [x] 8 unit tests covering all acceptance criteria
- [x] Integration with existing callers (Tauri commands) — `get_workspace_locations_scoped` + `invalidate_location_cache_scoped` commands added to `apps/desktop-client/src/commands/inventory.rs`

### 2. Synchronous Alert Engine

**Status:** ✅ IMPLEMENTED
**File:** `crates/oz-core/src/db/products.rs` (private method `check_stock_threshold_and_alert_in_tx`, called from `adjust_stock_at_location_with_reason` step 4)

**Acceptance criteria:**
- [x] After any `adjust_stock_at_location_with_reason` call, check configured thresholds for the changed product+location
- [x] If stock drops below threshold: INSERT into `stock_alert_events` (deduped — no duplicate active alerts per threshold)
- [x] If stock recovers above threshold: UPDATE active alerts to `resolved` status (auto-resolve)
- [x] Threshold lookup order: product+location specific → product+global (location_id NULL) → skip
- [x] 7 unit tests: threshold trigger, no alert above, dedup, recovery auto-resolve, global fallback, no-threshold skips, location-specific precedence

---

## 🟡 Medium Backend Gaps

### 3. `low_stock_alerts_at_location` — Location-Aware Variant

**Status:** ✅ IMPLEMENTED (backend only)
**File:** `crates/oz-core/src/db/reports.rs`

The existing `get_low_stock_alerts` Tauri command takes only a global `threshold` parameter — no `location_id` filter.

**Acceptance criteria:**
- [x] Add `pub fn low_stock_alerts_at_location(&self, location_id: &str, default_threshold: i64) -> Result<Vec<LowStockAlert>, CoreError>` — uses `stock_summary` per-location + COALESCE of custom/product-global/default threshold
- [x] Add `pub fn active_stock_alerts(&self, location_id: &str) -> Result<Vec<StockAlertEvent>, CoreError>` — queries `stock_alert_events` LEFT JOINed with `products` for SKU/name enrichment
- [x] `StockAlertEvent` struct with 13 fields (incl. product_sku, product_name)
- [x] Scoped Tauri command: `get_low_stock_alerts_at_location_scoped` — added to `apps/desktop-client/src/commands/inventory.rs`
- [x] Frontend API wrapper: `getLowStockAlertsAtLocation` + `WorkspaceLocationBinding` interface + `getWorkspaceLocations` + `invalidateLocationCache` — added to `ui/src/api/inventory.ts`
- [x] 6 unit tests: per-location alerts, location with no alerts, custom threshold, active-only, excludes resolved
- [x] Deprecated old `low_stock_alerts` with `#[deprecated]` note

### 4. `stock.negative` Event Emission

**Status:** ✅ IMPLEMENTED (production code, test deferred)
**Files:** `crates/oz-core/src/cache.rs` (trait + RedisCache), `crates/oz-core/src/db/products.rs` (step 5 in adjust_stock_at_location_with_reason)

When `allow_negative_stock` is enabled and a deduction goes below zero, the ADR §4 says the backend MUST emit a warning event.

**Acceptance criteria:**
- [x] After `adjust_stock_at_location_with_reason` with resulting qty < 0 AND `allow_negative_stock == true`: emit `stock.negative` event via `cache.publish_negative_stock_event()`
- [x] Event payload: `{ product_id, sku, location_id, delta, current_qty, terminal_id, timestamp }`
- [x] Cache trait + NoopCache (no-op default) + RedisCache (publishes to `stock:negative` channel)
- [x] Unit test: negative stock event fires correctly — **implemented** via `seed_allow_negative_terminal` helper with ALTER TABLE to add `workspace_instance_id` column. Two tests: negative event fires (qty=-3) and normal deduction does not fire event. Also fixed production code bug where `inventory` table CHECK (qty >= 0) blocked negative writes even when `allow_negative_stock=true` — step 3 now catches and handles the constraint violation gracefully.

---

## 🔵 Frontend — Missing Components

### 5. `StockAlertPanel` — Alert Sidebar/Badge

**Status:** ✅ IMPLEMENTED
**Files:** `ui/src/features/inventory/StockAlertPanel.tsx`, `ui/src/features/inventory/StockAlertPanel.css`, `ui/src/__tests__/StockAlertPanel.test.tsx`

Dashboard widget or right-side drawer panel showing active alerts with badge count, integrated into ProductManagementScreen.

**Acceptance criteria:**
- [x] `StockAlertPanel.tsx` component with alert list, loading/error/empty states, severity indicators (critical=red for qty=0, warning=amber for qty>0), and relative time formatting
- [x] Bell toggle button in ProductManagementScreen header opens/closes drawer
- [x] Each alert shows: SKU, product name, current qty vs threshold, time triggered
- [x] [Acknowledge] button records who saw it via `acknowledge_stock_alert` Tauri command, removes from local state immediately
- [x] Polling interval (30s default, configurable via `pollIntervalMs` prop)
- [x] Filterable by location (via `locationId` prop)
- [x] Backend: `active_stock_alerts_scoped` + `acknowledge_stock_alert_scoped` Tauri commands in `apps/desktop-client/src/commands/inventory.rs`
- [x] Backend: `acknowledge_stock_alert` method in `crates/oz-core/src/db/reports.rs`
- [x] Frontend API wrappers: `getActiveStockAlerts` + `acknowledgeStockAlert` in `ui/src/api/inventory.ts`
- [x] 6 unit tests: loading state, alert rendering, badge count, severity classes, empty state, acknowledge button, error state

### 6. Location Picker in Inventory Workspace Header

**Status:** ✅ IMPLEMENTED
**Files:** `ui/src/features/inventory/LocationPicker.tsx`, `ui/src/features/inventory/LocationPicker.css`, `ui/src/__tests__/LocationPicker.test.tsx`

The ADR §5 specifies a location switcher dropdown in the inventory workspace header so the user can switch between locations without leaving the workspace.

**Acceptance criteria:**
- [x] Dropdown in inventory workspace header (ProductManagementScreen) showing all active locations for the store
- [x] Current location highlighted with `aria-selected` + active CSS class; selecting a new location re-scopes the view
- [x] Location type metadata displayed (warehouse, store, transit)
- [x] StockAlertPanel dynamically scoped to selected location
- [x] Outside-click and Escape key close dropdown
- [x] 9 unit tests: render, open/close, selection, empty state, ARIA compliance

---

## 🔴 §13 Post-Decision Review Amendments

### 7. Finding #34 — `received_partial` CHECK + `receive_transfer` Code

**Status:** ✅ FIXED
**Files:** `crates/oz-core/migrations/081_stock_transfers_received_partial.sql`, `crates/oz-core/src/db/stock_transfers.rs`

Migration 081 adds `received_partial` to the CHECK constraint, but `receive_transfer` was writing `'in_transit'` instead of `'received_partial'` on partial receives.

- [x] Migration 081 CHECK allows `'received_partial'` ✅
- [x] `receive_transfer` was writing `'in_transit'` instead of `'received_partial'` ❌ **FIXED**
- [x] Updated `receive_transfer` to write `'received_partial'` when at least one line has `received_qty > 0` but not all lines are fully received
- [x] Added `has_any_received` guard: all-zero receipt stays `'in_transit'`
- [x] Renamed test `partial_receive_leaves_in_transit` → `partial_receive_writes_received_partial_status`

---

## 🟡 New ADR Required

### 8. Finding #31 — Payment-Capture Ordering (Stock Reservation)

**Status:** ✅ DRAFTED
**File:** `docs/decisions/2026-07-19-payment-capture-ordering.md`

Draft ADR-20 for "Payment-Capture Ordering" specifying the stock-reservation-before-payment-capture pattern to prevent stranded funds during concurrent checkout races.

- [x] Draft ADR-20 spec in `docs/decisions/2026-07-19-payment-capture-ordering.md`
- [x] Define `create_pending_sale` / `create_pending_sale_with_resolution` flow with `BEGIN IMMEDIATE` atomicity
- [x] Define `finalize_sale` (on capture success) and `void_pending_sale` (on capture failure/abandon) with FIFO oldest-credit stock restoration
- [x] Acceptance criteria (6 criteria: dedup, serialization, finalize, void, stale-reap, concurrent finalize/void)
- [x] Migration spec (095: add `pending` status to sales CHECK constraint)
- [x] Tauri command spec (3 scoped commands)
- [x] Background worker spec (30-min pending sale timeout reaper)
- [x] Frontend impact (PaymentModal three-phase flow with error states)

---

## ✅ Already Verified Done

| Item | Status | Evidence |
|------|--------|----------|
| Tauri commands for location CRUD | ✅ Done | Desktop client `inventory.rs` commands |
| StockShortfallDialog | ✅ Done | `ui/src/features/sales/StockShortfallDialog.tsx` |
| TransitAuditScreen | ✅ Done | `ui/src/features/inventory/TransitAuditScreen.tsx` |
| TransactionLogScreen | ✅ Done | `ui/src/features/inventory/TransactionLogScreen.tsx` |
| ThresholdConfigScreen | ✅ Done | `ui/src/features/inventory/ThresholdConfigScreen.tsx` |
| ShiftBar | ✅ Done | `ui/src/features/inventory/ShiftBar.tsx` |
| API wrappers (locations, thresholds, shifts) | ✅ Done | `ui/src/api/inventory.ts` |
| Per-location stock in POS product lookup | ✅ Partial (ADR-19) | Deduction location flow wired |
| Finding #33 — deduction_locations JSON schema | ✅ Done | ADR-19 FIFO refund |
| Finding #35 — stock_movements location index | ✅ Done | Migration 080 |
| Finding #36 — Stable UUIDs for default/transit | ✅ Done | Migration 078 |
| Finding #37 — Rename cascade coordination | ✅ Done | Migration 091 |

---

## 🧭 Dependency Graph

```
🔴 get_workspace_locations ──────┐
🔴 Alert engine ─────────────────┤
🟡 low_stock_alerts_at_location ─┤
                                 ├──→ ADR-18 Complete
🔵 StockAlertPanel ──────────────┤
🔵 Location picker ──────────────┤
❓ received_partial verify ──────┘

🟡 New ADR (Payment-Capture) ─── independent
```

---

## 🎯 Quick Reference

| Priority | Item | Est. Effort | Dependencies |
|----------|------|-------------|--------------|
| 🔴 | `get_workspace_locations` resolver | 2–3 hrs | ✅ Done |
| 🔴 | Synchronous alert engine | 3–4 hrs | ✅ Done |
| 🟡 | `low_stock_alerts_at_location` | 1–2 hrs | ✅ Done (backend) — Tauri + frontend deferred |
| 🟡 | `stock.negative` event emission | 1 hr | ✅ Done (test deferred — terminal FK setup) |
| 🔵 | `StockAlertPanel` frontend | 2–3 hrs | ✅ Done |
| 🔵 | Location picker in header | 2–3 hrs | ✅ Done |
| 🔴 | Finding #34 verification | 30 min | ✅ Done |
| 🟡 | Payment-Capture ADR draft | 2 hrs | None |
