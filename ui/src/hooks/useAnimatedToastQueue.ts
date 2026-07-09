import { useCallback, useEffect, useRef, useState } from 'react';
import { animDuration } from '@/utils/animation';

/**
 * Options for {@link useAnimatedToastQueue}.
 *
 * Generic state machine for a list surface (notifications, banners,
 * toast queue) that needs a brief exit animation when items are
 * dismissed. Each item has its own independent isExiting flag —
 * multiple items can be fading concurrently. Optional per-item
 * auto-dismiss via {@link UseAnimatedToastQueueOptions.getAutoDismissMs}.
 *
 * The race-safety contract on `clearAll()` mirrors the undo-pill
 * pattern (commit [`fcf1d07`](https://github.com/) → refactor
 * `2d8bab9`): a snapshot of the dismiss-time ids is captured; if
 * an `enqueue()` lands during the fade, the new items (whose ids
 * are NOT in the snapshot) survive the clear. The timer body uses
 * an id-set compare so the check is explicit and the contract is
 * documented in the test suite.
 *
 * @typeParam T - Item shape. Identity exposed via `getId`.
 */
export interface UseAnimatedToastQueueOptions<T> {
  /**
   * Maximum items retained. FIFO eviction when over:
   * the OLDEST entry is dropped. Defaults to `Infinity`
   * (no eviction — caller decides when to enqueue).
   */
  maxSize?: number;
  /** Exit-fade duration in ms. Defaults to 200 to match CSS. */
  fadeMs?: number;
  /** Identity function for id-set compare in the timer body. */
  getId: (item: T) => string;
  /**
   * Returns ms until auto-dismiss for an item. The hook schedules
   * `dismiss(item.id)` after this delay at enqueue time. Returns
   * `0`, `undefined`, or a negative number to mark the item
   * persistent (no auto-dismiss). If omitted entirely, all items
   * are persistent and the caller is responsible for removal.
   */
  getAutoDismissMs?: (item: T) => number | undefined;
}

export interface AnimatedToastQueue<T> {
  /** Live items in insertion order (oldest at index 0). */
  items: readonly T[];
  /**
   * Ids of items currently in the exit fade. Use this Set at
   * render time to gate `aria-busy`, `pointer-events`, and the
   * `--exiting` className on each rendered item. The set is
   * a separate state from `items`, so items in `exitingIds`
   * are ALSO still in `items` until the fade completes.
   */
  exitingIds: ReadonlySet<string>;
  /**
   * Append an item. Oldest dropped when over `maxSize`. Schedules
   * the auto-dismiss timer via `getAutoDismissMs` if positive.
   */
  enqueue: (item: T) => void;
  /**
   * Dismiss a single item. Triggers the exit fade. Idempotent:
   * re-calling while a fade is already in progress is a no-op.
   * Also cancels any pending auto-dismiss for the same id so
   * the user-initiated dismiss doesn't leak a stale 4 s timer.
   */
  dismiss: (id: string) => void;
  /**
   * Dismiss all currently-visible items with a coordinated,
   * race-safe fade. A snapshot of ids is captured at dismiss
   * time. After the 200 ms fade, only snapshot ids are removed;
   * items enqueued during the fade (whose ids are not in the
   * snapshot) survive. No-op if the queue is already empty.
   */
  clearAll: () => void;
}

