# Empty States Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Empty states — no-data views, no-results views, recovery actions, accessibility, localization, theming, and responsive consistency
> **Status:** AUDITED · cross-screen empty-state consistency and recovery findings require remediation
> **Production code changed:** None

## Scope

This audit evaluates sector 24 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers the distinction between a successful empty result and a failed request, search/filter no-results states, actionable onboarding, loading/error/empty transitions, accessibility semantics, localized copy, theme-token usage, responsive behavior, and automated coverage.

Inspected areas:

- `ui/src/components/EmptyState.tsx`
- `ui/src/frontend/shared/EmptyState.tsx`
- `ui/src/components/EmptyStateIllustrations.tsx`
- `ui/src/features/audit/AuditLogScreen.tsx`
- `ui/src/features/categories/CategoryManagementScreen.tsx`
- `ui/src/features/customers/CustomerManagementScreen.tsx`
- `ui/src/features/gift-cards/GiftCardsScreen.tsx`
- `ui/src/features/inventory/StockCountDetail.tsx`
- `ui/src/features/inventory/StockCountHistory.tsx`
- `ui/src/features/inventory/StockCountsScreen.tsx`
- `ui/src/features/kds/KdsHistoryPanel.tsx`
- `ui/src/features/kds/KdsLayoutFocus.tsx`
- `ui/src/features/kds/KdsLayoutKanban.tsx`
- `ui/src/features/kds/KdsLayoutMetro.tsx`
- `ui/src/features/kds/components/KdsProductPickerModal.tsx`
- `ui/src/features/loyalty/LoyaltyManagementScreen.tsx`
- `ui/src/features/offline/OfflineQueueScreen.tsx`
- `ui/src/features/products/ProductLookupScreen.tsx`
- `ui/src/features/products/ProductManagementScreen.tsx`
- `ui/src/features/reports/CustomReportScreen.tsx`
- `ui/src/features/reports/DashboardScreen.tsx`
- `ui/src/features/reports/InventoryReportScreen.tsx`
- `ui/src/features/reports/MenuEngineeringScreen.tsx`
- `ui/src/features/reports/SalesReportScreen.tsx`
- `ui/src/features/retail/RetailCartPanel.tsx`
- `ui/src/features/sales/PaymentModal.tsx`
- `ui/src/features/sales/SalesHistoryScreen.tsx`
- `ui/src/features/sales/VoidOrdersScreen.tsx`
- `ui/src/features/settings/FeatureToggleScreen.tsx`
- `ui/src/features/shifts/ShiftManagementScreen.tsx`
- `ui/src/features/staff/StaffManagementScreen.tsx`
- `ui/src/features/stock-transfers/StockTransfersScreen.tsx`
- `ui/src/features/terminals/TerminalManagementScreen.tsx`
- `ui/src/__tests__/EmptyState.test.tsx`
- `ui/src/__tests__/a11y/SalesHistoryScreen.a11y.test.tsx`
- `ui/src/__tests__/focusVisibleCompliance.test.ts`
- `ui/src/__tests__/noiseDitherCompliance.test.ts`

## Architecture summary

The repository contains two near-identical EmptyState implementations. `ui/src/components/EmptyState.tsx` is the older component and always renders an `h3`; `ui/src/frontend/shared/EmptyState.tsx` is the newer implementation and adds a configurable heading level. Both expose an optional icon, description, primary action, children, and `role="status"`.

Adoption is partial. Product management, sales history, shifts, and staff use the shared component, while many management, reporting, inventory, KDS, payment, and retail surfaces use feature-specific wrappers or plain localized paragraphs. Search-aware screens such as CustomerManagement and SalesHistory distinguish an empty database from zero matches, but this behavior is not a shared contract. The result is a visually and semantically mixed set of empty states across otherwise similar list and workflow screens.

## Findings

### EMPTY-01 — Duplicate EmptyState primitives create API and semantics drift

