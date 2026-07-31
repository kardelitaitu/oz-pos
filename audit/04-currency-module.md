# Currency Module Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** 04 — Currency module  
> **Status:** PARTIALLY REMEDIATED · IPC fixed-point contract aligned; settlement, command scoping, and remaining UX/validation findings require follow-up  
> **Scope:** Currency and exchange-rate UI, TypeScript API contracts, Tauri commands, `Money`/`Currency` arithmetic, exchange-rate persistence and migrations, default-currency settings, multi-currency checkout integration, permissions, localization, theming, performance, tests, and module documentation.

## Executive summary

The Currency surface has a strong foundation in the domain layer: `Money` uses integer minor units, `Currency` validates three-letter codes, exchange rates were migrated from floating-point storage to six-decimal integer fixed point, and the repository has extensive validation tests. Focused validation is green: the exchange-rate screen has **12 passing UI tests**, `CurrencyContext` has **10 passing tests**, `modules-currency` has **54 passing tests**, and the executed `oz-core` currency/settings integration filters also passed.

The audit confirmed and remediated the critical IPC contract mismatch between the TypeScript currency API/UI and the Rust command DTOs. The renderer now uses `rate_millionths`, `from_currency`, `to_currency`, and `effective_date`, converts decimal form input to fixed-point millionths, and formats fixed-point responses without exposing raw integers. A focused IPC contract test now asserts the exact renderer payload and response field names.

The multi-currency checkout path still requires a separate settlement design: PaymentModal now interprets the fixed-point rate correctly for display, but it continues to use the existing base-currency sale/payment commands. Selecting another charge currency must not be considered fully settled until the backend records the tender currency, rounded amount, rate snapshot, and receipt/refund metadata atomically. Currency and exchange-rate management commands also remain unscoped and unauthorised in the inspected Tauri handlers.

Other findings remain open: stale-rate selection, strict effective-date validation, default-currency command scope, locale/theme gaps, missing delete confirmation, bounded/latest-rate APIs, and end-to-end multi-currency settlement tests. This follow-up changed the TypeScript currency API, exchange-rate screen, PaymentModal rate interpretation, and focused tests; unrelated working-tree changes were preserved.

## Architecture and data flow

- **Currency/exchange-rate UI:** `ui/src/features/currency/ExchangeRateScreen.tsx` and `ExchangeRateScreen.css`.
- **Feature registration:** `ui/src/features/currency/register.tsx`; the screen is also embedded in `SettingsPage.tsx`.
- **Frontend API:** `ui/src/api/currency.ts`.
- **Default currency context:** `ui/src/contexts/CurrencyContext.tsx`.
- **Checkout integration:** `ui/src/features/sales/PaymentModal.tsx` loads currencies/rates/default currency and renders the charge-currency selector.
- **Desktop commands:** `apps/desktop-client/src/commands/currencies.rs` and `exchange_rates.rs`.
- **Tablet commands:** corresponding `apps/tablet-client/src/commands/currencies.rs` and `exchange_rates.rs`.
- **Domain module:** `modules/currency/src/{commands,models,repository,error,lib}.rs`.
- **Persistence:** `modules/currency::CurrencyRepository`, with `oz-core` settings delegation.
- **Schema:** `crates/oz-core/migrations/006_currencies.sql`, `071_exchange_rate_minor_units.sql`, and global currency settings migration `075_global_currency_settings.sql`.
- **Money primitives:** foundation/`oz-core` `Money` and `Currency` re-exports; integration coverage in `crates/oz-core/tests/currency_integration.rs`.
- **Module manifest:** `modules/currency/manifest.json` declares `currency:view` and `currency:edit`.
- **Localization:** `ui/src/locales/currency.ftl` and `currency.id.ftl`, with payment labels in `sales.ftl`/`sales.id.ftl`.
- **Tests:** `ExchangeRateScreen.test.tsx`, `CurrencyContext.test.tsx`, `currency.test.ts`, currency IPC/API contract coverage, `modules-currency` tests, and `oz-core` currency/settings integration tests.

## Findings

### CUR-01 — TypeScript and Rust exchange-rate IPC contracts are incompatible

**Severity:** P0 — exchange-rate management is functionally broken across the real IPC boundary  
**Status:** Implemented and validated

**Evidence:**

- `ui/src/api/currency.ts::ExchangeRateDto` now declares `rate_millionths`, matching Rust `modules/currency/src/commands.rs::ExchangeRateDto`.

