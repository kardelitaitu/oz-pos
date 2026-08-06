# Reporting Module Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** 03 — Reporting module  
> **Status:** REMEDIATED IN PART · security boundary and limit validation implemented; financial/reporting UX findings remain open  
> **Scope:** Reporting screens, report API clients and Tauri commands, SQL aggregations, custom report/export paths, EOD exports, localization, accessibility, theming, performance, permissions, tests, and module documentation.

## Executive summary

The Reporting surface is broad and has a solid baseline: sales, dashboard, inventory, menu-engineering, custom-report, analytics-bundle, CSV, print, and EOD paths are present. The current focused front-end validation is green with **141 passing tests** across seven suites (five report screens plus the scoped IPC contract and mock compile-guard suites). The `oz-reporting` crate has **64 passing tests**, and `modules-reporting` has **12 passing tests**.

The audit found a critical security boundary problem: the primary reporting commands and custom report builder operate on `state.db` without a session token or backend permission check. The UI's navigation/role gating is not a security boundary, and custom reports can select customer, staff, sales, and shift data. The same unscoped pattern exists in legacy EOD/daily-summary commands, although scoped history variants are available.

The most important correctness problem is multi-currency aggregation in the UI. SQL groups revenue by currency, but the screens choose the first returned currency and sum all rows into one displayed total. A date range containing USD and IDR, for example, can be rendered as one meaningless USD total. Date filtering also uses SQLite `DATE(...)`/UTC-like string handling without an explicit store-timezone contract, and the screens do not reject an invalid start/end range.

Other significant gaps include stale-request races, unbounded custom-report results, incomplete refund/void semantics in the inspected reporting queries, CSV escaping inconsistencies, generic/no-retry error states, hardcoded locale and theme values, and incomplete keyboard semantics for report controls. The session-scoping and top-product bound remediations changed production code; the remaining findings below are still open.

## Architecture and data flow

- **Sales dashboard UI:** `ui/src/features/reports/DashboardScreen.tsx`.
- **Sales analytics UI:** `ui/src/features/reports/SalesReportScreen.tsx`.
- **Custom report UI:** `ui/src/features/reports/CustomReportScreen.tsx`.
- **Inventory report UI:** `ui/src/features/reports/InventoryReportScreen.tsx`.
- **Menu engineering UI:** `ui/src/features/reports/MenuEngineeringScreen.tsx`.
- **Frontend APIs:** `ui/src/api/reports.ts`.
- **Primary desktop commands:** `apps/desktop-client/src/commands/reports.rs`.
- **Legacy/EOD export commands:** `apps/desktop-client/src/commands/history.rs`.
- **Core report queries:** `crates/oz-core/src/db/reports.rs`.
- **Custom reports and analytics exports:** `crates/oz-core/src/export/mod.rs`.
- **Additional reporting crate:** `crates/oz-reporting/src/daily_summary.rs` and `menu_engineering.rs`.
- **Reporting module:** `modules/reporting/src/lib.rs` and `handlers.rs`.
- **Report locales:** `ui/src/locales/reports.ftl`, `reports.id.ftl`, `inventory.ftl`, and `inventory.id.ftl`.
- **Command registration:** only session-scoped report commands are registered in `apps/desktop-client/src/lib.rs` and `apps/tablet-client/src/lib.rs`; legacy unscoped implementations remain as deprecated Rust functions for compatibility but are not exposed through the Tauri handler lists.
- **Tests:** `ui/src/__tests__/SalesReportScreen.test.tsx`, `DashboardScreen.test.tsx`, `CustomReportScreen.test.tsx`, `InventoryReportScreen.test.tsx`, `crates/oz-reporting` tests, `oz-core` report/export tests, and `modules/reporting` tests.

## Findings

### REP-01 — Primary reporting commands bypass session scope and backend authorization

**Severity:** P0 — tenant-isolation and sensitive-data exposure risk  
**Status:** Implemented for the registered Tauri surface; legacy functions retained but unregistered

**Original evidence:**

- The former primary commands accepted `start_date`, `end_date`, `limit`, or a `CustomReportRequest` without a `session_token`.
- They read the process-global `state.db` without `resolve_store` or `require_permission_for_user`.
- `CustomReportRequest` supports `sales`, `inventory`, `customers`, `staff`, `tax_rates`, and `shifts`, including customer contact fields and staff records.

**Implemented evidence:**

