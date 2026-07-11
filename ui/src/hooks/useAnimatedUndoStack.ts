import { useCallback, useEffect, useRef, useState } from 'react';
import { animDuration } from '@/utils/animation';

/**
 * Options for {@link useAnimatedUndoStack}.
 *
 * Generic state machine for a stacked list that needs a brief exit
 * animation when cleared or popped-empty: the cashier's undo-pill
 * (most recent removed lines back to recoverable depth), a sibling
 * "held-carts" pill, etc. The hook owns two reactive surfaces:
 *
 * - {@link AnimatedUndoStack.stack} — the live list, most recent at
 *   index 0, capped at {@link UseAnimatedUndoStackOptions.maxSize}.
 * - {@link AnimatedUndoStack.isExiting} — true during the exit fade
 *   scheduled by `dismiss()` or a `pop()` that empties the stack.
 *
 * The race-safety contract: if `push()` fires during the exit fade,
 * the snapshot taken at dismiss-time no longer matches the live
 * stack (compared by id-set, not ref equality, so slice/push/pop
 * mutations are also caught), and the clear is aborted. The
 * cashier keeps both the recovered item and the newly removed
 * item. Detected by:
 *
 *   liveIds.size === snapshot.length
 *   && snapshot.every((item) => liveIds.has(getId(item)))
 *
 * @typeParam T - The item type. Identity exposed via `getId`.
 */
export interface UseAnimatedUndoStackOptions<T> {
  /** Maximum items retained. Older entries are pushed off. */
  maxSize: number;
  /** Exit-fade duration in ms. Defaults to 200 to match CSS. */
  durationMs?: number;
  /** Identity function used to compare stack entries. */
  getId: (item: T) => string;
}

export interface AnimatedUndoStack<T> {
  /** Live stack, most recent at index 0. */
  stack: readonly T[];
  /** True while the exit fade is playing. */
  isExiting: boolean;
  /**
   * True if the stack has items OR is playing its exit fade.
   * Use this to gate the JSX so the pill stays mounted long enough
   * for the CSS keyframe to finish.
   */
  shouldRender: boolean;
  /**
   * Push an item onto the top. Oldest entry dropped when over
   * {@link UseAnimatedUndoStackOptions.maxSize}.
   */
  push: (item: T) => void;
  /**
   * Pop the top item. When the stack becomes empty after pop, an
   * exit fade runs and the clear is aborted if any `push()` lands
   * during the fade (race-safety). Returns the popped item, or
   * `undefined` if the stack is empty.
   */
  pop: () => T | undefined;
  /**
   * Dismiss the entire stack with an exit fade. The same race-safety
   * contract as the pop-to-empty path. No-op when the stack is
   * already empty (and clears `isExiting` so a stray flag doesn't
   * leak across an empty-then-dismiss-then-push-then-dismiss cycle).
   */
  dismiss: () => void;
}

export function useAnimatedUndoStack<T>(
  options: UseAnimatedUndoStackOptions<T>,
): AnimatedUndoStack<T> {
  const { maxSize, durationMs = 200, getId } = options;

  const [stack, setStack] = useState<T[]>([]);
  const [exiting, setExiting] = useState(false);

  // Live mirror so the timer callback reads the post-snapshot stack
  // if a push/pop happened during the fade. The setState closure
  // would otherwise only see the dismiss-time value.
  const stackRef = useRef<T[]>(stack);
  useEffect(() => {
    stackRef.current = stack;
  }, [stack]);

  // Captured at dismiss-time so we can detect concurrent activity
  // when the timer fires. Compared by id-set inside the timer body.
  const snapshotRef = useRef<T[] | null>(null);

  // Held so we can clear a stale timer if the host unmounts mid-
  // animation (avoids setState on an unmounted component) and so
  // `dismiss`/`pop` debounce their predecessor's timer.
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Cleanup: cancel any pending timer if the hook's host unmounts
  // mid-fade. Without this, the timer could fire after unmount and
  // call setState against an unmounted component, causing a React
  // warning and a wasted render.
  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, []);

  // Shared clear-fade routine. Used by both `dismiss()` and the
  // pop-to-empty path so the snapshot, timer, and race-safety check
  // stay in lockstep.
  const scheduleClearFade = useCallback(
    (snapshot: T[]) => {
      snapshotRef.current = snapshot;
      setExiting(true);
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
      timerRef.current = setTimeout(() => {
        const liveStack = stackRef.current;
        const captured = snapshotRef.current;
        const liveIds = new Set(liveStack.map(getId));
        const sameAsSnapshot =
          captured !== null &&
          liveIds.size === captured.length &&
          captured.every((item) => liveIds.has(getId(item)));
        if (sameAsSnapshot) {
          setStack([]);
        }
        setExiting(false);
        snapshotRef.current = null;
        timerRef.current = null;
      }, animDuration(durationMs));
    },
    [durationMs, getId],
  );

  const push = useCallback(
    (item: T) => {
      setStack((prev) => [item, ...prev].slice(0, maxSize));
    },
    [maxSize],
  );

  const pop = useCallback((): T | undefined => {
    const live = stackRef.current;
    if (live.length === 0) return undefined;
    const top = live[0] as T;
    if (live.length === 1) {
      // Popping the last item: schedule the fade with the
      // dismiss-time snapshot. The caller is expected to have
      // already re-inserted `top` into its own state (e.g. the
      // cart's `lines`) before calling pop — the snapshot here is
      // the undoStack's view of the world, not the cart's.
      scheduleClearFade([top]);
    } else {
      setStack((prev) => prev.slice(1));
    }
    return top;
  }, [scheduleClearFade]);

  const dismiss = useCallback(() => {
    if (stackRef.current.length === 0) {
      // Empty: clear any stray exit flag and return. This mirrors
      // the original `clearUndoStackAnimated` early-return which
      // calls `setUndoExiting(false)` so stale flags don't leak.
      setExiting(false);
      return;
    }
    scheduleClearFade(stackRef.current);
  }, [scheduleClearFade]);

  return {
    stack,
    isExiting: exiting,
    shouldRender: stack.length > 0 || exiting,
    push,
    pop,
    dismiss,
  };
}
