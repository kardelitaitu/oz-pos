# Analytics cards — deferred items needing visual confirmation

Created 2026-08-13 during the analytics cards audit. Each item below was
found in code review but left unchanged because the right fix depends on
seeing it rendered (or on a product/design call). Verify each in the running
app, then either fix it or strike the entry.

## Verify in the UI

### 1. Card options menu is clipped by `overflow: hidden`

- **Where:** `ui/src/features/analytics/AnalyticsScreen.css` — `.analytics-card`
  has `overflow: hidden`, and `.analytics-card-menu` is `position: absolute`
  inside `.analytics-card-actions` (`position: relative`).
- **What to check:** open the "⋮" menu on a **collapsed** card (header only)
  and on a card near the bottom of the grid. The dropdown is expected to be
  cut off at the card's edge.
- **Why deferred:** the clipping is reasoned from the CSS, not observed, and
  the fix (portal the menu, or move overflow off the card) risks the rounded
  corners. Needs a visual pass before touching it.
- **Likely fix:** render the menu through a portal (the heatmap tooltips
  already portal), or restructure the overflow clipping to the body only.

### 2. Heatmap level-1 is nearly indistinguishable from an empty cell

- **Where:** `--analytics-heat-1: #dbeafe` vs the zero-cell fill
  `--analytics-bg-subtle: #eef2f7` (`.analytics-heat-block` default).
- **What to check:** a low-activity cell (level 1) next to a no-activity cell —
  confirm they read as different, including under colorblind simulation.
- **Likely fix:** start the ramp at a mid blue so level 1 steps clearly away
  from the empty-cell gray.

### 3. Pervasive 10px caption text

- **Where:** `.analytics-kpi-label`, `.analytics-card-insight`,
  `.analytics-delta`, `.analytics-legend-item`, `.analytics-heat-label`
  (all `font-size: 10px`).
- **What to check:** readability of the KPI captions/insights on the target
  devices.
- **Likely fix:** bump the ones that carry meaning to 11–12px.

### 4. Payments % shares may not total 100

- **Where:** `PaymentsCard` in `ui/src/features/analytics/AnalyticsCardContent.tsx`
  — each method's share is `Math.round((total_minor / total) * 100)`
  independently.
- **What to check:** the stacked bar end gap when shares don't sum to 100.
- **Likely fix:** distribute the rounding remainder to the largest segment.

### 5. Drag / hover affordances

- **Where:** `.analytics-card-grip` shows `cursor: grab` but is decorative
  (`aria-hidden`) while the **whole card** is `draggable`; cards also lift on
  hover though the body isn't clickable.
- **What to check:** whether the grip implies only-the-grip drags, and whether
  the hover lift implies the body is clickable.
- **Likely fix:** drag from the grip only (or drop the grip), and limit the
  hover lift to the header.

### 6. Refunds vs Voids cards overlap (restaurant view)

- **Where:** `ANALYTICS_CARDS` in `ui/src/features/analytics/AnalyticsScreen.tsx`
  — `refunds` is shared (both workspaces) and `voids` is restaurant-only; both
  render the same voided-items ranked list.
- **What to check:** the restaurant dashboard showing two near-identical
  voided-item cards side by side.
- **Likely fix:** differentiate them (refunds → money/totals, voids → items),
  or drop one.

## Also deferred (accessibility, not strictly visual)

- **Heatmap per-cell tooltips are hover-only** — `heatCell()` in
  `AnalyticsScreen.tsx` wraps each cell in `Tooltip`, but the trigger has no
  `tabIndex`, so keyboard/AT users can't reach per-cell revenue/order detail.
  Likely fix: make cells focusable or surface the data another way.
- **`role="group"` + `aria-label` duplicates the card heading** — the card
  container carries `aria-label={title}` while the `<h2>` already names it.
  Likely fix: use `aria-labelledby` pointing at the `<h2>`, or drop the
  `aria-label`.

## Deferred — cache & query layer design items

Found during the cache/query audit (`analytics-cache.ts`,
`useAnalyticsQuery.ts`). Left unchanged because each needs a product or
perf call; the safe bug fixes and dead-code cleanup from that audit were
applied directly.

### 7. Eviction is FIFO, not LRU, and stale entries are never purged

- **Where:** `TtlCache.set` evicts `this.entries.keys().next().value` — the
  first-inserted key — and `get` does not promote recency. Expired entries
  stay in the map until evicted or explicitly cleared.
- **Why deferred:** at a 200-entry cap and 5-minute TTL the practical impact
  is low, but a full cache can fill with expired entries and keep evicting
  still-fresh ones.
- **Likely fix:** promote on `get` (Map delete + re-set) to make eviction LRU,
  and/or drop expired entries on write.

### 8. Side effect during render

- **Where:** `useAnalyticsQuery` runs `fetcher()` and
  `analyticsDataCache.set(...)` (which writes sessionStorage) synchronously
  during render for sync fetchers.
- **Why deferred:** documented and defended as idempotent/deterministic, and
  the screen has relied on it for instant no-flash rendering. Moving to
  `useEffect` would reintroduce an empty flash without extra state.
- **Likely fix:** keep, or shift the write into a `useLayoutEffect`/effect
  while retaining the sync `get` for the no-flash hit.

### 9. `failures` map is sticky per key

- **Where:** module-level `failures` in `useAnalyticsQuery.ts` persists until
  `clearAnalyticsErrors()` (the refresh action).
- **Why deferred:** the stickiness is what prevents the infinite-retry loop,
  but switching away from a failed workspace and back re-shows the old error
  without refetching. A product call: should re-navigation retry?
- **Likely fix:** key failures by something that also expires, or clear a key's
  failure when the query key changes.

### 10. Unvalidated cache cast + fragile partition parsing

- **Where:** `readCached` casts `unknown` → `T` with no shape check;
  `cachePartition` derives the workspace from `entryKey.split(':')` indexes.
- **Why deferred:** the version stamp guards schema drift at the persistence
  layer, and the key format is internal. But a shape change without a version
  bump would crash a card, and a key-format change silently misroutes to the
  `shared` partition.
- **Likely fix:** validate cached values at the type boundary, and derive the
  partition from a structured key object rather than string splitting.
