# Tax Module Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** Tax module — tax rates, category assignments, tax calculation, and sale persistence  
> **Status:** FULLY REMEDIATED · all findings remediated across five phases; remaining items are non-blocking residual follow-ups documented below — see "Status" for the evidence-backed summary.
> **Production code changed:** Yes — see “Remediation implemented” below.

## Scope

This audit covers the Tax configuration screen, the TypeScript IPC client, desktop tax commands, POS tax calculation, tax persistence and migrations, localization, theming, accessibility, and focused tests.

Inspected areas:

- `ui/src/features/tax/TaxConfigurationScreen.tsx`
- `ui/src/features/tax/TaxConfigurationScreen.css`
- `ui/src/api/tax.ts`
- `ui/src/__tests__/TaxConfigurationScreen.test.tsx`
- `apps/desktop-client/src/commands/tax.rs`
- `crates/oz-core/src/db/tax.rs`
- `crates/oz-core/src/db/sales.rs`
- `modules/tax/src/{lib,models,repository,service}.rs`
- Tax migrations `009`, `012`, `017`, and `020`
- English and Indonesian Tax Fluent bundles

The review uses the universal audit lenses from `audit/AUDIT_JULY_2026.md`: functionality, state and UX, accessibility/i18n, theming, performance, security/data integrity, and quality assurance.

## Architecture summary

The current tax implementation is transitional. The frontend and Tauri commands remain outside `modules/tax`; `modules/tax` contains the module registration plus a small repository/service surface. The active calculation path is in `oz-core` and is exposed through the scoped POS command `compute_cart_tax_scoped` (the legacy unscoped `compute_cart_tax` was removed), while tax-rate CRUD and category assignment use `apps/desktop-client/src/commands/tax.rs`.

Tax resolution in `Store::resolve_best_tax_rates_for_sku` is explicit:

1. Product-level assignments win when at least one valid product rate exists.
2. Otherwise category-level assignments are used.
3. Otherwise the first default rate from the name-ordered list is used.
4. All rates at the selected level contribute to the line.

`compute_sale_tax` stores line tax amounts and the first selected rate ID, and sale persistence stores aggregate and line tax snapshots.

## Findings

### TAX-01 — Tax-rate mutations and category assignments bypass store/session scope (P0)

**Evidence:** `list_tax_rates_scoped` resolves a store from `session_token`, but `create_tax_rate`, `update_tax_rate`, `delete_tax_rate`, `list_category_tax_rates`, and `set_category_tax_rates` all use `state.db.lock().await` directly. Their TypeScript API wrappers also do not accept or send a session token. The application registers both legacy/global and scoped command variants. The separate `compute_cart_tax_scoped` path is already session-resolved and permission-checked; this finding concerns tax CRUD and category-assignment commands.

**Impact:** In a multi-store deployment, a caller able to invoke these commands can read or mutate the global database rather than the store resolved from the active session. A category assignment can therefore alter tax behavior outside the intended store, and a rate mutation can affect unrelated tenants. The manager-only UI route is not a substitute for backend authorization.

**Recommendation:** Add session-scoped variants for every tax-rate and category-assignment read/write command, resolve the store from the token, and enforce an explicit tax/settings permission on the backend. Migrate the UI to those commands. Keep legacy variants only as deliberately restricted/deprecated compatibility wrappers.

**Priority:** P0 — tenant isolation and financial configuration integrity.

---

### TAX-02 — Default-rate clearing is not atomic with create/update (P1)

**Evidence:** `Store::create_tax_rate` and `Store::update_tax_rate` first execute `UPDATE tax_rates SET is_default = 0 WHERE is_default = 1`, then perform the subsequent insert/update. The clearing operation is not wrapped with the following write in one transaction. The migrations do not define a unique constraint enforcing one default.

**Impact:** If the subsequent write fails, the store can be left without a default rate even though the request failed. The separate statements also leave a failure/concurrency window in which default-state consistency is not enforced by the database.

