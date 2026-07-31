# Loading States Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Loading states — skeleton consistency, loading indicators, progress feedback, async transitions, and recovery UX
> **Status:** AUDITED · cross-screen loading and failure-state findings require remediation
> **Production code changed:** None

## Scope

This audit evaluates sector 23 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers initial loads, refreshes, mutations, skeleton fidelity, spinners, disabled/loading controls, empty-versus-error distinction, cancellation and race safety, accessibility announcements, localization, theming, responsive behavior, and automated coverage.

Inspected areas:

- `ui/src/components/Skeleton.tsx`
- `ui/src/frontend/shared/Skeleton.tsx`
- `ui/src/components/Spinner.tsx`
- `ui/src/components/Button.tsx`
- `ui/src/features/kds/KdsScreen.tsx`
- `ui/src/features/kds/KdsHistoryPanel.tsx`
- `ui/src/features/kds/components/KdsProductPickerModal.tsx`
- `ui/src/features/retail/RetailPosScreen.tsx`
- `ui/src/features/retail/RetailProductGrid.tsx`
- `ui/src/features/products/ProductLookupScreen.tsx`
- `ui/src/features/products/useProducts.ts`
- `ui/src/features/customers/CustomerManagementScreen.tsx`
- `ui/src/features/categories/CategoryManagementScreen.tsx`
- `ui/src/features/currency/ExchangeRateScreen.tsx`
- `ui/src/features/audit/AuditLogScreen.tsx`
- `ui/src/features/offline/OfflineQueueScreen.tsx`
- `ui/src/features/restaurant/RestaurantMenu.tsx`
- `ui/src/features/sales/SalesHistoryScreen.tsx`
- `ui/src/features/settings/SettingsPage.tsx`
- `ui/src/features/workspaces/WorkspaceHome.tsx`
- `ui/src/__tests__/Skeleton.test.tsx`
- `ui/src/__tests__/Spinner.test.tsx`
- `ui/src/__tests__/animationCompliance.test.ts`
- `docs/ui-state-audit-2026-07-20.md`

## Architecture summary

The UI has reusable loading primitives, but two separate `Skeleton` implementations exist: `ui/src/components/Skeleton.tsx` and `ui/src/frontend/shared/Skeleton.tsx`. `Spinner` exposes `role="status"` and a localized label, while `Button` exposes `aria-busy` and disables itself during processing. Screens otherwise implement loading state locally, using a mixture of shared skeletons, custom skeleton CSS, plain text, and spinners.

The strongest pattern is a three-way initial state: loading, error, and data/empty. It is present in screens such as ExchangeRate, OfflineQueue, AuditLog, and several inventory and management screens. However, this is not universal: some screens catch load failures and leave an empty collection with no visible error, while others intentionally replace failed backend data with demo products. Refreshes and secondary requests also use different policies, and there is no shared request lifecycle or loading-state contract.

## Findings

### LOAD-01 — Two separate Skeleton components create visual and behavioral drift

**Evidence:** Both `ui/src/components/Skeleton.tsx` and `ui/src/frontend/shared/Skeleton.tsx` export a `Skeleton` with the same variants and `aria-hidden="true"`, but they are separate implementations. Feature imports are split between `@/components/Skeleton` and `@/frontend/shared`.

**Impact:** A token, animation, accessibility, or API correction can be applied to one primitive while screens using the other remain inconsistent. The duplicate ownership makes the loading system harder to audit and increases the chance of different theme or reduced-motion behavior over time.

**Severity:** P2 · design-system consistency

**Affected files:** `ui/src/components/Skeleton.tsx`, `ui/src/frontend/shared/Skeleton.tsx`, and feature imports using both public paths.

**Recommendation:** Retain one canonical Skeleton export and make the other path a compatibility re-export. Add a test that imports the public paths and verifies identical semantics, class names, reduced-motion behavior, and token usage. Document when a custom skeleton is justified.

**Status:** Open

### LOAD-02 — Several load failures are silently converted into an apparently empty screen

**Evidence:** `CategoryManagementScreen`, `CustomerManagementScreen`, and `SalesHistoryScreen` catch initial list failures without setting or rendering a user-visible error; comments say `IPC unavailable`. Their `loading` flags are then cleared, so the normal empty branch can be displayed. Similar silent catches exist in other feature loaders.

**Impact:** A disconnected or failing backend can look like a valid empty database. Operators may conclude that customers, categories, or sales do not exist and take incorrect follow-up actions. There is no retry affordance or explanation that the data is unavailable.

**Severity:** P1 · operational correctness

