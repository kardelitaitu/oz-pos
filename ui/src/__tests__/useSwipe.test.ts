import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import * as React from 'react';
import { useSwipe } from '@/hooks/useSwipe';

function createTouchEvent(
  type: 'touchstart' | 'touchend',
  clientX: number,
  clientY: number,
  prevClientX?: number,
): React.TouchEvent {
  const touches: React.Touch[] = [{ clientX, clientY } as unknown as React.Touch];
  const changedTouches: React.Touch[] = [{ clientX, clientY } as unknown as React.Touch];
  const target = document.createElement('div');

  if (type === 'touchend' && prevClientX !== undefined) {
    // For touchend, we need the start position stored in the ref
    // The onTouchEnd handler uses touchStart.current which was set by onTouchStart
  }

  return {
    touches: type === 'touchstart' ? touches : [],
    changedTouches: type === 'touchend' ? changedTouches : [],
    target,
    preventDefault: vi.fn(),
  } as unknown as React.TouchEvent;
}

describe('useSwipe', () => {
  it('returns onTouchStart and onTouchEnd handlers', () => {
    const { result } = renderHook(() => useSwipe({}));
    expect(result.current.onTouchStart).toBeDefined();
    expect(result.current.onTouchEnd).toBeDefined();
  });

  it('calls onSwipeLeft when swiping left past threshold', () => {
    const onSwipeLeft = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft }));

    // Start at x=200
    act(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 200, 100));
    });

    // End at x=100 (deltaX = -100, exceeds threshold of 60)
    act(() => {
      result.current.onTouchEnd(createTouchEvent('touchend', 100, 100));
    });

    expect(onSwipeLeft).toHaveBeenCalledTimes(1);
  });

  it('calls onSwipeRight when swiping right past threshold', () => {
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeRight }));

    act(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    act(() => {
      result.current.onTouchEnd(createTouchEvent('touchend', 200, 100));
    });

    expect(onSwipeRight).toHaveBeenCalledTimes(1);
  });

  it('does not trigger swipe when deltaX is below threshold', () => {
    const onSwipeLeft = vi.fn();
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft, onSwipeRight }));

    act(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    // Only 30px movement — below 60px threshold
    act(() => {
      result.current.onTouchEnd(createTouchEvent('touchend', 130, 100));
    });

    expect(onSwipeLeft).not.toHaveBeenCalled();
    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('does not trigger swipe when vertical movement dominates', () => {
    const onSwipeLeft = vi.fn();
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft, onSwipeRight }));

    act(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 100, 100));
    });

    // deltaY = 200 > deltaX = 50 → vertical scroll, not swipe
    act(() => {
      result.current.onTouchEnd(createTouchEvent('touchend', 150, 300));
    });

    expect(onSwipeLeft).not.toHaveBeenCalled();
    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('does nothing when onTouchEnd is called without prior onTouchStart', () => {
    const onSwipeLeft = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft }));

    // No prior touchstart
    act(() => {
      result.current.onTouchEnd(createTouchEvent('touchend', 100, 100));
    });

    expect(onSwipeLeft).not.toHaveBeenCalled();
  });

  it('does not call missing callback handlers', () => {
    const { result } = renderHook(() => useSwipe({}));

    act(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 200, 100));
    });

    // Should not throw even though no handler is provided
    expect(() => {
      act(() => {
        result.current.onTouchEnd(createTouchEvent('touchend', 100, 100));
      });
    }).not.toThrow();
  });

  it('clears touch start ref after touch end', () => {
    const onSwipeLeft = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft }));

    act(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 200, 100));
    });

    // First swipe
    act(() => {
      result.current.onTouchEnd(createTouchEvent('touchend', 100, 100));
    });

    // Second touchEnd without new touchStart should be a no-op
    onSwipeLeft.mockClear();
    act(() => {
      result.current.onTouchEnd(createTouchEvent('touchend', 50, 100));
    });

    expect(onSwipeLeft).not.toHaveBeenCalled();
  });
});