- Desktop and tablet now expose `_scoped` commands for menu engineering, daily/weekly/monthly revenue, top products, hourly heatmap, low-stock alerts, category breakdown, and custom reports.
- Each scoped command resolves the opaque session token, derives the store from `session.store_id`, and checks the authenticated user's database role before reading the store database.
- Desktop and tablet `invoke_handler!` lists register only the scoped report commands; the old global functions are retained but no longer form part of the Tauri IPC surface.
- `ui/src/api/reports.ts` and all report screens/widgets now pass the active workspace session token and invoke the scoped command names.
- Focused desktop and tablet scope-resolver tests cover invalid-session rejection and denial for a seeded custom role with no `reports:view` permission; existing state tests cover store-database isolation. A full command-level two-store fixture remains follow-up coverage.

**Impact:** A renderer or compromised/incorrect caller that can invoke a report command may read data from the process-global database without the intended store boundary or report permission. The custom report path materially expands the exposure surface beyond aggregate sales metrics to customer, staff, tax, and shift data.

**Residual follow-up:** Remove the retained unscoped Rust functions after downstream/internal callers are confirmed absent. Review the dataset policy for custom reports: the current scoped builder requires `reports:export` because it can expose customer/staff data, while aggregate report commands require `reports:view`. Add a real command-level two-store fixture and dataset-specific permissions before declaring this finding fully closed.

### REP-02 — Revenue UI combines different currencies into one displayed total

**Severity:** P0 — financial reporting integrity risk  
**Status:** Open

**Evidence:**

- `crates/oz-core/src/db/reports.rs` correctly includes `currency` in the daily, weekly, and monthly result and groups by the time bucket plus currency.
- `SalesReportScreen.tsx` sets `currency` to `revenueData[0].currency` or `USD`, then reduces every `revenueData` row into one `totalRevenue` and formats the sum using that first currency.
- The same screen sums all rows for `totalOrders` and comparison totals without partitioning by currency.
- `DashboardScreen.tsx` similarly selects `revenue[0].currency` and sums all returned rows into one KPI; its weekly bars can display rows from multiple currencies against one `maxWeekly` scale.

**Impact:** A multi-currency date range can show a mathematically invalid total and misleading comparison/KPI values. The backend's currency grouping prevents SQL arithmetic from mixing currencies, but the front end defeats that protection when it collapses the rows.

**Recommendation:** Choose an explicit product policy: restrict a report to one currency, render separate totals/series per currency, or convert using a recorded exchange-rate policy. Never sum minor units across currencies. Add UI and API tests with two currencies in the same period and ensure charts, totals, exports, and comparisons preserve the partition.

### REP-03 — Date boundaries have no store-timezone contract or input validation

**Severity:** P1 — period totals can be assigned to the wrong business day  
**Status:** Open

**Evidence:**

- The core queries filter with `DATE(created_at) BETWEEN ?1 AND ?2` and group with SQLite `DATE`, `strftime`, or `SUBSTR` expressions.
- `SalesReportScreen.tsx`, `DashboardScreen.tsx`, and `CustomReportScreen.tsx` derive dates with `toISOString().slice(0, 10)`, which is UTC rather than the store's configured business timezone.
- The UI permits `startDate > endDate` and does not present a validation error; the query then returns an empty result that can look like no activity.
- The custom report backend appends `23:59:59` to the end date but does not validate ISO format, timezone, or start/end ordering.

**Impact:** Sales near midnight can appear on the adjacent business date for stores outside UTC. Invalid or malformed dates can silently produce empty or surprising results. EOD and dashboard numbers can disagree with a cashier's local business day.

**Recommendation:** Define a store timezone/business-day policy and apply it consistently in SQL or in a validated UTC range generated from that timezone. Validate `YYYY-MM-DD`, reject impossible dates and reversed ranges, and return a field-specific error before querying. Add boundary tests around midnight, DST transitions where applicable, and reversed/invalid input.

### REP-04 — Report queries do not show explicit refund/void/net-sales treatment

**Severity:** P1 conditional — financial decision-making risk  
**Status:** Open

> The inspected sources prove that the reporting queries contain no refund/void reconciliation logic. They do not, by themselves, prove that every refund currently leaves the original sale in `completed` status; that behavior must be verified against the sale/refund state machine.

**Evidence:**

