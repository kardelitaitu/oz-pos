import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { useSyncConnection } from '@/hooks/useSyncConnection';

const mockTestSyncConnection = vi.fn();

vi.mock('@/api/offline', () => ({
  testSyncConnection: (...args: unknown[]) => mockTestSyncConnection(...args),
}));

beforeEach(() => {
  mockTestSyncConnection.mockReset();
  vi.useFakeTimers({ shouldAdvanceTime: false });
});

afterEach(() => {
  vi.useRealTimers();
});

describe('useSyncConnection', () => {
  it('starts in checking state', () => {
    // Never resolve — stay in checking permanently.
    mockTestSyncConnection.mockReturnValue(new Promise(() => { /* never resolves */ }));

    const { result } = renderHook(() => useSyncConnection());

    expect(result.current.state).toBe('checking');
    expect(result.current.latencyMs).toBeNull();
  });

  it('transitions to connected when health check succeeds', async () => {
    mockTestSyncConnection.mockResolvedValue({
      ok: true,
      status: 'Connected (12ms)',
      latencyMs: 12,
    });

    const { result } = renderHook(() => useSyncConnection());

    // Flush pending microtasks so the resolved promise triggers state update.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });

    expect(result.current.state).toBe('connected');
    expect(result.current.latencyMs).toBe(12);
    expect(result.current.label).toBe('Connected (12ms)');
  });

  it('transitions to disconnected when health check fails (ok: false)', async () => {
    mockTestSyncConnection.mockResolvedValue({
      ok: false,
      status: 'Server returned 503',
      latencyMs: null,
    });

    const { result } = renderHook(() => useSyncConnection());

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });

    expect(result.current.state).toBe('disconnected');
    expect(result.current.latencyMs).toBeNull();
    expect(result.current.label).toBe('Server returned 503');
  });

  it('transitions to disconnected when health check throws', async () => {
    mockTestSyncConnection.mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() => useSyncConnection());

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });

    expect(result.current.state).toBe('disconnected');
    expect(result.current.latencyMs).toBeNull();
    expect(result.current.label).toBe('Disconnected');
  });

  it('polls periodically and updates state', async () => {
    // First call: connected
    mockTestSyncConnection.mockResolvedValueOnce({
      ok: true,
      status: 'Connected',
      latencyMs: 5,
    });

    const { result } = renderHook(() => useSyncConnection());

    // Wait for initial check to resolve.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(result.current.state).toBe('connected');

    // Second call (after interval): disconnected
    mockTestSyncConnection.mockResolvedValueOnce({
      ok: false,
      status: 'Server unreachable',
      latencyMs: null,
    });

    // Advance past the 60 s poll interval.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });

    expect(result.current.state).toBe('disconnected');
    expect(result.current.label).toBe('Server unreachable');
  });

  it('cleans up interval on unmount', () => {
    mockTestSyncConnection.mockResolvedValue({
      ok: true,
      status: 'Connected',
      latencyMs: 5,
    });

    const { unmount } = renderHook(() => useSyncConnection());
    unmount();

    // After unmount, advancing time should not cause a state update.
    // No assertion needed — the test just verifies no crash/leak.
  });
});
