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

describe('analytics query keys', () => {
  it('builds canonical, stable keys', () => {
    expect(analyticsQueryKey('retail', 'daily', '2026-08-01', '2026-08-12'))
      .toBe('query:retail:daily:2026-08-01:2026-08-12');
    expect(cardQueryKey('revenue', 'retail', 'daily')).toBe('card:revenue:retail:daily');
  });

  it('distinguishes queries that differ in any dimension', () => {
    const base = analyticsQueryKey('retail', 'daily', '2026-08-01', '2026-08-12');
    expect(analyticsQueryKey('restaurant', 'daily', '2026-08-01', '2026-08-12')).not.toBe(base);
    expect(analyticsQueryKey('retail', 'weekly', '2026-08-01', '2026-08-12')).not.toBe(base);
    expect(analyticsQueryKey('retail', 'daily', '2026-08-02', '2026-08-12')).not.toBe(base);
    expect(analyticsQueryKey('retail', 'daily', '2026-08-01', '2026-08-13')).not.toBe(base);
    expect(cardQueryKey('revenue', 'retail', 'daily')).not.toBe(cardQueryKey('revenue', 'retail', 'weekly'));
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
