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
   lines. The shortcuts help popover, node finder, canvas minimap,
   relationship picker, and validation issues widget are now extracted into
   `topologyShortcutsHelp.tsx`, `topologyNodeFinder.tsx`,
   `topologyMinimap.tsx`, `topologyRelationshipPicker.tsx`, and
   `topologyValidationWidget.tsx` (each behavior-preserving, 541 topology
   tests green, plus isolated overlay unit tests). The main component's
   remaining bulk is the drag/undo/rename/simulate state machine, which is
   not trivially separable. **Surfaced + fixed while extracting:** the
   validation-jump actions (`handleAddStockWireHint` / `handleJumpToWire`)
   called the minimap's `recenterViewOn` (which converts minimap PIXELS to
   canvas coords) with CANVAS coords, so "jump and center" panned to a
   wildly wrong spot. They now use `centerViewportOn`, centering on the
   actual node/wire canvas position.
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

---

# Tax configuration audit (2026-08-13)

Audit of `TaxConfigurationScreen.tsx` + `tax.ftl` / `tax.id.ftl`.

## Fixed (committed)

- ✅ **Inclusive/Exclusive toggle keyboard access** — the two `role="radio"`
  buttons now form a real WAI-ARIA radiogroup: Arrow keys move focus + selection
  and a roving tabindex keeps only the checked option in the Tab order.
- ✅ **Rate parsing** — `parseInt` (which silently truncated `825.5` → `825`)
  replaced with `Number` + `Number.isInteger`, so non-integer bps is rejected
  with the existing localized error instead of being saved as the wrong rate.
- ✅ **Name trimming** — the tax name is trimmed before save (the Save button
  already required a non-blank name, but surrounding whitespace was preserved).
- ✅ **Dead `aria-label` on the actions `<th>`** — both tables' actions column
  headers carried `aria-label={getString('actions-aria')}` under a
  `<Localized attrs={{ "aria-label": true }}>` wrapper that already injects the
  localized `tax-config-col-actions` attribute, overriding the explicit prop.
  Removed the dead prop.
- ✅ **Orphan FTL keys removed** — `tax-config-loading`, `tax-config-modal-aria`,
  `tax-config-cat-modal-aria`, `tax-config-modal-close` (both locales) and
  `tax-config-field-name-aria` (en only) were never read by any code.

## Still open

1. ✅ **Non-functional `setForm` updates fixed** — the name/rate/checkbox/radio
   handlers now use functional updaters (`setForm((prev) => …)`), removing the
   latent stale-closure risk.
2. ✅ **Category modal no-op save fixed** — `SettingsPopup` now receives
   `saveDisabled` (a sorted-id diff against `catTaxRates.get(editingCatId)`), so
   an untouched assignment can't round-trip the IPC write.
3. **Redundant `aria-label` on picker labels** — the category rate `<label>`s
   set `aria-label={r.name}`, which may override the richer label text (rate % +
   type). Confirm whether the bare name is the intended accessible name.

---

# Customer management audit (2026-08-13)

Audit of `CustomerManagementScreen.tsx` + `customers.ftl` / `customers.id.ftl`.

## Fixed (committed)

- ✅ **Dead + hardcoded-English `aria-label` props** — the search input
  (`aria-label={getString('search-customers-aria')}`), actions column header
  (`aria-label={getString('actions-aria')}`), and the history/edit/delete row
  buttons (`aria-label={`View history for ${name}`}` etc.) were all overridden
  by their `<Localized attrs={{ 'aria-label': true }}>` wrappers. Removed; the
  wrappers now provide the localized labels.
- ✅ **Hardcoded `en-US` formatting** — `formatSaleTotal`, `formatDate`, and the
  loyalty point counts now derive the locale from the active Fluent bundle
  (`[...l10n.bundles][0]?.locales[0]`) instead of hardcoding `en-US` / the
  browser locale.
- ✅ **Name field updater** — the name input now uses the functional
  `updateField('name', …)` like the other fields instead of a stale-closure
  `setForm({ ...form, … })` spread.
- ✅ **Stray `{ }` JSX** — removed five empty expressions (table row + four
  placeholder `Localized` wrappers).
