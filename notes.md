# Analytics cards — deferred items needing visual confirmation

Created 2026-08-13 during the analytics cards audit. Items found in code
review but left for a later pass are listed below. Completed items are noted
with ✅, so this file stays a live index rather than a stale backlog.

## Completed (no longer open)

- ✅ **Options menu clipping** — the ⋮ menu is now rendered through
  `createPortal(document.body)` with a fixed viewport anchor, so the card's
  `overflow: hidden` can't clip it.
- ✅ **Heatmap level-1 contrast** — `--analytics-heat-1` moved from `#dbeafe`
  to `#bfdbfe`, stepping clearly away from the empty-cell gray.
- ✅ **10px caption text** — `.analytics-kpi-label`, `.analytics-delta`,
  `.analytics-card-insight`, `.analytics-legend-item`, and `.analytics-heat-label`
  bumped to 11px.
- ✅ **Payments % shares** — `largestRemainderPcts` makes the stacked bar
  always sum to 100 (was independent `Math.round` per method).
- ✅ **Refunds vs Voids overlap** — `refunds` is now a money/totals summary
  (count + amount + average) with no item list; `voids` keeps the voided-items
  list. The unused `getVoidedItems(25)` fetch was dropped from the refunds loader.
- ✅ **Cache eviction** — `TtlCache` now evicts LRU (reads promote recency)
  and purges expired entries before evicting live ones.
- ✅ **Sticky failure map** — recorded query failures are cleared on every
  filter change, so re-navigating back to a previously-failed card retries.
- ✅ **Heatmap per-cell data reachable by AT** — the grid dropped its
  `role="img"` (which made descendants presentational); each cell with data is
  now `role="img"` + `aria-label` carrying its revenue/order detail.
- ✅ **Card container `aria-label` duplicates the heading** — the card group
  now uses `aria-labelledby` pointing at its `<h2>` instead of a duplicated
  `aria-label`.

## Still open — verify in the UI

### 1. Drag / hover affordances

- **Where:** `.analytics-card-grip` shows `cursor: grab` but is decorative
  (`aria-hidden`) while the **whole card** is `draggable`; cards also lift on
  hover though the body isn't clickable.
- **What to check:** whether the grip implies only-the-grip drags, and whether
  the hover lift implies the body is clickable.
- **Likely fix:** drag from the grip only (or drop the grip), and limit the
  hover lift to the header.

## Still open — cache & query layer design items

### 2. Side effect during render

- **Where:** `useAnalyticsQuery` runs `fetcher()` and
  `analyticsDataCache.set(...)` (which writes sessionStorage) synchronously
  during render for sync fetchers.
- **Why deferred:** documented and defended as idempotent/deterministic, and
  the screen has relied on it for instant no-flash rendering. Moving to
  `useEffect` would reintroduce an empty flash without extra state.
- **Likely fix:** keep, or shift the write into a `useLayoutEffect`/effect
  while retaining the sync `get` for the no-flash hit.

### 3. ✅ Unvalidated cache cast + fragile partition parsing (fixed)

- **`cachePartition`** no longer guesses a workspace from arbitrary segment
  indexes (the stale `query:` branch is gone). It recognizes only the
  `card:<cardKey>:<workspace>:…` shape, sharing its prefix constant with
  `cardQueryKey`; anything else routes to the `shared` snapshot.
- **Cached values are now shape-checked at the read boundary.**
  `useAnalyticsQuery` accepts an optional `validate` guard; a cached value
  that fails it is invalidated and refetched as a miss instead of being cast
  blindly into `T`. `CARD_PAYLOAD_VALIDATORS` (colocated with
  `CARD_LOADERS`) supplies a loose structural guard per card, and the heatmap
  query passes its own.

---

# Five-file deep audit (2026-08-13)

Follow-up audit of the five files that previously carried eslint warnings
(DashboardScreen, RetailProductGrid, AnalyticsCardContent, AnalyticsScreen,
NodeTopologyEditor). Fixed items are committed; the rest remain open.

