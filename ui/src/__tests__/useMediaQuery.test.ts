import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act } from 'react';
import { renderHook } from '@testing-library/react';
import { useMediaQuery } from '@/hooks/useMediaQuery';

function mockMatchMedia(initialMatches: boolean) {
  const listeners = new Set<(e: MediaQueryListEvent) => void>();
  const mql = {
    matches: initialMatches,
    media: '(min-width: 768px)',
    addEventListener: vi.fn((_event: string, handler: (e: MediaQueryListEvent) => void) => {
      listeners.add(handler);
    }),
    removeEventListener: vi.fn((_event: string, handler: (e: MediaQueryListEvent) => void) => {
      listeners.delete(handler);
    }),
  };

  const matchMediaFn = vi.fn().mockReturnValue(mql);

  // Expose helper to simulate a change event
  (matchMediaFn as unknown as Record<string, unknown>)['_dispatchChange'] = (newMatches: boolean) => {
    mql.matches = newMatches;
    const event = { matches: newMatches, media: mql.media } as MediaQueryListEvent;
    for (const listener of listeners) {
      listener(event);
    }
  };

  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: matchMediaFn,
  });

  return matchMediaFn as typeof matchMediaFn & { _dispatchChange: (v: boolean) => void };
}

describe('useMediaQuery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns false when the initial match is false', () => {
    mockMatchMedia(false);
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(false);
  });

  it('returns true when the initial match is true', () => {
    mockMatchMedia(true);
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(true);
  });

  it('updates when the media query change event fires', () => {
    const mqlMock = mockMatchMedia(false);
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(false);

    act(() => {
      mqlMock._dispatchChange(true);
    });
    expect(result.current).toBe(true);

    act(() => {
      mqlMock._dispatchChange(false);
    });
    expect(result.current).toBe(false);
  });

  it('registers a change listener on mount', () => {
    mockMatchMedia(false);
    renderHook(() => useMediaQuery('(min-width: 768px)'));
    const mql = (window.matchMedia as ReturnType<typeof vi.fn>).mock.results[0]!.value as {
      addEventListener: ReturnType<typeof vi.fn>;
    };
    expect(mql.addEventListener).toHaveBeenCalledWith('change', expect.any(Function));
  });

  it('removes the listener on unmount', () => {
    mockMatchMedia(false);
    const { unmount } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    const mql = (window.matchMedia as ReturnType<typeof vi.fn>).mock.results[0]!.value as {
      removeEventListener: ReturnType<typeof vi.fn>;
    };
    unmount();
    expect(mql.removeEventListener).toHaveBeenCalled();
  });

  it('re-initialises when the query string changes', () => {
    mockMatchMedia(false);
    const { rerender } = renderHook(({ query }) => useMediaQuery(query), {
      initialProps: { query: '(min-width: 768px)' },
    });

    // Clear calls to measure new calls after rerender
    const matchMediaFn = window.matchMedia as ReturnType<typeof vi.fn>;
    matchMediaFn.mockClear();

    rerender({ query: '(min-width: 1024px)' });
    expect(matchMediaFn).toHaveBeenCalledWith('(min-width: 1024px)');
  });

  it('uses the value from matchMedia.matches as initial state', () => {
    mockMatchMedia(true);
    const { result } = renderHook(() => useMediaQuery('(prefers-color-scheme: dark)'));
    expect(result.current).toBe(true);
    expect(window.matchMedia).toHaveBeenCalledWith('(prefers-color-scheme: dark)');
  });

  it('cleans up old listener when query changes', () => {
    mockMatchMedia(false);
    const { rerender } = renderHook(({ query }) => useMediaQuery(query), {
      initialProps: { query: '(min-width: 768px)' },
    });

    const firstMql = (window.matchMedia as ReturnType<typeof vi.fn>).mock.results[0]!.value as {
      removeEventListener: ReturnType<typeof vi.fn>;
    };

    rerender({ query: '(min-width: 1024px)' });

    // The old listener should have been removed
    expect(firstMql.removeEventListener).toHaveBeenCalled();
  });
});
