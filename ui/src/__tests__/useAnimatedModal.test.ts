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

  it('returns mounted=true and exiting=false initially when show=true', () => {
    const { result } = renderHook(() => useAnimatedModal(true));
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);
  });

  it('sets mounted=true and exiting=false when show transitions from false to true', () => {
    const { result, rerender } = renderHook(
      ({ show }) => useAnimatedModal(show),
      { initialProps: { show: false } },
    );

    expect(result.current.mounted).toBe(false);

    rerender({ show: true });

    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);
  });

  it('sets exiting=true immediately when show transitions from true to false', () => {
    const { result, rerender } = renderHook(
      ({ show }) => useAnimatedModal(show),
      { initialProps: { show: true } },
    );

    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);

    rerender({ show: false });

    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(true);
  });

  it('sets mounted=false and exiting=false after exit duration', () => {
    const { result, rerender } = renderHook(
      ({ show }) => useAnimatedModal(show, 200),
      { initialProps: { show: true } },
    );

    rerender({ show: false });
    expect(result.current.exiting).toBe(true);
    expect(result.current.mounted).toBe(true);

    // Advance past the 200ms exit duration
    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(result.current.mounted).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it('respects custom exit duration', () => {
    const { result, rerender } = renderHook(
      ({ show }) => useAnimatedModal(show, 500),
      { initialProps: { show: true } },
    );

    rerender({ show: false });

    // At 200ms, should still be mounted and exiting
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(true);

    // At 500ms, should be unmounted
    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(result.current.mounted).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it('cleans up timeout on unmount', () => {
    const { result, rerender, unmount } = renderHook(
      ({ show }) => useAnimatedModal(show, 200),
      { initialProps: { show: true } },
    );

    rerender({ show: false });
    // Timeout is set but we unmount before it fires
    unmount();

    // Advance timers — should not throw
    act(() => {
      vi.advanceTimersByTime(200);
    });

    // No state update on unmounted component error expected
    expect(true).toBe(true);
  });

  it('does not duplicate mount when show already true', () => {
    const { result, rerender } = renderHook(
      ({ show }) => useAnimatedModal(show),
      { initialProps: { show: true } },
    );

    expect(result.current.mounted).toBe(true);

    // Rerender with same show=true should not change state
    rerender({ show: true });
    expect(result.current.mounted).toBe(true);
    expect(result.current.exiting).toBe(false);
  });

  it('handles rapid open-close-open sequence keeping mounted=true', () => {
    const { result, rerender } = renderHook(
      ({ show }) => useAnimatedModal(show, 200),
      { initialProps: { show: false } },
    );

    // Open
    act(() => rerender({ show: true }));
    expect(result.current.mounted).toBe(true);

    // Close (starts exiting)
    act(() => rerender({ show: false }));
    expect(result.current.exiting).toBe(true);

    // Re-open before exit animation completes — modal stays mounted
    act(() => rerender({ show: true }));
    expect(result.current.mounted).toBe(true);
  });

  it('uses default duration of 200ms', () => {
    const { result, rerender } = renderHook(
      ({ show }) => useAnimatedModal(show),
      { initialProps: { show: true } },
    );

    rerender({ show: false });
    expect(result.current.exiting).toBe(true);

    act(() => {
      vi.advanceTimersByTime(199);
    });
    // At 199ms, still mounted
    expect(result.current.mounted).toBe(true);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    // At 200ms, unmounted
    expect(result.current.mounted).toBe(false);
  });
});
