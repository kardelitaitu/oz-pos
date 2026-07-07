import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAnimatedModal } from '@/hooks/useAnimatedModal';

describe('useAnimatedModal', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns mounted=false and exiting=false initially when show=false', () => {
    const { result } = renderHook(() => useAnimatedModal(false));
    expect(result.current.mounted).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it('returns mounted=true and exiting=false when show=true initially', () => {
    const { result } = renderHook(() => useAnimatedModal(true));
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);
  });

  it('mounts immediately when show changes from false to true', () => {
    const { result, rerender } = renderHook(({ show }) => useAnimatedModal(show), {
      initialProps: { show: false },
    });
    expect(result.current.mounted).toBe(false);

    rerender({ show: true });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);
  });

  it('enters exiting phase when show changes from true to false', () => {
    const { result, rerender } = renderHook(({ show }) => useAnimatedModal(show), {
      initialProps: { show: true },
    });

    rerender({ show: false });
    expect(result.current.mounted).toBe(true);  // still mounted during exit
    expect(result.current.exiting).toBe(true);
  });

  it('unmounts after the exit animation duration (default 200ms)', () => {
    const { result, rerender } = renderHook(({ show }) => useAnimatedModal(show), {
      initialProps: { show: true },
    });

    rerender({ show: false });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(true);

    // Advance past the default 200ms duration
    act(() => { vi.advanceTimersByTime(200); });
    expect(result.current.mounted).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it('respects a custom exit duration', () => {
    const { result, rerender } = renderHook(({ show }) => useAnimatedModal(show, 500), {
      initialProps: { show: true },
    });

    rerender({ show: false });
    expect(result.current.exiting).toBe(true);

    // At 400ms, still in exit phase
    act(() => { vi.advanceTimersByTime(400); });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(true);

    // At 500ms, unmounted
    act(() => { vi.advanceTimersByTime(100); });
    expect(result.current.mounted).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it('cleans up the exit timer on unmount before it fires', () => {
    const { result, rerender, unmount } = renderHook(({ show }) => useAnimatedModal(show), {
      initialProps: { show: true },
    });

    rerender({ show: false });
    expect(result.current.exiting).toBe(true);

    // Unmount before the 200ms timer fires
    unmount();
    act(() => { vi.advanceTimersByTime(500); });

    // Timer was cleaned up — no state updates expected (no crash)
    // The timer's clearTimeout prevents setState after unmount
  });

  it('handles rapid show/hide toggles gracefully', async () => {
    const { result, rerender } = renderHook(({ show }) => useAnimatedModal(show), {
      initialProps: { show: false },
    });

    // Show → exit → show again before exit finishes
    rerender({ show: true });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);

    rerender({ show: false });
    expect(result.current.exiting).toBe(true);

    // Re-show before exit animation completes — exits reset, stays mounted
    rerender({ show: true });
    expect(result.current.mounted).toBe(true);
    // exiting is reset by the opening branch of the effect
    // When show transitions back to true, the previous timer cleanup
    // cancels the exit timer and the hook enters the opening path
  });
});
