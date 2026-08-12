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

/**
 * A small generic time-to-live cache. Reads of expired entries still
 * return the value with `fresh: false` (stale-while-revalidate) so a
 * stale result can render while a revalidation happens.
 */
export class TtlCache<T> {
  private readonly entries = new Map<string, CacheEntry<T>>();

  constructor(
    private readonly ttlMs: number = ANALYTICS_CACHE_TTL_MS,
    private readonly maxEntries: number = ANALYTICS_CACHE_MAX_ENTRIES,
  ) {}

  /** Read an entry; `undefined` when the key was never stored. */
  get(key: string): CachedValue<T> | undefined {
    const entry = this.entries.get(key);
    if (!entry) return undefined;
    return { value: entry.value, fresh: Date.now() < entry.expiresAt };
  }

  /** `true` when a fresh (unexpired) entry exists for `key`. */
  hasFresh(key: string): boolean {
    return this.get(key)?.fresh ?? false;
  }

  /** Store `value` under `key`, expiring after the TTL. */
  set(key: string, value: T): void {
    this.entries.set(key, { value, expiresAt: Date.now() + this.ttlMs });
    // Keep the cache bounded: Map preserves insertion order, so the
    // first key is the oldest — evict it.
    if (this.entries.size > this.maxEntries) {
      const oldest = this.entries.keys().next().value;
      if (oldest !== undefined) this.entries.delete(oldest);
    }
  }

  /** Drop a single entry. */
  invalidate(key: string): void {
    this.entries.delete(key);
  }

  /** Drop every entry (tests, and any future full-refresh path). */
  clear(): void {
    this.entries.clear();
  }

  /** Number of stored entries (fresh or expired). */
  get size(): number {
    return this.entries.size;
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