**Affected files:** `ui/src/features/categories/CategoryManagementScreen.tsx`, `ui/src/features/customers/CustomerManagementScreen.tsx`, `ui/src/features/sales/SalesHistoryScreen.tsx`.

**Recommendation:** Use a shared async state model that distinguishes `loading`, `refreshing`, `ready`, `empty`, and `error`. Preserve the error after a failed initial load, render a localized error state with Retry, and only render an empty state after a successful zero-item response. Add tests for rejected initial loads and successful empty responses separately.

**Status:** Open

### LOAD-03 — Demo-data fallbacks can mask a production load failure

**Evidence:** `useProducts` catches product/category loading errors, stores an error internally, and replaces the result with `SAMPLE_PRODUCTS` and `SAMPLE_CATEGORY_META`; `ProductLookupScreen` then renders the sample catalog and exposes only a fallback notice. `RetailPosScreen` similarly replaces failed product/category requests with `RETAIL_SAMPLE_PRODUCTS` and `RETAIL_SAMPLE_CATEGORIES` while continuing to expose product actions.

**Impact:** In a production terminal, a cashier can see and select products that are not in the current store catalog. A fallback intended for browser development can be mistaken for live inventory, causing incorrect sales or stock decisions. The visual presence of products also makes the failure easy to miss.

**Severity:** P1 · transaction and inventory integrity

**Affected files:** `ui/src/features/products/useProducts.ts`, `ui/src/features/products/ProductLookupScreen.tsx`, `ui/src/features/retail/RetailPosScreen.tsx`.

**Recommendation:** Gate demo data behind an explicit development/demo mode that cannot activate merely because IPC fails. In production, render a clear localized unavailable state and a Retry action; if a fallback is retained for development, add a prominent non-actionable demo banner and disable checkout-affecting actions. Add tests proving a rejected live request cannot expose demo products in production mode.

**Status:** Open

### LOAD-04 — Loading semantics are inconsistent for initial load versus refresh

**Evidence:** `KdsScreen` has a dedicated initial skeleton, but later `fetchOrders` updates the board directly without a separate refreshing state. `OfflineQueueScreen.load()` sets `loading=true` for refresh and replaces the current table with the full skeleton. `AuditLogScreen` preserves existing entries while appending/loading more. `KdsHistoryPanel` replaces its content with plain loading text on every status-filter change.

**Impact:** Users receive different feedback for equivalent operations: some screens preserve usable stale data, some blank the screen, and some provide only text. In a busy POS/KDS workflow, hiding a known-good board during refresh can interrupt operation, while failing to indicate background refresh can leave users unsure whether a filter or action was accepted.

**Severity:** P2 · workflow continuity

**Affected files:** `ui/src/features/kds/KdsScreen.tsx`, `ui/src/features/kds/KdsHistoryPanel.tsx`, `ui/src/features/offline/OfflineQueueScreen.tsx`, `ui/src/features/audit/AuditLogScreen.tsx`.

**Recommendation:** Separate `initialLoading` from `refreshing` and `loadingMore`. Preserve existing data during refresh, mark the region `aria-busy`, disable only conflicting actions, and show a compact localized progress indicator. Use full skeletons only when no usable data exists. Add transition tests for initial, refresh, retry, pagination, and filter changes.

**Status:** Open

### LOAD-05 — Custom skeletons and plain loading text do not consistently announce progress

**Evidence:** Shared `Spinner` and `Button` have explicit status/busy semantics, but many screen skeleton wrappers are only `aria-hidden="true"` or have no loading role at all. `KdsScreen`'s `.kds-loading-container` has no `role="status"` or `aria-busy`; `RestaurantMenu` renders localized text without a status role; `KdsHistoryPanel` renders a spinner span and text without a status wrapper; `SettingsPage`'s Suspense fallback is a plain `div`. The retail table uses `role="status"`, but its parent loading state is otherwise custom.

**Impact:** Screen-reader users may receive no announcement when a screen begins loading, or may hear inconsistent announcements across screens. A skeleton being hidden from assistive technology is correct only when an accessible status message exists alongside it.

**Severity:** P1 · loading accessibility

**Affected files:** `ui/src/features/kds/KdsScreen.tsx`, `ui/src/features/kds/KdsHistoryPanel.tsx`, `ui/src/features/kds/components/KdsProductPickerModal.tsx`, `ui/src/features/restaurant/RestaurantMenu.tsx`, `ui/src/features/settings/SettingsPage.tsx`, `ui/src/features/retail/RetailProductGrid.tsx`.

