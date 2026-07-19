import { describe, it, expect, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useSwipe } from '@/hooks/useSwipe';

/** Stub Date.now() to return a fixed time during tests. */
let MOCK_NOW = 1000;
vi.spyOn(Date, 'now').mockImplementation(() => MOCK_NOW);

function advanceTime(ms: number) {
  MOCK_NOW += ms;
}

function createTouchEvent(
  type: 'touchstart' | 'touchend',
  clientX: number,
  clientY: number,
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

  it('calls onSwipeRight when swiping right past threshold within time limit', () => {
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeRight }));

    // Start touch at x=50
    result.current.onTouchStart(createTouchEvent('touchstart', 50, 100));
    advanceTime(50); // well within 300ms

    // End touch at x=110 (delta=60, above 50 threshold)
    result.current.onTouchEnd(createTouchEvent('touchend', 110, 100));

    expect(onSwipeRight).toHaveBeenCalledTimes(1);
  });

  it('calls onSwipeLeft when swiping left past threshold within time limit', () => {
    const onSwipeLeft = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft }));

    // Start touch at x=200
    result.current.onTouchStart(createTouchEvent('touchstart', 200, 100));
    advanceTime(150); // within 300ms

    // End touch at x=120 (delta=-80, below -50 threshold)
    result.current.onTouchEnd(createTouchEvent('touchend', 120, 100));

    expect(onSwipeLeft).toHaveBeenCalledTimes(1);
  });

  it('does not trigger for sub-threshold horizontal swipes', () => {
    const onSwipeLeft = vi.fn();
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft, onSwipeRight }));

    // Delta=30, below 50 threshold
    result.current.onTouchStart(createTouchEvent('touchstart', 50, 100));
    advanceTime(50);
    result.current.onTouchEnd(createTouchEvent('touchend', 80, 100));

    expect(onSwipeLeft).not.toHaveBeenCalled();
    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('does not trigger when vertical movement dominates', () => {
    const onSwipeLeft = vi.fn();
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeLeft, onSwipeRight }));

    // Horizontal delta=70, vertical delta=90 — vertical dominates
    result.current.onTouchStart(createTouchEvent('touchstart', 50, 50));
    advanceTime(100);
    result.current.onTouchEnd(createTouchEvent('touchend', 120, 140));

    expect(onSwipeLeft).not.toHaveBeenCalled();
    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('does not throw when no handlers are provided', () => {
    const { result } = renderHook(() => useSwipe({}));

    expect(() => {
      result.current.onTouchStart(createTouchEvent('touchstart', 50, 50));
      advanceTime(10);
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

  it('does not trigger when swipe exceeds time limit (too slow)', () => {
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() => useSwipe({ onSwipeRight }));

    // Start touch at x=50
    result.current.onTouchStart(createTouchEvent('touchstart', 50, 100));
    advanceTime(301); // exceeds 300ms default

    // End touch at x=120 (delta=70, above 50 distance threshold but too slow)
    result.current.onTouchEnd(createTouchEvent('touchend', 120, 100));

    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('uses latest callback when handlers change between touchStart and touchEnd', () => {
    const onSwipeLeft1 = vi.fn();
    const onSwipeLeft2 = vi.fn();
    const { result, rerender } = renderHook(
      ({ cb }) => useSwipe({ onSwipeLeft: cb }),
      { initialProps: { cb: onSwipeLeft1 } },
    );

    // Start touch with callback v1
    result.current.onTouchStart(createTouchEvent('touchstart', 200, 100));
    advanceTime(20);

    // Rerender with a new callback before touchEnd
    rerender({ cb: onSwipeLeft2 });

    // End touch — should use callback v2 (the latest)
    result.current.onTouchEnd(createTouchEvent('touchend', 100, 100));

    expect(onSwipeLeft1).not.toHaveBeenCalled();
    expect(onSwipeLeft2).toHaveBeenCalledTimes(1);
  });

  it('respects custom minDistance and maxTimeMs options', () => {
    const onSwipeRight = vi.fn();
    const { result } = renderHook(() =>
      useSwipe({ onSwipeRight }, { minDistance: 100, maxTimeMs: 200 }),
    );

    // Start touch at x=10
    result.current.onTouchStart(createTouchEvent('touchstart', 10, 100));
    advanceTime(100); // within 200ms

    // End touch at x=100 (delta=90, above 50 but below 100 custom threshold)
    result.current.onTouchEnd(createTouchEvent('touchend', 100, 100));
    expect(onSwipeRight).not.toHaveBeenCalled();

    // Now exceed custom distance
    result.current.onTouchStart(createTouchEvent('touchstart', 10, 100));
    advanceTime(100);
    result.current.onTouchEnd(createTouchEvent('touchend', 130, 100));
    expect(onSwipeRight).toHaveBeenCalledTimes(1);

    // Exceed custom time limit
    result.current.onTouchStart(createTouchEvent('touchstart', 10, 100));
    advanceTime(201);
    result.current.onTouchEnd(createTouchEvent('touchend', 130, 100));
    expect(onSwipeRight).toHaveBeenCalledTimes(1); // no extra call — too slow
  });
});
