// ── useSessionKeepalive + useInvalidSession hook tests ─────────────────
//
// 1. useSessionKeepalive fires `session_keepalive` on mount with a token
// 2. useSessionKeepalive skips pinging when no token is provided
// 3. useSessionKeepalive cleans up the interval on unmount
// 4. useInvalidSession returns false initially
// 5. useInvalidSession returns true after an `invalidSession` IPC error
// 6. useInvalidSession auto-clears after 5 seconds

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act } from 'react';
import { renderHookInAct } from '@/test-utils/renderInAct';
import { useSessionKeepalive } from '@/hooks/useSessionKeepalive';
import { useInvalidSession } from '@/hooks/useInvalidSession';

// ── Hoisted mocks ─────────────────────────────────────────────────

const mocks = vi.hoisted(() => ({
  invoke: vi.fn<(...args: unknown[]) => unknown>(),
  onIpcError: vi.fn<(fn: (event: { command: string; error: { kind: string } }) => void) => () => void>(),
  unsubscribe: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mocks.invoke(...args),
}));

// The hook uses `loggedInvoke` from `@/utils/logged-invoke` which wraps
// `invoke` from `@tauri-apps/api/core`. The factory above already mocks
// the core invoke; `loggedInvoke` re-exports the same function.

vi.mock('@/utils/app-error', () => ({
  onIpcError: (fn: (event: { command: string; error: { kind: string } }) => void) => {
    mocks.onIpcError(fn);
    return mocks.unsubscribe;
  },
  emitIpcError: vi.fn(),
  redactedDiagnostic: vi.fn(() => 'redacted'),
  userErrorKey: vi.fn(() => 'app-error-session'),
  parseAppError: vi.fn((err: unknown) => {
    if (typeof err === 'object' && err !== null && 'kind' in err) return err as { kind: string };
    return null;
  }),
}));

// ── useSessionKeepalive ─────────────────────────────────────────────

describe('useSessionKeepalive', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mocks.invoke.mockReset();
    mocks.invoke.mockResolvedValue({ expires_at: Math.floor(Date.now() / 1000) + 86400 });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('fires session_keepalive on mount with a valid token', async () => {
    await renderHookInAct(() => useSessionKeepalive('mock-token'));
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.invoke).toHaveBeenCalledWith('session_keepalive', { sessionToken: 'mock-token' });
  });

  it('does not fire when token is null or empty', async () => {
    await renderHookInAct(() => useSessionKeepalive(null));
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it('fires again after the interval (10 min) while visible', async () => {
    // Override document.visibilityState for the mock
    Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true });
    await renderHookInAct(() => useSessionKeepalive('mock-token'));
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);

    // Advance 10 minutes — should fire again
    await vi.advanceTimersByTimeAsync(10 * 60 * 1000);
    expect(mocks.invoke).toHaveBeenCalledTimes(2);

    // Advance another 10 minutes
    await vi.advanceTimersByTimeAsync(10 * 60 * 1000);
    expect(mocks.invoke).toHaveBeenCalledTimes(3);
  });

  it('does not fire while the document is hidden', async () => {
    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true });
    await renderHookInAct(() => useSessionKeepalive('mock-token'));
    await vi.advanceTimersByTimeAsync(0);
    // Initial ping fires regardless of visibility
    expect(mocks.invoke).toHaveBeenCalledTimes(1);

    // Advance 10 minutes — should NOT fire while hidden
    await vi.advanceTimersByTimeAsync(10 * 60 * 1000);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
  });

  it('fires on visibility change back to visible', async () => {
    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true });
    await renderHookInAct(() => useSessionKeepalive('mock-token'));
    await vi.advanceTimersByTimeAsync(0);
    mocks.invoke.mockClear();

    // Simulate visibility change to visible
    Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true });
    act(() => { document.dispatchEvent(new Event('visibilitychange')); });
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.invoke).toHaveBeenCalled();
  });
});

// ── useInvalidSession ────────────────────────────────────────────────

describe('useInvalidSession', () => {
  let listeners: Array<(event: { command: string; error: { kind: string } }) => void> = [];

  beforeEach(() => {
    vi.useFakeTimers();
    listeners = [];
    mocks.onIpcError.mockImplementation((fn) => {
      listeners.push(fn);
      return mocks.unsubscribe;
    });
    mocks.unsubscribe.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns false when no IPC error has occurred', async () => {
    const { result } = await renderHookInAct(() => useInvalidSession());
    expect(result.current).toBe(false);
  });

  it('returns true after an invalidSession IPC error', async () => {
    const { result } = await renderHookInAct(() => useInvalidSession());
    expect(result.current).toBe(false);

    // Simulate an invalidSession IPC error
    act(() => {
      for (const fn of listeners) {
        fn({ command: 'get_weekly_revenue', error: { kind: 'invalidSession' } });
      }
    });
    expect(result.current).toBe(true);
  });

  it('returns true for 5 seconds then auto-clears', async () => {
    const { result } = await renderHookInAct(() => useInvalidSession());

    // Fire the error
    act(() => {
      for (const fn of listeners) {
        fn({ command: 'get_weekly_revenue', error: { kind: 'invalidSession' } });
      }
    });
    expect(result.current).toBe(true);

    // Advance 4.9s — still true
    act(() => { vi.advanceTimersByTime(4900); });
    expect(result.current).toBe(true);

    // Advance past 5s — clears
    act(() => { vi.advanceTimersByTime(200); });
    expect(result.current).toBe(false);
  });

  it('ignores non-session IPC errors', async () => {
    const { result } = await renderHookInAct(() => useInvalidSession());

    act(() => {
      for (const fn of listeners) {
        fn({ command: 'get_weekly_revenue', error: { kind: 'core' } });
        fn({ command: 'get_daily_revenue', error: { kind: 'permissionDenied' } });
      }
    });
    expect(result.current).toBe(false);
  });

  it('unsubscribes on unmount', async () => {
    const { unmount } = await renderHookInAct(() => useInvalidSession());
    unmount();
    expect(mocks.unsubscribe).toHaveBeenCalled();
  });
});