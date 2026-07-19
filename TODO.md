# 0.0.11 — Release Checklist

> **Goal:** Polish everything for release-quality — close all a11y gaps, harden offline resilience, push test coverage, add KDS/reporting features.

**Current state:** 13 / 101 items complete (12.9%) · Updated 2026-07-19

---

## Progress Summary

| Area | Total | Done | Progress |
|------|-------|------|----------|
| ♿ Accessibility | 17 | 4 | ███░░░░░░░ 24% |
| 🔌 Offline & Data | 8 | 0 | ░░░░░░░░░░ 0% |
| 🧪 Rust Test Coverage | 13 | 2 | █░░░░░░░░░ 15% |
| 🧪 UI Test Coverage | 8 | 0 | ░░░░░░░░░░ 0% |
| 🧹 Tech Debt | 11 | 2 | █░░░░░░░░░ 18% |
| 🍳 KDS Enhancements | 9 | 0 | ░░░░░░░░░░ 0% |
| 🧾 Reporting & Analytics | 6 | 0 | ░░░░░░░░░░ 0% |
| 🛒 Payment Gateway | 6 | 0 | ░░░░░░░░░░ 0% |
| 🏪 Multi-Store UX | 4 | 0 | ░░░░░░░░░░ 0% |
| 📦 Release Ops | 19 | 5 | ██░░░░░░░░ 26% |
| **Total** | **101** | **13** | **░░░░░░░░░░ 13%** |

---

## P0 — Critical (must ship)

### ♿ 1. Form Validation & Error Messages

All forms must surface clear, specific validation errors with `role="alert"`.

**1.1 StaffLoginScreen** — PIN mismatch, username not found, rate-limit lockout
- [ ] Add `role="alert"` wrapper around error display
- [ ] Ensure `aria-live="polite"` for dynamic error appearance
- [ ] Show rate-limit remaining attempts countdown

**1.2 CreatePinScreen** — min length, mismatch, complexity requirements
- [x] 17 hardcoded strings already fixed
- [ ] Audit remaining error messages for specificity
- [ ] Add `role="alert"` to all error surfaces

**1.3 PaymentModal** — insufficient stock, payment declined, connection lost
- [ ] Surface backend error messages with `role="alert"`
- [ ] Differentiate between retryable (network) vs terminal (declined) errors
- [ ] Add retry button for recoverable errors

**1.4 PriceOverrideModal** — reason required, amount out of range
- [ ] Add inline field validation with error messages
- [ ] Validate reason field not empty before submit

**1.5 SettingsPage**
- [x] Already implemented with `role="alert"` on errors
- [x] Field-level `fieldErrors` state + `validateField` / `clearFieldError`

### ♿ 2. Fluent Strings Audit

- [ ] Scan all TSX files in `src/features/` for hardcoded English strings
- [ ] Fix all violations with `<Localized id="...">` wrappers
- [ ] Add new Fluent keys to `sales.ftl`, `settings.ftl`, `inventory.ftl`, etc.

### ♿ 3. Mobile / Tablet Viewport

- [ ] **Touch spacing** — Verify ≥ 8px gap between all touchable elements on tablet (768–1024px)
  - [ ] Run `touchTargetSizing.test.tsx`
  - [ ] Fix any violations found
- [ ] **No horizontal scroll** — Verify all screens fit within tablet viewport
  - [ ] POS screens (retail, restaurant)
  - [ ] Settings pages
  - [ ] Reports & management screens
  - [ ] KDS screens
- [ ] Add `overflow-x: hidden` and responsive width constraints where needed

### ♿ 4. `aria-live` Regions for Dynamic Content

- [ ] `AuditLogScreen.tsx` — real-time log feed
- [ ] `TransactionLogScreen.tsx` — transaction feed
- [ ] `StockShortfallDialog.tsx` — dynamic stock count updates

### ♿ 5. ARIA Role Audit on Custom Components

- [ ] Toggle switches — `role="switch"`, `aria-checked`
- [ ] Slider selects — `role="slider"`, `aria-valuemin/max/now`
- [ ] Chip groups — `role="listbox"`, `aria-selected`
- [ ] Context menus — `role="menu"`, `aria-expanded`

---

## P1 — High Priority

