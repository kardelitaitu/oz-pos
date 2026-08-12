//! Cache-first data hook for analytics queries.
//!
//! `useAnalyticsQuery(key, fetcher)` returns the query's result from the
//! shared TTL cache when one exists, and otherwise runs `fetcher` and
//! stores the result. Identical queries made within the cache TTL are
//! served instantly — switching granularity or workspace back and forth
//! does not recompute or refetch.
//!
//! `fetcher` may be synchronous (the current deterministic demo
//! generators) or return a Promise (the real IPC commands once wired);
//! async fetchers surface a `loading` status until they resolve.

import { useRef, useState } from 'react';
import { analyticsDataCache } from './analytics-cache';

export type QueryStatus = 'loading' | 'ready';

export interface AnalyticsQueryResult<T> {
  data: T | null;
  status: QueryStatus;
}

/** Read the shared cache with the right payload type. */
function readCached<T>(key: string): AnalyticsQueryResult<T> | undefined {
  const hit = analyticsDataCache.get(key);
  if (!hit) return undefined;
  return { data: hit.value as T, status: 'ready' };
}

/**
 * Cache-first query hook.
 *
 * Sync fetchers (the demo generators) resolve during render, so there
 * is never an empty flash: a miss computes once, stores the result in
 * the shared cache, and every later render reads the cache hit. The
 * write is idempotent and deterministic, which keeps it safe to run
 * during render. Async fetchers render `loading` until the promise
 * resolves, then cache the resolved value.
 */
export function useAnalyticsQuery<T>(key: string, fetcher: () => T | Promise<T>): AnalyticsQueryResult<T> {
  const [, forceRender] = useState(0);
  const inflight = useRef<string | null>(null);

  const cached = readCached<T>(key);
  if (cached) return cached;

  // A miss — but skip re-invoking the fetcher while an async request
  // for this same key is already in flight (StrictMode double-renders
  // would otherwise fire the fetch twice).
  if (inflight.current === key) {
    return { data: null, status: 'loading' };
  }

  const out = fetcher();
  if (out instanceof Promise) {
    inflight.current = key;
    out.then(
      (value) => {
        analyticsDataCache.set(key, value);
        inflight.current = null;
        forceRender((n) => n + 1); // next render reads the cache hit
      },
      () => {
        inflight.current = null;
        forceRender((n) => n + 1);
      },
    );
    return { data: null, status: 'loading' };
  }

  analyticsDataCache.set(key, out);
  return { data: out, status: 'ready' };
}
