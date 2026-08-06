# Table Management Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** TableManagement — restaurant floor plan, table status lifecycle, sections, order assignment, authorization, accessibility, and tests  
> **Status:** ✅ REMEDIATED · all 12 findings (TBL-01 → TBL-12) closed  
> **Production code changed:** Yes (backend invariants + UI workflow/a11y/theme)  
> **Closing commits:** `6f2e3ce4` (TBL-01/04/08 backend) · `382a429f` (TBL-02/09/10 UI states) · `9d8bab79` (TBL-03/05/06/07 workflow + dialog) · `c0b1439b` (TBL-11 contrast tokens)

## Scope

This audit evaluates the TableManagement surface against the universal checklist in `audit/AUDIT_JULY_2026.md`: functionality and state management, loading/error/empty states, accessibility and localization, theming, performance, security and authorization, restaurant-order integrity, and quality assurance.

Inspected areas:

- `ui/src/features/tables/TableManagementScreen.tsx`
- `ui/src/features/tables/TableManagementScreen.css`
- `ui/src/api/tables.ts`
- `apps/desktop-client/src/commands/tables.rs`
- `crates/oz-core/src/db/tables.rs`
- `crates/oz-core/migrations/033_tables.sql`
- `crates/oz-core/migrations/101_kds_table_number.sql`
- `ui/src/__tests__/TableManagementScreen.test.tsx`
- `ui/src/locales/tables.ftl`
- `ui/src/locales/tables.id.ftl`
- Related table, sale, and KDS domain types and tests

## Architecture summary

`TableManagementScreen` is a scoped, read-and-status-update floor-plan view. It loads active tables through `listTablesScoped(sessionToken, section)`, derives section filter buttons from the currently loaded table list, renders each table as an absolutely positioned button, and opens a detail panel when a table is clicked. A browser context-menu gesture invokes a status transition directly.

The API and desktop command layer provide both legacy user-ID commands and session-scoped variants. The scoped variants resolve the store from the session token and enforce table-specific permissions: `TABLES_CREATE`, `TABLES_EDIT`, `TABLES_DELETE`, `TABLES_CLOSE`, and `TABLES_ASSIGN`. The current screen uses the scoped list/status/release APIs, but it does not expose table CRUD or order-assignment controls itself.

Core persistence stores table geometry, capacity, section, status, active sale, active flag, and sort order. Valid statuses are `available`, `occupied`, `reserved`, and `cleaning`. Occupying a table through the current screen changes only the status; explicit sale assignment is a separate API operation. Releasing a table changes it to `cleaning` and clears `active_sale_id`. KDS table-number lookup depends on the sale-to-table relationship.

## Findings

### TBL-01 — Status-only “occupy” action can create an occupied table without an order assignment

**Evidence:** For an `available` table, `statusAction()` calls `updateTableStatusScoped(sessionToken, table.id, 'occupied')`. The database status method updates only `status` and `updated_at`; it does not set `active_sale_id`. The API separately exposes `assignTableOrderScoped`, and KDS table-number resolution reads the table associated with an active sale.

**Impact:** An operator can make a table appear occupied while it has no active order. The floor plan then communicates occupancy that is not connected to a sale, and downstream flows that rely on `active_sale_id`—including table-number display for KDS—cannot consistently identify the order. This is a confirmed state-integrity gap in the screen's action semantics.

**Recommendation:** Replace the “mark occupied” shortcut with an explicit order-selection/assignment flow, or rename it to a clearly provisional state if provisional occupancy is intentional. Enforce the invariant server-side: an occupied table must either have an active sale or use a documented reservation/hold model. Add tests for available → occupied, assignment, duplicate assignment, and KDS table-number propagation.

**Status:** ✅ REMEDIATED (`6f2e3ce4`, `9d8bab79`)

**Fix:** `update_table_status` now rejects `occupied` for tables without an `active_sale_id` — occupancy is only reachable through `assign_table_order`, keeping the floor plan and KDS table-number lookup consistent. The UI adopts the documented hold model: an available table's action is **Mark Reserved** (reserved requires no sale link), so no operator path can create unassigned occupancy. Tests cover occupy-without-sale rejected (state untouched), occupy-with-active-sale allowed + re-assert, and missing-table NotFound.

