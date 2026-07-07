import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useMediaQuery } from '@/hooks/useMediaQuery';

describe('useMediaQuery', () => {
  let listeners: Array<(e: MediaQueryListEvent) => void> = [];
  let currentMatches = true;

  const createMockMql = (matches: boolean) => {
    currentMatches = matches;
    listeners = [];
    return {
      matches,
      media: '(min-width: 768px)',
      onchange: null,
      addEventListener: vi.fn((_event: string, fn: (e: MediaQueryListEvent) => void) => {
        listeners.push(fn);
      }),
      removeEventListener: vi.fn((_event: string, fn: (e: MediaQueryListEvent) => void) => {
        listeners = listeners.filter((l) => l !== fn);
      }),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(() => false),
    };
  };

  let mockMql: ReturnType<typeof createMockMql>;

  beforeEach(() => {
    mockMql = createMockMql(true);
    vi.spyOn(window, 'matchMedia').mockImplementation(() => mockMql as unknown as MediaQueryList);
  });

  it('returns the initial match value', () => {
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(true);
  });

  it('returns false when matchMedia reports false', () => {
    mockMql = createMockMql(false);
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(false);
  });

  it('registers a change listener', () => {
    renderHook(() => useMediaQuery('(min-width: 768px)'));
    // addEventListener should have been called with 'change' and a function
    const addCalls = (mockMql.addEventListener as ReturnType<typeof vi.fn>).mock.calls;
    const changeCall = addCalls.find(([event]: [string, unknown]) => event === 'change');
    expect(changeCall).toBeDefined();
  });

  it('updates matches when the media query changes', () => {
    const { result } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    expect(result.current).toBe(true);

    // Simulate a media query change
    act(() => {
      const event = { matches: false } as MediaQueryListEvent;
      listeners.forEach((fn) => fn(event));
    });

    expect(result.current).toBe(false);
  });

  it('removes the listener on unmount', () => {
    const { unmount } = renderHook(() => useMediaQuery('(min-width: 768px)'));
    unmount();

    const removeCalls = (mockMql.removeEventListener as ReturnType<typeof vi.fn>).mock.calls;
    const changeCall = removeCalls.find(([event]: [string, unknown]) => event === 'change');
    expect(changeCall).toBeDefined();
  });
});
