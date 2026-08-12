import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import {
  TtlCache,
  analyticsDataCache,
  analyticsQueryKey,
  cardQueryKey,
  clearAnalyticsCache,
  ANALYTICS_CACHE_STORAGE_KEY,
  ANALYTICS_CACHE_VERSION,
  type CachePersistence,
} from '@/features/analytics/analytics-cache';

/** In-memory storage double for persistence tests. */
function memStorage(): { store: Map<string, string>; api: CachePersistence } {
  const store = new Map<string, string>();
  return {
    store,
    api: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => { store.set(k, v); },
      removeItem: (k: string) => { store.delete(k); },
      keys: () => [...store.keys()],
    },
  };
}

describe('TtlCache', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('stores and reads a fresh entry', () => {
    const cache = new TtlCache<string>();
    cache.set('a', 'hello');
    expect(cache.get('a')).toEqual({ value: 'hello', fresh: true });
    expect(cache.hasFresh('a')).toBe(true);
  });

  it('expires entries after the TTL, keeping stale values readable', () => {
    const cache = new TtlCache<string>(1000);
    cache.set('a', 'hello');
    vi.advanceTimersByTime(999);
    expect(cache.hasFresh('a')).toBe(true);
    vi.advanceTimersByTime(2);
    expect(cache.hasFresh('a')).toBe(false);
    // Stale-while-revalidate: the value is still readable, just not fresh.
    expect(cache.get('a')).toEqual({ value: 'hello', fresh: false });
  });

  it('returns undefined for unknown keys', () => {
    const cache = new TtlCache<string>();
    expect(cache.get('nope')).toBeUndefined();
    expect(cache.hasFresh('nope')).toBe(false);
  });

  it('invalidates and clears entries', () => {
    const cache = new TtlCache<string>();
    cache.set('a', '1');
    cache.set('b', '2');
    cache.invalidate('a');
    expect(cache.hasFresh('a')).toBe(false);
    expect(cache.hasFresh('b')).toBe(true);
    cache.clear();
    expect(cache.size).toBe(0);
    expect(cache.hasFresh('b')).toBe(false);
  });

  it('evicts the oldest entry when over capacity', () => {
    const cache = new TtlCache<string>(1000, 2);
    cache.set('a', '1');
    cache.set('b', '2');
    cache.set('c', '3');
    expect(cache.size).toBe(2);
    expect(cache.hasFresh('a')).toBe(false); // evicted
    expect(cache.hasFresh('b')).toBe(true);
    expect(cache.hasFresh('c')).toBe(true);
  });
});

describe('TtlCache metrics', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('counts hits, misses, and expired reads per key', () => {
    const cache = new TtlCache<string>(1000);
    cache.set('a', 'hello');

    cache.get('a'); // hit
    cache.get('missing'); // miss

    vi.advanceTimersByTime(1001);
    cache.get('a'); // expired read

    const { perKey, totals } = cache.metrics();
    expect(perKey.get('a')).toEqual({ hits: 1, misses: 0, expiries: 1, sets: 1, evictions: 0 });
    expect(perKey.get('missing')).toEqual({ hits: 0, misses: 1, expiries: 0, sets: 0, evictions: 0 });
    expect(totals).toMatchObject({ hits: 1, misses: 1, expiries: 1, sets: 1, evictions: 0, reads: 3 });
    expect(totals.hitRate).toBeCloseTo(1 / 3);
  });

  it('reports hitRate null before any read and 100% on all hits', () => {
    const cache = new TtlCache<string>(1000);
    expect(cache.metrics().totals.hitRate).toBeNull();
    cache.set('a', 'x');
    cache.get('a');
    cache.get('a');
    expect(cache.metrics().totals.hitRate).toBe(1);
  });

  it('counts capacity evictions for the dropped key', () => {
    const cache = new TtlCache<string>(1000, 2);
    cache.set('a', '1');
    cache.set('b', '2');
    cache.set('c', '3'); // evicts 'a'
    const { perKey } = cache.metrics();
    expect(perKey.get('a')?.evictions).toBe(1);
    expect(perKey.get('c')?.evictions).toBe(0);
  });

  it('hasFresh peeks without double-counting reads', () => {
    const cache = new TtlCache<string>(1000);
    cache.set('a', 'x');
    cache.hasFresh('a');
    cache.hasFresh('a');
    const { totals } = cache.metrics();
    expect(totals.reads).toBe(0);
    expect(totals.hitRate).toBeNull();
  });

  it('clear resets metrics history', () => {
    const cache = new TtlCache<string>(1000);
    cache.set('a', 'x');
    cache.get('a');
    cache.clear();
    const { perKey, totals } = cache.metrics();
    expect(perKey.size).toBe(0);
    expect(totals.reads).toBe(0);
  });

  it('invalidate drops the entry but keeps metrics history', () => {
    const cache = new TtlCache<string>(1000);
    cache.set('a', 'x');
    cache.get('a');
    cache.invalidate('a');
    expect(cache.get('a')).toBeUndefined(); // miss
    expect(cache.metrics().perKey.get('a')?.hits).toBe(1);
    expect(cache.metrics().perKey.get('a')?.misses).toBe(1);
  });
});