**Recommendation:** Provide a shared `LoadingState`/`SkeletonScreen` wrapper with localized status text, `role="status"`, `aria-live="polite"`, and `aria-busy` on the affected region. Keep decorative skeleton children `aria-hidden`. For refreshes, announce “Updating…” without repeatedly interrupting the user. Add axe and transition tests for initial and refresh states.

**Status:** Open

### LOAD-06 — Loading copy still contains hardcoded English or inconsistent fallback strings

**Evidence:** `SettingsPage` uses `Loading...` in its Suspense fallback; `AuditLogScreen` uses `Loading…` in the load-more button fallback; `KdsHistoryPanel` has `Loading history...`; `KdsProductPickerModal` uses `Loading products...`; `RetailProductGrid` and `RestaurantMenu` use English fallbacks after `l10n.getString()`. Several components rely on `|| 'Loading...'` or similar patterns instead of a guaranteed value-bearing Fluent message.

**Impact:** Loading feedback can switch languages within one screen or appear with inconsistent punctuation and wording. Missing translations are hidden rather than detected, weakening the reliability of the localization contract during a state where users are waiting for important data.

**Severity:** P2 · localization consistency

**Affected files:** `ui/src/features/settings/SettingsPage.tsx`, `ui/src/features/audit/AuditLogScreen.tsx`, `ui/src/features/kds/KdsHistoryPanel.tsx`, `ui/src/features/kds/components/KdsProductPickerModal.tsx`, `ui/src/features/retail/RetailProductGrid.tsx`, `ui/src/features/restaurant/RestaurantMenu.tsx`.

**Recommendation:** Add value-bearing loading keys to every supported bundle and use a shared localized loading component. Avoid silent English fallbacks for production UI; make bundle parity and missing-key checks fail in CI. Standardize wording for initial loading, refresh, load-more, and processing.

**Status:** Open

### LOAD-07 — Several asynchronous loaders lack cancellation or request-generation protection

**Evidence:** `useProducts` and the retail product/category loader include cancellation guards. In contrast, many screen-level `load` callbacks simply await API calls and set state in `finally`; examples include `CategoryManagementScreen`, `CustomerManagementScreen`, `ExchangeRateScreen`, `KdsHistoryPanel`, and `OfflineQueueScreen`. `OfflineQueueScreen` also runs asynchronous polling every ten seconds without an in-flight generation or mounted guard.

**Impact:** A rapid unmount, store/session change, filter change, or overlapping manual refresh can allow an older response to overwrite newer state or update an unmounted component. The result can be stale lists, incorrect loading flags, or a loading indicator that stops for the wrong request.

**Severity:** P1 · async correctness

**Affected files:** `ui/src/features/categories/CategoryManagementScreen.tsx`, `ui/src/features/customers/CustomerManagementScreen.tsx`, `ui/src/features/currency/ExchangeRateScreen.tsx`, `ui/src/features/kds/KdsHistoryPanel.tsx`, `ui/src/features/offline/OfflineQueueScreen.tsx`.

**Recommendation:** Standardize request cancellation/generation tokens for every loader whose inputs can change. Serialize or deduplicate refreshes, ignore stale responses, and guard both success and `finally` updates. Add tests for out-of-order responses, unmount during a request, rapid filter changes, and refresh-button double clicks.

**Status:** Open

### LOAD-08 — Error, empty, and fallback states are not always mutually exclusive or sufficiently actionable

**Evidence:** `ProductLookupScreen` can hold an internal `error` while rendering fallback products; `RetailPosScreen` can set `loadError` while showing demo data; `KdsHistoryPanel` renders a raw `String(e)` error with no Retry control; `KdsProductPickerModal` renders a raw error paragraph; several screens render empty data after silent catches. The shared `EmptyState` component offers an action slot, but many feature states do not use it.

**Impact:** Users cannot consistently tell whether data is empty, stale, unavailable, or intentionally demo content. Some errors have no retry path, and raw technical messages may expose implementation details or be difficult to understand.

**Severity:** P1 · recovery UX

**Affected files:** `ui/src/features/products/ProductLookupScreen.tsx`, `ui/src/features/retail/RetailPosScreen.tsx`, `ui/src/features/kds/KdsHistoryPanel.tsx`, `ui/src/features/kds/components/KdsProductPickerModal.tsx`, and multiple feature list screens.

**Recommendation:** Define a common state contract: initial loading, success with data, success empty, recoverable error, and unavailable/offline. Every recoverable error should provide a localized explanation and Retry action; technical details belong in diagnostics/logging. Add a state matrix test for representative list, grid, modal, and dashboard screens.

**Status:** Open

### LOAD-09 — Loading placeholders are not consistently shape-matched to their final content