## Fixed (committed)

- ✅ **AnalyticsCardContent** — reuse `CRITICAL_STOCK_LEVEL` in the alert rows,
  give the tables card a bad-tone delta in both modes, drop the refunds card's
  unused `title`/`expanded` props, key ranked/alert rows by name+index, and
  format the customers-card counts with the Fluent locale formatter.
- ✅ **AnalyticsScreen** — add `setData` on drag start (Firefox), throttle the
  scroll handler with rAF, and manage the card-menu focus/keyboard (focus first
  item, Arrow/Home/End, restore focus on close).
- ✅ **DashboardScreen** — remove the unreachable full-screen skeleton, build
  ranges from local dates (not UTC), rename `today*` → `range*`, resolve the
  donut fill from `--color-fg`, localize currency/compact formatting, the
  heatmap tooltip, and CSV headers, and rename the `today-revenue`/`orders-today`
  keys (copy now says "Revenue"/"Orders").
- ✅ **RetailProductGrid** — keyboard add-to-cart on the name button, sortable
  headers as real `<button>`s, `aria-hidden` sort glyph, `scope="col"` on the
  non-sortable headers, and removal of dead out-of-stock guards/`aria-disabled`.
- ✅ **NodeTopologyEditor** — debounce the per-frame viewport `localStorage`
  write (250ms + unmount flush).

> Correction: the NodeTopologyEditor `localStorage` **reads** are already
> wrapped in try/catch with fallbacks; the earlier finding flagged them as
> unguarded in error.

## Still open

### Dashboard

1. **Eager loading** — `loadData` fetches all 8 datasets (daily/weekly/monthly
   revenue, top products, low stock, category, heatmap, prev-daily) regardless
   of the selected granularity; `getLowStockAlerts(10)` also ignores the date
   range (same non-time-bounded snapshot as the analytics low-stock card).
2. **`fmtDelta` edge case** — returns `+∞` / `−` when the previous period is
   zero; these are math symbols and arguably fine, but not localized.

### AnalyticsScreen

3. ✅ **`menuAnchor` staleness fixed** — the portaled card menu now re-anchors
   to its trigger on window scroll (capture-phase, since scroll doesn't bubble)
   and resize, with a 0.5px change-guard so per-frame scroll events don't
   churn the grid.
4. ✅ **Popover dismissal unified** — the zoom/shortcuts/cache popovers now
   close on outside pointerdown (toggle buttons excluded so a close-then-
   reopen never happens), and Escape also closes the cache metrics popover.

### RetailProductGrid

5. ✅ **Stock-threshold magic numbers fixed** — added
   `DEFAULT_LOW_STOCK_THRESHOLD` (5) / `DEFAULT_HIGH_STOCK_THRESHOLD` (10) to
   `types/domain.ts` and used them across `AddProductModal`, `EditProductModal`,
   `RetailProductGrid`, and `RetailPosScreen`. (Analytics' `CRITICAL_STOCK_LEVEL`
   stays its own "critical" severity tier — semantically distinct.)
6. ✅ **Minor fixes** — the category bar's `onWheel` now maps both trackpad
   `deltaX` and wheel `deltaY` to horizontal scroll and `preventDefault`s
   (guarded by `cancelable`); `cellValue`'s `—` em-dash is a named
   `EMPTY_CELL` constant.

### NodeTopologyEditor

7. **Size (in progress: overlays extracted)** — the component is ~6,900
   lines. The shortcuts help popover, node finder, and canvas minimap are
   now extracted into `topologyShortcutsHelp.tsx`, `topologyNodeFinder.tsx`,
   and `topologyMinimap.tsx` respectively (each behavior-preserving, with
   541 topology tests green). The main component's remaining bulk is the
   drag/undo/rename/simulate state machine, which is not trivially
   separable. **Surfaced + fixed while extracting:** the validation-jump
   actions (`handleAddStockWireHint` / `handleJumpToWire`) called the
   minimap's `recenterViewOn` (which converts minimap PIXELS to canvas
   coords) with CANVAS coords, so "jump and center" panned to a wildly
   wrong spot. They now use `centerViewportOn`, which centers on the actual
   node/wire canvas position.