**Evidence:** `ui/src/components/EmptyState.tsx` and `ui/src/frontend/shared/EmptyState.tsx` expose overlapping props and markup. The shared implementation supports `headingLevel={1|2|3}`, while the component-path implementation always emits an `h3`. Both own the same `.empty-state` classes and `role="status"` behavior.

**Impact:** A screen importing the older path cannot participate in heading hierarchy improvements or future empty-state accessibility changes. Fixes to action behavior, live-region semantics, or tokenized layout can diverge between the two implementations.

**Severity:** P2 · design-system consistency

**Affected files:** `ui/src/components/EmptyState.tsx`, `ui/src/frontend/shared/EmptyState.tsx`, and consumers importing either path.

**Recommendation:** Keep one canonical implementation and convert the other path into a compatibility re-export. Preserve the `headingLevel` API, document the heading rule, and add a contract test proving both public import paths have identical output during the migration.

**Status:** Open

### EMPTY-02 — Empty-state rendering has no shared state contract

**Evidence:** Screens implement empty branches independently. `CustomerManagementScreen` and `SalesHistoryScreen` distinguish `collection.length === 0` from `filtered.length === 0`, while KDS layouts use the same `kds-no-orders` copy for an empty board and an empty filtered view. Inventory, reports, payments, retail, and management screens use a mixture of custom containers, plain paragraphs, and the shared component.

**Impact:** Equivalent user situations receive different copy, hierarchy, illustrations, spacing, and actions. More importantly, there is no enforceable contract requiring a screen to distinguish `loading`, successful empty, no search matches, stale data, and request failure.

**Severity:** P1 · workflow clarity

**Affected files:** `ui/src/frontend/shared/EmptyState.tsx`, `ui/src/features/kds/KdsLayoutFocus.tsx`, `ui/src/features/kds/KdsLayoutKanban.tsx`, `ui/src/features/kds/KdsLayoutMetro.tsx`, `ui/src/features/customers/CustomerManagementScreen.tsx`, `ui/src/features/sales/SalesHistoryScreen.tsx`, and the feature-specific empty branches listed in scope.

**Recommendation:** Define a small state matrix and shared variants: `no-data`, `no-results`, `error`, and `offline/unavailable`. Require each async list/grid to select a variant based on request outcome and active filters. Add representative transition tests rather than relying only on primitive tests.

**Status:** Open

### EMPTY-03 — Successful empty data and failed loading can be indistinguishable

**Evidence:** `CustomerManagementScreen.tsx:54-64` catches `listCustomersScoped` failures with only the comment `IPC unavailable`, then clears `loading`; its normal `customers.length === 0` branch at `CustomerManagementScreen.tsx:232-252` renders “No customers yet.” Similar silent initial-load catches were identified in `CategoryManagementScreen` and `SalesHistoryScreen`. Other feature loaders also render empty collections after a failed request.

**Impact:** A disconnected or failing backend can look like a valid empty tenant. Staff may assume records were deleted or never created, and creation/import decisions can be made on false information. There is no retry affordance in the affected branch.

**Severity:** P1 · operational correctness

**Affected files:** `ui/src/features/customers/CustomerManagementScreen.tsx`, `ui/src/features/categories/CategoryManagementScreen.tsx`, `ui/src/features/sales/SalesHistoryScreen.tsx`, plus other list screens with silent load catches.

**Recommendation:** Preserve a user-visible error state after an initial request failure and render a localized Retry action. Render a no-data state only after a successful zero-item response. Add tests that reject the first request and assert the error branch, separately from tests that resolve `[]` and assert the empty branch.

**Status:** Open

### EMPTY-04 — No-results copy is not consistently distinct from no-data copy

