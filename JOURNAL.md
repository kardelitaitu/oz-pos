<!-- Audit stamp: 2026-07-29 · Codebuff · status: UPDATED — July 29 full-codebase i18n audit session appended -->

# OZ-POS Development Journal

## 2026-08-06 — TDD cycle: checked BOM deduction quantities (MONEY-03)

### `complete_sale*` BOM ingredient totals overflow silently
**Problem:** Both stock-deduction entry points multiplied the sale-line qty by the recipe's `quantity_required` with a bare `line.qty * ingredient.quantity_required` — `complete_sale_deduction` (line ~247) and `complete_sale_with_resolved_shortfalls` (non-resolution BOM branch, line ~644). `line.qty` comes from the front-end sale over IPC (untrusted) and dev/test builds disable overflow checks, so an overflowing qty silently wrapped: the Red run showed both paths returning `Ok(CompleteSaleResult)` while the ingredient stock was **credited** by ~4.6e18 — the register completed a sale with a corrupt stock delta instead of failing.

**Solution:** Red→Green TDD cycle. RED tests `complete_sale_deduction_bom_quantity_overflow_returns_validation_error` and `complete_sale_with_resolved_shortfalls_bom_quantity_overflow_returns_validation_error` pin the contract — `(i64::MAX / 2) × 3` overflows, and both paths must return `CoreError::Validation { field: "qty", message: "ingredient deduction quantity overflow" }` with stock untouched. Both failed on `Ok(CompleteSaleResult …)` (the silent wrap) before the fix. GREEN: both sites now use `checked_mul(...).ok_or_else(Validation { field: "qty", … })?` — the same pattern as `compute_line_tax` (TAX-04) and MONEY-01. `quantity_required` is DB-backed with a `CHECK (quantity_required > 0)` so that operand needs no validation. Field `"qty"` deliberately avoids `"stock"`, which the caller special-cases to deserialize `PartialStockResult`.

**Refactor:** extracted `seed_bom_composite` test helper (composite `service` product + tracked ingredient + recipe row) per review; 79/79 sales-module tests, 1623/1623 oz-core lib.

**Deliberately NOT done (follow-ups):** (1) negative `line.qty` on a hand-built `Sale` remains unchecked on this path — `checked_mul` rejects only overflow, and a negative qty would *credit* stock (same MONEY-02 gap class); unreachable from the front-end since `CartLine::new` asserts `qty > 0` and `Sale::from_cart` is the only real producer, but worth a validation slice; (2) `oz-lua` plugin `apply_discount` (lib.rs 577/608) and purchase-order subtotals still use unchecked `qty × price` — same class, separate slices.

## 2026-08-06 — TDD cycle: reject negative cart-tax inputs (MONEY-02)

### `compute_cart_tax` negative `qty` / `unit_price_minor` accepted
**Problem:** Follow-up from the MONEY-01 cycle's review note. `CartLineTaxInput` arrives over IPC (untrusted), and `Store::compute_cart_tax` accepted negative `qty` or `unit_price_minor` — a negative line total flows into `compute_line_tax` and the preview returns a **negative tax** the front-end renders raw (Red run proved it: `qty: -2, price: 350` → `Ok(tax = -69)`). The cart model never allows negative qty/price (`CartLine::new` asserts `qty > 0`), so a hostile renderer could distort the displayed tax.

**Solution:** Red→Green TDD cycle. RED test `compute_cart_tax_rejects_negative_qty_and_price` asserts both cases return `CoreError::Validation` with the right field (`qty` / `price`) — failed on `Ok(-69)`. GREEN: the loop now rejects `qty < 0` (`field: "qty"`, "qty must be positive, got {n}") and `unit_price_minor < 0` (`field: "price"`, "unit price must be non-negative, got {n}") with early returns, mirroring the existing `set_cart_discount` message style. `qty == 0` and `unit_price_minor == 0` remain **allowed by deliberate scope**: zero contributes zero tax, zero price = free item, and the slice was "negative" only — noted here so the boundary is explicit.

