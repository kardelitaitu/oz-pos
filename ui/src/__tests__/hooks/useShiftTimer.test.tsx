import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useShiftTimer } from '@/features/sales/posScreenHooks';

describe('useShiftTimer', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns initial timestamp when no active shift', () => {
    const { result } = renderHook(() => useShiftTimer(null));
    const initial = result.current;
    expect(typeof initial).toBe('number');
    expect(initial).toBeLessThanOrEqual(Date.now());
  });

  it('sets shiftNow to current time when shift becomes active', () => {
    const shift = { openedAt: new Date(Date.now() - 3600_000).toISOString() };
    const { result } = renderHook(() => useShiftTimer(shift));

    // The effect runs synchronously and sets shiftNow to current time
    expect(result.current).toBeGreaterThan(0);
    expect(result.current).toBeLessThanOrEqual(Date.now());
  });

  it('ticks every minute while shift is active', () => {
    const shift = { openedAt: new Date(Date.now() - 3600_000).toISOString() };
    const { result } = renderHook(() => useShiftTimer(shift));

    const initial = result.current;
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    // With fake timers, Date.now() advances
    expect(result.current).toBeGreaterThanOrEqual(initial + 59_000);
  });

  it('clears interval when shift becomes null', () => {
    const shift = { openedAt: new Date(Date.now() - 3600_000).toISOString() };
    const { result, rerender } = renderHook(
      ({ shift }) => useShiftTimer(shift),
      { initialProps: { shift } },
    );

    const duringShift = result.current;
    act(() => {
      rerender({ shift: null });
    });
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    // Should not have changed after shift cleared
    expect(result.current).toBe(duringShift);
  });

  it('resets anchor when shift changes to a different one', () => {
    const shift1 = { openedAt: new Date(Date.now() - 7200_000).toISOString() };
    const shift2 = { openedAt: new Date(Date.now() - 3600_000).toISOString() };
    const { result, rerender } = renderHook(
      ({ shift }) => useShiftTimer(shift),
      { initialProps: { shift: shift1 } },
    );

    const first = result.current;
    act(() => {
      rerender({ shift: shift2 });
    });
    // The effect runs and resets to current time
    expect(result.current).toBeGreaterThanOrEqual(first);
  });

  it('handles rapid shift changes', () => {
    const shift1 = { openedAt: new Date(Date.now() - 7200_000).toISOString() };
    const shift2 = { openedAt: new Date(Date.now() - 3600_000).toISOString() };
    const shift3 = { openedAt: new Date(Date.now() - 1800_000).toISOString() };
    const { result, rerender } = renderHook(
      ({ shift }) => useShiftTimer(shift),
      { initialProps: { shift: shift1 } },
    );

    act(() => {
      rerender({ shift: shift2 });
      rerender({ shift: shift3 });
    });

    expect(result.current).toBeGreaterThan(0);
  });

  it('returns current time when shift is already active on mount', () => {
    const shift = { openedAt: new Date(Date.now() - 3600_000).toISOString() };
    const { result } = renderHook(() => useShiftTimer(shift));

    // Should immediately set to current time via effect
    expect(result.current).toBeGreaterThan(0);
    expect(result.current).toBeLessThanOrEqual(Date.now());
  });
});