- The inspected revenue, top-product, hourly, and category queries filter on `status = 'completed'` and aggregate `sales.total_minor` or sale-line values.
- The queries do not join a refund/void ledger or subtract reversal amounts. The inspected reporting command layer has no parameter for gross/net mode.
- Legacy EOD logic in `apps/desktop-client/src/commands/history.rs` also filters completed sales for payment breakdowns.
- The repository contains separate refund/void flows, but no reporting reconciliation path was found in the inspected report query code that ties those reversals back to each aggregate.

**Impact:** If a refund or void leaves the original sale completed and records the reversal separately, reports can overstate revenue, orders, product volume, and category contribution. Even if current sale-status transitions happen to exclude some reversals, the reporting contract is implicit and untested.

**Recommendation:** Define gross, refund, void, and net metrics explicitly. Build aggregates from an auditable sales/reversal ledger or apply a documented status/reversal join. Include refund/void counts and amounts in EOD and exports. Add fixtures covering completed sale, partial refund, full refund, void, and repeated reversal events.

### REP-05 — Current product/category joins can erase or rewrite historical sales attribution

**Severity:** P1 — historical-report consistency risk  
**Status:** Open

**Evidence:**

- `top_products` joins `sale_lines` to the current `products` table by SKU; the existing core test explicitly verifies that deleting the product makes the historical row disappear.
- `category_breakdown` also joins sale lines to current products and current categories rather than using an immutable sale-time product/category snapshot.
- The sales model preserves line amounts and SKUs, but the report query does not use a preserved product name/category snapshot for the display dimensions.

**Impact:** Deleting a product removes its historical sales from top-product results. Renaming or moving a product to another category can rewrite prior-period category reports. Historical financial reports should remain stable after catalog maintenance.

**Recommendation:** Preserve sale-time product name, category, and relevant cost/dimension values on sale lines, or maintain a historical dimension table. Use those snapshots for reporting and define behavior for legacy rows. Add tests for product deletion, rename, SKU reuse, and category reassignment after a completed sale.

### REP-06 — Report fetches can race and older responses can overwrite newer filters

**Severity:** P1 — stale report data can be presented as current

**Evidence:**

- `SalesReportScreen.tsx` calls four independent APIs in `Promise.all` whenever view or date state changes.
- There is no request sequence ID, abort signal, or mounted/current-filter guard before applying `setRevenueData`, `setTopProducts`, `setHeatmap`, and `setCategoryBreakdown`.
- A user can change dates or view modes before the previous promises resolve. Whichever request completes last can update the screen, regardless of which filter is currently selected.
- `CustomReportScreen.tsx` disables the Run Report button while `loading` is true, so repeated clicks are normally prevented; however, responses are not associated with the filter/dataset state that initiated them. Changing inputs while a request is in flight can leave stale results visible.

**Impact:** A report can display values for one date range or dataset while its controls show another. This is particularly dangerous for financial decisions because the screen remains visually valid.

**Recommendation:** Add an abort/request-generation guard around every fetch and ignore results from superseded requests. Associate each custom-report response with the request state that created it, and keep the run button disabled while the request is active. Add deterministic deferred-promise tests that resolve requests out of order.

### REP-07 — Custom reports are unbounded and can exhaust memory/IPC capacity

**Severity:** P1 — availability/performance risk

**Evidence:**

- `Store::build_custom_report` constructs a dynamic `SELECT` and collects every returned row into `Vec<Vec<String>>`.
- The `CustomReportRequest` has no page size, offset, maximum row count, or streaming export mode.
- `CustomReportScreen.tsx` renders all returned rows in one table and exports the complete response in the browser.
- Date filters are optional for some datasets, so a report can request all customers, staff, inventory, or tax rows in one IPC response.

**Impact:** Large stores can cause expensive SQLite scans, large serialized IPC payloads, high browser memory use, and slow table rendering. Sensitive datasets are also copied into the renderer unnecessarily.

**Recommendation:** Enforce server-side maximum rows and pagination, with an explicit “truncated” indicator. Provide a streaming/file-based export path for large CSVs and require narrower permissions for unbounded export. Add query-plan/index review and representative-volume performance tests.

### REP-08 — CSV escaping is inconsistent across report screens

**Severity:** P1 — export correctness and data-integrity risk

**Evidence:**

