import { useCallback, useRef } from 'react';

interface SwipeHandlers {
  onSwipeLeft?: () => void;
  onSwipeRight?: () => void;
}

interface SwipeOptions {
  /** Minimum horizontal distance (px) to qualify as a swipe. Default 50. */
  minDistance?: number;
  /** Maximum elapsed time (ms) between touch start and end. Default 300. */
  maxTimeMs?: number;
}

/**
 * Detect horizontal swipe gestures on a touch surface.
 * Returns `onTouchStart` and `onTouchEnd` handlers to spread onto the
 * target element. Swipes shorter than `minDistance` px or slower than
 * `maxTimeMs` ms are ignored.
 */
export function useSwipe(
  { onSwipeLeft, onSwipeRight }: SwipeHandlers,
  options?: SwipeOptions,
) {
  const { minDistance = 50, maxTimeMs = 300 } = options ?? {};
  const touchStart = useRef<{ x: number; y: number; time: number } | null>(null);

  const onTouchStart = useCallback((e: React.TouchEvent) => {
    const touch = e.touches[0];
    if (!touch) return;
    touchStart.current = {
      x: touch.clientX,
      y: touch.clientY,
      time: Date.now(),
    };
  }, []);

  const onTouchEnd = useCallback((e: React.TouchEvent) => {
    if (!touchStart.current) return;
    const touch = e.changedTouches[0];
    if (!touch) return;

    const dt = Date.now() - touchStart.current.time;
    const deltaX = touch.clientX - touchStart.current.x;
    const deltaY = touch.clientY - touchStart.current.y;
    const distance = Math.abs(deltaX);

    if (
      distance > Math.abs(deltaY) &&
      distance > minDistance &&
      dt <= maxTimeMs
    ) {
      if (deltaX > 0 && onSwipeRight) {
        onSwipeRight();
      } else if (deltaX < 0 && onSwipeLeft) {
        onSwipeLeft();
      }
    }
    touchStart.current = null;
  }, [onSwipeLeft, onSwipeRight, minDistance, maxTimeMs]);

  return { onTouchStart, onTouchEnd };
}
