import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSwipe } from '@/hooks/useSwipe';

// ── helpers ────────────────────────────────────────────────────────────

/** Build a minimal fake TouchEvent for onTouchStart. */
function makeTouchStart(clientX: number, clientY: number): React.TouchEvent {
  return {
    touches: [{ clientX, clientY }],
  } as unknown as React.TouchEvent;
}

/** Build a minimal fake TouchEvent for onTouchEnd. */
function makeTouchEnd(clientX: number, clientY: number): React.TouchEvent {
  return {
    changedTouches: [{ clientX, clientY }],
  } as unknown as React.TouchEvent;
}

function render(handlers: { onSwipeLeft?: () => void; onSwipeRight?: () => void }) {
  return renderHook(() => useSwipe(handlers));
}

// ── tests ──────────────────────────────────────────────────────────────
describe('useSwipe', () => {
  it('returns onTouchStart and onTouchEnd handlers', () => {
    const { result } = render({});
    expect(result.current.onTouchStart).toBeDefined();
    expect(result.current.onTouchEnd).toBeDefined();
  });

  it('calls onSwipeRight when user swipes right past threshold', () => {
    const onSwipeRight = vi.fn();
    const { result } = render({ onSwipeRight });

    act(() => { result.current.onTouchStart(makeTouchStart(100, 200)); });
    act(() => { result.current.onTouchEnd(makeTouchEnd(200, 200)); }); // +100px horizontally

    expect(onSwipeRight).toHaveBeenCalledTimes(1);
  });

  it('calls onSwipeLeft when user swipes left past threshold', () => {
    const onSwipeLeft = vi.fn();
    const { result } = render({ onSwipeLeft });

    act(() => { result.current.onTouchStart(makeTouchStart(300, 100)); });
    act(() => { result.current.onTouchEnd(makeTouchEnd(200, 100)); }); // -100px horizontally

    expect(onSwipeLeft).toHaveBeenCalledTimes(1);
  });

  it('does not trigger swipe when movement is below threshold', () => {
    const onSwipeRight = vi.fn();
    const onSwipeLeft = vi.fn();
    const { result } = render({ onSwipeRight, onSwipeLeft });

    act(() => { result.current.onTouchStart(makeTouchStart(100, 200)); });
    act(() => { result.current.onTouchEnd(makeTouchEnd(140, 200)); }); // +40px < 60 threshold

    expect(onSwipeRight).not.toHaveBeenCalled();
    expect(onSwipeLeft).not.toHaveBeenCalled();
  });

  it('does not trigger swipe for vertical movement', () => {
    const onSwipeRight = vi.fn();
    const onSwipeLeft = vi.fn();
    const { result } = render({ onSwipeRight, onSwipeLeft });

    // deltaX = 20, deltaY = 100 — vertical dominates
    act(() => { result.current.onTouchStart(makeTouchStart(100, 100)); });
    act(() => { result.current.onTouchEnd(makeTouchEnd(120, 200)); });

    expect(onSwipeRight).not.toHaveBeenCalled();
    expect(onSwipeLeft).not.toHaveBeenCalled();
  });

  it('does not throw when onTouchEnd is called without prior onTouchStart', () => {
    const onSwipeRight = vi.fn();
    const { result } = render({ onSwipeRight });

    // No onTouchStart — should silently return
    expect(() => {
      act(() => { result.current.onTouchEnd(makeTouchEnd(200, 200)); });
    }).not.toThrow();

    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('does not throw when no handler is provided for the swipe direction', () => {
    const onSwipeRight = vi.fn();
    const { result } = render({ onSwipeRight });

    // Swipe left with no onSwipeLeft handler
    act(() => { result.current.onTouchStart(makeTouchStart(300, 100)); });
    act(() => { result.current.onTouchEnd(makeTouchEnd(200, 100)); });

    expect(onSwipeRight).not.toHaveBeenCalled();
  });

  it('resets touch state after each onTouchEnd', () => {
    const onSwipeRight = vi.fn();
    const { result } = render({ onSwipeRight });

    // First swipe
    act(() => { result.current.onTouchStart(makeTouchStart(100, 200)); });
    act(() => { result.current.onTouchEnd(makeTouchEnd(200, 200)); });
    expect(onSwipeRight).toHaveBeenCalledTimes(1);

    // Second onTouchEnd without a new onTouchStart should do nothing
    act(() => { result.current.onTouchEnd(makeTouchEnd(300, 200)); });
    expect(onSwipeRight).toHaveBeenCalledTimes(1);
  });
});
