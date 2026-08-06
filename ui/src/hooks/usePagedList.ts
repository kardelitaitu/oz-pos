import { useCallback, useMemo, useState, type Dispatch, type SetStateAction } from 'react';
import { LIST_PAGE_SIZE, paginate, totalPages } from '@/utils/list-policy';

/**
 * PERF-07 — Shared bounded-list hook.
 *
 * The cross-feature data-list primitive: wraps any collection in bounded
 * pagination with a single `LIST_PAGE_SIZE` contract. Screens use this
 * instead of hand-rolling page state + `.slice()`, so every unbounded
 * collection shares the same limits and math.
 *
 * Rows are only materialized for the active page, keeping DOM/node count
 * bounded regardless of the underlying dataset size.
 */
export function usePagedList<T>(items: T[], pageSize: number = LIST_PAGE_SIZE) {
  const [page, setPageState] = useState(0);

  const total = useMemo(() => totalPages(items.length, pageSize), [items.length, pageSize]);

  // Clamp so a shrink (filter/search/sort) never leaves an out-of-range page.
  const safePage = Math.min(page, total - 1);

  const pageItems = useMemo(
    () => paginate(items, safePage, pageSize),
    [items, safePage, pageSize],
  );

  /** Reset to the first page (call on filter/search changes). */
  const resetPage = useCallback(() => setPageState(0), []);

  // Expose the raw setter so consumers can pass either a number or a
  // functional update (Dispatch<SetStateAction<number>>). Out-of-range
  // values are clamped on read via `safePage`, never in the state itself.
  const setPage: Dispatch<SetStateAction<number>> = setPageState;

  return { page: safePage, total, pageItems, setPage, resetPage };
}
