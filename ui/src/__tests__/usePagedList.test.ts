import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePagedList } from '@/hooks/usePagedList';
import { LIST_PAGE_SIZE } from '@/utils/list-policy';

/**
 * PERF-07 — unit tests for the shared bounded-list hook.
 */
describe('usePagedList (PERF-07)', () => {
  const make = (n: number) => Array.from({ length: n }, (_, i) => i);

  it('renders only the active page', () => {
    const { result } = renderHook(() => usePagedList(make(200)));
    expect(result.current.page).toBe(0);
    expect(result.current.total).toBe(4);
    expect(result.current.pageItems).toHaveLength(LIST_PAGE_SIZE);
  });

  it('navigates pages and clamps out-of-range pages', () => {
    const { result } = renderHook(() => usePagedList(make(100)));
    act(() => result.current.setPage(1));
    expect(result.current.page).toBe(1);
    expect(result.current.pageItems[0]).toBe(LIST_PAGE_SIZE);

    // Jump past the end — clamped to the last valid page.
    act(() => result.current.setPage(99));
    expect(result.current.page).toBe(1);
  });

  it('resets to the first page', () => {
    const { result } = renderHook(() => usePagedList(make(150)));
    act(() => result.current.setPage(2));
    act(() => result.current.resetPage());
    expect(result.current.page).toBe(0);
    expect(result.current.pageItems[0]).toBe(0);
  });

  it('keeps the page in range when the collection shrinks', () => {
    const { result, rerender } = renderHook(
      ({ items }) => usePagedList(items),
      { initialProps: { items: make(500) } },
    );
    act(() => result.current.setPage(9)); // last page (10 total)
    expect(result.current.page).toBe(9);

    // Shrink to a single page — page clamps back to 0.
    rerender({ items: make(10) });
    expect(result.current.page).toBe(0);
    expect(result.current.pageItems).toHaveLength(10);
  });

  it('handles an empty collection', () => {
    const { result } = renderHook(() => usePagedList([]));
    expect(result.current.total).toBe(1);
    expect(result.current.page).toBe(0);
    expect(result.current.pageItems).toHaveLength(0);
  });
});