**Evidence:** Management screens commonly render table skeletons, while `ProductLookupScreen`, `RestaurantMenu`, and KDS history use compact text/empty containers. Retail has a custom five-column skeleton, KDS has three status columns, and many feature screens define their own row counts and dimensions. There is no shared contract for preserving toolbar height, table width, or responsive card geometry.

**Impact:** Layout shift varies substantially between screens. On tablet and POS layouts, large shifts can move controls under a user's finger or change scroll position while data arrives. A low-fidelity placeholder also makes slow network conditions feel less stable.

**Severity:** P3 · visual stability

**Affected files:** `ui/src/features/products/ProductLookupScreen.tsx`, `ui/src/features/restaurant/RestaurantMenu.tsx`, `ui/src/features/kds/KdsHistoryPanel.tsx`, `ui/src/features/retail/RetailProductGrid.tsx`, and management-screen skeleton implementations.

**Recommendation:** Co-locate skeletons with stable layout shells and match the final toolbar, filters, columns, and card dimensions. Add visual or DOM-level tests for reserved regions at desktop/tablet widths, especially retail, KDS, RestaurantMenu, and settings sections.

**Status:** Open

### LOAD-10 — Loading-state test coverage validates primitives and selected screens, not the cross-screen contract

**Evidence:** `Skeleton.test.tsx`, `Spinner.test.tsx`, animation compliance, and many screen tests provide useful local coverage. However, no discovered executable suite enforces that every async screen has distinct initial/error/empty states, a retry path, localized status messaging, cancellation protection, or `aria-busy` semantics. The existing `docs/ui-state-audit-2026-07-20.md` describes loading as comprehensive, but the current cross-screen evidence does not support that conclusion; this sector-23 report supersedes that verdict for current planning.

**Impact:** New screens can implement a visually plausible spinner while omitting error recovery, accessible announcements, or an empty-versus-failure distinction. Regressions can pass primitive tests and still fail the operator workflow.

**Severity:** P2 · quality assurance gap

**Affected files:** `ui/src/__tests__/Skeleton.test.tsx`, `ui/src/__tests__/Spinner.test.tsx`, `ui/src/__tests__/animationCompliance.test.ts`, and the async feature-screen test suites.

**Recommendation:** Add a lightweight static/runtime compliance gate for async screen patterns, with explicit exceptions for specialized flows. Add representative transition tests covering initial load, success-empty, success-data, refresh, failure, retry, unmount, and localization. Update the older UI-state audit to point to current evidence rather than asserting universal coverage.

**Status:** Open

## Positive controls observed

- `Skeleton` and `Spinner` primitives exist and use design-system classes.
- `Spinner` provides `role="status"` and a localized accessible label.
- `Button` exposes `aria-busy`, disables processing controls, and renders a hidden processing label.
- Several screens use faithful table/grid skeletons rather than a blank page.
- KDS, Retail POS, Audit Log, Offline Queue, and many management screens distinguish initial loading from data/empty rendering.
- `useProducts` and the Retail product loader include cancellation guards for their request paths.
- Animation compliance tests explicitly treat skeleton and spinner keyframes as essential feedback.
- A broad collection of screen tests covers loading and empty branches, providing a useful base for a shared contract.

## Test and validation results

Focused validation performed during this audit:

```text
cd ui
npx vitest run src/__tests__/Skeleton.test.tsx src/__tests__/Spinner.test.tsx src/__tests__/animationCompliance.test.ts
npm run typecheck
```

Results:

- Primitive and animation loading tests: **passed**; 3 files, 23 tests, 0 failures
- UI typecheck: **passed**; `tsc --noEmit` completed with 0 errors
- Report formatting/static evidence review: **passed**; no trailing whitespace and 10 findings reviewed
- Production code changed during this audit: **none**
- Existing unrelated staged loyalty changes were intentionally not modified.

The focused tests, once run, validate primitives and animation policy only; they do not negate LOAD-02, LOAD-03, LOAD-05, LOAD-07, or LOAD-10 because those findings require screen-level state-transition and failure-path coverage.

## Recommended remediation order

1. **LOAD-03:** Prevent demo catalog data from masquerading as live production inventory.
2. **LOAD-02/LOAD-08:** Standardize error-versus-empty state handling and make recovery actionable.
3. **LOAD-05/LOAD-06:** Create a localized, accessible loading wrapper and remove inconsistent fallback copy.
4. **LOAD-07:** Add cancellation/generation safety to loaders and refresh polling.
5. **LOAD-01/LOAD-04/LOAD-09/LOAD-10:** Consolidate primitives, preserve data during refresh, reduce layout shift, and establish a cross-screen compliance gate.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests and validation results.
