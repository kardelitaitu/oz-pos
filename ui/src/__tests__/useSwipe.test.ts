import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useSwipe } from '@/hooks/useSwipe';

function createTouchEvent(
  type: 'touchstart' | 'touchend',
  clientX: number,
  clientY: number,
  startX?: number,
  startY?: number,
): React.TouchEvent {
  const touches: React.Touch[] = [{
    clientX,
    clientY,
    identifier: 0,
    target: document.createElement('div'),
    pageX: clientX,
    pageY: clientY,
    screenX: clientX,
    screenY: clientY,
    force: 1,
    radiusX: 1,
    radiusY: 1,
    rotationAngle: 0,
  }];

  const changedTouches: React.Touch[] = [{
    clientX,
    clientY,
    identifier: 0,
    target: document.createElement('div'),
    pageX: clientX,
    pageY: clientY,
    screenX: clientX,
    screenY: clientY,
    force: 1,
    radiusX: 1,
    radiusY: 1,
    rotationAngle: 0,
  }];

  return {
    type,
    touches: type === 'touchstart' ? touches as unknown as React.TouchList : [] as unknown as React.TouchList,
    changedTouches: type === 'touchend' ? changedTouches as unknown as React.TouchList : [] as unknown as React.TouchList,
    targetTouches: [] as unknown as React.TouchList,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as unknown as React.TouchEvent;
}

describe('useSwipe', () => {
  it('returns touch event handlers', () => {
    const { result } = renderHook(() => useSwipe({}));

    expect(typeof result.current.onTouchStart).toBe('function');
    expect(typeof result.current.onTouchEnd).toBe('function');
  });

  it('calls onSwipeRight when swiping right past threshold', () => {
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeRight }));

    // Start touch at x=50
    result.current.onTouchStart(createTouchEvent('touchstart', 50, 100));

    // End touch at x=120 (delta=70, above 60 threshold)
    result.current.onTouchEnd(createTouchEvent('touchend', 120, 100));

    expect(onSwipeRight).toHaveBeenCalledTimes(1);
  });

  it('calls onSwipeLeft when swiping left past threshold', () => {
    const onSwipeLeft = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft }));

    // Start touch at x=200
    result.current.onTouchStart(createTouchEvent('touchstart', 200, 100));

    // End touch at x=120 (delta=-80, below -60 threshold)
    result.current.onTouchEnd(createTouchEvent('touchend', 120, 100));

    expect(onSwipeLeft).toHaveBeenCalledTimes(1);
  });

  it('does not trigger for sub-threshold horizontal swipes', () => {
    const onSwipeLeft = vi.fn();
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft, onSwipeRight }));

    // Delta=40, below 60 threshold
    result.current.onTouchStart(createTouchEvent('touchstart', 50, 100));
    result.current.onTouchEnd(createTouchEvent('touchend', 90, 100));

    expect(onSwipeLeft).not.toHaveBeenCalled();
    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('does not trigger when vertical movement dominates', () => {
    const onSwipeLeft = vi.fn();
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft, onSwipeRight }));

    // Horizontal delta=70, vertical delta=90 — vertical dominates
    result.current.onTouchStart(createTouchEvent('touchstart', 50, 50));
    result.current.onTouchEnd(createTouchEvent('touchend', 120, 140));

    expect(onSwipeLeft).not.toHaveBeenCalled();
    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('does not throw when no handlers are provided', () => {
    const { result } = renderHook(() => useSwipe({}));

    expect(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 50, 50));
      result.current.onTouchEnd(createTouchEvent('touchend', 150, 55));
    }).not.toThrow();
  });

  it('ignores touchEnd without a prior touchStart', () => {
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeRight }));

    // touchEnd without touchStart — should not trigger
    result.current.onTouchEnd(createTouchEvent('touchend', 200, 100));

    expect(onSwipeRight).not.toHaveBeenCalled();
  });
});