- ✅ **Orphan FTL keys** — removed 12 never-read keys per locale
  (`customer-mgmt-loading`, `-name-aria`, `-email-aria`, `-phone-aria`,
  `-notes-aria`, `-modal-add-aria`, `-modal-edit-aria`, `-modal-close`, and
  `-history-sale-date/total/items/status`).

## Still open

1. **`search-customers-aria` in shared.ftl is now orphan** — after removing the
   dead prop, the shared `search-customers-aria` key has no remaining consumers
   (`PaymentModal` uses a separate `payment-search-customers-aria` key). Left in
   place to avoid touching the shared bundle in this pass.

---

# Terminal management audit (2026-08-13)

Audit of `TerminalManagementScreen.tsx` + `terminals.ftl` / `terminals.id.ftl`.

## Fixed (committed)

- ✅ **Dead + hardcoded-English `aria-label` props** — the actions column header
  (`aria-label={getString('actions-aria')}`) and the edit/delete buttons
  (`aria-label={`Edit ${name}`}` etc.) were overridden by their `<Localized
  attrs>` wrappers. Removed.
- ✅ **`formatDate` ignored hour/minute** — it called `toLocaleDateString`, which
  silently drops the `hour`/`minute` options, so "Last Seen" never showed a time.
  Switched to `toLocaleString` with the active Fluent locale and an invalid-date
  guard.
- ✅ **Functional `setForm` updates** — the five form field handlers now use
  `setForm((prev) => …)` instead of spreading `form` from the closure.
- ✅ **Orphan FTL keys** — removed 11 never-read keys per locale
  (`terminal-management-loading`, `terminal-secret`, `terminal-metadata`,
  `-register/update/delete-success`, `-name-required`, `-device-id-required`,
  `-modal-close`, `-loading-overrides`, `-delete-aria`).

## Still open

1. **Redundant label `aria-label`s** — the four modal field `<label>`s set an
   `aria-label` (`terminal-field-*-aria`). For name/device-id this duplicates the
   visible text; for secret/metadata it is a deliberate shorter accessible name
   than the verbose visible label. Left as-is (intentional concise naming).

---

# Category management audit (2026-08-13)

Audit of `CategoryManagementScreen.tsx` + `products.ftl` / `settings.ftl`.

## Fixed (committed)

- ✅ **Icon/colour radiogroups keyboard access** — the four pickers (icon and
  colour, in both create and edit modals) had every radio in the Tab order and
  no arrow-key navigation. Added WAI-ARIA roving tabindex (only the checked
  option is tabbable) and Arrow-key navigation that moves focus + selection,
  via a shared `nextRadioValue` helper. Regression test for the icon picker.
- ✅ **`useState(randomColour())` side effect** — the random colour/icon were
  evaluated on every render (discarded) because the call was an argument rather
  than a lazy initializer. Switched to `useState(() => randomColour())`.
- ✅ **ID colour-swatch parity** — `category-colour-swatch-aria` in the
  Indonesian bundle omitted the `{ $colour }` variable that the English bundle
  interpolates; added it so both locales name the swatch's colour.

## Still open

1. **Icon `label` field is dead** — `ICON_OPTIONS`' `label` ("Food", "Generic ·",
   …) is never rendered; the aria-label comes from the FTL ternary. The three
   dot icons (dots-1/2/3) collapse to a single `categories-icon-generic` label,
   so AT users can't tell them apart. Minor; needs a product call on distinct
   labels.

---

# Gift cards audit (2026-08-13)

Audit of `GiftCardsScreen.tsx` + `IssueGiftCardModal.tsx` + `GiftCardPayment.tsx`
and `gift-cards.ftl` / `sales.ftl`.

## Fixed (committed)

- ✅ **Raw backend status/txn type** — the card status badge and transaction
  type rendered the raw backend values (`"active"`, `"redeem"`, `"topup"`, …) in
  English. Mapped both through the Fluent bundle (adding
  `gift-cards-txn-issue/redeem/topup/refund` keys) with a raw-value fallback.
- ✅ **`aria-expanded`** — the expandable card summary button now exposes its
  toggle state to assistive tech.
- ✅ **Unnamed dialog** — the issue modal `role="dialog"` had no accessible name;
  added `aria-labelledby` → its `<h2>`.
- ✅ **Dead `cardInputRef`** — removed the unused ref in `IssueGiftCardModal`.
- ✅ **Top-up parsing** — `parseInt` (silently truncating `500.5` → `500`)
  replaced with `Number` + `Number.isInteger`.