### 🔌 6. Offline Resilience

**6.1 Offline queue reconciler** — Design and implement state machine for the two-command sale flow
- [ ] Write ADR-21 spec in `docs/decisions/`
- [ ] Implement queue persistence with deduplication keys (uuid v7 sale id)
- [ ] Add reconciler that replays failed commands when back online
- [ ] Wire reconciler into the POS `complete` flow

**6.2 Cache strategy for `resolve_primary_location`**
- [ ] Add in-memory cache in `location_resolver.rs` with 30s TTL
- [ ] Invalidate on workspace change

**6.3 Held carts `deduction_location_id` lock**
- [ ] Migrate `active_carts.deduction_location_id` to the `held_carts` table
- [ ] Restore deduction location when cart is un-held

### 🧪 7. Rust Test Coverage — Gap Modules

| Module | Current | Target | Est. tests | Depends on |
|--------|---------|--------|-----------|------------|
| `db/payments.rs` | 10 | 15+ | 5 new | — |
| `db/stock_transfers.rs` | 12 | 18+ | 6 new | — |
| `db/audit.rs` | 10 | 15+ | 5 new | — |
| `db/purchase_orders.rs` | 10 | 15+ | 5 new | — |
| `db/cash_payouts.rs` | 7 | 15+ | 8 new | — |
| `db/suppliers.rs` | 10 | 15+ | 5 new | — |
| `db/reports.rs` | 17 | 20+ | 3 new | — |

**7.1 Payments**
- [ ] `test_refund_full_sale_credits_correct_amount`
- [ ] `test_partial_refund_leaves_remaining`
- [ ] `test_multi_tender_refund_splits_correctly`
- [ ] `test_refund_already_refunded_sale_errors`
- [ ] `test_void_payment_reverses_inventory`

**7.2 Stock transfers**
- [ ] `test_transfer_full_lifecycle` (create → approve → send → receive)
- [ ] `test_transfer_cancel_before_send_allows`
- [ ] `test_transfer_receive_partial_creates_correct_movements`
- [ ] `test_transfer_receive_excess_stock_errors`
- [ ] `test_transfer_approval_required_before_send`
- [ ] `test_transfer_reject_returns_stock_to_sender`

**7.3 Purchase orders**
- [ ] `test_po_full_lifecycle`
- [ ] `test_po_receive_partial`
- [ ] `test_po_receive_over_ordered_errors`
- [ ] `test_po_close_prevents_further_receiving`
- [ ] `test_po_reopen_allows_more_receiving`

**7.4 Cash payouts**
- [ ] `test_payout_above_float_errors`
- [ ] `test_reconcile_matches_expected_amount`
- [ ] `test_negative_payout_errors`
- [ ] `test_payout_in_different_currencies`
- [ ] `test_payout_zero_amount_errors`
- [ ] `test_payout_during_closed_shift_errors`
- [ ] `test_reconcile_mismatch_logs_discrepancy`
- [ ] `test_payout_with_invalid_staff_errors`

**7.5 Audit log**
- [ ] `test_audit_log_entry_created_on_sale_complete`
- [ ] `test_audit_log_entry_created_on_inventory_change`
- [ ] `test_audit_log_entries_filterable_by_type`
- [ ] `test_audit_log_pagination_works`
- [ ] `test_audit_log_large_entries_truncated`

**7.6 Suppliers**
- [ ] `test_supplier_crud_full_cycle`
- [ ] `test_supplier_duplicate_name_errors`
- [ ] `test_supplier_with_active_po_cannot_be_deleted`
- [ ] `test_supplier_contact_info_validated`
- [ ] `test_supplier_search_by_name_partial_match`

### 🧪 8. UI Test Coverage

- [ ] **Cross-reference all 55+ feature components against their test files**
  - [ ] `DataManagementScreen.test.tsx` — backup/export/import all flows
  - [ ] `RetailPosScreenCheckout.test.tsx` — discount + refund in retail
  - [ ] `RetailPosScreenInteractions.test.tsx` — multi-tender edge cases
  - [ ] `StockShortfallDialog.test.tsx` — ADR-19 split-fulfillment coverage
  - [ ] `TransactionLogScreen.test.tsx` — new component, needs coverage
  - [ ] `SettingsPage.test.tsx` — all tabs render correctly
  - [ ] `FastPINOverlay.test.tsx` — PIN entry, error states, verification flow
  - [ ] `QrisQrDisplay.test.tsx` — QR renders, expiry handling

