# 0.0.12 — ADR-18 Implementation Gaps

> **Goal:** Close all remaining ADR-18 Multi-Location Inventory gaps — unified resolver, alert engine, frontend components, and §13 amendments.

**Current state:** 7 / 31 items complete (23%) · Updated 2026-07-26

---

## Progress Summary

| Area | Total | Done | Progress |
|------|-------|------|----------|
| 🔴 Backend — Critical | 2 | 0 | ░░░░░░░░░░ 0% |
| 🟡 Backend — Medium | 2 | 0 | ░░░░░░░░░░ 0% |
| 🧪 Rust Test Coverage | 14 | 7 | █████░░░░░ 50% |
| 🧪 UI Test Coverage | 7 | 0 | ░░░░░░░░░░ 0% |
| 🔵 Frontend — Missing | 2 | 0 | ░░░░░░░░░░ 0% |
| 🔴 §13 Amendments | 1 | 0 | ░░░░░░░░░░ 0% |
| 🟡 §13 Amendments | 1 | 0 | ░░░░░░░░░░ 0% |
| ❓ Verification | 1 | 0 | ░░░░░░░░░░ 0% |
| 🟡 New ADR | 1 | 0 | ░░░░░░░░░░ 0% |
| **Total** | **31** | **7** | **█████░░░░░ 23%** |

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
| `terminal_profiles.rs` | 10 | 15+ | 5 |
| `refunds.rs` | 11 | 20+ | 9 |
| `cart.rs` | 12 | 20+ | 8 |
| `gift_cards.rs` | 12 | 15+ | 3 |
| `kds.rs` | 12 | 20+ | 8 |
| `customers.rs` | 13 | 15+ | 2 |
| `offline.rs` | 14 | 20+ | 6 |
| `store_profiles.rs` | 14 | 20+ | 6 |
| `audit.rs` | 15 | 20+ | 5 |
| `cash_payouts.rs` | 15 | 20+ | 5 |
| `payments.rs` | 15 | 20+ | 5 |
| `purchase_orders.rs` | 15 | 20+ | 5 |
| `suppliers.rs` | 15 | 20+ | 5 |
| `reports.rs` | 17 | 25+ | 8 |
| `settings.rs` | 17 | 25+ | 8 |
| `terminals.rs` | 17 | 25+ | 8 |
| `stock_transfers.rs` | 18 | 25+ | 7 |
| `inventory.rs` | 19 | 30+ | 11 |
| `tax.rs` | 19 | 25+ | 6 |

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
| `KdsLayoutFocus` | kds | 8 | Urgency sorting, status filter pills, action buttons, empty state |
| `KdsLayoutKanban` | kds | 8 | Column rendering, drag-between-columns, SLA colour thresholds |
| `KdsLayoutMetro` | kds | 8 | Responsive grid, overdue tile styling, action buttons per tile |
| `KdsLayoutSwitcher` | kds | 6 | Popover open/close, layout selection, display toggles, persistence |
| `ShiftBar` | inventory | 6 | Active shift display, end-shift flow, transaction summary, empty state |
| `ThresholdConfigScreen` | inventory | 8 | Table rendering, add/edit/delete threshold, validation, location filter |
| `TransitAuditScreen` | inventory | 8 | Overdue detection, reverse transfer, empty state, location filter |

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

**Status:** ❌ NOT IMPLEMENTED
**File:** `crates/oz-core/src/db/inventory.rs` (test reference exists at line 833)

The single entry point that prevents split-brain between `bound_location_id` (on `workspace_instances`) and `workspace_inventory_locations` (the dedicated binding table). Without it, the dual-binding scenario silently creates undefined behavior.

**Acceptance criteria:**
- [ ] `pub fn get_workspace_locations(store: &Store, instance_id: &str, type_key: &str) -> Result<Vec<WorkspaceLocationBinding>, CoreError>` — resolves locations from `workspace_inventory_locations` for `store-pos` type, from `bound_location_id` for `warehouse` type, returns empty for other types
- [ ] Returns `CoreError::Validation` if BOTH `bound_location_id` AND rows in `workspace_inventory_locations` exist (split-brain)
- [ ] Returns ALL active locations when `bound_location_id IS NULL` on a `warehouse` type (unbound admin view)
- [ ] `WorkspaceLocationBinding` struct with `location_id`, `location_name`, `is_primary`, `allow_negative_stock`
- [ ] Unit tests: unbound, single-binding, dual-binding error, store-pos type, warehouse type, other type returns empty
- [ ] Integration with existing callers (inventory commands, sale deduction flow)

### 2. Synchronous Alert Engine

**Status:** ❌ NOT WIRED
**Files:** `crates/oz-core/src/db/inventory.rs`, `crates/oz-core/src/db/products.rs`

`stock_thresholds` and `stock_alert_events` tables exist (migration 087) but no code checks thresholds during stock changes.

**Acceptance criteria:**
- [ ] After any `adjust_stock_at_location_with_reason` call, check configured thresholds for the changed product+location
- [ ] If stock drops below threshold: INSERT into `stock_alert_events` (deduped — no duplicate active alerts per threshold)
- [ ] If stock recovers above threshold: UPDATE active alerts to `resolved` status (auto-resolve)
- [ ] Threshold lookup order: product+location specific → product+global (location_id NULL) → skip
- [ ] Unit tests: threshold trigger, recovery auto-resolve, dedup, global threshold, no-threshold skips

