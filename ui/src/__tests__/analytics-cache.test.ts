import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import {
  TtlCache,
  analyticsDataCache,
  analyticsQueryKey,
  cardQueryKey,
  clearAnalyticsCache,
} from '@/features/analytics/analytics-cache';

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