- `CustomReportScreen.tsx` correctly quotes every cell and doubles embedded quotes.
- `SalesReportScreen.tsx` joins period, numeric value, currency, and count without CSV escaping. These current fields are mostly controlled, but the format is not robust to future text dimensions or unexpected currency values.
- `InventoryReportScreen.tsx` wraps the product name in quotes but does not double embedded quotes. A product name containing `"` can produce malformed CSV; SKU and other fields are not passed through a shared escape helper.
- Core Rust export code has a `csv_cell` helper, but the browser exports do not share that implementation.

**Impact:** Spreadsheet imports can shift columns, truncate values, or misinterpret product names. Export consumers cannot rely on a consistent format across report screens.

**Recommendation:** Use one tested CSV-escape helper for every browser export, including CR/LF, commas, quotes, and formula-injection policy. Prefer the core export path for large or sensitive data. Add fixtures with commas, quotes, newlines, Unicode, and cells beginning with `=`, `+`, `-`, or `@`.

### REP-09 — Error states are generic, non-retryable, and inconsistently localized

**Severity:** P1 — operational recovery gap

**Evidence:**

- `DashboardScreen.tsx` renders a generic localized “An error occurred” after any of four requests fail, with no retry action and no indication of which dataset failed.
- `SalesReportScreen.tsx` renders the same generic message and does not expose the captured error or a retry button.
- `InventoryReportScreen.tsx` displays the raw exception text directly, which may be technical or English and is not consistently localized.
- `CustomReportScreen.tsx` also appends raw error text and has no retry-specific state.
- Focused tests cover rejection rendering but do not cover retry behavior or partial failure recovery.

**Impact:** Operators cannot recover without navigating away or changing a filter, and backend implementation details can leak into the UI. Partial data failure is not distinguished from a genuinely empty report.

**Recommendation:** Add typed/localized error categories, a retry button, and `role="alert"`. Preserve the last successful data while clearly marking it stale, or isolate errors by card/dataset. Add retry, partial-failure, and locale tests.

### REP-10 — Locale handling is incomplete and hardcoded to English in report presentation

**Severity:** P2 — localization/accessibility inconsistency

**Evidence:**

- `SalesReportScreen.tsx` and `DashboardScreen.tsx` use `new Intl.NumberFormat('en', ...)` rather than the active application locale.
- `SalesReportScreen.tsx` hardcodes `DAY_NAMES = ['Sun', 'Mon', ...]` and uses raw `mode` values as radio `aria-label`s.
- `CustomReportScreen.tsx` hardcodes dataset names and every column label in the `DATASETS` constant, so the selector and result headers are not Fluent-backed.
- `reports.id.ftl` contains dashboard/menu/custom-report translations and report accessibility keys, but does not define the full English sales-report key set such as the title, view labels, and section labels. Several screens therefore rely on English JSX fallback children.
- `InventoryReportScreen.tsx` uses some localized labels but still exposes raw API errors.

**Impact:** Indonesian users receive mixed-language labels, dates, currency formatting, and screen-reader names. Report numbers may use the wrong separators and currency presentation for the active locale.

**Recommendation:** Pass the active locale and store currency policy to shared formatters. Localize day names and view-mode accessible names. Move dataset/column display labels to Fluent keys with stable identifiers in code. Run bundle-parity and attribute/value audits for both report and inventory bundles.

### REP-11 — Theme-token and contrast violations remain in charts and heatmaps

**Severity:** P2 — dark-mode and contrast risk

**Evidence:**

- `SalesReportScreen.tsx` defines hardcoded `PIE_COLORS` and `HEATMAP_COLORS` palettes.
- The chart fill uses `var(--color-accent, #4f46e5)` and the zero-value heatmap cell uses `var(--color-bg-hover, #f3f4f6)` fallbacks.
- Heatmap cell background colors are applied inline, with no contrast validation for the cell's text/accessible representation or theme adaptation.
- Skeleton and comparison presentation also use inline style values where shared CSS classes could enforce theme behavior.

**Impact:** Fixed palettes may be low-contrast in dark mode or against custom themes. Token-compliance tooling cannot validate arbitrary chart colors consistently, and a theme change can make the visual scale misleading.

**Recommendation:** Define semantic report-chart tokens in the theme system, with light/dark variants and tested contrast. Keep computed intensity as a CSS custom property over a tokenized scale. Add automated token and contrast checks for report screens.

### REP-12 — Keyboard and screen-reader semantics are incomplete for report controls

**Severity:** P2 — accessibility gap

**Evidence:**

