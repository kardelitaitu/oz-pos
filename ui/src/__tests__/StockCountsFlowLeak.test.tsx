import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { render } from '@testing-library/react';

// Mock all child components (same as StockCountsFlow.test.tsx).
vi.mock('@/features/inventory/StockCountsScreen', () => ({
  default: () => <div data-testid="stock-counts-screen">List View</div>,
}));
vi.mock('@/features/inventory/StockCountForm', () => ({
  default: ({ onCancel }: { onCancel: () => void }) => (
    <div data-testid="stock-count-form">
      <button onClick={onCancel}>Cancel Form</button>
    </div>
  ),
}));
vi.mock('@/features/inventory/StockCountDetail', () => ({
  default: ({ countId }: { countId: string }) => (
    <div data-testid="stock-count-detail">Detail: {countId}</div>
  ),
}));
vi.mock('@/features/inventory/StockCountHistory', () => ({
  default: () => <div data-testid="stock-count-history">History View</div>,
}));

import StockCountsFlow from '@/features/inventory/StockCountsFlow';

describe('StockCountsFlow hashchange listener leak (Bug #12)', () => {
  beforeEach(() => {
    window.location.hash = '';
  });

  afterEach(() => {
    window.location.hash = '';
  });

  it('removes the hashchange listener on unmount (no leak)', () => {
    // Spy on add/remove so we can assert the net listener balance
    // after a mount → unmount cycle.
    const addSpy = vi.spyOn(window, 'addEventListener');
    const removeSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = render(<StockCountsFlow />);

    // Snapshot the hashchange add/remove counts at mount time.
    // The render-body dance (removeEventListener then addEventListener
    // on every render) produces 1 add + 1 remove during mount.
    const addsAtMount = addSpy.mock.calls.filter(([e]) => e === 'hashchange').length;
    const removesAtMount = removeSpy.mock.calls.filter(([e]) => e === 'hashchange').length;

    unmount();

    // After unmount, a correctly-cleaned-up component must have called
    // removeEventListener('hashchange', ...) once MORE than it had at
    // mount time — that extra remove is the useEffect cleanup removing
    // the listener it added.
    const removesAfterUnmount = removeSpy.mock.calls.filter(([e]) => e === 'hashchange').length;

    // The component must add at least one hashchange listener to be
    // useful (otherwise hash-based navigation wouldn't work at all).
    expect(addsAtMount).toBeGreaterThanOrEqual(1);

    // BUG: the listener is added in the render body, not in a
    // useEffect with a cleanup. On unmount, NO cleanup runs, so the
    // hashchange listener is never removed — the remove count does not
    // increase across unmount.
    expect(removesAfterUnmount).toBeGreaterThan(removesAtMount);

    addSpy.mockRestore();
    removeSpy.mockRestore();
  });

  it('does not react to hashchange after unmount (no stale-state update)', () => {
    // A leaked listener will call setHash on an unmounted component.
    // React 18 logs a warning ("Can't perform a state update on an
    // unmounted component") — we capture console.error to detect it.
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { unmount } = render(<StockCountsFlow />);
    unmount();

    // Simulate a hash change after unmount (e.g. another screen, or a
    // late navigation). A leaked listener fires setHash on the
    // unmounted component.
    window.location.hash = '#stock-count-new';
    window.dispatchEvent(new HashChangeEvent('hashchange'));

    // Flush microtasks so any state update from a leaked listener lands.
    // (setState in an event listener is synchronous in React 18's
    // batching, but the warning is queued.)
    expect(consoleErrorSpy).not.toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
    window.location.hash = '';
  });
});
