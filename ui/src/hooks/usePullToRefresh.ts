import { useCallback, useRef, useState } from 'react';

type PullState = 'idle' | 'pulling' | 'ready' | 'loading';

interface PullToRefreshOptions {
  /** Callback invoked when the user triggers a refresh. */
  onRefresh: () => Promise<void> | void;
  /** Distance in px the user must pull down before release triggers. Default 60. */
  threshold?: number;
  /** Maximum pull distance in px. Default 120. */
  maxPullDistance?: number;
}

interface PullToRefreshResult {
  /** Spread these props onto the scrollable container. */
  containerProps: {
    onTouchStart: (e: React.TouchEvent) => void;
    onTouchMove: (e: React.TouchEvent) => void;
    onTouchEnd: (e: React.TouchEvent) => void;
    onScroll: (e: React.UIEvent) => void;
  };
  /** Current pull state for conditional rendering. */
  state: PullState;
  /** Current pull distance in px (0 when idle). */
  pullDistance: number;
}

/**
 * Pull-to-refresh gesture hook for scrollable lists.
 *
 * Tracks touch events to detect a pull-down gesture at the top of a
 * scroll container. When released past `threshold`, calls `onRefresh`.
 * Shows the pull distance so a spinner/indicator can be rendered.
 */
export function usePullToRefresh({
  onRefresh,
  threshold = 60,
  maxPullDistance = 120,
}: PullToRefreshOptions): PullToRefreshResult {
  const [state, setState] = useState<PullState>('idle');
  const [pullDistance, setPullDistance] = useState(0);
  const touchStartY = useRef(0);
  const isRefreshing = useRef(false);

  const onTouchStart = useCallback((e: React.TouchEvent) => {
    if (isRefreshing.current) return;
    const container = e.currentTarget as HTMLElement;
    // Only activate if scrolled to the top
    if (container.scrollTop > 5) return;
    touchStartY.current = e.touches[0]?.clientY ?? 0;
  }, []);

  const onTouchMove = useCallback((e: React.TouchEvent) => {
    if (isRefreshing.current) return;
    const container = e.currentTarget as HTMLElement;
    if (container.scrollTop > 5) {
      setPullDistance(0);
      setState('idle');
      return;
    }

    const currentY = e.touches[0]?.clientY ?? 0;
    const dy = currentY - touchStartY.current;
    if (dy <= 0) {
      setPullDistance(0);
      setState('idle');
      return;
    }

    // Apply resistance so it doesn't feel too stiff
    const resisted = Math.min(dy * 0.5, maxPullDistance);
    setPullDistance(resisted);
    setState(resisted >= threshold ? 'ready' : 'pulling');
  }, [threshold, maxPullDistance]);

  const onTouchEnd = useCallback(async () => {
    if (isRefreshing.current) return;
    if (state === 'ready') {
      setState('loading');
      isRefreshing.current = true;
      try {
        await onRefresh();
      } finally {
        isRefreshing.current = false;
        setState('idle');
        setPullDistance(0);
      }
    } else {
      setState('idle');
      setPullDistance(0);
    }
  }, [state, onRefresh]);

  const onScroll = useCallback((e: React.UIEvent) => {
    const el = e.currentTarget as HTMLElement;
    if (el.scrollTop <= 5 && pullDistance > 0) {
      // User is still at the top during a touch — keep the pull distance
    }
  }, [pullDistance]);

  return {
    containerProps: {
      onTouchStart,
      onTouchMove,
      onTouchEnd,
      onScroll,
    },
    state,
    pullDistance,
  };
}