- `ui/src/api/currency.ts::CreateExchangeRateArgs` now sends `from_currency`, `to_currency`, `rate_millionths`, and `effective_date`, matching the Rust deserializer.
- `ExchangeRateScreen.tsx` converts the decimal form value to a validated integer millionths value and renders via `formatExchangeRate`.
- `ui/src/__tests__/currency-ipc-contract.test.ts` asserts the exact command payload and fixed-point response shape; `ExchangeRateScreen.test.tsx` uses the real DTO field.
- The module's DTO tests continue to assert `rate_millionths`, confirming fixed-point is the canonical boundary contract.

**Impact resolved:** The real create/list IPC boundary and exchange-rate screen now use the same fixed-point field names and payload shape. Remaining precision limits for JavaScript numbers and backend range policy are tracked under CUR-07/CUR-05.

**Implemented:** The canonical boundary is fixed-point `rate_millionths` with matching snake_case field names. The UI converts form decimals to millionths and formats values only at the display boundary. A focused IPC contract test and ExchangeRateScreen regression coverage validate the contract.

### CUR-02 — PaymentModal displays converted currency but settles the base currency amount

**Severity:** P0 — charge/receipt currency integrity risk  
**Status:** Open

**Evidence:**

- `PaymentModal.tsx` loads exchange rates and computes `exchangeRateInfo` when `selectedCurrency !== total.currency`.
- The UI displays a converted “Charge amount” using `Math.round(total.minor_units * exchangeRateInfo.rate)` and labels the selected currency.
- In the inspected `PaymentModal` completion paths, `startSale`/`startSaleScoped` receive `{ currency: total.currency }`, not `selectedCurrency`.
- The inspected `completeSale`/`completeSaleScoped` payment split amounts, QRIS amounts, tendered amounts, and receipt amounts use `total.minor_units` and `total.currency`; no converted charge minor amount is passed from this component. Any additional backend conversion outside this inspected path must be verified separately.
- `exchangeRateInfo` is used for display but is not used to alter the settlement payload or the base sale total.

**Impact:** Unless an additional backend conversion exists outside the inspected component, a cashier can select a non-base charge currency and see one amount while the sale/payment pipeline records or charges the base-currency amount. This can cause undercharging, overcharging, incorrect payment reconciliation, and misleading receipts.

**Recommendation:** Define whether the sale is denominated in the store/base currency with a separate tender currency, or whether the sale itself changes currency. Implement that policy atomically at the backend command boundary: validate the selected rate, calculate converted minor units using currency exponents and checked arithmetic, persist both base and tender amounts/rate metadata, and reconcile payment/refund amounts in the same model. Disable completion when a valid rate is unavailable. Add end-to-end tests for direct, inverse, multi-decimal, rounding, split, QRIS, refund, and receipt behavior.

### CUR-03 — Currency and exchange-rate commands are not consistently session-scoped or permission-checked

**Severity:** P0 for unauthorised rate/default-currency mutation — cross-store configuration and financial authorization risk  
**Status:** Open

**Evidence:**

- `list_currencies_scoped` resolves a session and opens the store database, but the primary `list_currencies` command uses global `state.db`; currency metadata itself is low sensitivity, but the scoped/unscoped contract is inconsistent.
- `get_default_currency` and `set_default_currency` in `apps/desktop-client/src/commands/currencies.rs` use global `state.db` and accept no session token.
- `list_exchange_rates`, `create_exchange_rate`, and `delete_exchange_rate` in `exchange_rates.rs` use global `state.db` and accept no session token.
- The inspected currency commands do not call `resolve_store`, `resolve_session`, or `require_permission_for_user`.
- `modules/currency/manifest.json` declares `currency:view` and `currency:edit`, but those permissions are not a backend security boundary in the inspected command handlers. The UI registration's `requiredRole: 'manager'` is also not sufficient protection against direct IPC invocation.
- PaymentModal still calls the unscoped `listCurrencies`, `listExchangeRates`, and `getDefaultCurrency` APIs.

**Impact:** In a multi-store deployment, a user can mutate the global/default store's currency configuration and rates rather than the active session store. A caller with command access may change rates or default currency without the intended manager permission. The unscoped currency metadata read is primarily a consistency concern rather than the main sensitive-data risk.

**Recommendation:** Add scoped variants for every currency read/write command, derive the authenticated user and store from the session, and enforce separate view/edit permissions. Migrate settings, ExchangeRateScreen, PaymentModal, and tablet callers. Deprecate or remove unscoped commands after migration. Add two-store isolation and denied-role integration tests.