- ✅ **Browser-locale dates** — issue/expiry/transaction dates now use the
  active Fluent locale instead of `toLocaleDateString()`'s browser default.

## Still open

1. **`formatMoney` default locale is `id-ID`** — the gift-card balances/totals
   call `formatMoney` without a locale, so English-locale users see Indonesian
   grouping ("Rp 50.000"). Systemic across the app; worth a dedicated pass that
   threads the active Fluent locale into `formatMoney` call sites (and reconciles
   with the per-store receipt `decimalSep` override).
2. **Hardcoded `currency: 'IDR'` + `created_by: 'staff'`** in
   `IssueGiftCardModal` — likely intentional until multi-currency gift cards and
   real operator identity land.
3. **`gift-cards-loading` orphan** (in `sales.ftl`) — the screen uses a skeleton.

---

# Promotions audit (2026-08-13)

Audit of `PromotionManagementScreen.tsx` + `promotions.ftl` / `promotions.id.ftl`.

## Fixed (committed)

- ✅ **Functional `setForm` updates** — the eleven promotion form field
  handlers used `setForm({ ...form, … })`; converted to functional updaters.
- ✅ **Locale-aware dates** — the Starts/Ends columns used
  `toLocaleDateString()` (browser locale); now formatted in the active Fluent
  locale via a shared `formatDate` helper.

## Still open

1. **`parseInt` truncation in numeric fields** — `value_minor`,
   `min_order_minor` (via `parseInt(e.target.value) || 0`) and `min_qty` /
   `reward_qty` (via `parseInt(e.target.value)`) silently truncate decimals
   like `500.5` → `500`. Number inputs with default `step=1` usually prevent
   this, but the truncation is silent if a decimal is typed.
2. **`datetime-local` timezone round-trip** — the Starts/Ends pickers write
   `new Date(value).toISOString()` (UTC) and read back
   `iso.substring(0, 16)` (treated as local), so an operator in UTC+7 sees the
   stored time shifted by the offset on reopen. Needs a deliberate decision on
   whether promotions store local wall-clock time.
3. **`value` column display** — for `fixed_amount` / `buy_x_get_y` the raw
   `value_minor` integer is shown without currency formatting; for
   `percentage` it renders `{n}%`. Confirm the intended display for non-
   percentage types.

---

# Loyalty audit (2026-08-13)

Audit of `LoyaltyManagementScreen.tsx` + `loyalty.ftl` / `loyalty.id.ftl`.

## Fixed (committed)

- ✅ **Functional `setTierForm` updates + stray `{ }`** — the five tier form
  fields now use functional updaters; nine stray empty `{ }` JSX expressions
  removed (tier edit form, table header, expand cell, txn rows).
- ✅ **Integer tier-field parsing** — `parseInt` silently truncated decimals
  (`"10.5"` → `10`) and turned an empty `min_points` into `0`. Now `Number` +
  `Number.isInteger` + non-empty checks reject fractional/blank integer fields
  with the localized error; the earn multiplier still accepts decimals.
  Regression test added.
- ✅ **Tier badge contrast** — the white text on the tier colour was unreadable
  for light colours; now uses `contrastFg(tier.colour)` (same utility as the
  category picker).
- ✅ **Locale-aware numbers/dates** — points, lifetime points, points-to-next,
  min-points, and transaction dates now use the active Fluent locale instead of
  the browser default.
- ✅ **`customerMap` memoized** — no longer rebuilt on every render.

## Still open

1. **Nested interactive row** — each loyalty account `<tr>` is
   `role="button" tabIndex={0}` with an expand handler, AND contains a real
   `<button>` doing the same toggle. Screen readers and keyboard users see two
   controls for one action. Needs a design decision (drop the row role, or the
   inner chevron button).
2. **Dynamic `loyalty-${txn.txn_type}` keys** — covered for the known types
   (`earn`/`redeem`/`adjust`); an unknown backend type falls back to the
   capitalized raw value.

---

# Exchange-rate audit (2026-08-13)

Audit of `ExchangeRateScreen.tsx` + `currency.ftl` / `currency.id.ftl`.

## Fixed (committed)