**Deliberately NOT done (follow-ups):** (1) `compute_sale_tax` has the same-class hole via a hand-built `Sale` with a negative `line_total` (it feeds `line.line_total` straight into `compute_line_tax`) — the natural next slice; (2) reviewer nit: the price `format!` could sit on one line — skipped as cosmetic churn in a volatile shared tree (code is fmt-clean and committed); (3) the pre-existing `clippy::type_complexity` in `products.rs:876` remains untouched.

**Commits:** (this cycle) — `crates/oz-core/src/db/sales.rs` + this journal + `CHANGELOG.md`. **Note on history:** another agent thread's `06e9fb7d` ("fix(restaurant): harden menu keyboard interactions") swept this cycle's `sales.rs` RED test + GREEN fix into its commit via a broad add. History was NOT rewritten (shared working tree); the `sales.rs` hunks are exactly this cycle's regression test + negative-input validation.

**Validation:** Red test failed (`Ok(-69)`) then passed; `db::sales::tests` 77/77 (76 + 1 new); full `cargo test -p oz-core --lib` 1621/1621 (includes the new test); `cargo fmt --all` clean; clippy `-D warnings` clean on the changed file (workspace still fails only on the pre-existing `products.rs:876`). One transient compile failure was observed mid-cycle from another agent's in-progress `db/offline.rs` edit — resolved on its own; no process was killed.

## 2026-08-06 — TDD cycle: dead-letter requeue workflow

### Dead-lettered remote items are now requeueable (audit/09 SYNC-01 follow-up)
**Problem:** Remote items that exhaust their apply retry budget were permanently quarantined. `sync_remote_failures` rows are only ever written (on failure) or deleted (on success) — once `dead_lettered = 1`, `apply_remote_atomic` skips the item and the daemon advances the pull anchor past it, so it is never retried. Migration 119's comment promised "an operator can inspect or manually requeue a quarantined item after remediation", but no store method, command, or UI existed (the workflow was explicitly deferred in audit/09 SYNC-08). An operator who fixed the source (e.g. created the missing product a remote sale referenced) had no way to make the item retry.

**Solution:** Red→Green TDD cycle. RED: store tests `requeue_remote_failure_clears_quarantine_and_rewinds_anchor` + `requeue_remote_failure_refuses_non_dead_lettered` (failed with `no method named requeue_remote_failure`). GREEN: `Store::requeue_remote_failure(item_id)` (oz-core `db/offline.rs`) — requires the item to be currently dead-lettered (else `NotFound`, never a silent no-op), deletes the quarantine row, and rewinds the durable `sync_pull_state` anchor (`since = NULL, cursor = NULL`) so the next daemon cycle re-fetches the item and retries it with a fresh 3-attempt budget. The full re-pull is safe because the `sync_applied_items` idempotency ledger skips every already-applied item. Command surface: `requeue_remote_failure` Tauri command (`RequeueRemoteFailureArgs { itemId }`) added to BOTH desktop (`oz-pos-app`) and tablet (`oz-pos-tablet`) `commands/offline.rs` + registered in both `lib.rs` invoke handlers; extracted `run_requeue_remote_failure` helper for command-level tests.

**Commits:** code swept into `06e9fb7d` (authored by another agent thread — see note). Docs in this cycle's follow-up commit.

**Validation:** oz-core `db::offline` 44/44; desktop `commands::offline` 18/18; tablet `commands::offline` 18/18; `cargo fmt` clean; clippy `--no-deps -D warnings` clean on desktop + tablet and no warnings in oz-core's new code (workspace clippy still fails only on the pre-existing `products.rs:876` type_complexity).

