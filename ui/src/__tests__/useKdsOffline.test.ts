import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useKdsOffline, MAX_RETRY_ATTEMPTS } from '@/hooks/useKdsOffline';
import type { KdsOrder } from '@/api/kds';
import type { PendingKdsAction } from '@/hooks/useKdsOffline';

// ── Helpers ──────────────────────────────────────────────────────────

const LS_CACHED_ORDERS = 'kds-cached-orders';
const LS_LAST_SYNC = 'kds-last-sync';
const LS_OFFLINE_QUEUE = 'kds-offline-queue';
const LS_DEAD_LETTER = 'kds-offline-dead-letter';

function makeOrder(overrides: Partial<KdsOrder> = {}): KdsOrder {
  return {
    id: 'o-1',
    sale_id: 's-1',
    store_id: null,
    status: 'pending',
    items_summary: 'Burger x1, Fries x1',
    item_count: 2,
    display_number: 101,
    received_at: new Date().toISOString(),
    started_at: null,
    ready_at: null,
    served_at: null,
    prep_time_seconds: 0,
    kitchen_zone: null,
    notes: '',
    table_number: null,
    priority: false,
    ...overrides,
  };
}

function seedLocalStorage<K>(key: string, value: K): void {
  localStorage.setItem(key, JSON.stringify(value));
}

function drainLocalStorage(): void {
  localStorage.clear();
}

/** A fresh ISO timestamp so the OFF-07 cache-expiry check passes. */
function freshIso(): string {
  return new Date().toISOString();
}

/** A timestamp old enough to trigger the OFF-07 expiry (age > CACHE_TTL_MS). */
function staleIso(): string {
  return new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString();
}

// ── Suite ─────────────────────────────────────────────────────────────

