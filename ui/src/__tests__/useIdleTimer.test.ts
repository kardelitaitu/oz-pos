import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useIdleTimer, getAutoLockMinutes, setAutoLockMinutes } from '@/hooks/useIdleTimer';

describe('useIdleTimer', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('fires onIdle after default timeout (5 minutes)', () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(onIdle));

    // Should not fire immediately
    expect(onIdle).not.toHaveBeenCalled();

    // Advance to just before 5 minutes
    act(() => {
      vi.advanceTimersByTime(5 * 60 * 1000 - 1);
    });
    expect(onIdle).not.toHaveBeenCalled();

    // Advance past 5 minutes
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets timer on mousedown activity', () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(onIdle));

    // Advance 4 minutes, then trigger mousedown
    act(() => {
      vi.advanceTimersByTime(4 * 60 * 1000);
    });
    window.dispatchEvent(new MouseEvent('mousedown'));

    // Advance another 4 minutes (8 total, but timer reset at 4)
    act(() => {
      vi.advanceTimersByTime(4 * 60 * 1000);
    });
    expect(onIdle).not.toHaveBeenCalled();

    // Advance past the reset point (4+1=5 since reset)
    act(() => {
      vi.advanceTimersByTime(60 * 1000);
    });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets timer on keydown activity', () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(onIdle));

    act(() => {
      vi.advanceTimersByTime(3 * 60 * 1000);
    });
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }));

    act(() => {
      vi.advanceTimersByTime(5 * 60 * 1000);
    });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets timer on touchstart activity', () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(onIdle));

    act(() => {
      vi.advanceTimersByTime(2 * 60 * 1000);
    });
    window.dispatchEvent(new Event('touchstart'));

    act(() => {
      vi.advanceTimersByTime(5 * 60 * 1000);
    });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('resets timer on scroll activity', () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(onIdle));

    act(() => {
      vi.advanceTimersByTime(4 * 60 * 1000);
    });
    window.dispatchEvent(new Event('scroll'));

    act(() => {
      vi.advanceTimersByTime(5 * 60 * 1000);
    });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('uses custom timeout from localStorage', () => {
    localStorage.setItem('auto-lock-minutes', '2');
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(onIdle));

    // Should not fire at 1:59
    act(() => {
      vi.advanceTimersByTime(2 * 60 * 1000 - 1);
    });
    expect(onIdle).not.toHaveBeenCalled();

    // Should fire at 2:00
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it('uses latest onIdle callback via ref', () => {
    const onIdle1 = vi.fn();
    const onIdle2 = vi.fn();

    const { rerender } = renderHook(
      ({ cb }) => useIdleTimer(cb),
      { initialProps: { cb: onIdle1 } },
    );

    // Switch to onIdle2
    rerender({ cb: onIdle2 });

    act(() => {
      vi.advanceTimersByTime(5 * 60 * 1000);
    });

    expect(onIdle1).not.toHaveBeenCalled();
    expect(onIdle2).toHaveBeenCalledTimes(1);
  });

  it('cleans up listeners on unmount', () => {
    const addSpy = vi.spyOn(window, 'addEventListener');
    const removeSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = renderHook(() => useIdleTimer(vi.fn()));

    // Should have added listeners
    expect(addSpy).toHaveBeenCalled();

    const addCount = addSpy.mock.calls.length;
    unmount();

    // Should have removed the same number of listeners
    expect(removeSpy.mock.calls.length).toBe(addCount);

    addSpy.mockRestore();
    removeSpy.mockRestore();
  });
});

describe('getAutoLockMinutes / setAutoLockMinutes', () => {
  it('returns default 5 when localStorage is empty', () => {
    expect(getAutoLockMinutes()).toBe(5);
  });

  it('returns value from localStorage', () => {
    localStorage.setItem('auto-lock-minutes', '10');
    expect(getAutoLockMinutes()).toBe(10);
  });

  it('handles invalid localStorage values gracefully', () => {
    localStorage.setItem('auto-lock-minutes', 'notanumber');
    expect(getAutoLockMinutes()).toBe(5);
  });

  it('clamps setAutoLockMinutes to minimum 1', () => {
    setAutoLockMinutes(0);
    expect(localStorage.getItem('auto-lock-minutes')).toBe('1');
    expect(getAutoLockMinutes()).toBe(1);
  });

  it('clamps setAutoLockMinutes to maximum 120', () => {
    setAutoLockMinutes(999);
    expect(localStorage.getItem('auto-lock-minutes')).toBe('120');
    expect(getAutoLockMinutes()).toBe(120);
  });

  it('stores valid minutes as-is', () => {
    setAutoLockMinutes(30);
    expect(localStorage.getItem('auto-lock-minutes')).toBe('30');
    expect(getAutoLockMinutes()).toBe(30);
  });
});