describe('TtlCache sessionStorage persistence', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('hydrates a fresh entry written by an earlier instance', () => {
    const { api } = memStorage();
    const writer = new TtlCache<string>(1000, 200, api);
    writer.set('a', 'hello');

    // A brand-new instance (e.g. after an in-tab navigation) reads the
    // same storage and serves the query from the snapshot.
    const reader = new TtlCache<string>(1000, 200, api);
    expect(reader.get('a')).toEqual({ value: 'hello', fresh: true });
    expect(reader.hasFresh('a')).toBe(true);
  });

  it('splits persistence per partition via the storage-key resolver', () => {
    const { api, store } = memStorage();
    // Mirrors the real resolver: card keys embed workspace at index 2.
    const partition = (entryKey: string) => {
      const ws = entryKey.split(':')[2]!;
      return `oz-part-${ws}`;
    };
    const writer = new TtlCache<string>(1000, 200, api, 'oz-part', 1, partition);
    writer.set('card:revenue:retail:daily', 'r1');
    writer.set('card:revenue:restaurant:daily', 'd1');

    // Two separate snapshots exist under the resolver's keys.
    expect(store.has('oz-part-retail')).toBe(true);
    expect(store.has('oz-part-restaurant')).toBe(true);
    // The base key itself is never written when a resolver is present.
    expect(store.has('oz-part')).toBe(false);

    // A fresh instance hydrates both partitions back into one cache.
    const reader = new TtlCache<string>(1000, 200, api, 'oz-part', 1, partition);
    expect(reader.get('card:revenue:retail:daily')).toEqual({ value: 'r1', fresh: true });
    expect(reader.get('card:revenue:restaurant:daily')).toEqual({ value: 'd1', fresh: true });

    // Invalidating the last retail entry prunes the retail partition.
    writer.invalidate('card:revenue:retail:daily');
    expect(store.has('oz-part-retail')).toBe(false);
    expect(store.has('oz-part-restaurant')).toBe(true);

    // clear removes every partition.
    writer.clear();
    expect(store.has('oz-part-restaurant')).toBe(false);
  });

  it('does not re-enumerate storage on every write — partitions are tracked in memory', () => {
    const { store, api } = memStorage();
    let enumerations = 0;
    const spyApi: CachePersistence = {
      getItem: api.getItem,
      setItem: api.setItem,
      removeItem: api.removeItem,
      keys: () => {
        enumerations += 1;
        return [...store.keys()];
      },
    };
    const partition = (entryKey: string) => `oz-spy-${entryKey.split(':')[2]!}`;

    // Construction enumerates once (hydration discovers existing snapshots).
    const cache = new TtlCache<string>(1000, 200, spyApi, 'oz-spy', 1, partition);
    expect(enumerations).toBe(1);

    // Every subsequent write must prune against the in-memory set — no
    // more enumeration calls.
    cache.set('card:revenue:retail:daily', 'r1');
    cache.set('card:revenue:restaurant:daily', 'd1');
    cache.invalidate('card:revenue:retail:daily');
    expect(enumerations).toBe(1);

    // Pruning still works without enumeration — the empty retail
    // partition was dropped and the restaurant snapshot remains.
    expect(store.has('oz-spy-retail')).toBe(false);
    expect(store.has('oz-spy-restaurant')).toBe(true);

    // A later write to the pruned partition recreates it — still with no
    // extra enumeration.
    cache.set('card:revenue:retail:weekly', 'r2');
    expect(store.has('oz-spy-retail')).toBe(true);
    expect(enumerations).toBe(1);
  });

  it('reports how many entries were restored and from how many partitions', () => {
    const { api } = memStorage();
    const partition = (entryKey: string) => `oz-part-${entryKey.split(':')[2]!}`;
    const writer = new TtlCache<string>(1000, 200, api, 'oz-part', 1, partition);
    writer.set('card:revenue:retail:daily', 'r1');
    writer.set('card:revenue:retail:weekly', 'r2');
    writer.set('card:revenue:restaurant:daily', 'd1');

    const reader = new TtlCache<string>(1000, 200, api, 'oz-part', 1, partition);
    const hyd = reader.hydration();
    expect(hyd.restored).toBe(3);
    expect(hyd.partitions).toBe(2); // retail + restaurant
    expect(hyd.durationMs).not.toBeNull();
    expect(hyd.durationMs!).toBeGreaterThanOrEqual(0);
  });

  it('reports zero restored entries when storage is cold or mismatched', () => {
    const { api } = memStorage();
    // Cold storage: nothing written yet.
    const cold = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 1);
    expect(cold.hydration()).toEqual({ restored: 0, partitions: 0, durationMs: expect.any(Number) });

    // Version mismatch: the snapshot is discarded, so nothing restores.
    const writer = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 1);
    writer.set('a', '1');
    const mismatched = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 2);
    expect(mismatched.hydration().restored).toBe(0);
    expect(mismatched.size).toBe(0);
  });

  it('keeps the original TTL: expired snapshots hydrate as stale, not fresh', () => {
    const { api } = memStorage();
    const writer = new TtlCache<string>(1000, 200, api);
    writer.set('a', 'hello');

    vi.advanceTimersByTime(1001);
    const reader = new TtlCache<string>(1000, 200, api);
    // Value still readable (stale-while-revalidate), but not fresh — a
    // back-navigation past the TTL must refetch.
    expect(reader.get('a')).toEqual({ value: 'hello', fresh: false });
    expect(reader.hasFresh('a')).toBe(false);
  });

  it('discards snapshots stamped with a different version', () => {
    const { api } = memStorage();
    const writer = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 1);
    writer.set('a', 'old');

    // Same storage key, newer schema version → old cache is dropped.
    const reader = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 2);
    expect(reader.get('a')).toBeUndefined();
    expect(reader.size).toBe(0);
  });

  it('ignores corrupt JSON instead of crashing', () => {
    const { api, store } = memStorage();
    store.set('oz-test-cache', 'not-json{{{');
    const cache = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 1);
    expect(cache.size).toBe(0);
    expect(cache.get('a')).toBeUndefined();
  });

  it('persists set and invalidate, and removes the key on clear', () => {
    const { api, store } = memStorage();
    const cache = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 1);
    cache.set('a', '1');
    expect(store.has('oz-test-cache')).toBe(true);

    cache.invalidate('a');
    const reader = new TtlCache<string>(1000, 200, api, 'oz-test-cache', 1);
    expect(reader.get('a')).toBeUndefined();

    cache.set('b', '2');
    cache.clear();
    expect(store.has('oz-test-cache')).toBe(false);
  });

  it('degrades gracefully when storage writes fail (quota)', () => {
    const failing: CachePersistence = {
      getItem: () => null,
      setItem: () => { throw new Error('QuotaExceededError'); },
      removeItem: () => { throw new Error('QuotaExceededError'); },
    };
    const cache = new TtlCache<string>(1000, 200, failing, 'oz-test-cache', 1);
    expect(() => cache.set('a', '1')).not.toThrow();
    expect(cache.get('a')).toEqual({ value: '1', fresh: true });
    expect(() => cache.clear()).not.toThrow();
  });

  it('degrades gracefully when storage reads fail (private mode)', () => {
    const failing: CachePersistence = {
      getItem: () => { throw new Error('SecurityError'); },
      setItem: () => { throw new Error('SecurityError'); },
      removeItem: () => { throw new Error('SecurityError'); },
    };
    expect(() => new TtlCache<string>(1000, 200, failing)).not.toThrow();
    const cache = new TtlCache<string>(1000, 200, failing);
    cache.set('a', '1');
    expect(cache.get('a')).toEqual({ value: '1', fresh: true });
  });

  it('shared analytics cache persists per-workspace snapshots', () => {
    clearAnalyticsCache();
    const retailKey = `${ANALYTICS_CACHE_STORAGE_KEY}-retail`;
    const restaurantKey = `${ANALYTICS_CACHE_STORAGE_KEY}-restaurant`;
    expect(sessionStorage.getItem(retailKey)).toBeNull();
    expect(sessionStorage.getItem(restaurantKey)).toBeNull();

    analyticsDataCache.set(cardQueryKey('revenue', 'retail', 'daily'), { ok: true });
    analyticsDataCache.set(cardQueryKey('revenue', 'restaurant', 'daily'), { ok: 2 });

    // Each workspace lands in its own partition, never the shared base key.
    expect(sessionStorage.getItem(ANALYTICS_CACHE_STORAGE_KEY)).toBeNull();
    const retailSnap = JSON.parse(sessionStorage.getItem(retailKey)!) as {
      version: number; entries: Array<[string, { value: unknown }]>;
    };
    const restaurantSnap = JSON.parse(sessionStorage.getItem(restaurantKey)!) as {
      version: number; entries: Array<[string, { value: unknown }]>;
    };
    expect(retailSnap.version).toBe(ANALYTICS_CACHE_VERSION);
    expect(retailSnap.entries.map(([k]) => k)).toContain('card:revenue:retail:daily::');
    expect(restaurantSnap.entries.map(([k]) => k)).toContain('card:revenue:restaurant:daily::');
    // Partitions stay separate: retail's snapshot never contains restaurant rows.
    expect(retailSnap.entries.some(([k]) => k.includes('restaurant'))).toBe(false);
    expect(restaurantSnap.entries.some(([k]) => k.includes('retail'))).toBe(false);

    // clearAnalyticsCache wipes both memory and every partition.
    clearAnalyticsCache();
    expect(sessionStorage.getItem(retailKey)).toBeNull();
    expect(sessionStorage.getItem(restaurantKey)).toBeNull();
    expect(analyticsDataCache.size).toBe(0);
  });
});