**Evidence:** Customer management and sales history have explicit search-empty branches with a clear-search action (`CustomerManagementScreen.tsx:253-265`; `SalesHistoryScreen.tsx:678-696`). In contrast, `ProductLookupScreen.tsx:328-336` and `KdsProductPickerModal.tsx:224-227` use generic no-results copy, while `KdsLayoutFocus.tsx:54-57` can show `kds-no-orders` after a status filter removes all tickets, even when orders exist in other columns. Report panels similarly reuse generic no-results copy for separate breakdowns.

**Impact:** Users may clear useful data, change configuration, or retry a backend request when the real issue is simply an active filter. In KDS, “No orders yet” is factually misleading when the selected status has no orders but other statuses are populated.

**Severity:** P2 · interaction clarity

**Affected files:** `ui/src/features/kds/KdsLayoutFocus.tsx`, `ui/src/features/kds/KdsLayoutKanban.tsx`, `ui/src/features/kds/KdsLayoutMetro.tsx`, `ui/src/features/products/ProductLookupScreen.tsx`, `ui/src/features/kds/components/KdsProductPickerModal.tsx`, `ui/src/features/reports/CustomReportScreen.tsx`, `ui/src/features/reports/InventoryReportScreen.tsx`, `ui/src/features/reports/MenuEngineeringScreen.tsx`, and `ui/src/features/reports/SalesReportScreen.tsx`.

**Recommendation:** Use filter-aware copy such as “No orders in this status” or “No products match your search,” with a localized clear/reset action where applicable. Add tests for an empty source and a non-empty source whose active filter produces zero matches.

**Status:** Open

### EMPTY-05 — Actionability is inconsistent across confirmed empty branches

**Evidence:** The shared EmptyState supports a primary `action`, and ProductManagement, SalesHistory, ShiftManagement, StaffManagement, and CustomerManagement provide creation or clear-search actions. Other inspected branches are only a `<p>` or `<div>`, including KDS history, payment customer search, and several report/inventory panels. This is a confirmed consistency gap; an action is not necessarily appropriate for every informational panel.

**Impact:** Where a safe next step exists, operators encounter dead-end feedback and must infer the action from surrounding controls. On touch terminals, this increases navigation cost and makes first-use onboarding less discoverable.

**Severity:** P2 · recovery and onboarding UX

**Affected files:** `ui/src/features/kds/KdsHistoryPanel.tsx`, `ui/src/features/sales/PaymentModal.tsx`, `ui/src/features/reports/*`, `ui/src/features/inventory/*`, and existing shared EmptyState consumers.

**Recommendation:** For each empty branch, explicitly record whether a primary action is applicable. Add a localized action for confirmed creation, clear-filter, retry, or return-to-board paths; leave genuinely informational states actionless. Test that each chosen action invokes the intended handler and remains keyboard/touch accessible.

**Status:** Open

### EMPTY-06 — Dynamic empty-state announcements are not covered by a shared contract

**Evidence:** Both shared EmptyState implementations use `role="status"`, and the SalesHistory accessibility suite covers one shared-component consumer. Many custom states are plain `<div>` or `<p>` elements, including KDS empty paragraphs, customer/category management wrappers, payment customer search, and report panels. A plain static paragraph is not automatically an accessibility defect, but dynamic search/filter transitions have no consistent announcement contract.

**Impact:** Screen-reader users may not be told that a result region became empty after a search, filter, or refresh. Conversely, adopting `role="status"` indiscriminately around large interactive regions could create noisy announcements; semantics need to be applied intentionally to the state message, not the entire table/grid.

**Severity:** P2 · dynamic accessibility contract

**Affected files:** `ui/src/frontend/shared/EmptyState.tsx`, `ui/src/components/EmptyState.tsx`, `ui/src/features/kds/KdsLayoutFocus.tsx`, `ui/src/features/kds/KdsLayoutKanban.tsx`, `ui/src/features/kds/KdsLayoutMetro.tsx`, `ui/src/features/customers/CustomerManagementScreen.tsx`, `ui/src/features/categories/CategoryManagementScreen.tsx`, `ui/src/features/sales/PaymentModal.tsx`, and report/inventory empty branches.

