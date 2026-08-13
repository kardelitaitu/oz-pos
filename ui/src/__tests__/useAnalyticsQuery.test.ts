import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { act } from 'react';
import { renderHookInAct } from '@/test-utils/renderInAct';
import { useAnalyticsQuery, clearAnalyticsErrors } from '@/features/analytics/useAnalyticsQuery';
import { analyticsDataCache, clearAnalyticsCache, ANALYTICS_CACHE_TTL_MS } from '@/features/analytics/analytics-cache';

describe('useAnalyticsQuery', () => {
  beforeEach(() => {
    // Fake only the Date so an entry can be aged past its TTL, while real
    // timers keep React's scheduler and act() working normally.
    vi.useFakeTimers({ toFake: ['Date'] });
    clearAnalyticsCache();
    clearAnalyticsErrors();
  });

  afterEach(() => {
    vi.useRealTimers();
    clearAnalyticsCache();
    clearAnalyticsErrors();
  });

  it('surfaces a failed stale revalidation as an error instead of serving stale data forever', async () => {
    const key = 'card:test:retail:daily';
    analyticsDataCache.set(key, { stale: true });
    vi.setSystemTime(Date.now() + ANALYTICS_CACHE_TTL_MS + 1);

    let rejectFetch!: (err: unknown) => void;
    const fetcher = () => new Promise((_resolve, reject) => { rejectFetch = reject; });

    const { result } = await renderHookInAct(() => useAnalyticsQuery(key, fetcher));

    // The stale value renders immediately while the revalidation is in flight.
    expect(result.current).toEqual({ data: { stale: true }, status: 'ready', error: null });

    const boom = new Error('boom');
    await act(async () => {
      rejectFetch(boom);
    });

    expect(result.current.status).toBe('error');
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBe(boom);
  });

  it('replaces a stale value after a successful revalidation', async () => {
    const key = 'card:test:retail:daily';
    analyticsDataCache.set(key, { stale: true });
    vi.setSystemTime(Date.now() + ANALYTICS_CACHE_TTL_MS + 1);

    let resolveFetch!: (value: unknown) => void;
    const fetcher = () => new Promise((resolve) => { resolveFetch = resolve; });

    const { result } = await renderHookInAct(() => useAnalyticsQuery(key, fetcher));
    expect(result.current).toEqual({ data: { stale: true }, status: 'ready', error: null });

    await act(async () => {
      resolveFetch({ fresh: true });
    });

    expect(result.current).toEqual({ data: { fresh: true }, status: 'ready', error: null });
  });

  it('serves a fresh cache hit without invoking the fetcher', async () => {
    const key = 'card:test:retail:daily';
    analyticsDataCache.set(key, { warm: true });

    const fetcher = vi.fn(() => ({ cold: true }));
    const { result } = await renderHookInAct(() => useAnalyticsQuery(key, fetcher));

    expect(result.current).toEqual({ data: { warm: true }, status: 'ready', error: null });
    expect(fetcher).not.toHaveBeenCalled();
  });

  it('drops and refetches a cached value that fails the shape guard', async () => {
    const key = 'card:test:retail:daily';
    analyticsDataCache.set(key, { wrong: 'shape' }); // not an array

    const fetcher = vi.fn(() => ['fresh']);
    const { result } = await renderHookInAct(() =>
      useAnalyticsQuery<string[]>(key, fetcher, true, Array.isArray),
    );

    // The mismatched value is invalidated and refetched as a miss.
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(result.current).toEqual({ data: ['fresh'], status: 'ready', error: null });
    // The cache now holds the refetched value, never the bad one.
    expect(analyticsDataCache.get(key)).toEqual({ value: ['fresh'], fresh: true });
  });

  it('serves a cached value that passes the shape guard without refetching', async () => {
    const key = 'card:test:retail:daily';
    analyticsDataCache.set(key, ['warm']);

    const fetcher = vi.fn(() => ['cold']);
    const { result } = await renderHookInAct(() =>
      useAnalyticsQuery<string[]>(key, fetcher, true, Array.isArray),
    );

    expect(fetcher).not.toHaveBeenCalled();
    expect(result.current).toEqual({ data: ['warm'], status: 'ready', error: null });
  });
});
