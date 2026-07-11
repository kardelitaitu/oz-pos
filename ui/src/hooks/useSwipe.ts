import { useCallback, useRef } from 'react';

interface SwipeHandlers {
  onSwipeLeft?: () => void;
  onSwipeRight?: () => void;
}

/**
 * Detect horizontal swipe gestures on a touch surface.
 * Returns `onTouchStart` and `onTouchEnd` handlers to spread onto the
 * target element. Swipes shorter than 60 px are ignored.
 */
export function useSwipe({ onSwipeLeft, onSwipeRight }: SwipeHandlers) {
  const touchStart = useRef<{ x: number; y: number } | null>(null);
  const SWIPE_THRESHOLD = 60;

  const onTouchStart = useCallback((e: React.TouchEvent) => {
    const touch = e.touches[0];
    if (!touch) return;
    touchStart.current = {
      x: touch.clientX,
      y: touch.clientY,
    };
  }, []);

  const onTouchEnd = useCallback((e: React.TouchEvent) => {
    if (!touchStart.current) return;
    const touch = e.changedTouches[0];
    if (!touch) return;
    const deltaX = touch.clientX - touchStart.current.x;
    const deltaY = touch.clientY - touchStart.current.y;

    if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > SWIPE_THRESHOLD) {
      if (deltaX > 0 && onSwipeRight) {
        onSwipeRight();
      } else if (deltaX < 0 && onSwipeLeft) {
        onSwipeLeft();
      }
    }
    touchStart.current = null;
  }, [onSwipeLeft, onSwipeRight]);

  return { onTouchStart, onTouchEnd };
}