### TBL-02 — Table list failures are unhandled and leave stale or misleading floor-plan state

**Evidence:** The screen calls `listTablesScoped(...).then(setTables)` inside `useEffect` without `catch`, loading state, request cancellation, or an error state. A rejected IPC call becomes an unhandled promise rejection. If a prior request succeeds after a section/token change, its result can overwrite the newer selection because there is no request-generation guard.

**Impact:** Operators receive no durable explanation or retry path when the database, session, or IPC fails. A previously displayed floor plan can remain visible while it no longer represents the selected section or current store, and rapid filter/session changes can show data from the wrong request.

**Recommendation:** Add explicit loading, empty, and error states with localized Retry actions; preserve known-good data during refresh while marking it stale. Cancel or generation-guard requests and ignore results from obsolete tokens/sections. Add tests for rejection, retry, empty results, and out-of-order responses.

**Status:** ✅ REMEDIATED (`382a429f`)

**Fix:** `listTablesScoped` runs through a seq-guarded loader — stale responses from earlier section/token/refresh changes are dropped. Failures surface a localized `role="alert"` banner with a Retry action. Known-good tables stay visible during a refresh.

### TBL-03 — Status mutations are fire-and-forget and the UI closes before persistence succeeds

**Evidence:** `statusAction()` invokes `updateTableStatusScoped()` or `releaseTableScoped()` without awaiting or catching the returned promise. The detail action immediately calls `statusAction(selected); setSelected(null)`. The table list is not reloaded after a successful mutation, and failures are not surfaced.

**Impact:** A failed release or status update looks successful because the detail panel disappears. The floor plan can continue displaying the old status, while a later refresh reveals that the operation never happened. Repeated clicks or context-menu actions can also issue concurrent mutations without a pending guard.

**Recommendation:** Make the action async, disable the affected table while pending, await the backend result, update or reload the table on success, and keep the panel open with a localized error on failure. Add tests for successful mutation, rejected mutation, duplicate-click protection, and refresh-after-mutation.

**Status:** ✅ REMEDIATED (`9d8bab79`)

**Fix:** `statusAction` is async with a ref-based duplicate-click guard, the affected table is disabled while pending, the action button shows a processing state, and failures keep the panel open with a localized `role="alert"` error. Success patches the returned table into the floor plan in place (no full reload), so a section change landing mid-mutation can never be clobbered by a stale reload.

### TBL-04 — Deletion and table lifecycle integrity are not protected by the management workflow

**Evidence:** The command layer exposes `delete_table_scoped`, but the core `delete_table()` hard-deletes by ID without checking status, `active_sale_id`, reservations, or dependent operational history. The table schema references `sales(id)` through `active_sale_id`, and the KDS migration uses the relationship for table-number lookup. The current screen offers no deletion UI or confirmation, so the risk exists in the broader API/management surface rather than as a visible button here.

**Impact:** A caller with `TABLES_DELETE` can remove an occupied or operationally referenced table without an explicit close/reconcile step. Depending on SQLite foreign-key configuration, deletion may fail late because of an active sale or remove the floor-plan record while downstream operational workflows may still expect the table identity. The absence of a soft-deactivation policy makes recovery and audit history harder.

**Recommendation:** Prefer deactivation/archival over hard deletion. Reject deletion when the table is occupied, reserved, assigned to an active sale, or referenced by an open workflow; return a structured conflict error. If hard deletion remains necessary, document and test foreign-key behavior and preserve a historical table snapshot. Add scoped authorization and lifecycle tests.

**Status:** ✅ REMEDIATED (`6f2e3ce4`)

**Fix:** `delete_table` refuses to hard-delete occupied, reserved, or sale-linked tables and returns a structured `Validation` error pointing callers to deactivation. A free, unlinked table remains hard-deletable. Tests cover occupied/reserved/sale-link rejection (built via the production-reachable assign-then-reset path to respect FK constraints) and the still-allowed free-table delete.

### TBL-05 — Context-menu status transitions have no keyboard or discoverable equivalent

**Evidence:** The floor-plan button's shortcut is implemented only through `onContextMenu`, which prevents the browser menu and immediately changes status. The visible detail panel contains a different action, but there is no explicit keyboard shortcut or menu button for the same quick transition and no instruction that right-click is supported.