**Recommendation:** Define announcement behavior in the canonical component. Use a concise `role="status"`/`aria-live="polite"` message for dynamic no-results transitions, keep decorative icons hidden, and avoid marking the entire data region as a live region. Add axe and interaction tests for search/filter transitions; treat static informational paragraphs as a lower-priority consistency issue.

**Status:** Open

### EMPTY-07 — Empty-state fallback copy and terminology are inconsistent

**Evidence:** Many branches use Fluent `Localized` wrappers or `l10n.getString()` with fallback text, so the inspected English literals are not by themselves proof of missing localization. The confirmed issue is inconsistent fallback wording and punctuation across feature-specific branches: “No results,” “No results found,” “No orders yet,” “No customers yet,” and similar variants. A smaller set of custom branches requires bundle-parity verification to ensure the referenced keys have value-bearing messages.

**Impact:** Missing translations can fall back to English, and inconsistent terminology reduces scanability across management modules. Without a bundle check tied to these branches, a missing or attribute-only key can silently produce incomplete empty feedback.

**Severity:** P2 · localization consistency and verification

**Affected files:** `ui/src/features/categories/CategoryManagementScreen.tsx`, `ui/src/features/gift-cards/GiftCardsScreen.tsx`, `ui/src/features/products/VariantManagementScreen.tsx`, `ui/src/features/promotions/PromotionManagementScreen.tsx`, `ui/src/features/purchasing/PurchaseOrdersScreen.tsx`, `ui/src/features/purchasing/SuppliersScreen.tsx`, `ui/src/features/terminals/TerminalManagementScreen.tsx`, and report/inventory/KDS empty branches.

**Recommendation:** Standardize no-data versus no-results terminology and punctuation. Verify every referenced Fluent key is value-bearing and present in supported bundles; add keys where a custom branch lacks one. Keep English fallback children only where the project’s localization contract intentionally requires them, and cover empty-state keys with bundle-parity checks.

**Status:** Open

### EMPTY-08 — Empty-state layout and touch behavior are feature-specific

**Evidence:** The shared `.empty-state` style provides tokenized spacing and the global coarse-pointer rules enlarge buttons and form controls. However, many feature-specific empty wrappers have their own classes and do not use the shared component or shared minimum-height/action layout. KDS, reports, modal search results, and management tables consequently reserve different amounts of space and expose different touch affordances.

**Impact:** On tablet and POS displays, an empty panel may be visually underfilled, shift surrounding controls, or provide a smaller/less discoverable action than an equivalent shared state. The inconsistency is especially visible when switching between filtered and unfiltered views.

**Severity:** P3 · responsive consistency

**Affected files:** `ui/src/frontend/themes/components.css`, `ui/src/frontend/shared/EmptyState.tsx`, `ui/src/features/kds/*.tsx`, report/inventory empty wrappers, `ui/src/features/sales/PaymentModal.tsx`, and management-screen CSS files.

**Recommendation:** Establish a tokenized empty-state layout primitive with variants for full-page, table-region, grid-region, and modal-region use. Ensure actions use the shared Button touch sizing, preserve stable surrounding geometry, and add responsive DOM/visual checks at desktop and tablet widths.

**Status:** Open

### EMPTY-09 — Illustration usage is incomplete despite a theme-safe illustration set

**Evidence:** `EmptyStateIllustrations.tsx` provides reusable `currentColor` SVGs for products, sales, staff, shifts, search/no-results, and generic empty boxes. The shared component supports an icon, and ProductManagement, SalesHistory, ShiftManagement, and StaffManagement use illustrations. Many other empty states use no icon or a feature-specific inline icon, so equivalent no-data screens have different visual hierarchy and theming behavior.

**Impact:** Operators lose a consistent visual cue for the state and translators/designers must maintain many one-off implementations. This is lower risk than the data/error ambiguity, but it increases UI drift and makes future theme corrections more expensive.