8. **Render-phase ref writes (assessed: intentional, leave as-is)** —
   `historyRef.current = history` (and similar mirrors, e.g. `panRef`,
   `nodesRef`, `pushHistoryRef`, `selectedNodeIdsRef`, `l10nRef`) assign
   during render. This is the deliberate "latest ref" pattern: it lets
   memoized drag/undo/rename handlers read the CURRENT value at call time
   without taking the value as a `useCallback` dep (which would churn the
   handler identity and defeat the card/wire memoization). Moving them to
   `useEffect` would introduce stale-closure windows (a passive effect runs
   after paint, so a pointermove between paint and effect would read stale
   pan/nodes). Same documented-but-impure pattern as `startRecalculating`
   and the cache write in the analytics screen; idempotent and deterministic.

---

# WorkspaceHome audit (2026-08-13)

Audit of the workspace selection page (`WorkspaceHome.tsx` + CSS).

## Fixed (committed)

- ✅ **`canAccess` predicate** — replaced the role-blind
  `cashierOnly.has(key) || (kitchen && kitchenOnly.has(key))` with a
  role-grouped gate: owner/admin/manager/staff → all, cashier → POS only,
  kitchen → KDS only (accepting both bare and `role-`-prefixed forms).
- ✅ **`savePins` / `saveLastUsed`** — now try/catch-guarded (mirroring the
  `load*` helpers) and called as a normal event-handler side effect instead of
  inside a `setState` updater (which StrictMode can run twice).
- ✅ **Ripple timers** — the 600ms fallback `setTimeout` is tracked and cleared
  on unmount (the `animationend` path also cancels it).
- ✅ **`getColumns`** — derives the column count from the grid's actual layout
  (first-row `offsetTop` break) instead of splitting `gridTemplateColumns`,
  so `repeat()`/`minmax()` resolved values can't miscount arrow-key movement.
- ✅ **Number-key quick-launch** — maps directly to `activateWorkspace` over the
  workspace list (not the enabled-card NodeList), so the Analytics/Reports
  shortcuts are no longer addressable by a phantom number key.
- ✅ **Pin button** — `tabIndex` `-1` → `0`, making the existing
  Enter/Space handler reachable.
- ✅ **SkeletonGrid** — dropped the `aria-label` on a role-less `<div>` (the
  `role="status"` span already announces loading).
- ✅ **Retry button** — floating retry `title` now uses the localized
  `workspace-home-retry-btn` key instead of a hardcoded `"Retry"`.
- ✅ **Dead exit animation** — removed `exitingWorkspace` state, the
  `workspace-card--exiting` classes + `ws-card-exit` keyframes (the card
  unmounts the same tick, so the animation never played), and the stray
  empty `{ }` JSX.
- ✅ **`getIcon` drift** — the icon-key allowlist is now derived from
  `WS_ORDER` + `COMING_SOON_CARDS` instead of a hardcoded list that had gone
  stale (`Analytics` listed, `Reports` missing).
- ✅ **`displayName` fallback** — no longer falls back to `role_name`, so the
  greeting can't read "Hello, owner".

## Still open

### Greeting

1. **Randomized multilingual greeting** — `pickGreeting()` runs inside a
   `useMemo` (so it may re-pick under StrictMode double-render) and can show
   "Selamat datang" to an English-locale user. Deliberate design flourish;
   needs a product call on whether the greeting should follow the active
   locale instead of cycling languages.

---

# Reports / Dashboard audit (2026-08-13)

Audit of the owner/admin reports dashboard (`DashboardScreen.tsx` +
`reports.ftl` / `reports.id.ftl`).

## Fixed (committed)