---

## P2 — Medium Priority

### 🍳 9. KDS Enhancements

**9.1 Configurable SLA timers** — Per-zone thresholds
- [ ] Add `kitchen_zone_sla_seconds` to workspace settings
- [ ] Wire SLA into KDS ticket colour thresholds
- [ ] Show remaining SLA time on each ticket
- [ ] Admin UI to configure SLA per zone

**9.2 KDS sound alert** — Play notification when ticket transitions `started_at → ready`
- [ ] Use Web Audio API for short chime (no external files)
- [ ] Gate behind `prefers-reduced-motion` check
- [ ] Volume control in KDS settings

**9.3 Bulk status update** — Multi-select tickets
- [ ] Add checkbox mode to KDS ticket cards
- [ ] Batch API call: `update_kds_orders_bulk`
- [ ] Confirmation dialog before bulk action

**9.4 KDS filtering & search**
- [ ] Filter by status, prep time remaining, kitchen zone
- [ ] Search by order number or item name
- [ ] Persist filter preference in `useKdsPreferences`

**9.5 KDS mobile responsive**
- [ ] Verify all 3 layouts (Kanban, Focus, Metro) on 768–1024px
- [ ] Fix any overflow or overlapping issues

### 🧾 10. Reporting & Analytics

**10.1 Sales dashboard trends**
- [ ] Recharts integration in `DashboardScreen.tsx`
- [ ] Period selector (7d / 30d / 90d / 1y)
- [ ] Period-over-period comparison (e.g., "↑12% vs last week")
- [ ] Top products & categories breakdown

**10.2 Inventory report — per-location**
- [ ] Backend: `get_inventory_report_by_location` in `db/inventory.rs`
- [ ] Wire into `InventoryReportScreen.tsx`
- [ ] Slow-mover detection (>90 days no movement)

**10.3 Staff performance report**
- [ ] Add `get_staff_sales_summary` query
- [ ] Sales per staff, transaction count, avg basket size
- [ ] Compare against store average

**10.4 Export to CSV**
- [ ] Use `@tauri-apps/plugin-dialog` for save dialog
- [ ] Format and write CSV with headers
- [ ] Export button on Sales, Inventory, Staff reports

---

## P3 — Nice to Have

### 🛒 11. Payment Gateway Expansion

**11.1 Stripe Terminal** — In-person card payments
- [ ] Research Tauri + Stripe Terminal compatibility
- [ ] Implement command flow: `process_card_presentation_scoped`
- [ ] Test with Stripe test cards

**11.2 QRIS dynamic QR** — Payment with callback confirmation
- [ ] Verify `QrisQrDisplay.tsx` polling for online and offline flows

**11.3 Multi-tender payments** — Split across cash + card + QR
- [ ] Design multi-tender payload in `CompleteSaleArgs`
- [ ] Implement split-payment processing in Rust
- [ ] UI: multi-tender selector in `PaymentModal.tsx`

**11.4 Payment retry with idempotency keys**
- [ ] Generate idempotency key per payment attempt (uuid v7)
- [ ] Implement retry with exponential backoff (3 attempts max)

### 🏪 12. Multi-Store UX

- [ ] **Store switcher persistence** — Remember last-active store per device
  - [ ] Save to `localStorage` on switch
  - [ ] Auto-restore on page load
- [ ] **Cross-store reporting** — Aggregate sales/inventory across all stores
  - [ ] New dashboard view with store-selector dropdown
  - [ ] Backend query: `get_aggregated_sales_by_date_range`
- [ ] **Centralised product management** — Admin pushes product changes from one store to all
  - [ ] "Push to all stores" button in `ProductManagementScreen.tsx`
  - [ ] Backend: `push_product_to_workspace` command

---

## 🧹 13. Tech Debt

**13.1 Remove deprecated APIs**
- [ ] Delete `adjust_stock` / `adjust_stock_with_reason` function bodies
- [ ] All callers already migrated to `adjust_stock_at_location_with_reason`
- [ ] Update remaining Tauri command references
- [ ] Run full test suite to confirm no breakage

