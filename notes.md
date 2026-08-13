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

### 3. Unvalidated cache cast + fragile partition parsing

- **Where:** `readCached` casts `unknown` → `T` with no shape check;
  `cachePartition` derives the workspace from `entryKey.split(':')` indexes.
- **Why deferred:** the version stamp guards schema drift at the persistence
  layer, and the key format is internal. But a shape change without a version
  bump would crash a card, and a key-format change silently misroutes to the
  `shared` partition.
- **Likely fix:** validate cached values at the type boundary, and derive the
  partition from a structured key object rather than string splitting.