### CUR-04 — PaymentModal chooses the first matching rate without selecting the effective historical rate

**Severity:** P1 — stale or incorrect conversion risk  
**Status:** Open

**Evidence:**

- `PaymentModal.tsx` uses `exchangeRates.find(...)` for the direct pair and then `find(...)` for the inverse pair.
- `CurrencyRepository::list_exchange_rates` orders only by `from_currency, to_currency`; it does not order by effective date descending or return the rate selected for a requested transaction date.
- The schema permits multiple rows for the same pair on different effective dates, and the UI displays the rate's `effective_date` without validating that it is current for the sale.
- No inspected payment code compares `effective_date` to the transaction/business date or records a selected rate snapshot as part of settlement.

**Impact:** When historical rates exist, the first list entry may be stale or arbitrary. A payment can use an incorrect conversion while showing a plausible rate and date.

**Recommendation:** Add a repository/API query for the latest rate effective on the transaction business date, with deterministic ordering and an explicit freshness policy. Persist the exact rate ID/value used on the sale/payment and refund records. Add tests for multiple dates, inverse pairs, future rates, missing rates, and boundary dates.

### CUR-05 — Exchange-rate input and effective-date validation is incomplete

**Severity:** P1 — invalid financial configuration can be persisted or silently rejected  
**Status:** Open

**Evidence:**

- `ExchangeRateScreen.tsx` checks only `parseFloat(form.rate) > 0` before submission; it does not reject non-finite values, malformed date strings, future/past policy violations, or whitespace/format anomalies beyond the browser input type.
- The UI's `formValid` checks that the two currencies differ but does not validate that the selected codes are present in the loaded currency list or that an effective date exists and is valid.
- `exchange_rates.rs` validates non-empty currency/source/date strings and positive fixed-point rate, but does not validate ISO-4217 membership, `from != to`, or `YYYY-MM-DD` date format.
- `CurrencyRepository` also checks only non-empty strings and positive rate; it relies on foreign keys for currency existence and does not validate date format or source length/content.

**Impact:** Direct IPC callers or malformed UI state can create semantically invalid rate records. Invalid dates make “latest effective rate” selection unsafe, and same-currency rates can create confusing configuration.

**Recommendation:** Validate currency codes through the canonical currency table/domain type, require distinct currencies, validate strict ISO dates, define allowed effective-date windows, bound source length, and reject non-finite/overflowing renderer values. Return field-specific errors and add command/repository tests for every boundary.

### CUR-06 — Exchange-rate delete has no confirmation and weak failure/recovery UX

**Severity:** P1 — destructive configuration UX gap  
**Status:** Open

**Evidence:**

- `ExchangeRateScreen.tsx` calls `confirmDelete(r.id)` directly from the row Delete button.
- The localized `currency-delete-confirm` message exists in both English and Indonesian bundles but is not used by the screen.
- There is no confirmation dialog, `window.confirm`, focus trap, or undo path before deletion.
- Delete failures are surfaced through a toast, but the rate list is only reloaded after success and there is no explicit retry action for the failed mutation.

**Impact:** A manager can accidentally remove a rate used by future checkout. Existing historical records may remain, but future rate selection can silently fail or fall back to an unavailable conversion.

**Recommendation:** Add a localized destructive confirmation dialog with pair/rate/date context, Escape handling, focus trapping, loading state, and a clear failure/retry action. Consider soft-delete or immutable rate history if rates may be referenced by completed transactions. Add cancel/confirm/failure/keyboard tests.

### CUR-07 — Rate display and API types lose fixed-point intent in the UI

**Severity:** P1 — precision/observability risk  
**Status:** Partially remediated — display and input paths fixed; settlement precision policy remains open

**Evidence:**

- The Rust domain stores `rate_millionths: i64` specifically to avoid floating-point arithmetic.
- `ui/src/api/currency.ts` now preserves the fixed-point field and exposes `formatExchangeRate`; ExchangeRateScreen uses that formatter instead of rendering a raw/obsolete field.
- PaymentModal now converts direct and inverse rates from `rate_millionths` before display calculations, so it no longer treats the fixed-point integer as a decimal.
- Currency exponent-aware settlement rounding and a lossless string/decimal IPC representation for values beyond JavaScript's safe integer range remain open.

**Impact reduced:** The UI no longer shows the wrong field or treats millionths as a whole-number rate. Exact settlement rounding, historical-rate snapshots, and lossless handling of values beyond JavaScript's safe integer range still require backend design.