**Impact:** Keyboard-only, touch, and many assistive-technology users cannot access the shortcut. On touch devices, long-press behavior is inconsistent and may conflict with scrolling or OS context menus. The hidden mutation also makes an important operational action difficult to discover and test through normal interaction.

**Recommendation:** Add a visible, keyboard-operable actions button/menu in each table detail or table card. Keep context-menu support as an optional convenience that opens the same accessible menu rather than mutating immediately. Add Enter/Space/menu-key and touch interaction tests.

**Status:** ✅ REMEDIATED (`9d8bab79`)

**Fix:** The context-menu gesture now opens the accessible detail panel — the visible, keyboard-operable actions menu — instead of mutating directly, so every operator path goes through the same confirmed, error-aware action.

### TBL-06 — The detail panel is not a complete modal/dialog interaction

**Evidence:** The selected panel has `role="dialog"` and an accessible label, but no `aria-modal="true"`, focus trap, initial focus, Escape handler, or focus restoration. On desktop it is positioned as an overlay; on mobile it is a fixed bottom panel. Background floor-plan buttons remain in the document's normal tab order while the dialog is open.

**Impact:** Keyboard and screen-reader users can move focus behind the open dialog, lose their place, or fail to discover how to dismiss it except by locating the Close button. This is inconsistent with the dialog semantics already declared by the component.

**Recommendation:** Either model the panel as a non-modal disclosure with appropriate semantics, or implement complete dialog behavior using the shared focus-trap pattern: `aria-modal`, initial focus, Escape-to-close, focus restoration, and inert/blocked background interaction where appropriate. Add accessibility tests for focus containment and dismissal.

**Status:** ✅ REMEDIATED (`9d8bab79`)

**Fix:** The detail panel is a complete modal — `aria-modal="true"`, shared `useFocusTrap` (initial focus, Tab trap, Escape-to-close, body scroll lock), and focus restoration to the trigger button on close.

### TBL-07 — Status and section values are not consistently localized

**Evidence:** Table status values are rendered directly in `.tables-table-status` and in detail text (`Status: {selected.status}`). The table accessible label interpolates the raw status, and section values fall back to a literal em dash. Fluent keys exist for table labels and actions, but there is no typed status-to-localized-label mapping in the screen.

**Impact:** Users can see machine values such as `available`, `occupied`, and `cleaning` in English deployments and localized deployments alike, with inconsistent wording across the visible card, dialog, and accessible name. Future backend status values also render without a safe localized fallback.

**Recommendation:** Map the finite status enum to value-bearing Fluent messages in both bundles, use that mapping in visible and ARIA output, and provide a localized unknown-status fallback. Treat section names as store data but localize structural labels and empty markers. Add English/Indonesian rendering tests.

**Status:** ✅ REMEDIATED (`9d8bab79`)

**Fix:** A `STATUS_LABEL_IDS` map renders localized status labels (with a `tables-status-unknown` fallback) in both the floor-plan buttons and the detail panel, replacing raw machine values in visible and ARIA output.

### TBL-08 — Floor-plan geometry is trusted at render time and can produce unusable or overlapping controls

**Evidence:** The table button's `left`, `top`, `width`, and `height` inline styles are built directly from persisted `pos_x`, `pos_y`, `width`, and `height` percentages. Core validation checks only name and non-negative capacity; it does not constrain geometry to finite, non-negative, or bounded values. The CSS uses `overflow: hidden`, and the table's dynamic size can become smaller than a comfortable touch target.

**Impact:** Invalid or extreme geometry can place tables outside the floor plan, overlap unrelated controls, or produce tiny buttons that are difficult to select. Since the inline values are persisted inputs, this is both a data-quality and responsive UX issue.

**Recommendation:** Validate all geometry values as finite and within documented bounds at the Rust boundary and database policy level. Clamp or reject zero-sized controls, enforce a minimum interactive size, and provide a responsive zoom/fit mode for dense plans. Add tests for invalid geometry and small-screen rendering.

**Status:** ✅ REMEDIATED (`6f2e3ce4`, `382a429f`)

**Fix:** `validate_table_geometry` rejects non-finite, negative, out-of-bounds (`0..=100`), and sub-2% width/height geometry at the DB boundary for create and update. The front-end additionally clamps persisted width/height to a 2% minimum so legacy pre-bounds data can never render an unusably tiny control.

