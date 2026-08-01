# Inventory Module Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** Inventory module — locations, stock adjustments, stock counts, transfers, shifts, thresholds, alerts, and transaction history  
> **Status:** ✅ **FULLY REMEDIATED** — all 11 findings closed (commits `a2c70848`, `45d65511`, `5be6de69`)
> **Production code changed:** Yes — session-scoped stock-transfer and stock-count commands, store isolation, server-side actor derivation, transfer cancellation reversal, count-number concurrency hardening, batch transit API, and migrations 112/113

## Scope

This audit covers the Inventory workspace UI and API clients, Tauri command authorization and session scope, stock adjustment and transfer lifecycles, physical stock counts, inventory locations and workspace bindings, inventory shifts, thresholds and alerts, transaction history, persistence and migrations, localization, theming, performance, and focused tests.

Inspected areas:

- `ui/src/features/inventory/InventoryAdjustmentScreen.tsx`
- `ui/src/features/inventory/StockCountsScreen.tsx`
- `ui/src/features/inventory/StockCountDetail.tsx`
- `ui/src/features/inventory/StockCountForm.tsx`
- `ui/src/features/inventory/LocationPicker.tsx`
- `ui/src/features/inventory/StockAlertPanel.tsx`
- `ui/src/features/inventory/TransactionLogScreen.tsx`
- `ui/src/features/inventory/ThresholdConfigScreen.tsx`
- `ui/src/features/inventory/TransitAuditScreen.tsx`
- `ui/src/features/inventory/ShiftBar.tsx`
- `ui/src/features/stock-transfers/StockTransfersScreen.tsx`
- `ui/src/api/inventory.ts`
- `ui/src/api/inventoryCounts.ts`
- `ui/src/api/stockTransfers.ts`
- `apps/desktop-client/src/commands/inventory.rs`
- `apps/desktop-client/src/commands/inventory_counts.rs`
- `apps/desktop-client/src/commands/stock_transfers.rs`
- `crates/oz-core/src/db/inventory.rs`
- `crates/oz-core/src/db/stock_counts.rs`
- `crates/oz-core/src/db/stock_transfers.rs`
- `modules/inventory/src/{lib,models,repository,service}.rs`
- Inventory, stock-counting, and stock-transfers Fluent bundles
- Inventory, stock-count, transfer, and related migration schemas
- Focused Inventory UI, API-contract, module, core, and integration tests

The review uses the universal audit lenses from `audit/AUDIT_JULY_2026.md`: functionality, state and UX, accessibility/i18n, theming, performance, security/data integrity, and quality assurance.

## Architecture summary