**Note on history:** commit `06e9fb7d` (restaurant agent's "harden menu keyboard interactions") swept this cycle's five files (`oz-core db/offline.rs`, desktop + tablet `commands/offline.rs` + `lib.rs`) into its diff via the shared index. The requeue code is intact and was verified green on identical content; history was NOT rewritten (shared working tree, agents actively committing).

**Known limitation (reviewed):** the requeue anchor rewind can be clobbered by the sync daemon if the command lands between the daemon's read phase and its apply-phase `set_sync_pull_state` write (the daemon re-writes the stale pre-rewind anchor it captured). Low probability (daemon cycle is 60–120s, requeue is a rare operator action), no data corruption — the requeue just doesn't take effect that cycle. Fix (separate TDD slice): in the daemon's apply phase, re-read `get_sync_pull_state()` before writing and skip the anchor advance when the stored `since` is `None` (operator rewind in flight).

**Follow-ups:** expose `list_remote_failures` as a command + UI surface so operators can discover dead-letter ids before requeueing; wire `requeue_remote_failure` into `ui/src/api/offline.ts` (+ IPC contract test) to make the workflow end-to-end; consider storing the remote item's `created_at` on the failure row so requeue can rewind the anchor precisely instead of a full re-pull.

## 2026-08-06 — TDD cycle: checked cart-tax line totals (MONEY-01)

### `compute_cart_tax` unchecked `qty × unit_price_minor` overflow
**Problem:** `Store::compute_cart_tax` (`crates/oz-core/src/db/sales.rs`) computed the per-line taxable total with a bare `line.qty * line.unit_price_minor`. `CartLineTaxInput` is deserialised straight off the IPC boundary (untrusted renderer input) and this function runs on the hot path — the live tax preview fires on every cart change in both desktop and tablet POS. The workspace deliberately sets `overflow-checks = false` for dev/test builds (`Cargo.toml` `[profile.dev]`), so an overflowing line total **silently wraps** and feeds a wrong tax to the register in every normal build; it would panic only under a profile with overflow checks on. This is the exact arithmetic class TAX-04 already eliminated in `compute_line_tax` (checked_mul + structured error) — it was missed at the line-total product. Red test `compute_cart_tax_line_total_overflow_returns_validation_error` (qty = i64::MAX/2, price = 4) failed for the right reason: `Ok(Money { minor_units: 0 })` — the wrapped tax — instead of an overflow error.

**Solution:** Red→Green TDD cycle. GREEN: the line total now uses `qty.checked_mul(unit_price_minor)` and returns `CoreError::Validation { field: "tax", message: "cart line total overflow" }` on overflow — the same structured error contract as `compute_line_tax`. No signature change, so no caller updates (`compute_cart_tax_scoped` in desktop + tablet `pos.rs` forward unchanged).

**Deliberately NOT done (follow-ups):** (1) same-class unchecked `qty × price` products remain in `crates/oz-lua/src/lib.rs:577/608` (plugin `apply_discount` line math — plugin-supplied values), `crates/oz-core/src/db/purchase_orders.rs:186/214` (PO subtotals), `crates/oz-core/src/db/sales.rs:247/644` (recipe BOM `line.qty * quantity_required`), and `modules/inventory/src/handlers.rs:141` — each a separate TDD slice; (2) the broader sale-to-ledger totals policy (recorded `sale.total` excludes tax / tip / service-charge that the UI charges separately; tax computed on pre-discount line totals) is a product-policy question (inclusive vs exclusive tax) and deliberately NOT changed here; (3) pre-existing `clippy::type_complexity` in `crates/oz-core/src/db/products.rs:876` remains (documented in the 08-06 session-mint entry) — unrelated to this change; (4) `CartLineTaxInput` still accepts non-positive `qty`/`unit_price_minor` (negative line total → negative tax preview) — a semantic-validation slice distinct from overflow, noted by review.

**Commits:** (this cycle) — `crates/oz-core/src/db/sales.rs` + this journal + `CHANGELOG.md`. **Note on history:** another agent thread's commit `42dab989` ("fix(authz): pin inactive-user session denial…") swept this cycle's `sales.rs` RED test + GREEN fix (12 lines) into its authz commit via a broad `git add` while the tree moved. History was NOT rewritten (shared working tree); the `sales.rs` hunks are exactly this cycle's regression test + checked-mul fix, and the code is byte-identical to what this cycle produced and verified.

**Validation:** Red test failed (silent wrap) then passed; `db::sales::tests` 76/76; full `cargo test -p oz-core --lib` 1618/1618; `cargo fmt --all` clean; clippy `-D warnings` clean on the changed file (workspace clippy still fails only on the pre-existing `products.rs:876`). `scripts/test-changed.sh` was blocked by a running `oz-pos-app.exe` holding the linker output lock — process left running per TDD skill rule 7; equivalent coverage obtained via the direct `oz-core --lib` run.

## 2026-08-06 — TDD cycle: session-mint authorization gate (right user, right store, right permission)

### `verify_instance_access` fail-closed identity binding (audit/06 residual)
**Problem:** The pre-session workspace picker ends in `create_session`, whose server-side gate `Store::verify_instance_access` trusted the caller-supplied `role_id` for the owner/manager bypass and never resolved the user. `create_session(user_id: <any known id>, role_id: "role-owner", store_id: ..., instance_id: ...)` passed the bypass whenever no `user_store_access` rows existed (single-store mode), minting an opaque session AS that user — without their PIN — in ANY store's active instance. Every subsequent `require_permission_for_user` then resolved the victim's DB role, so a caller who knew an owner's user id inherited full permissions (privilege escalation) and could open sessions in stores/instances they were never assigned (cross-store session minting). This was the residual recorded in `audit/06-staff-module.md`: "the pre-session workspace picker still accepts role/user/store identifiers supplied by the client… the caller identity is not cryptographically bound before an opaque session exists." The gate had zero unit tests.

**Solution:** Red→Green TDD cycle. RED: 3 oz-core tests (`verify_instance_access_denies_unknown_user`, `_rejects_forged_owner_role`, `_denies_inactive_user`) + 3 desktop command tests (`create_session_rejects_forged_role_id`, `_rejects_unknown_user`, plus positive `create_session_allows_real_owner`). All negative tests failed for the right reason (session was minted). GREEN: `verify_instance_access` now resolves the user from `users`, fails closed (returns `Ok(false)`) for unknown/inactive users and for a claimed `role_id` that differs from the user's actual DB role, then runs the existing owner-bypass / explicit-assignment / role-based branches using the REAL role. `Ok(false)` (not `Err`) keeps the caller's wire error uniform, so the gate cannot be used to enumerate user ids. No frontend change needed: every honest flow (login, workspace picker, FastPIN hot-swap) sends the role returned by `staff_login`, which equals the DB role.

**Deliberately NOT done (follow-ups):** (1) the pre-session `list_workspaces`/`list_workspace_screens` reads still trust the claimed role for listing (workspace-name disclosure only, no data access) — a server-issued picker credential remains the architectural fix per audit/06; (2) `create_session` does not cross-check `type_key` against the instance's real type (UI-routing cosmetic); (3) pre-existing `clippy::type_complexity` in `crates/oz-core/src/db/products.rs:876` remains (documented in the 08-06 pull-parity entry).

**Commits:** gate + desktop tests were swept into `da842f32` (another thread's mixed commit, same as the sync hunks — history not rewritten per shared-tree convention); this cycle's follow-up `42dab989` pins the inactive-user test uniquely and adds tablet `create_session` parity tests. The `.githooks/pre-commit` fmt gate swept a third file — `crates/oz-core/src/db/sales.rs` (another thread's uncommitted work) — into `42dab989`; its content is intact in the commit and the working tree matches HEAD, splitting left to the owner if desired.

**Validation:** `cargo test -p oz-core --lib db::workspaces` 54/54 (6 new); `cargo test -p oz-pos-app --lib commands::auth` 18/18 (3 new); `cargo test -p oz-pos-tablet --lib commands::auth` 9/9; `store_scoping_integration` 9/9; `cargo fmt --all -- --check` clean; clippy `-D warnings` clean on the changed files (workspaces.rs, auth.rs); tablet lib compiles.

## 2026-08-06 — TDD cycle: engine pull parity for replay idempotency + durable anchor

### SyncEngine pull path: durable anchor + atomic replay (SYNC-01 parity)
**Problem:** `platform_sync::SyncEngine::run_sync_cycle()` — the immediate/manual sync path — did not share the SYNC-01 safeguards the daemon got. It derived its pull `since` anchor from `queue.last_synced_at()` (the local offline queue's `synced_at` timestamps), which pulled remote items never move, and applied remote mutations via the non-atomic `apply_remote()`. Consequence: every manual sync cycle re-fetched the same remote pages and re-applied stock/sale mutations (silent inventory corruption), and the durable `sync_pull_state` anchor was never persisted. The daemon path (fixed in `a1ea01e7`) was atomic + anchor-advanced; the engine was not.

**Solution:** Red→Green TDD cycle. RED test `engine_applies_replayed_remote_item_only_once` (in `platform/sync/src/lib.rs`) spins a mock server that always returns the same `stock.adjusted` +10 item (ignores `since`), runs two engine cycles, and asserts: stock 50→60 after cycle 1, the durable anchor is persisted, and stock stays 60 after cycle 2 (not 70) with exactly one ledger receipt. It failed for the right reason (`since: None`). GREEN: the pull phase now reads the durable `sync_pull_state` anchor, applies each item via `apply_remote_atomic` (mutation + idempotency receipt in one transaction, dead-letter quarantine for poison items — matching the daemon), advances the anchor only after a page applied successfully, and retains the anchor + stops pagination on a retryable failure.

**Commits:** swept into `da842f32` (see note below) — my `platform/sync/src/lib.rs` hunks only.

**Validation:** `bash scripts/test-tdd.sh -p platform/sync` — 238/238 passed (19 slow-tests ignored); full `--features slow-tests` integration suite — 19/19 passed incl. cross-terminal relay + throughput; `cargo clippy -p platform-sync --all-targets --no-deps -- -D warnings` clean; `cargo fmt` applied. Note: `cargo clippy -D warnings` on the workspace currently fails pre-existing in `crates/oz-core/src/db/products.rs:876` (`type_complexity`, committed code, not touched here).

**Note on history:** commit `da842f32` (authored by another agent thread) swept this lib.rs change — plus 16 unrelated files (UI autofill, auth, workspaces) — into one commit titled "fix(ui): suppress saved-info autofill in search fields". The lib.rs hunks are exactly this cycle's RED test + GREEN refactor. History was NOT rewritten (shared working tree, another agent actively editing `sales.rs`); splitting the mixed commit is left to the owner if desired.

## 2026-08-06 — TDD cycle: LOY-10 loyalty expand-control accessible name

### LoyaltyManagementScreen expand control (LOY-10)
**Problem:** The expandable loyalty account row (`tr role="button"`) and its nested expand button exposed a generic `aria-label` ("Expand"/"Collapse") with no customer identity. Screen-reader users could not tell which account a control would expand. The nested button had no handler and relied on click bubbling to the row. Evidence: `audit/02-loyalty-module.md` LOY-10 (P2, still open — verified in code at `ui/src/features/loyalty/LoyaltyManagementScreen.tsx`).

**Solution:** Red→Green TDD cycle per `.agents/skills/tdd/SKILL.md`. RED test `names the expand control with the customer (LOY-10)` asserted the row + button accessible names include the customer name — failed on `'Expand'`. GREEN: added `loyalty-expand-account`/`loyalty-collapse-account` Fluent keys with `{ $name }` var (en + id), threaded the customer name through both controls, and gave the nested button a real `onClick` handler (`toggleExpand`) instead of relying on bubbling.

**Commits:** (this cycle) — see `git log`.

**Validation:** LoyaltyManagementScreen 20/20 vitest; api-loyalty-contract 5/5; typecheck clean; eslint clean on changed files; i18n lint clean; bundle-parity 0 missing; FTL dedupe clean. Area-scoped per tdd skill (no full workspace run — not pre-push).

### Attribute-Only FTL Sweep (TODO #3)
**Problem:** ~268 attribute-only FTL messages (`.aria-label = ...` with no message value) silently returned `undefined` when accessed via `l10n.getString()`, causing empty aria-labels and placeholders across 25 files.

**Solution:** Cross-referenced all 1212 `l10n.getString()` calls against the 268 attribute-only keys. Found 75 keys used without fallbacks across 25 files. Verified `<Localized>` usage: 72 keys safe to convert to simple `key = value` format (125 conversions, 16 bundles via `scripts/convert-safe-attr-ftl.py`). 3 keys shared with `<Localized attrs>` received `||` fallbacks in code.

**Commits:** `104c4891`, `ee5a4f96`

### RestaurantMenu.tsx Audit (TODO #2)
**Problem:** 795-line restaurant/KDS screen was completely un-audited, with 11 missing FTL keys and 2 hardcoded English strings (`aria-label="Menu items"`, hex color codes as aria-labels).

**Solution:** Added 13 FTL keys (en + id): search-aria, search-clear-aria, context-pin/unpin, context-available/unavailable, card-pin-title, sort-manual/a-z/date/popularity, menu-items-aria, color-swatch-aria. Localized the grid aria-label and color swatch labels. CSS audit: 0 hardcoded hex, all tokens. Hooks: all cleanup + deps correct.

**Commits:** `b3307810`, `446a88f3`

### SettingsPage.tsx Audit (TODO #1)
**Problem:** Largest UI file (1081 lines) was surprisingly clean — 244 CSS tokens, zero hardcoded hex, correct hook deps. Only 2 hardcoded strings: `placeholder="Search"` and Suspense fallback `Loading...`.

**Solution:** Added `settings-search-placeholder` and `settings-section-loading` FTL keys (en + id). Localized both strings with `l10n.getString()` and `<Localized>`.

**Commits:** `533247bc`, `de1517dc`

### PosScreen.tsx Audit
**Problem:** Largest file in codebase (2212 lines TSX + 682 CSS). 26 attribute-only bugs already fixed by the FTL sweep. After sweep: 216 CSS tokens (all hex in var() fallbacks), 40 hooks with correct deps, ESLint zero errors.

**Bugs found:** 5 hardcoded strings missed by the sweep:
- Course fire button: `aria-label={`Fire ${course.label} (${holdCount} items)`}` — not inside `<Localized>`
- Fire All button: `<span>Fire All</span>` — not wrapped in `<Localized>`
- Override button in CartLineItem: `aria-label={...}` and `Override` text — both hardcoded
- Missing FTL keys: `pos-cart-course-fire-aria`, `pos-cart-course-btn--all`, `pos-cart-line-override`, `pos-cart-line-override-aria`

**Commit:** `0796d835`

### ProductManagementScreen + CategoryManagementScreen Audit
**Problem:** Two ~640-line screens flagged in the original audit for hardcoded aria-labels. Both were clean after the attribute-only sweep: 92+135 CSS tokens, zero true hardcoded hex.

**Bugs found:** 3 hardcoded strings:
- CategoryManagementScreen: `aria-label={`Edit category ${cat.name}`}` — not inside `<Localized>`
- ProductManagementScreen: Stock alert bell aria-label in English
- ProductManagementScreen: Product type dropdown options (Retail/Restaurant/Service) — not localized

**Commit:** `13023004`

### Session Totals
| Metric | Count |
|--------|-------|
| Bugs fixed | **88** |
| FTL keys added | **28** (en + id) |
| Files changed | **25** |
| Commits | **5** fix + **4** docs |
| Tests | **3324/3324 passing, 221/221 files** |
| TypeScript | Clean (0 errors) |
| Bundle parity | 0 missing keys |


## 2026-07-02 — i18n Migration & Test Fixes

### Test Infrastructure Fixes
- **SettingsPage.test.tsx**: Wrapped with `AuthProvider` context + added `get_brand_settings` mock to fix pre-existing failures.
- **SetupWizard.test.tsx**: Corrected Launch button test to use `onLaunch` prop instead of `onSkip`.
- **CSS Extraction Tests**: Removed duplicate/dead CSS classes in `CartPanelActions.css`, added `url()` stripping in `extractClassSelectors` to fix `w3` false positive, added `externalClasses` support.
- **WorkspaceEntry.test.tsx**: Fixed unused `screen` import and `registerNavItem` import path (was pointing to `page-registry` instead of `menu-registry`).
- **Fluent missing-ID warnings**: Added 15 missing `setup-feature-*-label` IDs to `settings.ftl`.

### i18n Migration — Wrapped hardcoded aria-labels with `<Localized attrs>`

| Component | Labels wrapped |
|-----------|---------------|
| **SalesHistoryScreen.tsx** | 16 — date from/to, cashier select, table, actions th, pagination nav/prev/next/per-page, void overlay/close/reason, detail overlay/close/lines/refund-lines |
| **VoidOrdersScreen.tsx** | 3 — search input, status filter radiogroup, custom reason input |
| **PaymentModal.tsx** | 17 — dialog overlay, close button, currency label/select, exchange notice, receipt currency, other-input, customer-name (was fully hardcoded), tendered-input, quick-tender (with vars), exact button, QRIS button, split-evenly, split-add, split-other, split-amount, split-remove |
| **TaxConfigurationScreen.tsx** | 9 — tax rates table, category tax rates table, tax name label, edit/delete/cat-edit buttons, tax-rate modal, tax-type radiogroup, category-tax modal |
| **CustomerManagementScreen.tsx** | 5 — customers table, name/email/phone/notes inputs |
| **LoyaltyManagementScreen.tsx** | 8 — accounts table, actions th, transactions table, 5 tier form inputs |

### FTL Files Modified
- `sales.ftl` — added 21 new IDs for sales history + void orders + payment modal
- `settings.ftl` — added 15 setup-feature-label IDs
- `tax.ftl` — added 3 new IDs (table-aria, cat-table-aria, field-name-aria)
- `customers.ftl` — added 5 new IDs (table-aria, 4 field aria)
- `loyalty.ftl` — modified `loyalty-table-actions` to `.aria-label` format + added 7 new tier/table IDs

## 2026-07-02 — White-Label Theming Improvements

### Changes Made

1. **BrandContext created** (`ui/src/contexts/BrandContext.tsx`) — New React context providing brand/white-label settings and a `refreshBrandSettings()` function to the entire app tree. Loads from backend on mount.

2. **ThemeProvider cleaned up** — Removed `BrandInfo` interface, `brand`/`updateBrand` state (now handled by BrandContext), and the direct `getBrandSettings` effect. Now uses `useBrand()` from BrandContext to reactively apply the accent palette whenever `primary_colour` changes.

3. **AppLayout sidebar header** — Replaced hardcoded "OZ-POS" with dynamic brand logo (if set) + store name (fallback to "OZ-POS"). Also sets `document.title` reactively to the store name.

4. **AppearanceSettings** — Replaced `useTheme().updateBrand` with `useBrand().refreshBrandSettings()`. `handlePickLogo` now also refreshes brand settings immediately so the sidebar shows the new logo without waiting for "Save".

5. **AppLayout.css** — Added `.app-sidebar-logo-img` (32×32, object-fit contain) and collapsed variant (28×28) styles.

6. **App.tsx** — Wrapped app with `<BrandProvider>` above `<ThemeProvider>`.

### TypeScript
Clean (0 errors).

## 2026-07-02 — Modal Exit Animations

**Problem:** Hold cart, held carts, and shift modals had entrance animations but snapped out on close — no exit animation.

**Solution:** Created reusable `useAnimatedModal` hook that manages entering/exiting phases. When `show` becomes `false`, the modal stays mounted for 200ms with `exiting=true` before unmounting, allowing CSS exit animations to play.

**Changes made:**
- NEW `ui/src/hooks/useAnimatedModal.ts` — animation phase management hook
- `PosScreen.css` — added `@keyframes pos-overlay-out` (fade) + `pos-modal-out` (fade+translate), `.pos-overlay-exit`/`.pos-modal-exit` classes
- `ShiftManagementScreen.css` — added identical shift-prefixed exit keyframes + classes
- `PosScreen.tsx` — applied hook to 5 modals (hold cart, held carts, close shift, shift summary, open shift)
- `ShiftManagementScreen.tsx` — applied hook to 5 modals (open, payout, close, closed summary, detail)
- Reduced-motion overrides extended to cover exit classes

**Null-safety:** Used IIFE pattern (`{mX && (() => { const s = nullable!; return ( ... ); })()}`) where hook conditions couldn't be tracked across the hook boundary.

### Bugs Fixed During Migration
- Nested `<label>` bug in PaymentModal currency selector (invalid HTML)
- `key` prop on quick-tender buttons moved to outermost `<Localized>` component
- Stale `l10n.getString()` call on loyalty `<th>` after converting ftl to attribute format
- Missing `</Localized>` closing tags for void and detail overlay wrappers

### Test Results
- **TypeScript**: Clean (0 errors)
- **Tests**: 261 passed / 15 failed (down from 31 failing pre-migration — all remaining failures are pre-existing FSI/PDI marker issues and structural WorkspaceEntry module-not-found)