describe('analytics query keys', () => {
  it('builds canonical, stable keys', () => {
    expect(analyticsQueryKey('retail', 'daily', '2026-08-01', '2026-08-12'))
      .toBe('query:retail:daily:2026-08-01:2026-08-12');
    expect(cardQueryKey('revenue', 'retail', 'daily')).toBe('card:revenue:retail:daily::');
    expect(cardQueryKey('revenue', 'retail', 'daily', '2026-08-01', '2026-08-12'))
      .toBe('card:revenue:retail:daily:2026-08-01:2026-08-12');
  });

  it('distinguishes queries that differ in any dimension', () => {
    const base = analyticsQueryKey('retail', 'daily', '2026-08-01', '2026-08-12');
    expect(analyticsQueryKey('restaurant', 'daily', '2026-08-01', '2026-08-12')).not.toBe(base);
    expect(analyticsQueryKey('retail', 'weekly', '2026-08-01', '2026-08-12')).not.toBe(base);
    expect(analyticsQueryKey('retail', 'daily', '2026-08-02', '2026-08-12')).not.toBe(base);
    expect(analyticsQueryKey('retail', 'daily', '2026-08-01', '2026-08-13')).not.toBe(base);
    expect(cardQueryKey('revenue', 'retail', 'daily')).not.toBe(cardQueryKey('revenue', 'retail', 'weekly'));
    expect(cardQueryKey('revenue', 'retail', 'daily', '2026-08-01', '2026-08-12'))
      .not.toBe(cardQueryKey('revenue', 'retail', 'daily', '2026-08-02', '2026-08-12'));
  });
});

describe('shared analytics cache', () => {
  it('is a real singleton with a clear helper', () => {
    analyticsDataCache.set('k', { anything: 1 });
    expect(analyticsDataCache.size).toBeGreaterThan(0);
    clearAnalyticsCache();
    expect(analyticsDataCache.size).toBe(0);
    expect(analyticsDataCache.hasFresh('k')).toBe(false);
  });
});
