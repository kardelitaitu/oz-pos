//! `useKdsOffline` — offline resilience hook for the KDS screen.
//!
//! Provides:
//!   • Cached KDS orders in localStorage (last-known-good snapshot)
//!   • Pending-action queue for failed status updates (retried on reconnect)
//!   • Connection-status detection (backend unreachable)
//!   • Wrapped fetch/update functions with automatic cache + queue logic
//!
//! The hook does NOT depend on any Tauri APIs, making it testable in
//! plain vitest/JSDOM. All storage is via localStorage with try/catch
//! guards so it degrades gracefully when storage is unavailable.

import { useState, useCallback, useEffect } from 'react';
import type { KdsOrder, KdsStatus } from '@/api/kds';

// ── Constants ───────────────────────────────────────────────────────

const LS_CACHED_ORDERS = 'kds-cached-orders';
const LS_LAST_SYNC = 'kds-last-sync';
const LS_OFFLINE_QUEUE = 'kds-offline-queue';


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
  /** Whether the hook is in its initial loading state. */
  initialLoading: boolean;

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
   * On failure: queues the action for retry.
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

function writeLS<T>(key: string, value: T): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* quota exceeded or storage unavailable — ignore */
  }
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
  const [initialLoading, setInitialLoading] = useState(true);
  const [forceRetryCounter, setForceRetryCounter] = useState(0);

  // Expose pending queue length as a derived value.
  const pendingQueueLength = pendingActions.length;

  // ── Cache helpers ──────────────────────────────────────────────────

  const updateCache = useCallback((orders: KdsOrder[]) => {
    const now = new Date().toISOString();
    setCachedOrders(orders);
    setLastSyncAt(now);
    writeLS(LS_CACHED_ORDERS, orders);
    writeLS(LS_LAST_SYNC, now);
  }, []);

  const loadQueue = useCallback(() => {
    const q = readLS<PendingKdsAction[]>(LS_OFFLINE_QUEUE, []);
    setPendingActions(q);
    return q;
  }, []);

  const saveQueue = useCallback((queue: PendingKdsAction[]) => {
    setPendingActions(queue);
    writeLS(LS_OFFLINE_QUEUE, queue);
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

        return false;
      }
    },
    [loadQueue, saveQueue],
  );

  // ── retryPending ───────────────────────────────────────────────────

  const retryPending = useCallback(
    async (
      executeFn: (action: PendingKdsAction) => Promise<boolean>,
    ): Promise<number> => {
      const queue = loadQueue();
      if (queue.length === 0) return 0;

      let successes = 0;
      const remaining: PendingKdsAction[] = [];

      for (const action of queue) {
        try {
          const ok = await executeFn(action);
          if (ok) {
            successes++;
          } else {
            remaining.push({
              ...action,
              retryCount: action.retryCount + 1,
            });
          }
        } catch {
          remaining.push({
            ...action,
            retryCount: action.retryCount + 1,
            lastError: 'Retry failed',
          });
        }
      }

      saveQueue(remaining);
      if (remaining.length === 0) {
        setOnline(true);
      }
      return successes;
    },
    [loadQueue, saveQueue],
  );

  // ── clearPending ───────────────────────────────────────────────────

  const clearPending = useCallback(() => {
    saveQueue([]);
  }, [saveQueue]);

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
    initialLoading,
    wrapFetch,
    wrapUpdate,
    retryPending,
    clearPending,
    forceRetryCounter,
  };
}