- ✅ **Functional `setForm` updates + tightened `formValid`** — all five modal
  fields now use functional updaters (no stale-closure spread). The rate
  validity check now survives the millionths conversion: a sub-0.000001 rate
  (which passed `parseFloat > 0` but rounded to 0 millionths and silently did
  nothing on Save) is rejected. `Number.isFinite` + `rateMillionths > 0` guard
  tiny/overflowing values.
- ✅ **Delete confirmation dialog** — deleting an exchange rate is now a
  two-step flow via `ConfirmDialog` (danger variant, uses the previously
  dead `currency-delete-confirm` key; added `currency-delete-title`). The
  delete test was updated to confirm the dialog.
- ✅ **Orphan FTL keys removed** — `currency-add`, `currency-loading`, and
  `currency-modal-add-label` had zero consumers; deleted from both bundles.

## Still open

None.

---

# Inventory audit (2026-08-13)

Audit of `src/features/inventory/` (11 screens) + `inventory.ftl` /
`stock-counting.ftl` / `*.id.ftl` bundles.

## Fixed (committed)

- ✅ **Fractional quantities rejected (no silent `parseInt` truncation)** —
  `InventoryAdjustmentScreen` (2.5 → 2), `StockCountDetail` expected/counted
  qty (10.5 → 10), and `ThresholdConfigScreen` (5.5 → 5) all accepted
  fractional input and silently truncated it. Now `Number` +
  `Number.isInteger`: adjustment shows the localized error and disables
  Apply, counted-qty keystrokes ignore fractional in-progress input,
  add-line shows a new `sc-error-qty-integer` message, thresholds toast the
  existing error. Regression test added.
- ✅ **Locale-aware dates** — 7 screens rendered dates/times with the
  browser default locale (`toLocaleDateString`/`toLocaleTimeString`/
  `toLocaleString` with no args). All now derive the locale from the active
  Fluent bundle (`[...l10n.bundles][0]?.locales[0]`, `en-US` fallback):
  ShiftBar, StockAlertPanel, StockCountDetail, StockCountHistory,
  StockCountsScreen, TransactionLogScreen, TransitAuditScreen.
- ✅ **Orphan FTL keys removed** — 8 never-read keys deleted from both
  bundles: `inv-alert-acknowledge-btn`, `inv-alert-col-triggered`,
  `inv-log-type-purchase-order-receive` (superseded by `-po-receive`),
  `inv-report-loading-aria`, `inv-shift-notes-label`,
  `inv-shift-select-location`, `inv-transit-col-overdue`, `sc-loading`.
  (`sc-status-*`/`sc-type-*` are used via dynamic ids — kept.)

## Still open

1. **`StockCountForm` type toggle** — the radiogroup (`role="radiogroup"`
   + plain buttons) predates the arrow-key/roving-tabindex pattern applied
   to tax and categories; low value since it has only 3 options.
2. **`stockStatus` low threshold** — hardcoded `< 10` in
   `InventoryAdjustmentScreen` while thresholds are configurable elsewhere.
3. **`ShiftBar` note field** — `inv-shift-notes-label` was orphan because
   the visible label isn't localized (the `<label>` renders raw English
   "Notes"); the placeholder is localized. Worth a visual check.

---

# Products audit (2026-08-13)

Audit of `src/features/products/` (4 screens) + `products.ftl` /
`bundles.ftl` / `*.id.ftl` bundles.

## Fixed (committed)

- ✅ **Bundles: strict integers + surfaced failures** — bundle price and
  item qty/unit-price were silently truncated by `parseInt` (4.50 → 4);
  now rejected with localized `bundles-error-invalid-*` messages. Save/delete
  failures were swallowed by empty catch blocks; now surfaced inline
  (`role="alert"`). 4 functional `setForm` updaters; 7 dead hardcoded
  aria-labels removed; 4 new `bundles-error-*` keys.
- ✅ **Variants: strict integers + surfaced failures** — `500.5` price
  silently saved as 500; sort order truncated too. Now rejected with new
  `variant-mgmt-error-invalid-*` keys. The previously-dead
  `variant-mgmt-error-save`/`-delete` keys (empty catches!) are now wired to
  inline alerts. 7 functional `setForm` updaters; 2 dead aria-labels removed.
  Regression test for fractional-price rejection.
