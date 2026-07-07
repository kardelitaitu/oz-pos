import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useMediaQuery } from '@/hooks/useMediaQuery';

describe('useMediaQuery', () => {
  let listeners: Map<string, (e: MediaQueryListEvent) => void>;

  beforeEach(() => {
    listeners = new Map();

    vi.spyOn(window, 'matchMedia').mockImplementation((query: string) => {
      const mql = {
        matches: query === '(min-width: 768px)',
        media: query,
        addEventListener: (event: string, listener: unknown) => {
          listeners.set(query, listener as (e: MediaQueryListEvent) => void);
        },
        removeEventListener: (event: string, listener: unknown) => {
          listeners.delete(query);
        },
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
        onchange: null,
      } as unknown as MediaQueryList;
      return mql;
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns true when media query matches', () => {
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(true);
  });

  it('returns false when media query does not match', () => {
    const { result } = renderHook(() => useMediaQuery('(min-width: 1200px)'));
    expect(result.current).toBe(false);
  });

  it('updates when the media query change event fires', () => {
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(true);

    const listener = listeners.get('(min-width: 768px)');
    expect(listener).toBeDefined();

    // Simulate a change event
    act(() => {
      listener!({ matches: false } as MediaQueryListEvent);
    });

    expect(result.current).toBe(false);
  });

  it('removes event listener on unmount', () => {
    const { unmount } = renderHook(() => useMediaQuery('(min-width: 768px)'));

    // Listener should be registered
    expect(listeners.has('(min-width: 768px)')).toBe(true);

    unmount();

    // Listener should be cleaned up
    expect(listeners.has('(min-width: 768px)')).toBe(false);
  });
});