export function useAnimatedToastQueue<T>(
  options: UseAnimatedToastQueueOptions<T>,
): AnimatedToastQueue<T> {
  const {
    maxSize = Infinity,
    fadeMs = 200,
    getId,
    getAutoDismissMs,
  } = options;

  const [items, setItems] = useState<T[]>([]);
  const [exitingIds, setExitingIds] = useState<Set<string>>(new Set());

  // Live mirror of `items` so the clearAll timer body can read
  // the post-snapshot value if an `enqueue()` or per-item
  // `dismiss()` mutated the queue during the fade.
  const itemsRef = useRef<T[]>(items);
  useEffect(() => {
    itemsRef.current = items;
  }, [items]);

  // Per-id pending fade-out timers. Cleared on host unmount and
  // when a new fade replaces an old one for the same id (rapid
  // double-dismiss). Stored in a Ref Map (not state) so changes
  // don't trigger re-renders.
  const fadeTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  // Per-id pending auto-dismiss timers. Cleared on host unmount
  // and when the user dismisses the same id manually mid-wait.
  const autoTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  // Captured at clearAll() time. The timer body uses an id-set
  // compare to confirm live ids match snapshot before removal,
  // keeping items enqueued during the fade safely. Mirrors the
  // undo-pill's `undoExitSnapshotRef` pattern.
  const clearAllSnapshotRef = useRef<Set<string> | null>(null);
  const clearAllTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Cleanup: cancel EVERY pending timer when the host unmounts.
  // Without this, timers fire against an unmounted component
  // (React warning + wasted render) and leak across navigations.
  useEffect(() => {
    const fadeTimers = fadeTimersRef.current;
    const autoTimers = autoTimersRef.current;
    return () => {
      fadeTimers.forEach((t) => clearTimeout(t));
      fadeTimers.clear();
      autoTimers.forEach((t) => clearTimeout(t));
      autoTimers.clear();
      if (clearAllTimerRef.current !== null) {
        clearTimeout(clearAllTimerRef.current);
        clearAllTimerRef.current = null;
      }
    };
  }, []);


  const dismiss = useCallback(
    (id: string) => {
      // Idempotent against two lifecycle paths:
      //   - per-id fade timer (from a previous dismiss call)
      //   - clearAll collective fade (snapshot's lifecycle is owned
      //     by the shared clearAll timer; scheduling a redundant
      //     per-id timer would fire a no-op setItems.filter 200 ms
      //     later, wasting a render).
      if (
        fadeTimersRef.current.has(id) ||
        clearAllSnapshotRef.current?.has(id) === true
      ) {
        return;
      }

      // Cancel any pending auto-dismiss for this id. The user-
      // initiated dismiss already started the fade, so the 4 s
      // auto timer would be wasted.
      const autoTimer = autoTimersRef.current.get(id);
      if (autoTimer !== undefined) {
        clearTimeout(autoTimer);
        autoTimersRef.current.delete(id);
      }

      setExitingIds((prev) => {
        if (prev.has(id)) return prev;
        const next = new Set(prev);
        next.add(id);
        return next;
      });

      const timer = setTimeout(() => {
        setItems((prev) => prev.filter((t) => getId(t) !== id));
        setExitingIds((prev) => {
          if (!prev.has(id)) return prev;
          const next = new Set(prev);
          next.delete(id);
          return next;
        });
        fadeTimersRef.current.delete(id);
      }, animDuration(fadeMs));
      fadeTimersRef.current.set(id, timer);
    },
    [getId, fadeMs],
  );

  const enqueue = useCallback(
    (item: T) => {
      const id = getId(item);
      // Update itemsRef synchronously inside the setter callback so
      // concurrent reads in the SAME React batch (e.g. a clearAll()
      // called from a subsequent click within the same act() scope)
      // see the freshly-enqueued id rather than a stale snapshot.
      // This mirrors the standard "ref-mirrors-state" hook pattern
      // where the ref is the always-fresh shadow used by timer
      // callbacks and synchronous reads.
      setItems((prev) => {
        const next = [...prev, item];
        // FIFO eviction when over maxSize — drop the oldest.
        if (next.length > maxSize) {
          next.splice(0, next.length - maxSize);
        }
        itemsRef.current = next;
        return next;
      });
      const ttl = getAutoDismissMs?.(item);
      if (typeof ttl === 'number' && ttl > 0) {
        // Defensive: clear any stale auto-dismiss for this id
        // (e.g., enqueue called twice with the same id).
        const existing = autoTimersRef.current.get(id);
        if (existing !== undefined) clearTimeout(existing);
        const timer = setTimeout(() => dismiss(id), ttl);
        autoTimersRef.current.set(id, timer);
      }
    },
    [maxSize, getId, getAutoDismissMs, dismiss],
  );

  const clearAll = useCallback(() => {
    const liveItems = itemsRef.current;
    if (liveItems.length === 0) return;

    const snapshotIds = new Set(liveItems.map(getId));
    if (snapshotIds.size === 0) return;

    // Mark every snapshot id as exiting so the CSS fade plays.
    setExitingIds((prev) => {
      const next = new Set(prev);
      snapshotIds.forEach((id) => next.add(id));
      return next;
    });

    // Cancel auto-dismiss timers for snapshot ids — they're being
    // cleared and the timers would be wasted otherwise.
    snapshotIds.forEach((id) => {
      const t = autoTimersRef.current.get(id);
      if (t !== undefined) {
        clearTimeout(t);
        autoTimersRef.current.delete(id);
      }
    });

    // Race-safety capture: the timer body will compare snapshot
    // ids against live ids. Items whose ids are NOT in the
    // snapshot (enqueued during the fade) survive.
    clearAllSnapshotRef.current = snapshotIds;

    // Cancel any in-flight clearAll timer — second call debounces.
    if (clearAllTimerRef.current !== null) {
      clearTimeout(clearAllTimerRef.current);
    }
    clearAllTimerRef.current = setTimeout(() => {
      const captured = clearAllSnapshotRef.current;
      clearAllSnapshotRef.current = null;
      clearAllTimerRef.current = null;
      if (captured === null) return;

      // Id-set compare: only remove ids that were in the snapshot
      // AND are still in the live queue. Items enqueued DURING
      // the fade (not in snapshot) stay. Items manually dismissed
      // during the fade are already gone from `items` — the
      // filter is a no-op for those, which is correct.
      const liveIds = new Set(itemsRef.current.map(getId));
      const idsToRemove = new Set<string>();
      captured.forEach((id) => {
        if (liveIds.has(id)) idsToRemove.add(id);
      });
      if (idsToRemove.size > 0) {
        setItems((prev) => prev.filter((t) => !idsToRemove.has(getId(t))));
      }
      // Clear exitingIds for ALL snapshot ids (whether still in
      // items or not — the fade is over either way).
      setExitingIds((prev) => {
        let changed = false;
        const next = new Set(prev);
        captured.forEach((id) => {
          if (next.has(id)) {
            next.delete(id);
            changed = true;
          }
        });
        return changed ? next : prev;
      });
    }, animDuration(fadeMs));
  }, [getId, fadeMs]);

  return {
    items,
    exitingIds,
    enqueue,
    dismiss,
    clearAll,
  };
}
