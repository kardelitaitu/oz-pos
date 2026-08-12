//! Client-side analytics data cache (TTL).
//!
//! The analytics page recomputes its dashboard whenever the workspace,
//! granularity, or custom date range changes. `TtlCache` stores the
//! result of each query under a canonical key
//! (`workspace:granularity:range`) so an identical query made inside the
//! TTL window is served instantly instead of refetching/recomputing.
//! Entries expire after the TTL and are then treated as misses (fresh
//! recalc skeleton + recompute).
//!
//! The cache is in-memory and page-lifetime; it deliberately does not
//! persist to localStorage so stale data never survives a restart.

/** How long a computed analytics query stays fresh (5 minutes). */
export const ANALYTICS_CACHE_TTL_MS = 5 * 60 * 1000;

/** Upper bound on cached entries; the oldest entry is evicted beyond it. */
export const ANALYTICS_CACHE_MAX_ENTRIES = 200;

export interface CacheEntry<T> {
  value: T;
  /** Absolute epoch ms at which the entry expires. */
  expiresAt: number;
}

export interface CachedValue<T> {
  value: T;
  /** `true` when read inside the TTL window. */
  fresh: boolean;
}

/** Per-key counters for the debug metrics readout. */
export interface CacheKeyMetrics {
  /** Reads that found a fresh (unexpired) entry. */
  hits: number;
  /** Reads that found no entry at all. */
  misses: number;
  /** Reads that found an entry already past its TTL (stale read). */
  expiries: number;
  /** Writes that stored a value under the key. */
  sets: number;
  /** Capacity evictions that dropped this key. */
  evictions: number;
}

/** Aggregate counters across all keys. */
export interface CacheTotals extends CacheKeyMetrics {
  /** Total reads (hits + misses + expiries). */
  reads: number;
  /** Fraction of reads served fresh, 0–1 (or `null` before any read). */
  hitRate: number | null;
}

/** Immutable snapshot returned by `TtlCache.metrics()`. */
export interface CacheMetricsSnapshot {
  perKey: ReadonlyMap<string, CacheKeyMetrics>;
  totals: CacheTotals;
}

function emptyKeyMetrics(): CacheKeyMetrics {
  return { hits: 0, misses: 0, expiries: 0, sets: 0, evictions: 0 };
}

/**
 * A small generic time-to-live cache. Reads of expired entries still
 * return the value with `fresh: false` (stale-while-revalidate) so a
 * stale result can render while a revalidation happens.
 *
 * Every `get` is metered: hits, misses, and expired reads are counted
 * per key (see {@link CacheKeyMetrics}) so the analytics status bar can
 * surface a debug hit/miss/expiry readout per query key.
 */
export class TtlCache<T> {
  private readonly entries = new Map<string, CacheEntry<T>>();
  private readonly counters = new Map<string, CacheKeyMetrics>();

  constructor(
    private readonly ttlMs: number = ANALYTICS_CACHE_TTL_MS,
    private readonly maxEntries: number = ANALYTICS_CACHE_MAX_ENTRIES,
  ) {}

  private bump(key: string, field: keyof CacheKeyMetrics): void {
    const m = this.counters.get(key) ?? emptyKeyMetrics();
    m[field] += 1;
    this.counters.set(key, m);
  }

  /** Read an entry; `undefined` when the key was never stored. */
  get(key: string): CachedValue<T> | undefined {
    const entry = this.entries.get(key);
    if (!entry) {
      this.bump(key, 'misses');
      return undefined;
    }
    if (Date.now() < entry.expiresAt) {
      this.bump(key, 'hits');
    } else {
      this.bump(key, 'expiries');
    }
    return { value: entry.value, fresh: Date.now() < entry.expiresAt };
  }

  /** `true` when a fresh (unexpired) entry exists for `key`. */
  hasFresh(key: string): boolean {
    // Peek without metering: `hasFresh` is a skeleton-suppression probe
    // that runs alongside a real `get`, so counting it would double-count.
    const entry = this.entries.get(key);
    return entry !== undefined && Date.now() < entry.expiresAt;
  }

  /** Store `value` under `key`, expiring after the TTL. */
  set(key: string, value: T): void {
    this.entries.set(key, { value, expiresAt: Date.now() + this.ttlMs });
    this.bump(key, 'sets');
    // Keep the cache bounded: Map preserves insertion order, so the
    // first key is the oldest — evict it.
    if (this.entries.size > this.maxEntries) {
      const oldest = this.entries.keys().next().value;
      if (oldest !== undefined) {
        this.entries.delete(oldest);
        this.bump(oldest, 'evictions');
      }
    }
  }

  /** Drop a single entry (metrics history is kept for the readout). */
  invalidate(key: string): void {
    this.entries.delete(key);
  }

  /** Drop every entry and reset all metrics (tests, full refresh). */
  clear(): void {
    this.entries.clear();
    this.counters.clear();
  }

  /** Number of stored entries (fresh or expired). */
  get size(): number {
    return this.entries.size;
  }

  /**
   * Snapshot of per-key counters plus totals. Used by the analytics
   * status-bar debug readout and by tests.
   */
  metrics(): CacheMetricsSnapshot {
    const totals: CacheTotals = { ...emptyKeyMetrics(), reads: 0, hitRate: null };
    for (const m of this.counters.values()) {
      totals.hits += m.hits;
      totals.misses += m.misses;
      totals.expiries += m.expiries;
      totals.sets += m.sets;
      totals.evictions += m.evictions;
    }
    totals.reads = totals.hits + totals.misses + totals.expiries;
    totals.hitRate = totals.reads > 0 ? totals.hits / totals.reads : null;
    return { perKey: new Map(this.counters), totals };
  }
}

// ── Analytics query keys ─────────────────────────────────────────────

/** Canonical cache key for a full dashboard query (drives skeleton suppression). */
export function analyticsQueryKey(workspace: string, granularity: string, from: string, to: string): string {
  return `query:${workspace}:${granularity}:${from}:${to}`;
}

/** Canonical cache key for one card's payload (range optional). */
export function cardQueryKey(
  cardKey: string,
  workspace: string,
  granularity: string,
  from = '',
  to = '',
): string {
  return `card:${cardKey}:${workspace}:${granularity}:${from}:${to}`;
}

/** Shared in-memory cache used by the analytics page and its cards. */
export const analyticsDataCache = new TtlCache<unknown>();

/** Wipe the shared cache — used by tests between cases. */
export function clearAnalyticsCache(): void {
  analyticsDataCache.clear();
}