**13.2 Squash migrations into initial schema**
- [ ] Create `migrations/000_initial.sql` with full schema
- [ ] Update `migrations.rs` to skip individual migrations when 000 exists

**13.3 ESLint `exhaustive-deps` cleanup**
- [ ] `ShiftBar.tsx` — verify dependency array
- [ ] `ThresholdConfigScreen.tsx` — verify dependency array
- [ ] `PaymentModal.tsx` — verify dependency array

**13.4 Remove `dev-mock/tauri-api.ts` — TODO stubs**
- [ ] Replace all `// TODO` stubs with real mock implementations
- [ ] Ensure all common `invoke` commands are handled

---

## 🧹 14. Housekeeping

- [ ] Remove junk files from repo root: `_doc_test_output.txt`
- [ ] Remove junk from `ui/`: `..violations.txt`, `test-output.txt`
- [ ] Delete `ui/src/__tests__/themeTokenCompliance_patched.ts` (was a one-off patch)

---

## 🧪 15. UI Theme Regression Tests

- [ ] Snapshot tests for all 3 themes on key screens
  - [ ] POS (retail + restaurant)
  - [ ] Settings pages
  - [ ] Auth / login
  - [ ] Reports
- [ ] Verify `--color-*` tokens resolve in JSDOM (`getComputedStyle`)

---

## 📋 16. Release Checklist

**Code quality gates**
- [ ] `cargo test --workspace` — all passing
- [ ] `npm run test` — all passing
- [ ] `npm run lint` — 0 errors, 0 warnings
- [ ] `npm run typecheck` — 0 errors
- [ ] Migration idempotency — all clean
- [ ] `skill-drift-guard` — 0 findings
- [ ] Design token compliance — 0 violations

**Version & changelog**
- [x] Bump version to 0.0.11 (done)
- [ ] Update `CHANGELOG.md` with all changes since 0.0.10
- [ ] Review `CHANGELOG.md` for accuracy and completeness

**Release & PR**
- [ ] All P0 items complete
- [ ] Push `0.0.11` branch
- [ ] Create PR with changelog summary
- [ ] Run CI checks on PR
- [ ] Merge PR after CI passes
- [ ] Delete `0.0.11` branch after merge

**Post-release**
- [ ] Create `0.0.12` branch for next cycle
- [ ] Update `TODO.md` for next version
- [ ] Archive PR in release notes

---

## 🧭 Dependency Graph

```
♿ Form errors ─────────┐
♿ Fluent audit ────────┤
♿ Tablet viewport ─────┤
                        ├──→ Release (all P0 must ship)
🔌 Offline resilience ──┤
🧪 Rust tests ──────────┤
🧪 UI tests ────────────┘

🍳 KDS (P2) ─────────── independent
🧾 Reporting (P2) ───── independent
🛒 Payments (P3) ────── depends on multi-tender design
🏪 Multi-store (P3) ─── independent
🧹 Tech debt ─────────── can be done anytime
🧪 Theme snapshots ───── after other UI changes settle
```

---

## 📝 Audit Log

| Date | Area | What was done | Status |
|------|------|---------------|--------|
| 2026-07-19 | All features | ESLint scan (60+ TSX files) | ✅ 0 errors, 0 warnings |
| 2026-07-19 | All features | TypeScript scan | ✅ 0 errors |
| 2026-07-19 | All CSS | Theme token compliance scan | ✅ 0 violations (720 fixed) |
| 2026-07-19 | Settings (6 pages) | Loading/error/ARIA audit | ✅ All passing |
| 2026-07-19 | All features | Loading state coverage check | ✅ 40+ data-fetching pages |
| 2026-07-19 | All features | Error handling (try/catch) check | ✅ 60+ pages |
| 2026-07-19 | All features | ARIA roles/regions check | ✅ 27+ pages |
| 2026-07-19 | settings.ftl, sales.ftl | Fluent strings audit | ✅ No hardcoded English |
| 2026-07-19 | SettingsPage | Form validation audit | ✅ Already complete |
| 2026-07-19 | CreatePinScreen | Hardcoded strings | ✅ Fixed 17 strings |