**Remaining recommendation:** Keep fixed-point values lossless across IPC, define exponent-aware checked conversion at the backend settlement boundary, persist the exact rounded rate snapshot, and add JPY/KWD plus large USD/IDR precision tests.

### CUR-08 — Exchange-rate list loading is unbounded and not scoped to a pair/date view

**Severity:** P2 — performance and operational clarity risk  
**Status:** Open

**Evidence:**

- `CurrencyRepository::list_exchange_rates` returns every stored rate with no pagination, date window, pair filter, or maximum.
- `ExchangeRateScreen` renders the complete result as one table and reloads the full list after every create/delete.
- The schema explicitly permits one row per pair/date, so an automatically synchronized history can grow without bound.
- PaymentModal also loads the entire rate list when opening the payment modal.

**Impact:** Long-lived stores accumulate larger IPC payloads and render costs. Checkout pays the cost of loading historical rates even though it needs only current applicable pairs.

**Recommendation:** Add paginated/admin-filtered history endpoints and a latest-effective-rate endpoint for checkout. Bound default windows, index `(from_currency, to_currency, effective_date)`, and avoid loading rate history into the payment renderer. Add representative-volume query and UI performance tests.

### CUR-09 — Currency locale bundles are incomplete for active UI contracts

**Severity:** P2 — localization/accessibility regression  
**Status:** Open

**Evidence:**

- `currency.ftl` defines `currency-delete-confirm`, but `ExchangeRateScreen.tsx` never uses it; this is a stale contract rather than a complete destructive flow.
- The screen uses `error-state-retry` from a shared bundle, but the report did not find a dedicated currency retry key or a localized retry context in `currency.ftl`.
- The source contains fallback literals such as `Failed to save exchange rate`, `Failed to delete exchange rate`, `Exchange Rates`, `Add`, `Delete`, and placeholder children. Some are inside `Localized` wrappers, while mutation toast fallbacks remain literal English.
- The Indonesian bundle uses attribute-only placeholder messages for rate/source placeholders, which is correct only when consumed through `Localized attrs`; direct `getString` use would not return a value.

**Impact:** English can leak into Indonesian when a message is missing or a toast fallback executes. Stale keys also suggest the intended confirmation and retry UX was not implemented consistently.

**Recommendation:** Make every visible label, fallback, toast, placeholder, and accessible name use a message with the correct value/attribute contract. Add the missing Indonesian parity checks and remove unused/stale keys or implement their intended flows. Run bundle-parity, Fluent duplicate, and attribute-only audits.

### CUR-10 — Exchange-rate action controls are below the project touch-target convention

**Severity:** P2 — tablet usability gap  
**Status:** Open

**Evidence:**

- `.exchange-rate-action-btn` uses `padding: var(--space-1) var(--space-3)` and `font-size: var(--text-sm)` without an explicit `min-height` or `min-width`.
- The screen is registered for settings/admin workflows that can be used on touch hardware, and the project has a 44px touch-target convention.
- The existing focused test suite does not assert the computed or declared target size.

**Impact:** Delete actions in dense rate tables can be difficult to activate accurately on tablet displays.

**Recommendation:** Apply the shared touch-target minimum or a documented compact exception, with responsive table behavior and adequate row spacing. Add a touch-target compliance assertion for the screen.

### CUR-11 — Currency module ownership and documentation still describe a transitional architecture

**Severity:** P2 — architecture and maintenance drift  
**Status:** Open

**Evidence:**

- `modules/currency/src/lib.rs` says the backend/frontend remain in original locations and that command handlers will be migrated later, while repository/DTO extraction phases are already complete and active callers use the module repository.
- The module's lifecycle methods still describe future cache initialization and scheduled auto-sync as unimplemented.
- The module manifest declares currency permissions, but live command handlers do not enforce them.
- `CHANGELOG.md` documents the R2 extraction as complete while the module source still presents the overall vertical as a registration/configuration layer.

**Impact:** Contributors may not know whether `modules/currency` or `oz-core` is authoritative for settings, rate selection, validation, and command contracts. Security and auto-sync responsibilities are especially easy to miss.

**Recommendation:** Update module README/lib documentation to identify the current source of truth and remaining migration boundaries. Document the permission enforcement owner, fixed-point IPC contract, rate freshness policy, and whether auto-sync is production-ready or explicitly future work. Add a docs drift check against command registrations and module manifests.

### CUR-12 — Focused tests do not exercise the real IPC or multi-currency settlement invariants