describe('useKdsOffline', () => {
  beforeEach(() => {
    drainLocalStorage();
  });

  afterEach(() => {
    drainLocalStorage();
  });

  // ── Initial state ──────────────────────────────────────────────────

  describe('initial state', () => {
    it('starts online with no cached orders and empty queue', () => {
      const { result } = renderHook(() => useKdsOffline());

      expect(result.current.online).toBe(true);
      expect(result.current.cachedOrders).toBeNull();
      expect(result.current.lastSyncAt).toBeNull();
      expect(result.current.pendingQueueLength).toBe(0);
      expect(result.current.pendingActions).toEqual([]);
      expect(result.current.initialLoading).toBe(true);
      expect(result.current.forceRetryCounter).toBe(0);
    });

    it('restores cached orders from localStorage on mount', () => {
      const orders = [makeOrder({ id: 'o-1' }), makeOrder({ id: 'o-2' })];
      const sync = freshIso();
      seedLocalStorage(LS_CACHED_ORDERS, orders);
      seedLocalStorage(LS_LAST_SYNC, sync);

      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.cachedOrders).toEqual(orders);
      expect(result.current.lastSyncAt).toBe(sync);
    });

    it('restores pending queue from localStorage on mount', () => {
      const queue: PendingKdsAction[] = [
        {
          id: 'o-1->preparing',
          orderId: 'o-1',
          targetStatus: 'preparing',
          retryCount: 1,
          createdAt: '2026-07-30T12:00:00.000Z',
          lastError: 'Network error',
        },
      ];
      seedLocalStorage(LS_OFFLINE_QUEUE, queue);

      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.pendingQueueLength).toBe(1);
      expect(result.current.pendingActions).toEqual(queue);
    });

    it('handles corrupted localStorage gracefully', () => {
      localStorage.setItem(LS_CACHED_ORDERS, 'not-json');
      localStorage.setItem(LS_OFFLINE_QUEUE, '{{{broken}}}');

      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.cachedOrders).toBeNull();
      expect(result.current.pendingActions).toEqual([]);
    });
  });

  // ── wrapFetch ──────────────────────────────────────────────────────

  describe('wrapFetch', () => {
    it('returns fresh orders on success and updates cache', async () => {
      const { result } = renderHook(() => useKdsOffline());
      const orders = [makeOrder()];

      let fetchResult: { orders: KdsOrder[]; fromCache: boolean } | null = null;
      await act(async () => {
        fetchResult = await result.current.wrapFetch(() => Promise.resolve(orders));
      });

      expect(fetchResult!.orders).toEqual(orders);
      expect(fetchResult!.fromCache).toBe(false);
      expect(result.current.online).toBe(true);
      expect(result.current.cachedOrders).toEqual(orders);
      expect(result.current.lastSyncAt).not.toBeNull();
      expect(result.current.initialLoading).toBe(false);

      // Verify cache persisted to localStorage
      const cached = JSON.parse(localStorage.getItem(LS_CACHED_ORDERS) || 'null');
      expect(cached).toEqual(orders);
    });

    it('returns cached orders on fetch failure when cache exists', async () => {
      const orders = [makeOrder()];
      seedLocalStorage(LS_CACHED_ORDERS, orders);
      seedLocalStorage(LS_LAST_SYNC, freshIso());

      const { result } = renderHook(() => useKdsOffline());

      let fetchResult: { orders: KdsOrder[]; fromCache: boolean } | null = null;
      await act(async () => {
        fetchResult = await result.current.wrapFetch(() => Promise.reject(new Error('Network down')));
      });

      expect(fetchResult!.orders).toEqual(orders);
      expect(fetchResult!.fromCache).toBe(true);
      expect(result.current.online).toBe(false);
      expect(result.current.initialLoading).toBe(false);
    });

    it('returns empty array on fetch failure when no cache exists', async () => {
      const { result } = renderHook(() => useKdsOffline());

      let fetchResult: { orders: KdsOrder[]; fromCache: boolean } | null = null;
      await act(async () => {
        fetchResult = await result.current.wrapFetch(() => Promise.reject(new Error('Offline')));
      });

      expect(fetchResult!.orders).toEqual([]);
      expect(fetchResult!.fromCache).toBe(true);
      expect(result.current.online).toBe(false);
    });

    it('clears initialLoading on both success and failure', async () => {
      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.initialLoading).toBe(true);

      // Failure path
      await act(async () => {
        await result.current.wrapFetch(() => Promise.reject(new Error('err')));
      });
      expect(result.current.initialLoading).toBe(false);

      // Reset hook for success path
      const { result: result2 } = renderHook(() => useKdsOffline());
      expect(result2.current.initialLoading).toBe(true);
      await act(async () => {
        await result2.current.wrapFetch(() => Promise.resolve([makeOrder()]));
      });
      expect(result2.current.initialLoading).toBe(false);
    });

    it('updates cache with fresh orders on subsequent successful fetches', async () => {
      const { result } = renderHook(() => useKdsOffline());

      // First fetch
      await act(async () => {
        await result.current.wrapFetch(() => Promise.resolve([makeOrder({ id: 'o-1' })]));
      });

      // Second fetch with different data
      const secondOrder = makeOrder({ id: 'o-2' });
      await act(async () => {
        await result.current.wrapFetch(() => Promise.resolve([secondOrder]));
      });

      expect(result.current.cachedOrders).toEqual([secondOrder]);
    });
  });

  // ── wrapUpdate ─────────────────────────────────────────────────────

  describe('wrapUpdate', () => {
    it('returns true on successful update and clears pending action', async () => {
      const { result } = renderHook(() => useKdsOffline());

      let success = false;
      await act(async () => {
        success = await result.current.wrapUpdate('o-1', 'preparing', () => Promise.resolve());
      });

      expect(success).toBe(true);
      expect(result.current.online).toBe(true);
      expect(result.current.pendingQueueLength).toBe(0);
    });

    it('queues action on update failure and returns false', async () => {
      const { result } = renderHook(() => useKdsOffline());

      let success = true;
      await act(async () => {
        success = await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('Timeout')));
      });

      expect(success).toBe(false);
      expect(result.current.online).toBe(false);
      expect(result.current.pendingQueueLength).toBe(1);
      expect(result.current.pendingActions[0]!.orderId).toBe('o-1');
      expect(result.current.pendingActions[0]!.targetStatus).toBe('preparing');
      expect(result.current.pendingActions[0]!.retryCount).toBe(0);
      expect(result.current.pendingActions[0]!.lastError).toBe('Timeout');

      // Verify persisted to localStorage
      const stored = JSON.parse(localStorage.getItem(LS_OFFLINE_QUEUE) || '[]');
      expect(stored.length).toBe(1);
      expect(stored[0].orderId).toBe('o-1');
    });

    it('deduplicates actions for the same order+status', async () => {
      const { result } = renderHook(() => useKdsOffline());

      // Two failures, same order+status
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('First')));
      });
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('Second')));
      });

      // Should only have one queued action (last one replaces previous)
      expect(result.current.pendingQueueLength).toBe(1);
      expect(result.current.pendingActions[0]!.lastError).toBe('Second');
    });

    it('maintains separate entries for different order+status combinations', async () => {
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err1')));
      });
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'ready', () => Promise.reject(new Error('err2')));
      });
      await act(async () => {
        await result.current.wrapUpdate('o-2', 'preparing', () => Promise.reject(new Error('err3')));
      });

      expect(result.current.pendingQueueLength).toBe(3);
    });

    it('clears only the matching pending action on success', async () => {
      const { result } = renderHook(() => useKdsOffline());

      // Queue three actions
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err1')));
      });
      await act(async () => {
        await result.current.wrapUpdate('o-2', 'preparing', () => Promise.reject(new Error('err2')));
      });
      expect(result.current.pendingQueueLength).toBe(2);

      // Successfully update o-1 — should remove only o-1's pending action
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.resolve());
      });

      expect(result.current.pendingQueueLength).toBe(1);
      expect(result.current.pendingActions[0]!.orderId).toBe('o-2');
    });
  });

  // ── retryPending ───────────────────────────────────────────────────

  describe('retryPending', () => {
    it('returns 0 when queue is empty', async () => {
      const { result } = renderHook(() => useKdsOffline());

      let count = -1;
      await act(async () => {
        count = await result.current.retryPending(() => Promise.resolve(true));
      });

      expect(count).toBe(0);
    });

    it('retries all queued actions and returns success count', async () => {
      const { result } = renderHook(() => useKdsOffline());

      // Queue two actions
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });
      await act(async () => {
        await result.current.wrapUpdate('o-2', 'ready', () => Promise.reject(new Error('err')));
      });
      expect(result.current.pendingQueueLength).toBe(2);

      // Retry both — both succeed
      let count = 0;
      await act(async () => {
        count = await result.current.retryPending(() => Promise.resolve(true));
      });

      expect(count).toBe(2);
      expect(result.current.pendingQueueLength).toBe(0);
      expect(result.current.online).toBe(true);
    });

    it('keeps actions whose retry fails and increments retryCount', async () => {
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });
      expect(result.current.pendingActions[0]!.retryCount).toBe(0);

      // Retry — fails again
      let count = 0;
      await act(async () => {
        count = await result.current.retryPending(() => Promise.resolve(false));
      });

      expect(count).toBe(0);
      expect(result.current.pendingQueueLength).toBe(1);
      expect(result.current.pendingActions[0]!.retryCount).toBe(1);
    });

    it('handles partial success — keeps only failed actions', async () => {
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });
      await act(async () => {
        await result.current.wrapUpdate('o-2', 'ready', () => Promise.reject(new Error('err')));
      });

      // First succeeds, second fails
      let callCount = 0;
      let count = 0;
      await act(async () => {
        count = await result.current.retryPending(() => {
          callCount++;
          return Promise.resolve(callCount === 1);
        });
      });

      expect(count).toBe(1);
      expect(result.current.pendingQueueLength).toBe(1);
      expect(result.current.pendingActions[0]!.orderId).toBe('o-2');
    });

    it('handles executeFn throwing an error', async () => {
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });

      let count = 0;
      await act(async () => {
        count = await result.current.retryPending(() => Promise.reject(new Error('Execute error')));
      });

      expect(count).toBe(0);
      expect(result.current.pendingQueueLength).toBe(1);
      expect(result.current.pendingActions[0]!.retryCount).toBe(1);
      // OFF-05: the real error is preserved (not a generic 'Retry failed').
      expect(result.current.pendingActions[0]!.lastError).toBe('Execute error');
    });

    it('returns to online when all pending actions succeed', async () => {
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });
      expect(result.current.online).toBe(false);

      await act(async () => {
        await result.current.retryPending(() => Promise.resolve(true));
      });

      expect(result.current.online).toBe(true);
    });
  });

  // ── OFF-03: durable optimistic projection ────────────────────────

  describe('OFF-03 durable optimistic projection', () => {
    it('persists the optimistic status into the cached snapshot on update failure', async () => {
      // Seed a cache, then fail an update — the cached order must flip to the
      // locally-advanced status and be written to localStorage.
      seedLocalStorage(LS_CACHED_ORDERS, [makeOrder({ id: 'o-1', status: 'pending' })]);
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('Down')));
      });

      const cached = JSON.parse(localStorage.getItem(LS_CACHED_ORDERS) || '[]');
      expect(cached[0].status).toBe('preparing');
      expect(result.current.pendingQueueLength).toBe(1);
    });

    it('reload keeps the projected status (localStorage survives a remount)', async () => {
      seedLocalStorage(LS_CACHED_ORDERS, [makeOrder({ id: 'o-1', status: 'pending' })]);
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('Down')));
      });

      // Simulate reload: new hook instance reads the same localStorage.
      const { result: result2 } = renderHook(() => useKdsOffline());
      expect(result2.current.cachedOrders?.[0]?.status).toBe('preparing');
      expect(result2.current.pendingActions).toHaveLength(1);
    });

    it('a successful fetch replays queued projections over fresh data', async () => {
      // Queue an action, then a fetch succeeds with the OLD server status.
      const { result } = renderHook(() => useKdsOffline());
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('Down')));
      });

      await act(async () => {
        await result.current.wrapFetch(() =>
          Promise.resolve([makeOrder({ id: 'o-1', status: 'pending' })]),
        );
      });

      // The cache must show the projected status, not the stale server value.
      expect(result.current.cachedOrders?.[0]?.status).toBe('preparing');
      expect(result.current.pendingQueueLength).toBe(1);
    });
  });

  // ── OFF-05: bounded retry + dead-letter ──────────────────────────

  describe('OFF-05 bounded retry and dead-letter', () => {
    it('moves an exhausted action to the dead-letter list', async () => {
      const { result } = renderHook(() => useKdsOffline());
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });

      // Retry until exhaustion. Each retry increments retryCount; the backoff
      // window (nextAttemptAt) is in the future, so we stub Date.now to keep
      // advancing past it.
      const realNow = Date.now;
      let fakeNow = realNow();
      vi.spyOn(Date, 'now').mockImplementation(() => fakeNow);
      try {
        for (let i = 0; i < MAX_RETRY_ATTEMPTS; i += 1) {
          let count = 0;
          await act(async () => {
            count = await result.current.retryPending(() => Promise.resolve(false));
          });
          expect(count).toBe(0);
          fakeNow += 120_000; // advance past the backoff window
        }
      } finally {
        vi.restoreAllMocks();
      }

      expect(result.current.pendingQueueLength).toBe(0);
      expect(result.current.deadLetterLength).toBe(1);
      expect(result.current.deadLetterActions[0]?.orderId).toBe('o-1');
      expect(result.current.deadLetterActions[0]?.retryCount).toBeGreaterThanOrEqual(MAX_RETRY_ATTEMPTS);
      // Dead letter persists to localStorage.
      const stored = JSON.parse(localStorage.getItem(LS_DEAD_LETTER) || '[]');
      expect(stored).toHaveLength(1);
    });

    it('skips actions whose backoff window has not elapsed', async () => {
      const { result } = renderHook(() => useKdsOffline());
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });

      // First retry fails → retryCount 1, nextAttemptAt in the future.
      await act(async () => {
        await result.current.retryPending(() => Promise.resolve(false));
      });
      expect(result.current.pendingActions[0]?.retryCount).toBe(1);
      expect(result.current.pendingActions[0]?.nextAttemptAt).toBeDefined();

      // Immediate retry within the backoff window must not execute the action.
      const execute = vi.fn(() => Promise.resolve(true));
      await act(async () => {
        await result.current.retryPending(execute);
      });
      expect(execute).not.toHaveBeenCalled();
      expect(result.current.pendingQueueLength).toBe(1);
    });

    it('clearDeadLetter empties the dead-letter list', async () => {
      const { result } = renderHook(() => useKdsOffline());
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });

      // Exhaust retries, advancing fake time past each backoff window so the
      // action actually reaches the dead-letter list.
      const realNow = Date.now;
      let fakeNow = realNow();
      vi.spyOn(Date, 'now').mockImplementation(() => fakeNow);
      try {
        for (let i = 0; i < MAX_RETRY_ATTEMPTS; i += 1) {
          await act(async () => {
            await result.current.retryPending(() => Promise.resolve(false));
          });
          fakeNow += 120_000;
        }
      } finally {
        vi.restoreAllMocks();
      }
      expect(result.current.deadLetterLength).toBe(1);

      act(() => result.current.clearDeadLetter());
      expect(result.current.deadLetterLength).toBe(0);
      expect(JSON.parse(localStorage.getItem(LS_DEAD_LETTER) || '[]')).toEqual([]);
    });
  });

  // ── OFF-07: store-scoped storage + cache expiry ──────────────────

  describe('OFF-07 store-scoped storage and expiry', () => {
    it('restores an unexpired cache but ignores an expired one', () => {
      const order = makeOrder({ id: 'o-1' });
      seedLocalStorage(LS_CACHED_ORDERS, [order]);
      seedLocalStorage(LS_LAST_SYNC, freshIso());
      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.cachedOrders).toEqual([order]);
    });

    it('treats a snapshot older than the TTL as expired (no stale data)', () => {
      seedLocalStorage(LS_CACHED_ORDERS, [makeOrder({ id: 'o-1', status: 'ready' })]);
      seedLocalStorage(LS_LAST_SYNC, staleIso());
      const { result } = renderHook(() => useKdsOffline());
      // OFF-07: stale orders must not linger indefinitely on a shared terminal.
      expect(result.current.cachedOrders).toBeNull();
    });

    it('isolates cache and queue per store scope', async () => {
      // Store A seeds a cache + queue.
      seedLocalStorage('kds-cached-orders:store-a', [makeOrder({ id: 'a-1' })]);
      seedLocalStorage('kds-last-sync:store-a', freshIso());
      const queueA: PendingKdsAction[] = [{
        id: 'a-1->preparing',
        orderId: 'a-1',
        targetStatus: 'preparing',
        retryCount: 1,
        createdAt: freshIso(),
        lastError: 'Down',
        storeId: 'store-a',
      }];
      seedLocalStorage('kds-offline-queue:store-a', queueA);

      const { result: storeA } = renderHook(() => useKdsOffline('store-a'));
      expect(storeA.current.cachedOrders?.[0]?.id).toBe('a-1');
      expect(storeA.current.pendingQueueLength).toBe(1);

      // Store B sees none of it.
      const { result: storeB } = renderHook(() => useKdsOffline('store-b'));
      expect(storeB.current.cachedOrders).toBeNull();
      expect(storeB.current.pendingQueueLength).toBe(0);
    });

    it('binds queued mutations to the store scope they were created in', async () => {
      const { result } = renderHook(() => useKdsOffline('store-a'));
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('Down')));
      });
      expect(result.current.pendingActions[0]?.storeId).toBe('store-a');

      // A different store replay never executes another store's action.
      const { result: storeB } = renderHook(() => useKdsOffline('store-b'));
      const execute = vi.fn(() => Promise.resolve(true));
      await act(async () => {
        await storeB.current.retryPending(execute);
      });
      expect(execute).not.toHaveBeenCalled();
    });
  });

  // ── OFF-10: cross-tab coordination ────────────────────────────────

  describe('OFF-10 cross-tab storage coordination', () => {
    it('reloads the queue when another tab writes the same scope', () => {
      const { result } = renderHook(() => useKdsOffline('store-a'));
      expect(result.current.pendingQueueLength).toBe(0);

      const otherTab: PendingKdsAction[] = [{
        id: 'o-9->ready',
        orderId: 'o-9',
        targetStatus: 'ready',
        retryCount: 0,
        createdAt: freshIso(),
        lastError: 'Down',
        storeId: 'store-a',
      }];
      localStorage.setItem('kds-offline-queue:store-a', JSON.stringify(otherTab));
      act(() => {
        window.dispatchEvent(
          new StorageEvent('storage', { key: 'kds-offline-queue:store-a' }),
        );
      });

      expect(result.current.pendingQueueLength).toBe(1);
    });

    it('ignores storage events for other keys', () => {
      const { result } = renderHook(() => useKdsOffline('store-a'));

      localStorage.setItem('kds-offline-queue:store-b', JSON.stringify([{
        id: 'b-1->ready',
        orderId: 'b-1',
        targetStatus: 'ready',
        retryCount: 0,
        createdAt: freshIso(),
        lastError: 'Down',
        storeId: 'store-b',
      }]));
      act(() => {
        window.dispatchEvent(
          new StorageEvent('storage', { key: 'kds-offline-queue:store-b' }),
        );
      });

      // Different scope — not this tab's queue.
      expect(result.current.pendingQueueLength).toBe(0);
    });
  });

  // ── OFF-08: persistence-failure visibility ───────────────────────

  describe('OFF-08 persistence-failure visibility', () => {
    it('surfaces storageUnavailable when localStorage writes fail', async () => {
      const setItemSpy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
        throw new Error('QuotaExceededError');
      });
      try {
        const { result } = renderHook(() => useKdsOffline());
        expect(result.current.storageUnavailable).toBe(false);

        await act(async () => {
          await result.current.wrapFetch(() => Promise.resolve([makeOrder()]));
        });

        // The operator must see that persistence is broken.
        expect(result.current.storageUnavailable).toBe(true);
      } finally {
        setItemSpy.mockRestore();
      }
    });
  });

  // ── clearPending ───────────────────────────────────────────────────

  describe('clearPending', () => {
    it('empties the pending queue', async () => {
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });
      expect(result.current.pendingQueueLength).toBe(1);

      act(() => {
        result.current.clearPending();
      });

      expect(result.current.pendingQueueLength).toBe(0);
      expect(result.current.pendingActions).toEqual([]);
      expect(JSON.parse(localStorage.getItem(LS_OFFLINE_QUEUE) || '[]')).toEqual([]);
    });

    it('is a no-op when queue is already empty', () => {
      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.pendingQueueLength).toBe(0);

      act(() => {
        result.current.clearPending();
      });

      expect(result.current.pendingQueueLength).toBe(0);
    });
  });

  // ── OS-level online event ──────────────────────────────────────────

  describe('OS-level online event', () => {
    it('increments forceRetryCounter when window fires online event', () => {
      const { result } = renderHook(() => useKdsOffline());
      const initialCounter = result.current.forceRetryCounter;

      act(() => {
        window.dispatchEvent(new Event('online'));
      });

      expect(result.current.forceRetryCounter).toBe(initialCounter + 1);
    });

    it('keeps online state unchanged (online is only set by fetch/update)', () => {
      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.online).toBe(true);

      act(() => {
        window.dispatchEvent(new Event('online'));
      });

      // The hook does NOT set online=true on OS-level events — only wrapFetch/wrapUpdate
      // control the online state. The counter exists for the parent to trigger re-fetch.
      expect(result.current.online).toBe(true);
    });

    it('removes the event listener on unmount', () => {
      const addSpy = vi.spyOn(window, 'addEventListener');
      const removeSpy = vi.spyOn(window, 'removeEventListener');

      const { unmount } = renderHook(() => useKdsOffline());
      expect(addSpy).toHaveBeenCalledWith('online', expect.any(Function));

      unmount();
      expect(removeSpy).toHaveBeenCalledWith('online', expect.any(Function));

      addSpy.mockRestore();
      removeSpy.mockRestore();
    });
  });

  // ── Edge cases ─────────────────────────────────────────────────────

  describe('edge cases', () => {
    it('handles localStorage setItem throwing (quota exceeded)', async () => {
      const setItemSpy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
        throw new Error('QuotaExceededError');
      });

      const { result } = renderHook(() => useKdsOffline());

      // wrapFetch should still work despite storage failure
      const insertedOrder = makeOrder();
      await act(async () => {
        const r = await result.current.wrapFetch(() => Promise.resolve([insertedOrder]));
        expect(r.orders).toHaveLength(1);
        expect(r.fromCache).toBe(false);
      });

      // In-memory cache should be updated even if localStorage failed
      expect(result.current.cachedOrders).toEqual([insertedOrder]);

      setItemSpy.mockRestore();
    });

    it('handles localStorage getItem throwing', () => {
      // Mock localStorage.getItem to throw once, then restore
      const originalGetItem = Storage.prototype.getItem;
      let callCount = 0;
      const getItemMock = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
        callCount++;
        if (callCount <= 3) throw new Error('Storage error');
        return originalGetItem.call(localStorage, '');
      });

      const { result } = renderHook(() => useKdsOffline());
      expect(result.current.cachedOrders).toBeNull();
      expect(result.current.pendingActions).toEqual([]);

      getItemMock.mockRestore();
    });

    it('recovers from offline to online after a successful wrapFetch', async () => {
      const { result } = renderHook(() => useKdsOffline());

      // Go offline
      await act(async () => {
        await result.current.wrapFetch(() => Promise.reject(new Error('Down')));
      });
      expect(result.current.online).toBe(false);

      // Come back online
      const freshOrder = makeOrder();
      await act(async () => {
        await result.current.wrapFetch(() => Promise.resolve([freshOrder]));
      });
      expect(result.current.online).toBe(true);
      expect(result.current.cachedOrders).toEqual([freshOrder]);
    });

    it('handles multiple sequential updates without race conditions', async () => {
      const { result } = renderHook(() => useKdsOffline());

      // Queue 3 actions sequentially
      for (const id of ['o-1', 'o-2', 'o-3']) {
        await act(async () => {
          await result.current.wrapUpdate(id, 'preparing', () => Promise.reject(new Error('err')));
        });
      }

      // Read React state to confirm all 3 are queued
      expect(result.current.pendingQueueLength).toBe(3);

      // Retry all 3
      let count = 0;
      await act(async () => {
        count = await result.current.retryPending(() => Promise.resolve(true));
      });
      expect(count).toBe(3);
      expect(result.current.pendingQueueLength).toBe(0);
    });

    it('preserves pending queue after a successful wrapFetch', async () => {
      const { result } = renderHook(() => useKdsOffline());

      // Queue a pending action first
      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('err')));
      });
      expect(result.current.pendingQueueLength).toBe(1);

      // Successful fetch should not clear the pending queue
      await act(async () => {
        await result.current.wrapFetch(() => Promise.resolve([makeOrder()]));
      });

      // Pending queue should still be intact
      expect(result.current.pendingQueueLength).toBe(1);
    });

    it('exposes pendingActions with correct structure for UI display', async () => {
      const { result } = renderHook(() => useKdsOffline());

      await act(async () => {
        await result.current.wrapUpdate('o-1', 'preparing', () => Promise.reject(new Error('Connection timeout')));
      });

      const action = result.current.pendingActions[0]!;
      expect(action).toHaveProperty('id');
      expect(action).toHaveProperty('orderId', 'o-1');
      expect(action).toHaveProperty('targetStatus', 'preparing');
      expect(action).toHaveProperty('retryCount', 0);
      expect(action).toHaveProperty('createdAt');
      expect(action).toHaveProperty('lastError', 'Connection timeout');
      expect(action.id).toBe('o-1->preparing');
    });
  });
});