- The sales heatmap uses `role="grid"`, rows, and gridcells, but cells are not focusable and there are no keyboard navigation semantics or column headers exposed as grid headers.
- The custom report column list uses `role="listbox"`/`role="option"` and draggable `div`s, but drag-and-drop reordering has no keyboard equivalent. Checkbox selection remains keyboard-operable, but the option container does not own that selection interaction.
- The custom report results table has no caption or accessible table label.
- Dataset labels, column labels, and sales view radio labels are partly hardcoded rather than localized accessible names.

**Impact:** Keyboard users cannot fully reproduce mouse drag/reorder workflows, while screen readers may receive ambiguous or incomplete context for chart/table data.

**Recommendation:** Prefer native table semantics for tabular results, add captions/labels, and provide a button-based move-up/move-down alternative for column ordering. Either implement full grid keyboard behavior or use a simpler accessible table/list representation for the heatmap. Add automated keyboard and axe coverage.

### REP-13 — Print actions use misleading zero-priced receipt data and lack failure feedback

**Severity:** P2 — operational output correctness gap

**Evidence:**

- `SalesReportScreen.tsx::printReport` maps top products to receipt items with `unitPrice` set to zero and labels the payment method `Report`.
- `InventoryReportScreen.tsx::printReport` sends all inventory items with USD zero prices and a zero total through `printSalesReceipt`.
- Both print handlers await the API without a local `try/catch` or localized failure state.

**Impact:** A kitchen/receipt printer may produce output that resembles a financial receipt while showing zero prices and a fake payment line. Hardware failures can become unhandled promise rejections or provide no recovery guidance.

**Recommendation:** Use a dedicated report-print payload/template rather than a sales-receipt contract. Clearly label non-financial report output, omit payment fields, use the report's actual currency where applicable, and show localized print success/failure feedback. Add printer-mock tests for payload shape and rejection.

### REP-14 — Reporting architecture and documentation have overlapping paths

**Severity:** P2 — maintenance and contract drift

**Evidence:**

- Report functionality is split among `apps/desktop-client/src/commands/reports.rs`, legacy/EOD commands in `commands/history.rs`, `crates/oz-core/src/db/reports.rs`, `crates/oz-core/src/export/mod.rs`, `crates/oz-reporting`, and `modules/reporting`.
- `modules/reporting` tests primarily exercise module lifecycle and event-handler recording, while the live report query path is in `oz-core`; this does not provide a single authoritative reporting service boundary.
- The custom report type documentation in `oz-core/src/export/mod.rs` still describes the dataset contract as “sales or inventory” in its request comment while the implementation supports six datasets and the UI exposes six.
- Legacy history functions are documented as deprecated for multi-store and scoped variants exist, but the primary reports commands have no equivalent scoped API in the inspected sources.

**Impact:** Contributors can update one report implementation while another export path remains inconsistent. Documentation and command contracts can drift, and security fixes may be applied to scoped history paths but missed by primary report commands.

**Recommendation:** Establish one reporting service/query boundary with explicit scoped command DTOs, shared date/currency/reversal semantics, and one export contract. Mark legacy global commands deprecated in registration and docs, or remove them after migration. Add API-reference parity and report-contract tests.

### REP-15 — Focused tests omit the highest-risk reporting invariants

**Severity:** P2 — regression-detection gap

**Evidence:**

- The four focused UI suites pass 79 tests covering normal rendering, loading/error/empty states, filters, chart presence, CSV/print invocation, and basic ARIA.
- They do not cover multi-currency totals, reversed date ranges, timezone boundaries, stale out-of-order requests, retry/partial failures, CSV special characters, localized number/day labels, theme contrast, keyboard drag alternatives, or print rejection.
- Core report tests cover completed/non-completed status, date ranges, multiple currencies at the SQL row level, deleted products, limits, zero revenue, and several boundaries, but the UI does not preserve those invariants when aggregating rows.
- The reporting module tests pass 12 tests but focus on module lifecycle and handler storage rather than the primary `oz-core` reporting commands and authorization boundary.

**Recommendation:** Add tests in this order: command scope/permission isolation, UI multi-currency rendering, out-of-order request protection, date/timezone validation, refund/void accounting, CSV escaping, print failures, localized labels/formatters, and accessible keyboard interactions. Add integration tests that invoke the real Tauri command boundary rather than only mocked APIs.

### REP-16 — Top-product limit is not validated before reaching SQLite