- ✅ **Products/Lookup: functional `setForm`** — 8 handlers in
  `ProductManagementScreen` converted; dead hardcoded aria-label on the
  lookup card button removed.
- ✅ **Orphan FTL keys removed** — 19 never-read keys deleted from both
  bundles (`bundles-loading`/`-modal-aria`/`-close-aria`,
  `categories-loading`, `product-lookup-add`/`-title`,
  `product-mgmt-deleting`/`-field-name`/`-field-sku`/`-loading`,
  `restaurant-sort-*` ×4, `variant-mgmt-close`/`-dialog-aria`/
  `-delete-confirm-aria`/`-loading`/`-modal-close`).

## Still open

1. **`BundleManagementScreen` `toggleActive`** — update failures are still
   silent (no toast/alert) and the toggle is optimistic; a failed toggle
   leaves the UI showing the flipped state until the next load.
2. **`ProductManagementScreen` delete confirm** — uses an inline `ConfirmDialog`
   but the `product-mgmt-deleting` plural key was dead (never read), so the
   deleting state renders nothing; harmless but dead code removed.
3. **`formatVariantPrice`** — hardcodes `Intl.NumberFormat('en-US', ...)`
   instead of the active Fluent locale; the domain `formatMoney` already
   handles IDR minor-unit exponents, so this could reuse it.

---

# Stock-transfers audit (2026-08-13)

Audit of `StockTransfersScreen.tsx` + `stock-transfers.ftl` /
`stock-transfers.id.ftl`.

## Fixed (committed)

- ✅ **`formatDate` bug** — called `toLocaleDateString` with `hour`/`minute`
  options, which are silently dropped, and used the browser default locale.
  Now `toLocaleString` with the active Fluent locale, so created/sent/received
  timestamps show the time and match the UI language.
- ✅ **Strict-integer quantities** — receive qty (`parseInt('4.5') || 0` sent
  4) and create-line qty (`parseInt` truncated) now reject fractional input
  with a new `stock-transfers-error-qty-integer` message instead of silently
  truncating. Regression test added for the receive flow.
- ✅ **Receive modal localized** — the `(ordered: {qty})` line label and the
  `{sku} received quantity` aria-label were hardcoded English; now Fluent
  (`stock-transfers-receive-line`, `stock-transfers-received-qty-aria`).
  Receive validation errors surface via `role="alert"` (new `receiveError`
  state); `setReceiveLines` uses a functional updater; stray `{ }` JSX in
  the product datalist removed.

## Still open

1. **`localizedStatusLabel` fallback** — unknown backend statuses render the
  capitalized raw value; the dynamic `stock-transfers-status-*` keys cover
  the known set (used by filter tabs + badges).
2. **Create-line `productName`** — when typing a SKU that doesn't match a
  known product, the line name falls back to the raw SKU text; intentional
  for free-form entries.

---

# Shifts audit (2026-08-13)

Audit of `ShiftManagementScreen.tsx` + `shifts.ftl` / `shifts.id.ftl`.

## Fixed (committed)

- ✅ **Strict-integer balances/payouts** — opening balance, closing balance,
  and payout amount were silently truncated by `parseInt` (500.5 → 500).
  Now `Number` + `Number.isInteger`: fractional opening balance is rejected
  with a new `shift-invalid-opening-balance` message, closing balance and
  payout keep their existing invalid messages, and the Apply/Close buttons
  disable for fractional input. Regression test added.
- ✅ **Locale-aware time/date** — `time()` and `dateTime()` passed `[]` as
  the locale (browser default); now the active Fluent locale.
- ✅ **Orphan FTL keys removed** — 11 never-read keys deleted from both
  bundles: `shift-open`/`shift-close` (buttons use `shift-btn-*`),
  `shift-closing-balance`, `shift-expected-cash`, `shift-actual-cash`,
  `shift-difference`, `shift-eod-report`, `shift-print-report`,
  `shift-loading`, `shift-recon-payouts-returned`, `shift-report-loading`.
- ✅ Stray `{ }` JSX in the history table row removed.

## Still open

1. **Open-shift default** — an empty opening-balance field opens with 0;
  intentional per SHIFT-03 but worth confirming the drawer-empty default.
2. **`reason` fallback** — payout reason defaults to the raw English string
  `'safe drop'` when blank; not localized (backend free-text field).
