import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useExitAnimation } from '../hooks/useExitAnimation';

describe('useExitAnimation', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shouldRender is true when open', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useExitAnimation(true, onClose));
    expect(result.current.shouldRender).toBe(true);
    expect(result.current.exiting).toBe(false);
  });

  it('shouldRender is false when closed and not exiting', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useExitAnimation(false, onClose));
    expect(result.current.shouldRender).toBe(false);
    expect(result.current.exiting).toBe(false);
  });

  it('requestClose sets exiting to true and keeps shouldRender true', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useExitAnimation(true, onClose));

    act(() => {
      result.current.requestClose();
    });

    expect(result.current.exiting).toBe(true);
    expect(result.current.shouldRender).toBe(true);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('calls onClose after the exit animation duration', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useExitAnimation(true, onClose, 200));

    act(() => {
      result.current.requestClose();
    });

    expect(result.current.exiting).toBe(true);

    // Advance timers by the animation duration
    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(result.current.exiting).toBe(false);
  });

  it('requestClose is a no-op when already exiting', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useExitAnimation(true, onClose, 200));

    act(() => {
      result.current.requestClose();
    });

    expect(result.current.exiting).toBe(true);

    // Second call during exit — should be no-op
    act(() => {
      result.current.requestClose();
    });

    expect(result.current.exiting).toBe(true);
    // Timer should still fire exactly once
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('reopen during exit cancels the fade and keeps surface mounted', () => {
    const onClose = vi.fn();
    const { result, rerender } = renderHook(
      ({ open }) => useExitAnimation(open, onClose, 200),
      { initialProps: { open: true } },
    );

    act(() => {
      result.current.requestClose();
    });

    expect(result.current.exiting).toBe(true);

    // Reopen during the fade: the parent sets open to false (external
    // close) then back to true, triggering the [open] dependency in the
    // hook's effect which cancels the exit timer and resets exiting.
    rerender({ open: false });
    rerender({ open: true });

    expect(result.current.exiting).toBe(false);
    expect(result.current.shouldRender).toBe(true);

    // onClose should never fire because we cancelled the exit
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closing externally (open→false without requestClose) does not trigger exit animation', () => {
    const onClose = vi.fn();
    const { result, rerender } = renderHook(
      ({ open }) => useExitAnimation(open, onClose),
      { initialProps: { open: true } },
    );

    // Parent sets open to false directly (without requestClose)
    rerender({ open: false });

    expect(result.current.exiting).toBe(false);
    expect(result.current.shouldRender).toBe(false);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('respects custom durationMs', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useExitAnimation(true, onClose, 500));

    act(() => {
      result.current.requestClose();
    });

    // At 200ms, onClose should NOT have been called (duration is 500)
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(onClose).not.toHaveBeenCalled();
    expect(result.current.exiting).toBe(true);

    // At 500ms, onClose should be called
    act(() => {
      vi.advanceTimersByTime(300);
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shouldRender stays true during entire exit animation', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useExitAnimation(true, onClose, 200));

    act(() => {
      result.current.requestClose();
    });

    // During the fade, shouldRender must stay true
    act(() => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current.shouldRender).toBe(true);
    expect(result.current.exiting).toBe(true);

    // After the fade completes
    act(() => {
      vi.advanceTimersByTime(100);
    });
    expect(result.current.exiting).toBe(false);
  });

  it('uses latest onClose callback when it changes during the exit animation', () => {
    const onClose1 = vi.fn();
    const onClose2 = vi.fn();
    const { result, rerender } = renderHook(
      ({ cb }) => useExitAnimation(true, cb, 200),
      { initialProps: { cb: onClose1 } },
    );

    act(() => {
      result.current.requestClose();
    });

    // Change the onClose callback mid-animation
    rerender({ cb: onClose2 });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onClose1).not.toHaveBeenCalled();
    expect(onClose2).toHaveBeenCalledTimes(1);
  });

  it('cancels the timer on unmount during exit animation', () => {
    const onClose = vi.fn();
    const { result, unmount } = renderHook(() =>
      useExitAnimation(true, onClose, 200),
    );

    act(() => {
      result.current.requestClose();
    });

    // Unmount before the timer fires
    unmount();

    act(() => {
      vi.advanceTimersByTime(200);
    });

    // onClose must NOT be called after unmount
    expect(onClose).not.toHaveBeenCalled();
  });

  it('fires onClose immediately when duration is 0', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() =>
      useExitAnimation(true, onClose, 0),
    );

    act(() => {
      result.current.requestClose();
    });

    // With 0 duration, onClose should fire immediately
    act(() => {
      vi.runAllTimers();
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(result.current.exiting).toBe(false);
  });
});