**Severity:** P2 — unbounded-query risk  
**Status:** Implemented for the registered scoped Tauri surface; legacy function retained but unregistered

**Original evidence:**

- The former unscoped command accepted an unrestricted `i64 limit` and forwarded it directly to `Store::top_products`. The retained function remains unregistered and is scheduled for removal after downstream callers are confirmed absent.
- `crates/oz-core/src/db/reports.rs::top_products` binds that value directly to SQLite's `LIMIT ?3` clause.

**Implemented evidence:**

- Both desktop and tablet scoped commands validate the limit before opening the store database.
- Valid values are explicitly bounded to `1..=100`; zero, negative, and oversized values return `AppError::Invalid`.
- Unit tests in both command modules cover the lower bound, upper bound, zero, negative, oversized, and `i64::MAX` inputs.

**Impact:** An invalid or hostile limit can turn a bounded report query into a large result set, increasing database work, serialization, IPC payload size, and renderer memory use.

**Status note:** The registered scoped path is remediated. The retained deprecated global function still accepts its historical unrestricted limit and should be removed with the legacy functions after callers are migrated.

## Positive observations

- Revenue SQL groups by currency, preventing direct database-level arithmetic across currencies.
- Report queries use parameterized date/limit values, and the custom-report builder validates dataset and column identifiers through hardcoded whitelists rather than interpolating arbitrary renderer input.
- Custom report CSV export correctly doubles embedded quotes and quotes cells consistently.
- The screens provide loading, successful-empty, and basic error branches; the sales and inventory report screens include loading skeletons.
- Sales reports expose daily/weekly/monthly modes, category breakdown, top products, hourly heatmap, CSV export, and print actions.
- Core report tests cover non-completed status exclusion, zero totals, date boundaries, leap-day handling, multiple currencies as separate rows, limits, and several historical-data edge cases.
- Focused validation is green: 141 report/UI contract tests, 64 `oz-reporting` tests, and 12 `modules-reporting` tests passed.

## Recommended implementation order

1. **Security boundary:** ✅ registered primary report/custom-report commands are session-scoped and enforce dataset-specific report/export permissions. Remove retained legacy functions and add command-level two-store isolation coverage.
2. **Financial correctness:** preserve currency partitions, define timezone and date validation, and implement explicit gross/refund/void/net semantics.
3. **Historical stability:** use sale-time dimension snapshots for product/category reports.
4. **Concurrency and scale:** guard stale requests and add server-side pagination/limits or streaming exports.
5. **Operational UX:** add localized typed errors, retry/partial-failure states, and a dedicated non-financial print contract.
6. **Localization, theming, and accessibility:** use active locale formatters, Fluent-backed labels, semantic chart/table patterns, keyboard alternatives, and theme tokens.
7. **Architecture and QA:** consolidate overlapping reporting paths and add command-boundary, multi-currency, reversal, export, and accessibility regression suites.

## Validation performed

- `cd ui && npm run typecheck && npx vitest run src/__tests__/reports-ipc-contract.test.ts src/__tests__/MockFactoriesCompile.test.tsx src/__tests__/SalesReportScreen.test.tsx src/__tests__/DashboardScreen.test.tsx src/__tests__/CustomReportScreen.test.tsx src/__tests__/InventoryReportScreen.test.tsx src/__tests__/MenuEngineeringScreen.test.tsx` — **141 passed, 0 failed**.
- `cd ui && npm run typecheck` — passed with no TypeScript errors.
- `cargo check -p oz-pos-app --lib` — passed.
- `cargo check -p oz-pos-tablet --lib` — passed.
- `cargo test -p oz-pos-app --lib commands::reports::tests` — **4 passed, 0 failed**.
- `cargo test -p oz-pos-tablet --lib commands::reports::tests` — **4 passed, 0 failed**.
- The focused screen suites and scoped API tests are green: **141 passed, 0 failed** across five report screens, the scoped IPC contract, and the mock compile guard.
- Source inspection covered report screens, API client, desktop/tablet commands, command registration, scoped/legacy history exports, core SQL reports, custom/analytics exports, reporting crate, module handlers, locale bundles, and focused tests.

## Fix status

The REP-01 registered IPC security boundary and REP-16 scoped limit validation are implemented and validated. Legacy unscoped Rust functions remain intentionally retained but unregistered; removing them and addressing REP-02 through REP-15 remain follow-up work. No claim is made that the entire reporting audit is closed.