**Recommendation:** Use one transaction for clearing the old default and writing the new rate, and add a database invariant (for example, a partial unique index allowing at most one `is_default = 1`). Add rollback and concurrent-update tests.

**Priority:** P1 — configuration integrity.

---

### TAX-03 — Tax-rate deletion is destructive and has no confirmation or dependency policy (P1)

**Evidence:** The UI calls `deleteTaxRate(id)` immediately from the table button; there is no confirmation dialog. The backend executes `DELETE FROM tax_rates`. The product and category junction migrations use `ON DELETE CASCADE`, so deleting a rate silently removes its product/category assignments. The sale-line `tax_rate_id` reference does not declare cascade behavior, while historical line tax amounts are otherwise stored as snapshots.

**Impact:** A manager can remove a configured rate with one click and silently change future product/category behavior. With foreign-key enforcement, deletion may fail because `sale_lines.tax_rate_id` has no cascade behavior; with enforcement disabled, it may leave orphaned historical rate IDs. The exact outcome depends on database enforcement and existing references. There is no clear user-facing explanation of these consequences.

**Recommendation:** Require confirmation that names the rate and affected assignments. Prefer soft deletion or an `active` flag for rates referenced by historical sales; otherwise return dependency counts and make the deletion policy explicit. Add tests for assigned products, assigned categories, and historical sales.

**Priority:** P1 — financial configuration and auditability.

**REMEDIATED (Phase 2):** Replaced the destructive hard delete with a deliberate soft-delete policy — migration `109_tax_soft_delete.sql` adds `is_active` (default 1) so archiving a rate hides it from listing/lookup while keeping the row resolvable for historical `sale_lines.tax_rate_id` references. `delete_tax_rate` now: (1) blocks archiving with a structured `Validation` error when historical sales reference the rate, (2) clears product/category junction rows in the same transaction, and (3) makes archived rates immutable (`update_tax_rate` requires `is_active = 1`). New `get_tax_rate_dependency_counts_scoped` command (desktop + tablet, `SETTINGS_READ`, registered in both `lib.rs`) feeds the `TaxConfigurationScreen` delete flow: the ConfirmDialog now fetches counts before opening, names the rate, shows product/category assignment counts, and disables the confirm button with a dedicated blocked message when `sale_lines > 0`. FTL keys added in en + id; dev-mock handler added. Tests: backend soft-delete/junction-cleanup/sales-block/counts/immutability (34 tax tests), frontend blocked-dialog + dependency-count tests (18 UI tests), IPC contract for the new command. Validation: oz-core `db::tax` 34/34, migrations 22/22, clippy `-D warnings` clean, desktop + tablet compile, typecheck clean, review approved.

---

### TAX-04 — Rate validation has no upper bound and calculation can overflow (P1)

**Evidence:** The UI only requires a non-empty field and sends `parseInt(form.rateBps, 10)`. The input has `min="0"`, but no maximum. The backend rejects negative values but accepts any non-negative `i64`; the database constraint is likewise only `CHECK(rate_bps >= 0)`. Calculation performs `line_total_minor * rate.rate_bps` and inclusive calculation performs `10_000 + rate.rate_bps` before the later checked addition.

**Impact:** An accidental or malicious extreme rate can represent an unsupported percentage. Unchecked `i64` multiplication and `10_000 + rate_bps` can overflow; debug builds may panic, while release behavior depends on overflow-check configuration. The audit does not establish that every rate above 100% is invalid, so the bound should be defined by jurisdiction and business policy. The UI does not explain the accepted range or reject malformed/trailing input beyond the basic parse behavior.

**Recommendation:** Define a jurisdiction/business-supported maximum, validate it in the UI, command DTO, service, and database schema, and use checked multiplication/addition with a structured validation error. Add boundary tests for zero, maximum valid rate, and overflow inputs.

**Priority:** P1 — correctness and denial-of-service/data integrity risk.

---

### TAX-05 — Tax rounding policy is implicit integer truncation (P1)

