import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act } from 'react';
import { renderHook } from '@testing-library/react';
import { useIdleTimer, getAutoLockMinutes, setAutoLockMinutes } from '@/hooks/useIdleTimer';

// ── getAutoLockMinutes / setAutoLockMinutes ─────────────────────────────
describe('getAutoLockMinutes', () => {
  beforeEach(() => { localStorage.clear(); });

  it('returns 5 by default when nothing is stored', () => {
    expect(getAutoLockMinutes()).toBe(5);
  });

  it('returns the stored value when it is a valid number', () => {
    localStorage.setItem('auto-lock-minutes', '10');
    expect(getAutoLockMinutes()).toBe(10);
  });

  it('returns 5 for invalid stored values', () => {
    localStorage.setItem('auto-lock-minutes', 'not-a-number');
    expect(getAutoLockMinutes()).toBe(5);
  });

  it('returns 5 for values below 1', () => {
    localStorage.setItem('auto-lock-minutes', '0');
    expect(getAutoLockMinutes()).toBe(5);
  });
});

describe('setAutoLockMinutes', () => {
  beforeEach(() => { localStorage.clear(); });

  it('persists a valid minute value', () => {
    setAutoLockMinutes(30);
    expect(localStorage.getItem('auto-lock-minutes')).toBe('30');
  });

  it('clamps values below 1 to 1', () => {
    setAutoLockMinutes(0);
    expect(localStorage.getItem('auto-lock-minutes')).toBe('1');
  });

  it('clamps values above 120 to 120', () => {
    setAutoLockMinutes(200);
    expect(localStorage.getItem('auto-lock-minutes')).toBe('120');
  });
});

// ── useIdleTimer ────────────────────────────────────────────────────────
describe('useIdleTimer', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('calls onIdle after the configured timeout', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1); // 1 minute = 60,000 ms
    renderHook(() => useIdleTimer(onIdle));

    expect(onIdle).not.toHaveBeenCalled();
    act(() => { vi.advanceTimersByTime(60_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('uses the default 5-minute timeout when nothing is configured', () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(onIdle));

    // 4 minutes should not trigger
    act(() => { vi.advanceTimersByTime(4 * 60_000); });
    expect(onIdle).not.toHaveBeenCalled();

    // 1 more minute = 5 total
    act(() => { vi.advanceTimersByTime(60_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets the timer on mousedown', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1);
    renderHook(() => useIdleTimer(onIdle));

    // Advance 30s then trigger mousedown
    act(() => { vi.advanceTimersByTime(30_000); });
    act(() => { window.dispatchEvent(new MouseEvent('mousedown')); });
    // Advance another 30s (total 60s from mount but 30s from reset)
    act(() => { vi.advanceTimersByTime(30_000); });
    expect(onIdle).not.toHaveBeenCalled();

    // Another 30s should fire (60s from reset)
    act(() => { vi.advanceTimersByTime(30_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets the timer on keydown', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1);
    renderHook(() => useIdleTimer(onIdle));

    act(() => { vi.advanceTimersByTime(45_000); });
    act(() => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' })); });
    act(() => { vi.advanceTimersByTime(45_000); });
    expect(onIdle).not.toHaveBeenCalled();

    act(() => { vi.advanceTimersByTime(15_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets the timer on touchstart', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1);
    renderHook(() => useIdleTimer(onIdle));

    act(() => { vi.advanceTimersByTime(59_000); });
    act(() => { window.dispatchEvent(new TouchEvent('touchstart')); });
    act(() => { vi.advanceTimersByTime(59_000); });
    expect(onIdle).not.toHaveBeenCalled();

    act(() => { vi.advanceTimersByTime(1_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets the timer on scroll', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1);
    renderHook(() => useIdleTimer(onIdle));

    act(() => { vi.advanceTimersByTime(50_000); });
    act(() => { window.dispatchEvent(new Event('scroll')); });
    act(() => { vi.advanceTimersByTime(50_000); });
    expect(onIdle).not.toHaveBeenCalled();

    act(() => { vi.advanceTimersByTime(10_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('clears the timeout on unmount', () => {
    const onIdle = vi.fn();
    const { unmount } = renderHook(() => useIdleTimer(onIdle));

    unmount();
    act(() => { vi.advanceTimersByTime(10 * 60_000); });
    expect(onIdle).not.toHaveBeenCalled();
  });

  it('uses the latest onIdle callback even after re-render', () => {
    const firstCallback = vi.fn();
    const secondCallback = vi.fn();

    // Set lock to 1 minute BEFORE mount so the timer starts with 60s
    localStorage.setItem('auto-lock-minutes', '1');

    const { rerender } = renderHook(
      ({ cb }) => useIdleTimer(cb),
      { initialProps: { cb: firstCallback } },
    );

    rerender({ cb: secondCallback });
    act(() => { vi.advanceTimersByTime(60_000); });

    expect(firstCallback).not.toHaveBeenCalled();
    expect(secondCallback).toHaveBeenCalledTimes(1);
  });

  it('resets timer on wheel event', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1);
    renderHook(() => useIdleTimer(onIdle));

    act(() => { vi.advanceTimersByTime(55_000); });
    act(() => { window.dispatchEvent(new Event('wheel')); });
    act(() => { vi.advanceTimersByTime(55_000); });
    expect(onIdle).not.toHaveBeenCalled();

    act(() => { vi.advanceTimersByTime(5_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('does not fire twice when multiple activity events occur within a single interval', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1);
    renderHook(() => useIdleTimer(onIdle));

    // Fire multiple events in quick succession
    for (let i = 0; i < 10; i++) {
      act(() => { window.dispatchEvent(new MouseEvent('mousedown')); });
    }
    act(() => { vi.advanceTimersByTime(60_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('does not call onIdle when timer is repeatedly reset before expiry', () => {
    const onIdle = vi.fn();
    setAutoLockMinutes(1);
    renderHook(() => useIdleTimer(onIdle));

    // Keep resetting the timer just before it would fire
    for (let i = 0; i < 10; i++) {
      act(() => { vi.advanceTimersByTime(55_000); });
      act(() => { window.dispatchEvent(new MouseEvent('mousedown')); });
    }

    // Should not have fired yet
    expect(onIdle).not.toHaveBeenCalled();

    // Let it finally expire
    act(() => { vi.advanceTimersByTime(60_000); });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });
});
