//! Client-side analytics data cache (TTL), persisted to sessionStorage.
//!
//! The analytics page recomputes its dashboard whenever the workspace,
//! granularity, or custom date range changes. `TtlCache` stores the
//! result of each query under a canonical key
//! (`workspace:granularity:range`) so an identical query made inside the
//! TTL window is served instantly instead of refetching/recomputing.
//! Entries expire after the TTL and are then treated as misses (fresh
//! recalc skeleton + recompute).
//!
//! The cache is also written through to `sessionStorage` under a
//! version-stamped key. sessionStorage survives in-tab navigation and
//! reloads (but is discarded when the tab closes), so going back to the
//! analytics page within the same session skips refetching queries that
//! are still inside their TTL window. The version stamp guards the
//! schema: caches written by an older build are discarded on hydration.
//! We deliberately do not use localStorage so stale data never survives
//! a browser restart.

/** How long a computed analytics query stays fresh (5 minutes). */
export const ANALYTICS_CACHE_TTL_MS = 5 * 60 * 1000;

/** Upper bound on cached entries; the oldest entry is evicted beyond it. */
export const ANALYTICS_CACHE_MAX_ENTRIES = 200;

/**
 * Bump when the persisted snapshot schema or cache-key shape changes.
 * Hydration discards any snapshot whose version differs.
 */
export const ANALYTICS_CACHE_VERSION = 1;

/** sessionStorage key under which the cache snapshot is stored. */
export const ANALYTICS_CACHE_STORAGE_KEY = 'oz-analytics-cache-v1';

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

/** Storage contract for persisting the cache (sessionStorage-shaped). */
export interface CachePersistence {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

/** On-disk shape of the persisted snapshot. */
interface PersistedSnapshot {
  version: number;
  /** Epoch ms when the snapshot was written (diagnostic only). */
  savedAt: number;
  entries: Array<[string, CacheEntry<unknown>]>;
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
 *
 * When constructed with a {@link CachePersistence}, every mutation is
 * written through to storage and the cache is hydrated from it on
 * construction. Hydration honors the version stamp (snapshots from a
 * different version are dropped) and keeps each entry's original
 * `expiresAt`, so a query still inside its TTL window survives an
 * in-tab navigation or reload without a refetch.
 */
export class TtlCache<T> {
  private readonly entries = new Map<string, CacheEntry<T>>();
  private readonly counters = new Map<string, CacheKeyMetrics>();

  constructor(
    private readonly ttlMs: number = ANALYTICS_CACHE_TTL_MS,
    private readonly maxEntries: number = ANALYTICS_CACHE_MAX_ENTRIES,
    private readonly persistence: CachePersistence | null = null,
    private readonly storageKey: string = ANALYTICS_CACHE_STORAGE_KEY,
    private readonly version: number = ANALYTICS_CACHE_VERSION,
  ) {
    if (persistence) this.hydrate();
  }

  private bump(key: string, field: keyof CacheKeyMetrics): void {
    const m = this.counters.get(key) ?? emptyKeyMetrics();
    m[field] += 1;
    this.counters.set(key, m);
  }

  /** Load a version-matching snapshot from storage; bad data is ignored. */
  private hydrate(): void {
    if (!this.persistence) return;
    let raw: string | null = null;
    try {
      raw = this.persistence.getItem(this.storageKey);
    } catch {
      return; // storage unavailable (private mode, quota, etc.)
    }
    if (!raw) return;

    let snap: PersistedSnapshot;
    try {
      const parsed = JSON.parse(raw) as Partial<PersistedSnapshot>;
      if (
        typeof parsed !== 'object' || parsed === null ||
        parsed.version !== this.version ||
        !Array.isArray(parsed.entries)
      ) {
        return; // different schema version or malformed — discard
      }
      snap = parsed as PersistedSnapshot;
    } catch {
      return; // corrupt JSON — discard
    }

    for (const [key, entry] of snap.entries) {
      if (this.entries.size >= this.maxEntries) break;
      if (
        typeof key !== 'string' || typeof entry !== 'object' || entry === null ||
        typeof entry.expiresAt !== 'number' || !('value' in entry)
      ) {
        continue; // skip malformed rows rather than dropping the snapshot
      }
      // Keep the original expiresAt: expired entries hydrate as stale
      // (readable with `fresh: false`), identical to in-memory behavior.
      this.entries.set(key, { value: entry.value as T, expiresAt: entry.expiresAt });
    }
  }

  /** Write the current entries to storage; failures degrade silently. */
  private persist(): void {
    if (!this.persistence) return;
    const snap: PersistedSnapshot = {
      version: this.version,
      savedAt: Date.now(),
      entries: [...this.entries.entries()],
    };
    try {
      this.persistence.setItem(this.storageKey, JSON.stringify(snap));
    } catch {
      // Quota exceeded / storage unavailable — the in-memory cache is
      // unaffected; persistence simply lapses for this write.
    }
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
    this.persist();
  }

  /** Drop a single entry (metrics history is kept for the readout). */
  invalidate(key: string): void {
    this.entries.delete(key);
    this.persist();
  }

  /** Drop every entry and reset all metrics (tests, full refresh). */
  clear(): void {
    this.entries.clear();
    this.counters.clear();
    if (this.persistence) {
      try {
        this.persistence.removeItem(this.storageKey);
      } catch {
        // storage unavailable — in-memory state is already cleared
      }
    }
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

/** sessionStorage instance when available; `null` in SSR/test envs. */
function sessionStorageOrNull(): CachePersistence | null {
  try {
    return typeof sessionStorage !== 'undefined' ? sessionStorage : null;
  } catch {
    return null; // some browsers throw on property access in sandboxes
  }
}

/**
 * Shared cache used by the analytics page and its cards. Persisted to
 * sessionStorage so in-tab navigation back to the page skips refetches
 * for queries still inside their TTL window.
 */
export const analyticsDataCache = new TtlCache<unknown>(
  ANALYTICS_CACHE_TTL_MS,
  ANALYTICS_CACHE_MAX_ENTRIES,
  sessionStorageOrNull(),
);

/** Wipe the shared cache (memory + persisted snapshot) — used by tests. */
export function clearAnalyticsCache(): void {
  analyticsDataCache.clear();
}