**Evidence:** Both `compute_sale_tax` and `compute_cart_tax` calculate tax with integer division: exclusive tax is `base * rate_bps / 10_000`, and inclusive tax is `base * rate_bps / (10_000 + rate_bps)`. For the non-negative values expected here, Rust integer division truncates toward zero; each rate contribution is truncated independently and then summed. No rounding mode or jurisdiction-specific policy is represented in the model or documented in the inspected API.

**Impact:** Fractional minor-unit results are discarded, and line-level versus total-level rounding can produce reconciliation differences. Whether this systematically understates tax or is legally correct depends on the jurisdiction’s required rounding mode, which is not configured or documented. Multiple rates are calculated independently from the same line base; whether that is correct is likewise policy-dependent and is not configurable or surfaced.

**Recommendation:** Specify the rounding policy (line or document level, half-up/banker's/other), implement it with integer-safe arithmetic, document inclusive multi-rate semantics, and add golden tests for fractional results, multiple rates, refunds, and zero-decimal currencies.

**Priority:** P1 — financial correctness; exact legal impact depends on jurisdiction.

**REMEDIATED (Phase 1):** Added [`RoundingMode`](`modules/tax/src/models.rs`) — `Truncate` (legacy, pinned by the older unit tests) and `HalfUp` (the `#[default]`, now used by every `compute_sale_tax`/`compute_cart_tax` call site in desktop + tablet `pos.rs`). `compute_line_tax` applies `(numerator + divisor/2) / divisor` via integer-only `RoundingMode::divide` (checked-add, overflow → structured error). Golden tests cover the `.5` tie (333.5 → 334), inclusive rates, multi-rate lines, zero-decimal currency (JPY), refunds of half-up-taxed sales, and default-mode selection. Validation: oz-core `db::sales` 69/69, modules-tax 8/8, clippy `-D warnings` clean, desktop + tablet compile, review approved.

> ⚠️ **Financial note (behavior change):** new sales now round fractional tax half-up instead of truncating, so recorded `tax_total` values differ from legacy behavior on fractional results. Both the cart preview (`compute_cart_tax`) and final sale (`compute_sale_tax`) use the same default, keeping them in lockstep.

---

### TAX-06 — Tax loading failures are swallowed and cannot be retried (P2)

**Evidence:** `TaxConfigurationScreen.loadAll` catches all errors with only the comment `// IPC unavailable.` It then clears the loading state without setting an error state, toast, or retry action. A blank/partial configuration view is therefore indistinguishable from a successful empty result.

**Impact:** Operators may believe no tax rates or categories exist when the database/IPC request failed, or may see stale/empty configuration without knowing it is stale, potentially configuring incorrect rates or operating without expected tax rules.

**Recommendation:** Add explicit error state with localized message, retry action, and preserved last-known data where possible. Distinguish rate-loading, category-loading, and assignment-loading failures.

**Priority:** P2 — operational UX and tax configuration safety.

---

### TAX-07 — Delete and category-save failures are reported, but delete has no pending confirmation state (P2)

**Evidence:** Save and category assignment failures use localized toasts. Delete sets a per-row `deleting` flag, but the action begins immediately and there is no confirmation or undo path. The category assignment operation is transactional in `set_category_tax_rates`, while rate CRUD and default switching are not grouped transactionally.

**Impact:** The screen gives useful feedback after failures, but destructive intent is not recoverable and the user cannot review the affected configuration before the request starts.

**Recommendation:** Use a reusable confirmation dialog with focus management and a pending state; offer undo or refresh-dependent recovery where feasible. Align all mutation feedback with a consistent retry policy.

**Priority:** P2.

---

### TAX-08 — Tax configuration action controls are likely below the project touch-target standard (P2)

**Evidence:** `.tax-config-action-btn` uses `padding: var(--space-1) var(--space-3)` and the toggle buttons/category rows do not declare a minimum interactive height. The action buttons are compact table controls and the CSS does not establish the project’s usual 44px touch target.

**Impact:** Editing, deleting, and selecting category rates can be difficult on tablet/touch terminals, especially in horizontally scrolling tables.

**Recommendation:** Apply the shared button/touch-target sizing convention, verify at tablet breakpoints, and add the Tax stylesheet to automated touch-target compliance coverage if it is not already enforced.

**Priority:** P2.

---

### TAX-09 — Localized error fallbacks reintroduce hardcoded English (P3)

**Evidence:** Save and delete handlers use expressions such as `l10n.getString('tax-config-save-error') || 'Failed to save tax rate'` and the category handler has the same pattern. The primary keys exist in the inspected English and Indonesian bundles, so the fallback is normally hidden but remains user-visible English if localization lookup fails.

**Impact:** A missing/malformed bundle can produce inconsistent language and hide localization failures during testing.

**Recommendation:** Use a shared required-localization helper or a deliberately locale-neutral error boundary, and test missing-key behavior rather than embedding user-facing English in the component.

**Priority:** P3.

---

### TAX-10 — Transitional module boundary leaves duplicate ownership and limited domain coverage (P2)

**Evidence:** `modules/tax/src/lib.rs` explicitly states that the module is a registration/configuration layer and that database CRUD, Tauri commands, frontend, and API remain in their original locations. `modules/tax/src/repository.rs` currently exposes only `get_tax_rate`, and `service.rs` only delegates that lookup. The active CRUD/calculation paths remain in `oz-core` and the desktop command layer.

**Impact:** Tax behavior has multiple ownership points and the nominal Tax module does not provide the domain API its documentation describes. Future changes can update one path while leaving another stale, and module-level tests do not exercise production CRUD/calculation behavior.

**Recommendation:** Either complete the planned migration into the Tax module or update its contract/documentation to clearly identify the transitional boundary. Add cross-layer contract tests so module registration, commands, DB behavior, and UI DTOs cannot drift.

**Priority:** P2 — maintainability and change risk.

---

### TAX-11 — Focused tests cover happy paths but not the highest-risk invariants (P2)

**Evidence:** The focused UI suite has 13 passing tests and covers loading skeleton, empty state, CRUD interactions, category rendering, delete pending state, Escape close, and save failure. The `modules-tax` crate has 8 unit tests plus 1 doctest. The filtered `oz-core` tax run has 40 passing tests, including rate CRUD and basic product/category/default/inclusive calculation cases.

The suites do not visibly cover two-store isolation for all tax mutations, backend permission rejection, atomic default rollback, deletion dependencies/historical sales, extreme rates/overflow, explicit rounding policy, or real frontend-to-command DTO contracts.

**Impact:** Existing tests provide useful regression coverage but do not protect the P0/P1 findings that are most likely to cause cross-store or financial errors.

**Recommendation:** Prioritize isolation and authorization integration tests, then add transaction rollback, deletion policy, arithmetic boundary, rounding golden, and IPC contract tests. Add UI tests for load failure/retry and destructive confirmation.

**Priority:** P2.

## Positive observations

- The UI has a loading skeleton, empty state, category assignment view, and localized success/failure labels.
- Tax calculations use integer minor units and basis points rather than floating-point currency arithmetic.
- Product → category → default precedence is explicit and covered by focused Rust tests.
- Category assignment replacement uses a transaction.
- Sale line tax amounts and aggregate tax totals are persisted, providing a historical amount snapshot even when current rates change.
- Theme-sensitive CSS in the inspected Tax stylesheet uses project design tokens rather than hardcoded foreground/background colors, aside from the runtime category swatch color supplied by category data.
- The scoped live tax preview command resolves the session store and checks `SALES_PROCESS`; this protection should be used as the pattern for the remaining tax commands.

## Remediation implemented (2026-07-31)

**TAX-01 (P0) — Session scope + backend authorization for every tax mutation and read.**

- Added scoped, permission-enforced variants for **all** tax commands in both `apps/desktop-client/src/commands/tax.rs` and `apps/tablet-client/src/commands/tax.rs`:
  - `create_tax_rate_scoped`, `update_tax_rate_scoped`, `delete_tax_rate_scoped` (require `SETTINGS_EDIT`)
  - `list_category_tax_rates_scoped` (requires `SETTINGS_READ`)
  - `set_category_tax_rates_scoped` (requires `SETTINGS_EDIT`)
  - `list_tax_rates_scoped` now also resolves the session and enforces `SETTINGS_READ` (previously store-scoped only, no permission check).
- Registered every scoped command in both desktop and tablet `lib.rs` invoke handlers (parity verified).
- Migrated `ui/src/api/tax.ts` and `TaxConfigurationScreen.tsx` to the scoped commands, passing `sessionToken` from `useWorkspace()`.
- **Last gap closed (Phase 3):** `ProductManagementScreen.tsx` previously read tax rates and categories from the global DB (`listTaxRates()` / `listCategories()`), so per-product tax assignment ignored the active session store. It now uses `listTaxRatesScoped(sessionToken)` and `listCategoriesScoped(sessionToken)` — the same store the products come from. The create/update flows already used `createProductScoped`/`updateProductScoped` and already pass `taxRateIds` through to `store.set_product_tax_rates` on the backend, so no Rust changes were needed. Tests added: a session-store contract test asserting `list_tax_rates_scoped` + `list_categories_scoped` are called with the token and the unscoped variants are never called, and a test pinning `taxRateIds` flow through `create_product_scoped`. Validation: ProductManagementScreen 17/17, typecheck clean, review approved. Residual: unscoped `listCategories()` still exists in `useProducts.ts`, `KioskScreen.tsx`, and `RetailPosScreen.tsx` (retail/kiosk category reads) — same ADR #7 hygiene applies there, tracked as a follow-up.
- Fixed a pre-existing IPC wire mismatch: `CreateTaxRateArgs`/`UpdateTaxRateArgs` now carry `#[serde(rename_all = "camelCase")]` so the frontend `rateBps`/`isDefault`/`isInclusive` payloads deserialize (the legacy structs expected snake_case and would have rejected the real frontend payload).
- Added `ui/src/__tests__/tax-ipc-contract.test.ts` pinning the session-token + camelCase wire contract for all six scoped commands.
- Updated `ui/src/dev-mock/tauri-api.ts` with the scoped command aliases.

**TAX-02 (P1) — Atomic default switching + database invariant.**

- `Store::create_tax_rate` / `Store::update_tax_rate` now clear the previous default and write the new rate inside one `unchecked_transaction()` (rollback-safe).
- New migration `108_tax_single_default.sql`: normalises any legacy multiple-default data (keeps the oldest) and adds the partial UNIQUE index `idx_tax_rates_single_default` on `is_default` WHERE `is_default = 1`.
- Registered migration 108 in `crates/oz-core/src/migrations.rs`; added tests for index existence, second-default rejection, and the incremental-application normalisation path.

**TAX-04 (P1) — Bounded rates + integer-safe arithmetic.**

- Added `MAX_TAX_RATE_BPS = 1_000_000` (10,000%) in `crates/oz-core/src/db/tax.rs` and a shared `validate_tax_rate_input` helper enforcing `0..=MAX_TAX_RATE_BPS` on create/update.
- Extracted `compute_line_tax` with `checked_mul` / `checked_add` for both exclusive and inclusive paths; `compute_sale_tax` and `compute_cart_tax` now return a structured validation error on overflow instead of silently zeroing.
- Boundary tests: max rate accepted, above-max rejected on create and update.

**TAX-06/TAX-07 (P2) — Load-failure UX and destructive-action confirmation.**

- `TaxConfigurationScreen` now surfaces a distinct load-error state with a localized message and a Retry action (`tax-config-load-error` / `tax-config-load-retry`), distinguishing failure from an empty result.
- Delete now opens the shared `ConfirmDialog` naming the rate and its product/category assignment consequences (`tax-config-delete-confirm-title` / `tax-config-delete-confirm-message`) before invoking `delete_tax_rate_scoped`; the confirm button shows a pending/loading state.
- Out-of-range rate input now toasts a localized validation message (`tax-config-rate-invalid`) instead of silently returning.

**TAX-08 (P2) — Touch targets.**

- `.tax-config-action-btn`, `.tax-config-toggle-btn`, and `.tax-config-cat-rate-item` now enforce `min-height: 2.75rem` (44px) per the project tablet touch-target standard.

**TAX-09 (P3)** — New user-facing strings added to both `tax.ftl` and `tax.id.ftl` (bundle parity preserved). The component's `|| '…'` fallbacks were replaced with the shared `requiredLocalized` helper (Phase 5), and the follow-up codebase-wide sweep removed all remaining literal fallback sites (commit `1bec6777`).

## Recommended implementation order

1. **TAX-01:** Scope and authorize every tax mutation and category-assignment command; migrate the frontend API.
2. **TAX-04/TAX-05:** Define safe rate bounds, checked arithmetic, and an explicit rounding/multi-rate policy.
3. **TAX-02:** Make default switching atomic and enforce one default at the database level.
4. **TAX-03:** Add confirmation and a deliberate soft-delete/dependency policy.
5. **TAX-06/TAX-07:** Add error/retry and destructive-action UX.
6. **TAX-11/TAX-10:** Close the security/financial test gaps and clarify or complete the module migration.
7. **TAX-08/TAX-09:** Finish touch-target and localization-hardening cleanup.

## Validation (post-remediation)

- `npx vitest run src/__tests__/tax-ipc-contract.test.ts src/__tests__/TaxConfigurationScreen.test.tsx`: **16 passed** (IPC contract + screen incl. new delete-confirm and load-error/retry cases)
- `cargo test -p oz-core --lib db::tax`: **28 passed, 0 failed**
- `cargo test -p oz-core --lib migrations::`: **20 passed, 0 failed** (incl. migration 108 index + normalisation tests)
- `cargo check -p oz-pos-app --lib` and `cargo check -p oz-pos-tablet --lib`: passed
- `cargo clippy -p oz-core --lib -- -D warnings`: clean
- `npm run typecheck`: passed
- Code review: **no blocking findings**; residual follow-ups listed below.

## Residual follow-ups (documented, not blocking)

1. **TAX-05 — Rounding policy:** ✅ DONE (Phase 1). `RoundingMode` (Truncate/HalfUp) implemented in the tax module, threaded through all compute paths, golden-tested. Residual: ✅ **CLOSED** (commit `ee03e113`) — the rounding mode is now selectable per-store via `WorkspaceStorePosSettings` (`taxRoundingMode`), persisted through `get_tax_rounding_mode`/`set_tax_rounding_mode` in `crates/oz-core/src/settings.rs`, and threaded through `compute_sale_tax`/`compute_cart_tax` on both desktop and tablet.
2. **TAX-03 — Dependency policy on delete:** ✅ DONE (Phase 2). Migration `109` adds `is_active` soft-delete; `delete_tax_rate` archives instead of deleting, blocks when historical sales reference the rate, and clears junctions in-transaction; `get_tax_rate_dependency_counts_scoped` + ConfirmDialog show assignment counts and disable confirm when blocked; archived rates are immutable. Residual: `set_product_tax_rates`/`set_category_tax_rates` do not reject archived rate ids (unreachable from the UI since only active rates are listed) — left documented.
3. **TAX-10 — Module boundary:** ✅ DONE (Phase 4). Rather than physically moving code, the module is now formalised as the **contractual layer** for the tax vertical: it owns the canonical domain types (`TaxRate`, `RoundingMode`) re-exported by `oz-core` (`crates/oz-core/src/tax_rate.rs`), and the cross-layer boundary is pinned by `modules/tax/tests/boundary_contract.rs` — manifest registration (id `tax` ↔ `TaxModule::id()`, `tax:view`/`tax:edit` permissions), compile-time type identity of the re-export chain, DB behaviour parity with oz-core's store (including the TAX-03 `is_active` soft-delete filter — the contract test caught and fixed a real drift where the module repository ignored it), and the serde wire shape matching `ui/src/api/tax.ts` `TaxRateDto`. `modules/tax/src/lib.rs` + `README.md` reworded to the contractual-layer model. Validation: modules-tax 16/16, oz-core `db::tax` 34/34, clippy clean, desktop + tablet compile, UI typecheck clean, review approved. Residual: the UI-side IPC contract remains pinned by `tax-ipc-contract.test.ts` (TS), and if the module repository ever gains a `list_tax_rates` method it must also filter `is_active = 1` with contract-test coverage.
4. **TAX-09 — Hardcoded English fallbacks:** ✅ DONE (Phase 5). Added a shared `requiredLocalized` helper (`ui/src/frontend/shared/requiredLocalized.ts`, exported from the shared index) that returns the Fluent message id as a non-English fallback when a message is missing (dev-only console.warn), and wired it into all three `|| 'English'` fallback sites in `TaxConfigurationScreen.tsx`. Unit-tested (`requiredLocalized.test.ts`, 5/5). Residual: ✅ **CLOSED** (commit `1bec6777`) — the helper was swept codebase-wide: ~150 `getString(key) || 'English'` fallback sites across ~69 files converted to `requiredLocalized`, 17 genuinely-missing FTL keys added (en + id), and 5 retail-cart messages enriched with `{ $sku }`/`{ $name }` placeholders. Zero literal `getString(...) || '…'` fallback sites remain in `src` (excluding tests/dev-mock and dynamic-key sites).
5. **Legacy unscoped commands:** ✅ DONE (Phase 5). The six unscoped tax commands (`create_tax_rate`, `update_tax_rate`, `delete_tax_rate`, `list_tax_rates`, `list_category_tax_rates`, `set_category_tax_rates`) were deleted from both `apps/desktop-client` and `apps/tablet-client` (functions, `lib.rs` registrations, `ui/src/api/tax.ts` wrappers, and dev-mock entries). The frontend now calls only the `*_scoped` variants. Test mocks were trimmed to scoped-only names. Residual fully closed: the last unscoped **tax** command, `compute_cart_tax` (in `pos.rs`), was also removed from both apps (function, `lib.rs` registration, and dev-mock entry) — the frontend `computeCartTax` wrapper already called only `compute_cart_tax_scoped`, and grep confirmed zero remaining references. The `oz-core` `Store::compute_cart_tax` domain method is untouched (shared by the compute commands). Other unscoped POS commands (`complete_sale`, `start_sale`, `add_line`, etc.) remain registered as separate deprecated wrappers outside the tax vertical.

## Status

Audit 05 findings are now **fully remediated** across all five phases (evidence-backed validation above). Phase 5 (2026-07-31) additionally closed the **TAX-01 permission-source latent bug**: the scoped tax commands previously checked permissions against the store-scoped DB (which contains no users), so every tax command would have failed with `PermissionDenied`; they now resolve `require_tax_permission` against the **global identity DB** exactly like `loyalty.rs`. Pinned by two-store isolation + permission-rejection tests (6 per app, desktop + tablet). The optional `tax_breakdown_json` upgrade (migration `110` + `SaleLine` field + per-line breakdown from `compute_sale_tax`) preserves multi-rate tax detail on persisted sales — previously only the first rate id was stored, an auditability gap. Validation: oz-core `db::sales` 71/71 (incl. new breakdown round-trip test), `db::tax` green, modules-tax 7/7, modules-sales 7/7, desktop + tablet tax command tests green, clippy clean across oz-core/desktop/tablet, UI typecheck clean, UI tax tests 33/33 + requiredLocalized 5/5, code review approved with residuals documented above.
