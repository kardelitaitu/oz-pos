//! `useKdsOffline` — offline resilience hook for the KDS screen.
//!
//! Provides:
//!   • Cached KDS orders in localStorage (last-known-good snapshot)
//!   • Pending-action queue for failed status updates (retried on reconnect)
//!   • Connection-status detection (backend unreachable)
//!   • Wrapped fetch/update functions with automatic cache + queue logic
//!   • OFF-03: durable optimistic projection — a locally-advanced status is
//!     persisted into the cached snapshot so a reload does not undo it
//!   • OFF-05: bounded retries with exponential backoff + a visible
//!     dead-letter list for exhausted/permanent failures
//!
//! The hook does NOT depend on any Tauri APIs, making it testable in
//! plain vitest/JSDOM. All storage is via localStorage with try/catch
//! guards so it degrades gracefully when storage is unavailable.
//!
//! OFF-08: persistence failures are surfaced via `storageUnavailable`
//! instead of being silently swallowed — a KDS that cannot persist has no
//! durable recovery record, and the operator should know.

import { useState, useCallback, useEffect } from 'react';
import type { KdsOrder, KdsStatus } from '@/api/kds';

// ── Constants ───────────────────────────────────────────────────────

const LS_CACHED_ORDERS = 'kds-cached-orders';
const LS_LAST_SYNC = 'kds-last-sync';
const LS_OFFLINE_QUEUE = 'kds-offline-queue';
const LS_DEAD_LETTER = 'kds-offline-dead-letter';

/** OFF-05: max attempts before an action moves to the dead-letter list. */
export const MAX_RETRY_ATTEMPTS = 5;

/** OFF-05: backoff base (ms) — 1s, 2s, 4s, 8s with ±30% jitter. */
const BACKOFF_BASE_MS = 1000;
const BACKOFF_JITTER = 0.3;

// ── Types ───────────────────────────────────────────────────────────

/** A KDS action queued for retry after reconnection. */
export interface PendingKdsAction {
  /** Uniquely identifies the action (derived from order ID + target status). */
  id: string;
  /** The KDS order ID this action targets. */
  orderId: string;
  /** Target status to apply. */
  targetStatus: KdsStatus;
  /** Current retry count (for exponential backoff display). */
  retryCount: number;
  /** Timestamp of when the action was originally attempted. */
  createdAt: string;
  /** Last error message. */
  lastError: string;
  /** OFF-05: earliest allowed retry time (ISO). Actions are skipped until then. */
  nextAttemptAt?: string;
}

/** OFF-05: an action that exhausted its retries and needs operator attention. */
export interface DeadLetterKdsAction extends PendingKdsAction {
  /** When the action was moved to the dead-letter list. */
  deadLetterAt: string;
}

/** Return type of the `useKdsOffline` hook. */
export interface UseKdsOfflineReturn {
  /** Whether the backend is currently reachable. */
  online: boolean;
  /** Last-known-good cached orders, or null if never fetched. */
  cachedOrders: KdsOrder[] | null;
  /** Timestamp of last successful backend sync (ISO string, or null). */
  lastSyncAt: string | null;
  /** Number of actions pending retry. */
  pendingQueueLength: number;
  /** Pending actions for display/debug. */
  pendingActions: PendingKdsAction[];
  /** OFF-05: actions that exhausted retries and need operator attention. */
  deadLetterActions: DeadLetterKdsAction[];
  /** OFF-05: number of dead-lettered actions. */
  deadLetterLength: number;
  /** Whether the hook is in its initial loading state. */
  initialLoading: boolean;
  /** OFF-08: true when localStorage writes are failing (quota/unavailable). */
  storageUnavailable: boolean;

  /**
   * Wrap a fetch operation with cache fallback.
   *
   * On success: updates the cache + clears initialLoading.
   * On failure: returns cached orders (may be null), marks offline.
   *
   * After a successful fetch, automatically flushes the pending queue.
   */
  wrapFetch: (
    fetchFn: () => Promise<KdsOrder[]>,
  ) => Promise<{ orders: KdsOrder[]; fromCache: boolean }>;