- ✅ **Broken `-aria` labels** — 8 keys per locale (`dashboard-granularity-aria`,
  `dashboard-chart-revenue-aria`, `dashboard-chart-category-aria`,
  `dashboard-chart-heatmap-aria`, `dashboard-chart-top-products-aria`,
  `dashboard-export-csv-aria`, `dashboard-category-clear-aria`,
  `dashboard-back-aria`) were written as `key = .aria-label = Text` on a single
  line. Fluent parses that as a literal text VALUE equal to
  `.aria-label = Text` (attributes need the indented multi-line form), so the
  rendered `aria-label` literally included the `.aria-label = ` prefix.
  Converted all 8 to plain values; added
  `i18nStrayAttributeSyntax.test.ts` as a permanent regression guard.
- ✅ **`getComputedStyle` during render** — the donut's `--color-fg` fill color
  is now read inside the `categoryDonutOption` `useMemo` (only when the donut
  inputs change) instead of on every render.
- ✅ **Stale category selection** — `selectedCategory` is reset on reload so a
  date-range change can't keep showing a category detail that no longer exists
  in the new data.
- ✅ **`.reverse()` mutation clarity** — top-products names/values are reversed
  once at declaration instead of mutating inside the axis/series config.
- ✅ **Granularity radiogroup** — added WAI-ARIA arrow-key navigation and a
  roving tabindex (checked option is the single tab stop; Arrow keys move
  focus + selection).

## Still open

### Dashboard

1. ✅ **Eager loading fixed** — `loadData` no longer fetches weekly/monthly
   up front. Daily + prev-daily + top products + low stock + category + heatmap
   load once; the weekly/monthly series is fetched on demand (cached keyed by
   granularity + range) when that granularity is selected.
2. ✅ **Full-screen spinner flash fixed** — the spinner/error replace the
   dashboard only on the first load. Reloads keep stale data visible with a
   `role="status"` "Refreshing…" indicator, and reload failures show an inline
   `role="alert"` banner instead of wiping the screen.
3. ✅ **`fmtDelta` edge case fixed** — the unlocalized `+∞` / `−` symbols are
   gone; a metric with no previous period now renders a localized
   `dashboard-delta-new` "New"/"Baru" badge (and `fmtDelta` is a pure
   `%` formatter whose caller guards `previous === 0`).
4. ✅ **Low-stock threshold context added** — the row now reads
   "2 left (below 10)" via the localized `dashboard-stock-below-threshold`
   message instead of a bare "2 left".

### Systemic i18n finding

5. ✅ **Swept** — `shared.ftl` + `shared.id.ftl` had the same broken
   single-line `.aria-label =` syntax (55 keys each, e.g. `clear-aria`,
   `workspaces-aria`, `search-aria`, `actions-aria`, …). Every `aria-label`
   consuming them app-wide rendered the literal `.aria-label = …` prefix.
   Converted all 55 per locale to plain values, plus three Indonesian
   `update-banner-*-aria` keys that were declared attribute-only but read
   via `getString` (rendering the raw key id). `i18nStrayAttributeSyntax.test.ts`
   now guards the shared + reports bundles against the single-line form, and
   pins the update-banner keys as plain values in both locales.
6. ✅ **Attribute-only-vs-getString sweep (round 169)** — audited every
   locale bundle for attribute-only messages (no value, e.g.
   `key =\n  .aria-label = …`) whose id is read by `getString` /
   `requiredLocalized` (value readers → raw key id). Fixed 8 keys per locale
   (`categories-name-aria`, `pos-cart-options-collapse/expand-aria`,
   `product-mgmt-variants-aria`, `retail-cart-course/modifier-aria`,
   `setup-step-aria`, `terminal-override-aria`) by converting them to plain
   values, and removed the redundant overridden `placeholder` prop on the
   `refund-reason/note-placeholder` inputs (correctly served by their
   `<Localized attrs>` wrappers). Added `scanAttributeOnlyGetString()` to the
   `barePlaceholderScan` gate so this class fails closed.