**Severity:** P3 · visual consistency

**Affected files:** `ui/src/components/EmptyStateIllustrations.tsx`, `ui/src/frontend/shared/EmptyState.tsx`, and feature-specific empty-state consumers.

**Recommendation:** Treat illustrations as optional, not mandatory, but define a small mapping for common resource types and a no-results variant. Keep all icons `aria-hidden`, use `currentColor`/tokens, and avoid adding decorative art where the modal or dense table needs compact feedback.

**Status:** Open

### EMPTY-10 — Test coverage validates the primitive, not the cross-screen empty-state contract

**Evidence:** `ui/src/__tests__/EmptyState.test.tsx` covers title, description, action, icon hiding, children, and `role="status"`. The SalesHistory accessibility test covers one shared-component consumer. The repository search found no common executable test enforcing that every async list distinguishes error from successful empty, that filtered no-results states offer a clear/reset action, or that custom empty wrappers have equivalent semantics.

**Impact:** A screen can pass its local rendering tests while silently showing “empty” after a failed request, omitting retry/clear actions, or emitting nonlocalized copy. Primitive coverage therefore overstates confidence in the application-wide behavior.

**Severity:** P2 · quality assurance gap

**Affected files:** `ui/src/__tests__/EmptyState.test.tsx`, `ui/src/__tests__/a11y/SalesHistoryScreen.a11y.test.tsx`, and async feature-screen test suites.

**Recommendation:** Add representative contract tests for management tables, searchable grids, KDS filters, modal search, and reports. Cover successful empty, filtered no-results, rejected initial load, retry, localization, heading level, and accessible announcement behavior. A lightweight static inventory can flag new custom empty branches for review without forcing every specialized layout into one component.

**Status:** Open

## Positive controls observed

- A reusable EmptyState primitive exists with icon, description, action, and child-content support.
- The newer shared component supports configurable heading levels, addressing page hierarchy concerns for consumers that adopt it.
- Shared empty-state CSS uses design tokens for spacing, typography, and foreground colors.
- Empty-state illustrations use `currentColor`, tokenized color, and `aria-hidden="true"`.
- Customer management and SalesHistory correctly demonstrate the distinction between no records and no search matches.
- Product management, shifts, staff, sales history, and several other screens provide useful creation or reset actions.
- Offline Queue distinguishes loading, error, and successfully empty queue states and provides retry/sync controls.
- Primitive and at least one feature-level accessibility tests provide a foundation for broader contract coverage.

## Test and validation results

The audit was evidence-only; no production code was changed. Focused validation for the report is:

```text
if grep -nE '[[:blank:]]+$' audit/24-empty-states.md; then exit 1; fi
git diff --check -- audit/24-empty-states.md
```

Results:

- Source inventory and report evidence review: **completed**
- Report whitespace and `git diff --check`: **passed**
- Production code changed during this audit: **none**
- Empty-state unit/a11y contract expansion: **not performed**; existing tests cover the primitive and selected consumers only
- Full UI tests/typecheck: **not required for this documentation-only change**

The open findings are not claims that every listed screen is broken in the same way. They identify confirmed inconsistency or missing contract coverage and should be remediated with screen-specific tests before broad component consolidation.

## Recommended remediation order

1. **EMPTY-03:** Preserve load errors instead of rendering a misleading successful empty state.
2. **EMPTY-02/EMPTY-04:** Define and implement distinct no-data, no-results, error, and offline variants.
3. **EMPTY-07:** Localize remaining literal empty copy and standardize terminology.
4. **EMPTY-05/EMPTY-06:** Add safe actions and intentional live-region semantics.
5. **EMPTY-01/EMPTY-10:** Consolidate the duplicate primitive and establish representative cross-screen contract tests.
6. **EMPTY-08/EMPTY-09:** Harmonize responsive layout, touch affordances, and optional illustration usage.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests and validation results.
