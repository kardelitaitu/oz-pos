import { describe, expect, it, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useTicketSla } from '@/features/kds/hooks/useTicketSla';

// ── Helpers ──────────────────────────────────────────────────────────

/** Create an ISO-8601 timestamp offset by `secondsAgo` seconds. */
function ago(secondsAgo: number): string {
  return new Date(Date.now() - secondsAgo * 1000).toISOString();
}

// ── Tests ────────────────────────────────────────────────────────────

describe('useTicketSla', () => {
  // ── Threshold levels (P3-1: green <5min, yellow 5-10min, red ≥10min, urgent ≥15min) ─

  it('returns green for tickets received less than 5 minutes ago', () => {
    const { result } = renderHook(() => useTicketSla(ago(0)));
    expect(result.current.level).toBe('green');
    expect(result.current.elapsedSeconds).toBeLessThan(300);
  });

  it('returns yellow for tickets received 5-10 minutes ago', () => {
    const { result } = renderHook(() => useTicketSla(ago(360)));
    expect(result.current.level).toBe('yellow');
  });

  it('returns red for tickets received more than 10 minutes ago', () => {
    const { result } = renderHook(() => useTicketSla(ago(660)));
    expect(result.current.level).toBe('red');
  });

  it('returns green at exactly 299 seconds (just under 5 min)', () => {
    const { result } = renderHook(() => useTicketSla(ago(299)));
    expect(result.current.level).toBe('green');
  });

  it('returns yellow at exactly 300 seconds (5 min boundary)', () => {
    const { result } = renderHook(() => useTicketSla(ago(300)));
    expect(result.current.level).toBe('yellow');
  });

  it('returns yellow at exactly 599 seconds (just under 10 min)', () => {
    const { result } = renderHook(() => useTicketSla(ago(599)));
    expect(result.current.level).toBe('yellow');
  });

  it('returns red at exactly 600 seconds (10 min boundary)', () => {
    const { result } = renderHook(() => useTicketSla(ago(600)));
    expect(result.current.level).toBe('red');
  });

  it('returns urgent=true at exactly 900 seconds (15 min)', () => {
    const { result } = renderHook(() => useTicketSla(ago(900)));
    expect(result.current.level).toBe('red');
    expect(result.current.urgent).toBe(true);
  });

  it('returns urgent=false below 15 min', () => {
    const { result } = renderHook(() => useTicketSla(ago(600)));
    expect(result.current.urgent).toBe(false);
  });

  // ── Display formatting ─────────────────────────────────────────

  it('displays seconds-only for < 1 minute', () => {
    const { result } = renderHook(() => useTicketSla(ago(30)));
    expect(result.current.display).toBe('30s');
  });

  it('displays minutes-only for exact minutes', () => {
    const { result } = renderHook(() => useTicketSla(ago(300)));
    expect(result.current.display).toBe('5m');
  });

  it('displays minutes and seconds', () => {
    const { result } = renderHook(() => useTicketSla(ago(330)));
    expect(result.current.display).toBe('5m 30s');
  });

  // ── Elapsed seconds ────────────────────────────────────────────

  it('returns elapsedSeconds matching the offset', () => {
    const { result } = renderHook(() => useTicketSla(ago(90)));
    expect(result.current.elapsedSeconds).toBeGreaterThanOrEqual(85);
    expect(result.current.elapsedSeconds).toBeLessThanOrEqual(95);
  });

  it('clamps elapsedSeconds to 0 for future timestamps', () => {
    const future = new Date(Date.now() + 60_000).toISOString();
    const { result } = renderHook(() => useTicketSla(future));
    expect(result.current.elapsedSeconds).toBe(0);
  });

  // ── Tick / recomputation behavior ──────────────────────────────

  it('recomputes elapsed time when createdAt changes to an older timestamp', () => {
    // Initial: 5 seconds ago → elapsed ~5
    const { result, rerender } = renderHook(
      (props: { createdAt: string }) => useTicketSla(props.createdAt),
      { initialProps: { createdAt: ago(5) } },
    );
    const initial = result.current.elapsedSeconds;
    expect(initial).toBeGreaterThanOrEqual(0);

    // Re-render with 10 seconds ago → elapsed should increase by ~5
    rerender({ createdAt: ago(10) });
    expect(result.current.elapsedSeconds).toBeGreaterThan(initial + 3);
  });

  it('recomputes elapsed time when createdAt changes to a newer timestamp', () => {
    // Initial: 10 seconds ago → elapsed ~10
    const { result, rerender } = renderHook(
      (props: { createdAt: string }) => useTicketSla(props.createdAt),
      { initialProps: { createdAt: ago(10) } },
    );
    const initial = result.current.elapsedSeconds;

    // Re-render with 2 seconds ago → elapsed should decrease
    rerender({ createdAt: ago(2) });
    expect(result.current.elapsedSeconds).toBeLessThan(initial);
  });

  it('sets up a setInterval on mount', () => {
    const spy = vi.spyOn(globalThis, 'setInterval');
    const { unmount } = renderHook(() => useTicketSla(ago(0)));

    expect(spy).toHaveBeenCalledWith(expect.any(Function), 1000);

    spy.mockRestore();
    unmount();
  });

  it('clears the interval on unmount', () => {
    const clearSpy = vi.spyOn(globalThis, 'clearInterval');
    const { unmount } = renderHook(() => useTicketSla(ago(0)));
    unmount();
    expect(clearSpy).toHaveBeenCalled();
    clearSpy.mockRestore();
  });

  // ── Edge cases ─────────────────────────────────────────────────

  it('handles empty createdAt string gracefully', () => {
    const { result } = renderHook(() => useTicketSla(''));
    expect(typeof result.current.level).toBe('string');
  });

  it('handles invalid date string gracefully', () => {
    const { result } = renderHook(() => useTicketSla('not-a-date'));
    expect(typeof result.current.level).toBe('string');
  });
});