  /**
   * Wrap a status-update operation with optimistic queue-on-failure.
   *
   * On success: clears any pending action for this order.
   * On failure: queues the action for retry AND persists the optimistic
   * status into the cached snapshot (OFF-03) so a reload keeps the
   * locally-advanced state visible.
   *
   * Returns `true` when the update succeeded, `false` when queued.
   */
  wrapUpdate: (
    orderId: string,
    targetStatus: KdsStatus,
    updateFn: () => Promise<unknown>,
  ) => Promise<boolean>;

  /** Manually retry all pending actions. Returns count of successes. */
  retryPending: (executeFn: (action: PendingKdsAction) => Promise<boolean>) => Promise<number>;

  /** Clear all pending actions (discard them). */
  clearPending: () => void;

  /** OFF-05: discard the dead-letter list (operator acknowledges failures). */
  clearDeadLetter: () => void;

  /** Counter incremented on OS-level `online` event for fetch effect dependency. */
  forceRetryCounter: number;
}

// ── LocalStorage helpers ─────────────────────────────────────────────

function readLS<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return fallback;
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

/** Write to localStorage. Returns false when persistence failed (OFF-08). */
function writeLS<T>(key: string, value: T): boolean {
  try {
    localStorage.setItem(key, JSON.stringify(value));
    return true;
  } catch {
    /* quota exceeded or storage unavailable */
    return false;
  }
}

/** OFF-05: compute the next retry timestamp with exponential backoff + jitter. */
function nextAttemptAt(retryCount: number): string {
  const exp = Math.pow(2, retryCount - 1); // retry 1 → 1s, 2 → 2s, 3 → 4s…
  const jitter = 1 + (Math.random() * 2 - 1) * BACKOFF_JITTER; // ±30%
  const delayMs = Math.round(BACKOFF_BASE_MS * exp * jitter);
  return new Date(Date.now() + delayMs).toISOString();
}

/** OFF-03: apply queued actions to a fetched snapshot deterministically. */
function applyProjections(
  orders: KdsOrder[],
  queue: PendingKdsAction[],
): KdsOrder[] {
  if (queue.length === 0) return orders;
  return orders.map((order) => {
    const action = queue.find(
      (a) => a.orderId === order.id && a.targetStatus !== order.status,
    );
    return action ? { ...order, status: action.targetStatus } : order;
  });
}

// ── Hook ─────────────────────────────────────────────────────────────

/**
 * Provide offline resilience for the KDS screen.
 *
 * Caches the last known good order list in localStorage, queues
 * failed status updates for later retry, and exposes connection
 * status so the UI can show a banner.
 *
 * @example
 * ```tsx
 * const {
 *   online, cachedOrders, pendingQueueLength,
 *   wrapFetch, wrapUpdate, retryPending,
 * } = useKdsOffline();
 *
 * const fetchOrders = useCallback(async () => {
 *   const { orders } = await wrapFetch(() => getKdsQueueScoped(token));
 *   setOrders(orders);
 *   // ... filter by store_id ...
 * }, [wrapFetch, ...]);
 *
 * const advanceStatus = useCallback(async (order: KdsOrder) => {
 *   const ok = await wrapUpdate(order.id, nextStatus, () =>
 *     updateKdsStatusScoped(token, order.id, nextStatus)
 *   );
 *   if (!ok) {
 *     // Optimistically advance in local state
 *     setOrders(prev => prev.map(o =>
 *       o.id === order.id ? { ...o, status: nextStatus } : o
 *     ));
 *   }
 * }, [wrapUpdate, ...]);
 * ```
 */