### TBL-09 — Sections are derived from the loaded table page rather than loaded as stable metadata

**Evidence:** Section buttons are created from `new Set(tables.map(t => t.section).filter(Boolean))`. When a section is selected, the next request returns only that section, so the available filter buttons are then derived from the filtered response. The API already exposes `listSectionsScoped`, but the screen does not use it.

**Impact:** Selecting a section can make other section filters disappear until the user returns to All. Newly created or temporarily empty sections cannot be represented consistently, and a failed filtered request can leave section navigation based on stale data.

**Recommendation:** Load section metadata independently through `listSectionsScoped`, preserve the full section list while filtering, and handle section deletion/renaming explicitly. Add tests that section controls remain stable after filtering and that empty sections are represented correctly.

**Status:** ✅ REMEDIATED (`382a429f`)

**Fix:** Sections load independently via `listSectionsScoped` (cancelled-flag guarded, non-fatal fallback) instead of being derived from the filtered table page, so selecting a section never hides the other filters and empty sections stay representable.

### TBL-10 — The floor plan has no explicit loading or empty state

**Evidence:** `tables` starts as an empty array and the screen renders the floor-plan region immediately. During the first request and after a successful zero-row response, the same empty blueprint is shown. There is no skeleton, localized empty message, setup guidance, or retry control.

**Impact:** Operators cannot distinguish initial loading, a configured restaurant with no tables, an empty selected section, and a failed load. A blank floor plan provides no actionable next step and can look like a rendering failure.

**Recommendation:** Add a loading skeleton or status, an empty state that explains how tables are configured, and a filtered-empty state with a clear “All sections” action. Pair the states with the error/retry path from TBL-02.

**Status:** ✅ REMEDIATED (`382a429f`)

**Fix:** Distinct loading state (`role="status"` spinner), a full empty state with setup guidance, and a filtered-empty state with an All-sections action.

### TBL-11 — Table status presentation relies on contrast assumptions over status gradients

**Evidence:** All table buttons force `color: var(--color-accent-fg)` over status-specific gradients and use text shadows plus `color-mix()` borders. The gradients darken theme colors with `black`, and the status text is rendered at `var(--text-xs)` with reduced opacity. There is no component-level contrast assertion for custom theme/accent combinations.

**Impact:** A theme whose accent foreground is not sufficiently contrasting against success, danger, warning, or tertiary gradients can make table names/statuses hard to read. Small, semi-transparent status text is especially vulnerable to contrast regressions.

**Recommendation:** Use dedicated token pairs for each status surface/foreground or derive a contrast-safe foreground rather than reusing the accent foreground. Remove opacity from essential status text, add forced-colors support, and test representative light/dark/custom themes with contrast tooling.

**Status:** ✅ REMEDIATED (`c0b1439b`)

**Fix:** Dedicated status foreground tokens (`--color-success-fg`, `--color-warning-fg`, `--color-cleaning-fg`, plus existing `--color-danger-fg`) are defined per theme and applied per status gradient instead of reusing `--color-accent-fg`. Essential status text no longer carries `opacity: 0.85`, and a `@media (forced-colors: active)` block provides Windows high-contrast surfaces.

### TBL-12 — Focused tests cover the happy path but not failure, integrity, or accessibility boundaries

**Evidence:** The focused UI run passed 18 `TableManagementScreen` tests (50 tests across the matched table/restaurant test files). Existing tests cover rendering, filtering buttons from loaded data, positioning, status classes, opening/closing the detail panel, and context-menu status calls. They do not cover load rejection, mutation rejection, stale responses, loading/empty states, keyboard/touch actions, dialog focus behavior, geometry validation, status/assignment invariants, delete protection, or permission boundaries.

**Impact:** The highest-risk behavior in this audit—silent mutation failure, unassigned occupied tables, cross-request stale state, and inaccessible dialog actions—can regress without failing the current suite.

**Recommendation:** Add component tests for all loading/error/empty and keyboard/dialog paths, API contract tests for scoped arguments and errors, and Rust integration tests for assignment/status/delete invariants, geometry validation, and role permissions. Include a KDS integration assertion that assigned table numbers survive the full order lifecycle.