**Severity:** P2 — high-risk regression-detection gap  
**Status:** Partially remediated — IPC contract coverage added; settlement/isolation coverage remains open

**Evidence:**

- `ExchangeRateScreen.test.tsx` still mocks the API module, but its fixtures now use `rate_millionths` and the save assertion verifies the fixed-point payload.
- `currency-ipc-contract.test.ts` now covers the exact create/list command contract and fixed-point formatter.
- `CurrencyContext.test.tsx` covers provider loading/update behavior but not scoped store selection or permission failure.
- Rust repository/module tests cover fixed-point persistence, validation, ordering, uniqueness, deletion, and Money arithmetic, but command-level desktop/tablet scope and permission integration remains open.
- PaymentModal currency behavior is not covered by a test proving selected currency changes the settlement payload, inverse-rate choice, rounding, rate freshness, refund, or receipt contract.
- No focused test yet proves unscoped-vs-scoped currency isolation, denial of unauthorized exchange-rate mutation, or multi-currency settlement payload invariants.

**Remaining recommendation:** Add desktop/tablet command integration tests for session/permission isolation and end-to-end PaymentModal tests for direct/inverse conversion, rate selection, minor-unit rounding, split/QRIS/refund settlement, and receipt metadata. Keep UI fixtures aligned with the real DTO fields.

## Positive observations

- `Money` uses integer minor units and checked arithmetic; same-currency operations reject mismatches rather than silently adding unlike currencies.
- `Currency` validates three-letter codes and provides known minor-unit exponents for zero- and three-decimal currencies.
- Migration 071 replaced legacy REAL exchange-rate storage with integer millionths, preserving a fixed-point financial representation.
- Exchange-rate persistence uses parameterized SQL, foreign keys, a unique pair/date constraint, and repository-level positive-rate validation.
- The exchange-rate UI has loading skeleton, error/retry, successful-empty, table, and modal states.
- Exchange-rate screen tests cover loading, error, empty, rendering, create, delete, and modal flows; CurrencyContext tests cover fallback, load success/failure, cancellation, and persistence failure behavior.
- Focused validation passed: 12 ExchangeRateScreen tests, 10 CurrencyContext tests, 3 currency IPC contract tests, 54 `modules-currency` tests, and the executed `oz-core` currency/settings test filters.

## Recommended implementation order

1. **Repair the IPC contract:** ✅ aligned fixed-point DTO names/types and added a real renderer payload contract test.
2. **Fix settlement correctness:** define base/tender currency semantics and make PaymentModal/backend payloads atomic and currency-aware.
3. **Secure and scope commands:** session-resolve currency/rate/default settings and enforce view/edit permissions in desktop and tablet clients.
4. **Rate selection policy:** select the latest valid effective rate, handle inverse rates deterministically, and persist the rate snapshot used.
5. **Validation and destructive UX:** validate codes/dates/ranges, add delete confirmation, and preserve immutable historical rates.
6. **Precision and scale:** use fixed-point formatting/conversion throughout and add bounded/latest-rate endpoints instead of loading all history.
7. **Localization, touch, docs, and QA:** close locale parity, meet touch targets, reconcile module documentation, and add real IPC/checkout/isolation regression tests.

## Validation performed

- `cd ui && npx vitest run src/__tests__/ExchangeRateScreen.test.tsx src/__tests__/currency-ipc-contract.test.ts` — **15 passed, 0 failed**.
- `cd ui && npm run typecheck` — passed.
- `cd ui && npx vitest run src/__tests__/CurrencyContext.test.tsx` — **10 passed, 0 failed**.
- `cargo test -p modules-currency` — **54 tests passed, 0 failed**; no doctest failures.
- Currency-related `oz-core` test runs executed by the audit passed: **12 unit tests**, **8 currency integration tests**, **35 settings integration tests**, and **5 corruption-recovery tests**.
- Source inspection covered Currency UI/CSS, API contracts, CurrencyContext, PaymentModal, desktop/tablet commands, module DTOs/repository/error types, migrations, Money/Currency integration, locales, feature registration, and tests.

## Fix status

CUR-01 is **Implemented and validated**. CUR-07 and CUR-12 are **Partially remediated** for the fixed-point UI/IPC path. CUR-02, CUR-03, CUR-04, CUR-05, CUR-06, CUR-08, CUR-09, CUR-10, and CUR-11 remain **Open** and require separate settlement, authorization, UX, scale, localization, or documentation work. Focused UI/Rust validation for this slice is recorded above.