export function useKdsOffline(): UseKdsOfflineReturn {
  const [online, setOnline] = useState(true);
  const [cachedOrders, setCachedOrders] = useState<KdsOrder[] | null>(
    () => readLS<KdsOrder[] | null>(LS_CACHED_ORDERS, null),
  );
  const [lastSyncAt, setLastSyncAt] = useState<string | null>(
    () => readLS<string | null>(LS_LAST_SYNC, null),
  );
  const [pendingActions, setPendingActions] = useState<PendingKdsAction[]>(
    () => readLS<PendingKdsAction[]>(LS_OFFLINE_QUEUE, []),
  );
  const [deadLetterActions, setDeadLetterActions] = useState<DeadLetterKdsAction[]>(
    () => readLS<DeadLetterKdsAction[]>(LS_DEAD_LETTER, []),
  );
  const [initialLoading, setInitialLoading] = useState(true);
  const [forceRetryCounter, setForceRetryCounter] = useState(0);
  // OFF-08: persistence failures are observable, not silently swallowed.
  const [storageUnavailable, setStorageUnavailable] = useState(false);

  // Expose pending queue length as a derived value.
  const pendingQueueLength = pendingActions.length;
  const deadLetterLength = deadLetterActions.length;

  // ── Cache helpers ──────────────────────────────────────────────────

  const updateCache = useCallback((orders: KdsOrder[]) => {
    const now = new Date().toISOString();
    // OFF-03: persist the optimistic projection over fresh server data so a
    // reload while actions are still queued keeps the locally-advanced state.
    const queue = readLS<PendingKdsAction[]>(LS_OFFLINE_QUEUE, []);
    const projected = applyProjections(orders, queue);
    setCachedOrders(projected);
    setLastSyncAt(now);
    const ok1 = writeLS(LS_CACHED_ORDERS, projected);
    const ok2 = writeLS(LS_LAST_SYNC, now);
    if (!ok1 || !ok2) setStorageUnavailable(true);
  }, []);

  /** OFF-03: project a single status change onto the cached snapshot. */
  const projectStatusOntoCache = useCallback((orderId: string, targetStatus: KdsStatus) => {
    setCachedOrders((prev) => {
      if (!prev) return prev;
      const next = prev.map((o) =>
        o.id === orderId ? { ...o, status: targetStatus } : o,
      );
      const ok = writeLS(LS_CACHED_ORDERS, next);
      if (!ok) setStorageUnavailable(true);
      return next;
    });
  }, []);

  const loadQueue = useCallback(() => {
    const q = readLS<PendingKdsAction[]>(LS_OFFLINE_QUEUE, []);
    setPendingActions(q);
    return q;
  }, []);

  const saveQueue = useCallback((queue: PendingKdsAction[]) => {
    setPendingActions(queue);
    const ok = writeLS(LS_OFFLINE_QUEUE, queue);
    if (!ok) setStorageUnavailable(true);
  }, []);

  const loadDeadLetter = useCallback(() => {
    const d = readLS<DeadLetterKdsAction[]>(LS_DEAD_LETTER, []);
    setDeadLetterActions(d);
    return d;
  }, []);

  const saveDeadLetter = useCallback((dead: DeadLetterKdsAction[]) => {
    setDeadLetterActions(dead);
    const ok = writeLS(LS_DEAD_LETTER, dead);
    if (!ok) setStorageUnavailable(true);
  }, []);

  // ── wrapFetch ──────────────────────────────────────────────────────

  const wrapFetch = useCallback(
    async (
      fetchFn: () => Promise<KdsOrder[]>,
    ): Promise<{ orders: KdsOrder[]; fromCache: boolean }> => {
      try {
        const orders = await fetchFn();
        // Backend is reachable — update cache, mark online.
        setOnline(true);
        updateCache(orders);
        if (initialLoading) setInitialLoading(false);

        return { orders, fromCache: false };
      } catch {
        // Backend unreachable — fall back to cache.
        setOnline(false);
        if (initialLoading) setInitialLoading(false);

        if (cachedOrders) {
          return { orders: cachedOrders, fromCache: true };
        }

        // No cache available either — return empty array.
        return { orders: [], fromCache: true };
      }
    },
    [cachedOrders, initialLoading, updateCache],
  );

  // ── wrapUpdate ─────────────────────────────────────────────────────

  const wrapUpdate = useCallback(
    async (
      orderId: string,
      targetStatus: KdsStatus,
      updateFn: () => Promise<unknown>,
    ): Promise<boolean> => {
      try {
        await updateFn();
        // Success — clear any pending action for this order.
        setOnline(true);
        const queue = loadQueue();
        const remaining = queue.filter((a) => a.orderId !== orderId);
        if (remaining.length !== queue.length) {
          saveQueue(remaining);
        }
        return true;
      } catch (e) {
        // Failure — queue the action for retry.
        setOnline(false);
        const action: PendingKdsAction = {
          id: `${orderId}->${targetStatus}`,
          orderId,
          targetStatus,
          retryCount: 0,
          createdAt: new Date().toISOString(),
          lastError: e instanceof Error ? e.message : String(e),
        };

        const queue = loadQueue();
        // Deduplicate — replace existing action for same order+status.
        const filtered = queue.filter((a) => a.id !== action.id);
        filtered.push(action);
        saveQueue(filtered);

        // OFF-03: persist the optimistic status into the cached snapshot so
        // a reload keeps the locally-advanced state visible.
        projectStatusOntoCache(orderId, targetStatus);

        return false;
      }
    },
    [loadQueue, saveQueue, projectStatusOntoCache],
  );

  // ── retryPending ───────────────────────────────────────────────────

  const retryPending = useCallback(
    async (
      executeFn: (action: PendingKdsAction) => Promise<boolean>,
    ): Promise<number> => {
      const queue = loadQueue();
      if (queue.length === 0) return 0;

      const now = Date.now();
      let successes = 0;
      const kept: PendingKdsAction[] = [];
      const dead: DeadLetterKdsAction[] = [];

      for (const action of queue) {
        // OFF-05: skip actions whose backoff window has not elapsed.
        if (action.nextAttemptAt && new Date(action.nextAttemptAt).getTime() > now) {
          kept.push(action);
          continue;
        }
        try {
          const ok = await executeFn(action);
          if (ok) {
            successes++;
          } else {
            const next = {
              ...action,
              retryCount: action.retryCount + 1,
              nextAttemptAt: nextAttemptAt(action.retryCount + 1),
            };
            if (next.retryCount >= MAX_RETRY_ATTEMPTS) {
              dead.push({
                ...next,
                deadLetterAt: new Date().toISOString(),
              });
            } else {
              kept.push(next);
            }
          }
        } catch (e) {
          const next = {
            ...action,
            retryCount: action.retryCount + 1,
            lastError: e instanceof Error ? e.message : 'Retry failed',
            nextAttemptAt: nextAttemptAt(action.retryCount + 1),
          };
          if (next.retryCount >= MAX_RETRY_ATTEMPTS) {
            dead.push({
              ...next,
              deadLetterAt: new Date().toISOString(),
            });
          } else {
            kept.push(next);
          }
        }
      }

      saveQueue(kept);
      if (dead.length > 0) {
        const existing = loadDeadLetter();
        saveDeadLetter([...existing, ...dead]);
      }
      if (kept.length === 0) {
        setOnline(true);
      }
      return successes;
    },
    [loadQueue, saveQueue, loadDeadLetter, saveDeadLetter],
  );

  // ── clearPending / clearDeadLetter ─────────────────────────────────

  const clearPending = useCallback(() => {
    saveQueue([]);
  }, [saveQueue]);

  const clearDeadLetter = useCallback(() => {
    saveDeadLetter([]);
  }, [saveDeadLetter]);

  // ── Listen for OS-level online/offline events ──────────────────────

  useEffect(() => {
    const handleOnline = () => {
      setForceRetryCounter((prev) => prev + 1);
    };

    window.addEventListener('online', handleOnline);
    return () => window.removeEventListener('online', handleOnline);
  }, []);

  return {
    online,
    cachedOrders,
    lastSyncAt,
    pendingQueueLength,
    pendingActions,
    deadLetterActions,
    deadLetterLength,
    initialLoading,
    storageUnavailable,
    wrapFetch,
    wrapUpdate,
    retryPending,
    clearPending,
    clearDeadLetter,
    forceRetryCounter,
  };
}