Inventory is a transitional module. `modules/inventory` contains domain models and a small repository/service surface, while the production multi-location logic remains in `oz-core` and Tauri commands. All inventory mutation/read commands — locations, alerts, transfers, stock counts, adjustments, and inventory transactions — now resolve `session_token` to a store (ADR #7) and enforce permissions; actors are derived from the session, never from request data.

The main inventory primitives are:

- Location records and workspace bindings for routing stock operations.
- Manual inventory transactions that record headers/lines and adjust stock in one database transaction.
- Stock counts that compare counted quantities with expected quantities and generate adjustments.
- Stock transfers with a draft → in-transit → received/cancelled lifecycle.
- Inventory shifts, threshold configuration, low-stock alert events, and transaction history.

## Findings

### INV-01 — Stock-transfer commands are global and unauthenticated (P0)

**Evidence:** `apps/desktop-client/src/commands/stock_transfers.rs` exposes `create_stock_transfer`, `get_stock_transfer`, `list_stock_transfers`, `get_stock_transfer_lines`, `add_stock_transfer_line`, `remove_stock_transfer_line`, `send_stock_transfer`, `receive_stock_transfer`, and `cancel_stock_transfer` with only `State<AppState>`. They do not accept `session_token`, resolve a store, or call `require_permission_for_user`. The corresponding frontend client (`ui/src/api/stockTransfers.ts`) has no session-token parameter and the Stock Transfers screen uses these unscoped functions even though it already has a workspace session.

**Impact:** A caller able to invoke these IPC commands can read or mutate transfer records without the normal store boundary and permission check. In a multi-store deployment, transfer history and stock movements can cross tenant boundaries; an unauthorized caller may also send, receive, or cancel stock.

**Remediation (Phase 1):** Implemented session-scoped transfer commands on desktop and tablet. Each command resolves the store from the opaque session token, checks `inventory:transfer`, and the create/receive paths derive actors from the session rather than request data. The UI/API now sends session tokens, and legacy unscoped handlers are no longer registered. Location and terminal identifiers are validated against the resolved store database. Migration `112_stock_transfer_actor_ids.sql` removes the obsolete local `users` foreign keys so global auth users are not cloned into store databases. Two-store list isolation, permission rejection, actor attribution, IPC contract, and desktop/tablet wiring tests pass.

**Remaining:** None within this audit's scope (store-boundary isolation, lifecycle tests, and cancellation reversal — all done in Phase 2). Note: routing a transfer between two separate store *databases* was never part of the scope and remains unimplemented; transfers move between locations/terminals inside the resolved store.

**Priority:** P0 — tenant isolation and inventory authorization.

---

### INV-02 — Stock-count commands are global and unauthenticated (P0)

**Evidence:** `apps/desktop-client/src/commands/inventory_counts.rs` implements all stock-count commands (`create_stock_count`, `get_stock_count`, `list_stock_counts`, `get_count_lines`, `add_count_line`, `update_count_line`, `remove_count_line`, `complete_stock_count`, `update_stock_count_status`, and `list_stock_adjustments`) against `state.db.lock().await`. None accepts a session token or performs a permission check. `ui/src/api/inventoryCounts.ts` likewise sends no session token.

**Impact:** Stock-count sessions, line contents, and resulting stock adjustments are not protected by the normal store/role boundary. An untrusted IPC caller could create or alter a count, complete it to change inventory, or read adjustment history outside the intended store.

**Recommendation:** Move the complete count lifecycle to session-scoped commands. Resolve the store and actor from the session, enforce inventory-count permissions, validate that products and locations belong to the resolved store, and migrate the UI. Add tests for unauthorized access, cross-store IDs, and completion by a user without adjustment permission.

**Priority:** P0 — direct inventory mutation and tenant-isolation risk.

---

### INV-03 — Inventory actors are accepted from request data instead of derived from the session (P1)

**Evidence:** `create_inventory_transaction` authorizes the session user but accepts a separate `staff_id` argument and passes it to `Store::create_inventory_transaction`. The stock-transfer commands accept client-provided `created_by` and `received_by` values. Stock-count creation accepts `counted_by`, and completion accepts `completed_by`; these values are written to persistence without session binding. The Stock Transfers UI passes `session.user_id`, but that is only a frontend convention.

**Impact:** Even where a command has a valid session, audit history can attribute an operation to another user or to a nonexistent actor. This weakens accountability and can allow a user to make an operation appear to have been performed or approved by an administrator.

**Remediation:** ✅ Fully addressed. Stock-transfer create/receive derive actors from the session; stock-count create/complete derive `counted_by`/`completed_by` from the session; `create_inventory_transaction` passes `&session.user_id` as `staff_id` (verified in `apps/desktop-client/src/commands/inventory.rs`) — no actor field is accepted from request data on any inventory path.

**Priority:** P1 — audit integrity and authorization boundary.

---

### INV-04 — Cancelling an in-transit transfer does not restore source stock despite the UI promise (P1)

**Evidence:** The Inventory DB method `send_transfer` decrements inventory when a transfer becomes `in_transit`. `cancel_transfer` only updates the transfer status to `cancelled`; it does not credit the source inventory or reverse the movement. The Transit Audit UI uses the same cancellation command for “Reverse Transfer,” while `inventory.ftl` tells the operator that “Stock will be returned to the source location.” The stock-transfer test explicitly records that inventory is “NOT restored on cancel (intentional design),” which conflicts with the user-facing copy and the reversal screen semantics.

**Impact:** A warehouse operator who reverses an in-transit transfer can permanently lose the deducted quantity from the source stock. The transfer history says cancelled while inventory no longer reflects the pre-transfer state, creating reconciliation and valuation errors.

**Recommendation:** Choose and document one invariant. For a true reversal, atomically credit the source location (and reverse any transit/destination movements) exactly once, then mark the transfer cancelled. If cancellation is intentionally non-reversing, rename the action and copy to “Cancel transfer,” prevent it after dispatch where appropriate, and provide a separate audited stock-reconciliation workflow. Add lifecycle tests asserting source and destination quantities for full, partial, and repeated cancellation.

**Priority:** P1 — stock ledger correctness.

---

### INV-05 — Stock-count quantities are insufficiently validated and can produce inconsistent adjustments (P1)

**Evidence:** `AddCountLineArgs.expected_qty` and `UpdateCountLineArgs.counted_qty` are accepted as arbitrary `i64` values by `inventory_counts.rs`; the command layer does not reject negative values and the migration has no non-negative checks on `expected_qty` or `counted_qty`. The UI uses `min="0"`, but IPC callers are not constrained by HTML. During completion, a negative counted quantity can produce a negative `adjusted_qty` in the adjustment record while the inventory write clamps the resulting inventory quantity through `.filter(|&v| v >= 0).unwrap_or(0)`.

**Impact:** A malformed or malicious count can record an impossible negative quantity and produce an adjustment audit row that disagrees with the final inventory quantity. The UI constraint is not a security or data-integrity boundary.

**Recommendation:** Validate `expected_qty >= 0` and `counted_qty >= 0` at the command/service and database layers. Reject overflow and impossible quantities with structured validation errors; do not silently clamp. Define whether expected quantity is a trusted snapshot or must be recomputed from the selected location, and store the exact location used by the count.

**Priority:** P1 — inventory and audit consistency.

---

### INV-06 — Count-line mutations do not enforce the count state or ownership boundary (P1)

**Evidence:** `add_count_line`, `update_count_line`, and `remove_count_line` call the store methods directly without checking whether the parent count is `draft` or `in_progress`. The DB methods themselves update or delete by line ID and do not enforce the parent status. The UI hides editing controls for completed counts, but the command boundary does not. `complete_stock_count` reads the count and lines before opening its write transaction, so two callers can race around the status check before either finalizes the count.

**Impact:** A completed or cancelled count can be modified through direct IPC calls, changing the evidence behind a previously applied adjustment. Concurrent completion attempts can both pass the initial status check and create duplicate or conflicting adjustment work unless the final status transition is guarded atomically.

**Recommendation:** Enforce parent status and session ownership/permission in every line mutation. Complete counts with an atomic conditional transition (`WHERE status IN (...)`) inside the same transaction, reject a second completion, and add tests for edits after completion, cancellation, and concurrent completion.

**Priority:** P1 — auditability and workflow correctness.

---

### INV-07 — Inventory-shift behavior contradicts the migration invariant and API contract

**Evidence:** Migration `086_inventory_shifts.sql` defines a unique active-shift index on `(user_id, location_id)` and documents that cross-location active shifts are allowed. `Store::start_inventory_shift` instead counts active shifts by `user_id` alone and rejects any second active shift, even at a different location. Separately, `ui/src/api/inventory.ts` declares `startInventoryShift(sessionToken, userId, locationId, notes)` and sends `userId`, while the desktop command accepts only `session_token`, `location_id`, and `notes` and derives the user from the session.

**Impact:** A worker cannot use the cross-location behavior promised by the schema/ADR. The frontend and command signatures also drift, making the API contract misleading and increasing the chance of broken calls when another client relies on the documented argument shape.

**Recommendation:** Decide whether the invariant is one active shift per user or one per user/location. Align the SQL index, Rust query, UI/API type, and documentation. Prefer removing `userId` from the frontend function and deriving it from the session. Add tests for same-location rejection and the chosen cross-location behavior.

**Priority:** P2 — workflow correctness and contract drift.

---

### INV-08 — Several inventory loading failures are silent or leave ambiguous state (P2)

**Evidence:** `LocationPicker.load` catches errors with `// silently fail` and renders nothing when loading finishes with no locations. `InventoryAdjustmentScreen` reports product-loading failure only through a toast and leaves the product search area otherwise indistinguishable from a usable empty screen. `TransactionLogScreen` has no rendered error/retry state; it only sends a toast on the initial `Promise.all` failure. `StockCountsScreen` and `StockCountDetail` similarly use toast-only failure handling in their primary load paths.

**Impact:** Operators can interpret an IPC/database failure as “no locations,” “no products,” or an empty history and continue with an incomplete inventory view. Toasts are transient and do not provide a durable recovery path.

**Recommendation:** Add persistent localized error states with retry actions to each primary data view. Preserve previously loaded rows during refresh failures, distinguish an empty result from an error, and make a missing location/session an explicit setup state rather than returning `null` silently.

**Priority:** P2 — operational safety and recoverability.

---

### INV-09 — Transit audit performs an avoidable N+1 request pattern (P2)

**Evidence:** `TransitAuditScreen.loadTransfers` first lists all transfers, filters to `in_transit`, and then calls `getStockTransferLines` once for every transfer using `Promise.all`. The stock-transfer API has no batch endpoint for in-transit transfers with lines.

**Impact:** A store with many active transfers causes a burst of IPC/database calls and a slow or fragile audit screen. One failed line request rejects the whole enrichment operation, so a single malformed transfer can hide all other transit records.

**Recommendation:** Add a backend query returning in-transit transfers and their lines in one scoped operation, or add a batch API. Paginate or cap the audit query, preserve successful cards when one line load fails, and add a test for partial enrichment failure.

**Priority:** P2 — performance and failure isolation.

**Status: ✅ REMEDIATED** — commit `45d65511` (Phase 2) + `list_in_transit_transfers_scoped` batch.

- **Core:** `Store::list_transfers_with_lines_by_status(status)` in `crates/oz-core/src/db/stock_transfers.rs` runs two queries (transfers, then one `IN` batch for all lines) and groups lines back onto their transfers — no per-transfer line fetch.
- **Commands:** `list_in_transit_transfers_scoped` added to desktop + tablet `commands/stock_transfers.rs`, registered in both `lib.rs` files, and pinned in `wiring_audit.rs`.
- **API:** `listInTransitTransfers(sessionToken)` in `ui/src/api/stockTransfers.ts`; `TransitAuditScreen` now loads everything in one IPC round-trip with a durable error state + Retry button (`inv-transit-error-load` / `retry` keys in both bundles).
- **Contract test:** `api-stock-transfers-contract.test.ts` pins the command name and the transfer+lines response shape.
- **Fixed en route:** `create_transfer` used `&id[..8]` (the millisecond-timestamp prefix of UUID v7) in `TRF-{ts}-{short}`, so two transfers created in the same millisecond collided on the UNIQUE `transfer_number`. Now uses the random tail `&id[24..]`. Core regression test `list_transfers_with_lines_by_status_batches_lines` covers the batch grouping.
- **Design note:** the batch filters `in_transit` only, matching the legacy screen; `received_partial` transfers continue to be received on StockTransfersScreen, not the transit audit.

---

### INV-10 — Inventory screens still contain localization and theme-compliance drift (P3)

**Evidence:** `LocationPicker` uses hardcoded English labels in `label = 'Location'`, the trigger `aria-label`, and listbox `aria-label`. `StockTransfersScreen` uses hardcoded status/date formatting and a literal `aria-label="Actions"` fallback. `TransactionLogScreen` uses inline hardcoded colors (`#22c55e`, `#ef4444`), inline padding, and literal `'-'`/`'—'` placeholders. `ShiftBar` contains inline presentation styles and hardcoded fallback text/color. Several screens use English fallback strings after `l10n.getString()`.

**Impact:** Non-English users receive mixed-language accessibility labels and dates, while inline colors bypass the token system and can regress in custom themes or dark mode. This also increases the number of CSS-compliance exceptions and makes visual behavior inconsistent across inventory surfaces.

**Recommendation:** Add all labels, placeholders, status names, dates, and empty markers to the Fluent bundles; use a locale-aware date formatter; replace inline colors/padding with tokenized CSS classes; and run bundle-parity, accessibility, and theme-token checks over the entire Inventory surface.

**Priority:** P3 — accessibility, localization, and theming quality.

**Status: ✅ REMEDIATED** — localization + theme cleanup across the four flagged surfaces:

- **`LocationPicker`:** `label = 'Location'`, the trigger `aria-label`, and the listbox `aria-label` are now Fluent-driven via `requiredLocalized` (`loc-picker-label`, `loc-picker-trigger-aria` with `{ $name }`, `loc-picker-listbox-aria`), added to both `inventory.ftl` / `inventory.id.ftl`.
- **`StockTransfersScreen`:** status badges (table + detail) use `localizedStatusLabel()` reading `stock-transfers-status-*` keys (reusing the `RequiredLocalizedL10n` type); added missing `stock-transfers-status-received_partial` to both bundles. The `aria-label="Actions"` was already Fluent-driven via the `stock-transfers-actions` attribute.
- **`TransactionLogScreen`:** inline `#22c55e`/`#ef4444` replaced with tokenized `.log-qty-positive`/`.log-qty-negative` CSS classes; inline padding moved to `.log-detail-btn`; type badges now resolve `inv-log-type-*` keys (added `inv-log-type-purchase-order-receive` alias to both bundles for the DB `purchase-order-receive` value).
- **`ShiftBar`:** `textTransform` inline style moved to `.summary-item-type`; `#ef4444` border moved to `.summary-item-empty` using `var(--color-danger)`; type badges localized via `inv-log-type-*` keys.
- All new keys exist in both en + id bundles (bundle-parity clean); 6 affected UI test files pass (56/56).

---

### INV-11 — Stock-count number allocation is vulnerable to concurrent duplicate generation (P2)

**Evidence:** `create_stock_count` calls `Store::next_count_number()` before `Store::create_stock_count()`. `next_count_number()` selects `MAX(...) + 1`, while the insert occurs afterward and outside one transaction. The schema correctly has a unique constraint on `count_number`, but there is no reservation or retry around the read-then-insert sequence.

**Impact:** Two simultaneous count creations can compute the same next number. One request will fail with a uniqueness error, and the UI reports a generic creation failure rather than retrying or explaining the contention. This is especially likely when multiple terminals start counts at the same time.

**Recommendation:** Allocate the number atomically with a transaction-backed counter or insert-and-retry on the unique constraint. Keep the unique index as the final guard and add a concurrency test with multiple creators.

**Priority:** P2 — multi-terminal workflow reliability.

## Phase 1 remediation evidence

- Desktop and tablet register the same nine `*_stock_transfer_scoped` commands; wiring tests assert scoped commands are present and legacy unscoped handlers are absent.
- `ui/src/api/stockTransfers.ts` and both transfer screens pass `sessionToken`; API contract tests assert exact IPC command names and payloads.
- `apps/desktop-client` tests prove actor derivation, two-store list isolation, denial of a cashier without `inventory:transfer`, and that no local auth user is manufactured in the store database.
- Migration `112_stock_transfer_actor_ids.sql` rebuilds `stock_transfers` without local `users` foreign keys while preserving transfer rows, location/terminal foreign keys, indexes, and the `stock_transfer_lines` relationship.
- Validation completed for Phase 1: desktop transfer tests (7), tablet transfer tests (4), migration tests (22), wiring tests (6), and focused UI typecheck/tests (29 tests across 3 files).

## Positive observations

- Location CRUD commands resolve `session_token` to a store and check `SALES_PROCESS` before accessing location data.
- Workspace location replacement and manual inventory transaction creation use database transactions.
- The stock-transfer send and receive workflows use transactions, check source availability, reject over-receipt, and record partial receipt status.
- Location deactivation checks active stock and in-flight transfers before marking a location inactive.
- Inventory shifts have a database-level active-shift uniqueness index, and shift start/end operations are transactional.
- Stock alert reads and acknowledgements are session-scoped, location-scoped, and include an optimistic local removal after acknowledgement.
- The UI provides loading skeletons or loading states, empty states, filters, retry UI for the main Stock Transfers screen, confirmation dialogs for threshold deletion and transit reversal, and focus traps for Stock Transfers modals.
- The focused suites cover normal and many negative lifecycle paths: transfer over-receipt, invalid status transitions, stock insufficiency, stock-count completion, and API IPC argument shapes.
- The inspected Inventory CSS predominantly uses design tokens and visible focus styles; the main theme drift is concentrated in inline styles and hardcoded fallbacks in secondary screens.

## Recommended implementation order (all complete)

1. **INV-02 and remaining INV-03:** ✅ Session-scoped stock-count + inventory-transaction actor paths (migration 113).
2. **INV-04/INV-05/INV-06:** ✅ Transfer reversal semantics + quantity/status invariants at the core transaction boundary.
3. **INV-07/INV-11:** ✅ Shift contract aligned per user+location; count-number allocation concurrency-safe (`BEGIN IMMEDIATE`).
4. **INV-08:** ✅ Durable error/retry states on StockCounts, History, Detail, and TransitAudit.
5. **INV-09:** ✅ Transit N+1 replaced by the scoped batch API.
6. **INV-10:** ✅ Fluent coverage + CSS token cleanup across the four flagged surfaces.

## Validation

### Phase 1 (transfer boundary)

- Desktop transfer tests (7), tablet transfer tests (4), migration tests (22), wiring tests (6), focused UI tests (29 across 3 files).

### Phase 2 (stock counts, lifecycle, concurrency)

- `cargo test -p oz-core --lib db::stock_counts`: **20 passed**
- `cargo test -p oz-core --lib db::stock_transfers`: **25 passed** (pre-INV-09)
- `cargo test -p oz-core --lib db::inventory`: **34 passed**
- `cargo test -p oz-core --test stock_count_integration`: **14 passed**
- UI typecheck clean; focused stock-count/transfer UI suites green.

### INV-09/INV-10 batch (commit `5be6de69`)

- `cargo test -p oz-core --lib db::stock_transfers`: **27 passed** (incl. batch-grouping test)
- `cargo test -p oz-pos-app --lib commands::stock_transfers`: **7 passed** · tablet: **4 passed** · wiring audit: **6 passed**
- UI typecheck + lint clean; inventory UI suites **122/122** (56 + 66 across 11 files)
- Code review (deepseek-flash): no blockers

## Status

✅ **FULLY REMEDIATED.** All 11 findings (INV-01 → INV-11) are implemented and validated locally:

- **INV-01/02/03 (session boundary):** all stock-transfer and stock-count commands are `*_scoped` on desktop + tablet with `resolve_scope` (ADR #7), permission checks, and server-side actor derivation; migration `113_stock_count_actor_ids.sql` drops local auth FKs. (Phases 1–2, commits `a2c70848`, `45d65511`.)
- **INV-04/05/06 (data integrity):** `cancel_transfer` reverses source inventory atomically for `in_transit` (received/partial rejected); receive lifecycle allows `in_transit` + `received_partial` continuation with delta-only crediting; quantity/status invariants enforced at the core transaction boundary; count completion claims status conditionally. (Phase 2, commit `45d65511`.)
- **INV-07/11 (concurrency):** shift contract aligned to per user+location; count-number allocation uses `BEGIN IMMEDIATE` + conditional claim with rollback hardening. (Phase 2.)
- **INV-08/09 (resilience/performance):** durable error + retry states on StockCounts/History/Detail/TransitAudit; transit N+1 replaced by the scoped batch API. (Phase 2 + this batch.)
- **INV-10 (i18n/theme):** Fluent coverage + tokenized CSS across LocationPicker, StockTransfersScreen, TransactionLogScreen, ShiftBar. (This batch.)

### Validation (this batch)

- `cargo test -p oz-core --lib db::stock_transfers`: **27 passed** (incl. the new batch-grouping test)
- UI typecheck: clean · UI lint: clean
- Focused UI tests (LocationPicker, TransactionLog, ShiftBar, StockTransfersScreen, TransitAuditScreen, api-stock-transfers-contract): **56/56 passed**
- Code review (deepseek-flash): no blockers

**Conscious retention:** `get_stock_transfer_lines_scoped` / `getStockTransferLines` now has no UI consumer (TransitAuditScreen was its only caller) but remains a registered, contract-tested public IPC surface — kept intentionally for direct line fetches from future screens.