---

## 🟡 Medium Backend Gaps

### 3. `low_stock_alerts_at_location` — Location-Aware Variant

**Status:** ❌ Still uses global
**File:** `crates/oz-core/src/db/reports.rs`

The existing `get_low_stock_alerts` Tauri command takes only a global `threshold` parameter — no `location_id` filter.

**Acceptance criteria:**
- [ ] Add `pub fn low_stock_alerts_at_location(&self, location_id: &str, default_threshold: i64) -> Result<Vec<LowStockAlert>, CoreError>`
- [ ] Add `pub fn active_stock_alerts(&self, location_id: &str) -> Result<Vec<StockAlertEvent>, CoreError>`
- [ ] Add scoped Tauri command: `get_low_stock_alerts_at_location_scoped`
- [ ] Add frontend API wrapper: `getLowStockAlertsAtLocation`
- [ ] Unit tests: per-location alerts, location with no alerts, mixed thresholds
- [ ] Deprecate old `get_low_stock_alerts` with `#[deprecated]` note

### 4. `stock.negative` Event Emission

**Status:** ❌ NOT WIRED
**File:** `crates/oz-core/src/db/products.rs`

When `allow_negative_stock` is enabled and a deduction goes below zero, the ADR §4 says the backend MUST emit a warning event.

**Acceptance criteria:**
- [ ] After `adjust_stock_at_location_with_reason` with resulting qty < 0 AND `allow_negative_stock == true`: emit `stock.negative` event
- [ ] Event payload: `{ product_id, location_id, current_qty, delta }`
- [ ] Event emission via existing event bus or logging mechanism
- [ ] Inventory dashboard badge shows affected SKUs and locations
- [ ] Unit test: negative stock event fires correctly

---

## 🔵 Frontend — Missing Components

### 5. `StockAlertPanel` — Alert Sidebar/Badge

**Status:** ❌ NOT FOUND
**File:** `ui/src/features/inventory/` (new file)

Dashboard widget or sidebar showing active alerts with badge count.

**Acceptance criteria:**
- [ ] `StockAlertPanel.tsx` component with alert list
- [ ] Badge count on inventory workspace header
- [ ] Each alert shows: SKU, product name, current qty vs threshold, time triggered
- [ ] [Acknowledge] button records who saw it via `acknowledge_stock_alert` Tauri command
- [ ] Filterable by location
- [ ] Polling or real-time refresh

### 6. Location Picker in Inventory Workspace Header

**Status:** ❌ NOT FOUND
**File:** Inventory workspace header component

The ADR §5 specifies a location switcher dropdown in the inventory workspace header so the user can switch between locations without leaving the workspace.

**Acceptance criteria:**
- [ ] Dropdown in workspace header showing all active locations for the store
- [ ] Current location highlighted; selecting a new location re-scopes the view
- [ ] Persisted per user or session
- [ ] Works with warehouse type workspaces (both bound and unbound)

---

## 🔴 §13 Post-Decision Review Amendments

### 7. Finding #34 — `received_partial` CHECK + `receive_transfer` Code

**Status:** ❓ NEED TO VERIFY
**Files:** `crates/oz-core/migrations/081_stock_transfers_received_partial.sql`, `crates/oz-core/src/db/stock_transfers.rs`

Migration 081 adds `received_partial` to the CHECK constraint, but does `receive_transfer` actually write `'received_partial'` instead of keeping `'in_transit'`?

- [ ] Verify migration 081 CHECK allows `'received_partial'`
- [ ] Verify `receive_transfer` in `stock_transfers.rs` writes `'received_partial'` on partial receive (qty < ordered)
- [ ] If missing: update `receive_transfer` status-update branch
- [ ] Test: partial receive writes `received_partial` status

---

## 🟡 New ADR Required

### 8. Finding #31 — Payment-Capture Ordering (Stock Reservation)

**Status:** ❌ NEW ADR NEEDED

Draft a new ADR for "Payment-Capture Ordering" that specifies the stock-reservation-before-payment-capture pattern to prevent stranded funds during concurrent checkout races.

- [ ] Draft ADR-20 (or ADR-21) spec in `docs/decisions/`
- [ ] Define `create_pending_sale` / `create_pending_sale_with_resolution` flow
- [ ] Define `finalize_sale` (on capture success) and `void_pending_sale` (on capture failure/abandon)
- [ ] Acceptance criteria for the reservation flow

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
| 🔴 | `get_workspace_locations` resolver | 2–3 hrs | None |
| 🔴 | Synchronous alert engine | 3–4 hrs | stock_thresholds table exists |
| 🟡 | `low_stock_alerts_at_location` | 1–2 hrs | None (parallel with alert engine) |
| 🟡 | `stock.negative` event emission | 1 hr | None |
| 🔵 | `StockAlertPanel` frontend | 2–3 hrs | Alert engine + API |
| 🔵 | Location picker in header | 2–3 hrs | location CRUD commands |
| 🔴 | Finding #34 verification | 30 min | Migration 081 |
| 🟡 | Payment-Capture ADR draft | 2 hrs | None |