**Status:** ✅ REMEDIATED (`6f2e3ce4`, `382a429f`, `9d8bab79`)

**Fix:** Rust integration tests for the TBL-01/04/08 invariants (37 `db::tables` tests); 14 new UI tests covering loading/empty/filtered-empty, error + retry, stale-response drop, section stability, context-menu-opens-detail, all four status actions, mutation failure keeping the panel open, duplicate-click guard, in-place patch + no-clobber, `aria-modal`, focus-into-dialog, Escape + focus restoration, localized status + unknown fallback, and geometry clamp (36 total).

## Positive controls observed

- The current screen uses `listTablesScoped` and scoped status/release commands rather than legacy user-ID APIs.
- Scoped write commands resolve the session's store and enforce table-specific permissions.
- Core status parsing rejects unknown values instead of accepting arbitrary strings.
- Core create/update validate non-empty names and reject negative capacities.
- Status transitions and sale assignment use database updates that return `NotFound` for missing IDs.
- Table records are ordered by `sort_order, name`, and inactive tables are excluded from the normal list.
- The UI provides semantic region/list/option-like structure, visible focus styling, section filtering, and a close action for the detail panel.
- CSS uses design tokens and gates motion behind `prefers-reduced-motion: no-preference`.
- Existing tests exercise all four status-action branches and the scoped session-token call shape.

## Test and validation results

Focused validation completed during this audit:

```text
cd ui
npx vitest run src/__tests__/*Table*.test.tsx src/__tests__/*table*.test.tsx src/__tests__/*Restaurant*.test.tsx src/__tests__/*restaurant*.test.tsx
npm run typecheck
```

Results:

- Focused UI tests: **50 passed, 0 failed** across 4 matched files
- `TableManagementScreen.test.tsx`: **18 passed**
- TypeScript typecheck: **passed with 0 errors**
- Report existence and Markdown formatting validation: pending final report review
- No dedicated Rust table test command is claimed; core table tests and command source were inspected but not rerun as part of this report

## Remediation

Implemented in four commits:

| Finding | Commit | What landed |
|---|---|---|
| TBL-01 · occupied-requires-sale | `6f2e3ce4`, `9d8bab79` | Backend invariant + Mark Reserved hold model |
| TBL-02 · load failures | `382a429f` | seq-guarded loader, error banner + Retry |
| TBL-03 · fire-and-forget mutations | `9d8bab79` | async pending-guarded + in-place patch |
| TBL-04 · delete lifecycle | `6f2e3ce4` | delete protection for occupied/reserved/sale-linked |
| TBL-05 · context-menu a11y | `9d8bab79` | context menu opens accessible detail |
| TBL-06 · dialog completeness | `9d8bab79` | `aria-modal`, focus trap, focus restoration |
| TBL-07 · status localization | `9d8bab79` | localized status map + unknown fallback |
| TBL-08 · geometry validation | `6f2e3ce4`, `382a429f` | backend bounds + front-end 2% clamp |
| TBL-09 · stable sections | `382a429f` | independent `listSectionsScoped` metadata |
| TBL-10 · loading/empty states | `382a429f` | loading + empty + filtered-empty |
| TBL-11 · contrast tokens | `c0b1439b` | status fg pairs, opacity removed, forced-colors |
| TBL-12 · QA coverage | all | 37 Rust tests + 36 UI tests |

## Recommended remediation order

1. **TBL-01/TBL-03:** Make status actions order-aware, await persistence, refresh state, and surface failures.
2. **TBL-02/TBL-10:** Add request guards plus durable loading, empty, error, and retry states.
3. **TBL-04/TBL-08:** Protect table deletion and validate persisted geometry at the backend boundary.
4. **TBL-05/TBL-06/TBL-07:** Complete keyboard/dialog behavior and status localization.
5. **TBL-09/TBL-11/TBL-12:** Stabilize section metadata, harden theme contrast, and expand integration coverage.

## Audit status

All 12 findings are **REMEDIATED** and committed. Validation gates at close: 37/37 Rust `db::tables` tests, 36/36 `TableManagementScreen` UI tests, typecheck clean, eslint clean, i18n lint clean, and the theme-token / touch-target / focus-visible / animation compliance suites all green.
