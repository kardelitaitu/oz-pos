/**
 * PERF-07 — Shared large-list policy.
 *
 * Every unbounded collection in the UI must render through bounded
 * pagination (this module) unless the surface truly needs virtualization.
 *
 * Contract:
 *   - `LIST_PAGE_SIZE` is the default page cap for all data screens.
 *   - `paginate()`/`totalPages()` are the single source of truth for
 *     slicing — screens must not hand-roll `.slice()` with their own
 *     constants.
 *   - Virtualization (`react-window`) is reserved for row sets where
 *     interaction semantics support it (e.g. `ProductLookupScreen`).
 *     For dense grids that need full-table semantics (sort headers,
 *     sticky rows, variable-height cells), bounded paging is the
 *     established cross-feature contract instead.
 */

/** Default page size for every unbounded collection. */
export const LIST_PAGE_SIZE = 50;

/**
 * Virtualization threshold. Collections larger than this page count
 * should be evaluated for `react-window` virtualization; below it,
 * bounded paging is sufficient.
 */
export const LIST_VIRTUALIZE_THRESHOLD_PAGES = 20;

/** Return the slice of `items` for the given zero-based page. */
export function paginate<T>(items: T[], page: number, pageSize: number = LIST_PAGE_SIZE): T[] {
  const safePage = Math.max(0, page);
  const start = safePage * pageSize;
  return items.slice(start, start + pageSize);
}

/** Total number of pages for `itemCount` rows (always >= 1). */
export function totalPages(itemCount: number, pageSize: number = LIST_PAGE_SIZE): number {
  return Math.max(1, Math.ceil(itemCount / pageSize));
}
