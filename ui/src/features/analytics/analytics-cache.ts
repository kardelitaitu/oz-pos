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
//! The persisted snapshots are split per workspace (retail vs
//! restaurant) so each dashboard keeps its own snapshot under
//! `oz-analytics-cache-v1-<workspace>`, and hydration only ever loads
//! back what this session actually queried. We deliberately do not use
//! localStorage so stale data never survives a browser restart.

/** How long a computed analytics query stays fresh (5 minutes). */
export const ANALYTICS_CACHE_TTL_MS = 5 * 60 * 1000;

/** Upper bound on cached entries; the oldest entry is evicted beyond it. */
export const ANALYTICS_CACHE_MAX_ENTRIES = 200;

/**
 * Bump when the persisted snapshot schema or cache-key shape changes.
 * Hydration discards any snapshot whose version differs.
 */
export const ANALYTICS_CACHE_VERSION = 1;

/**
 * sessionStorage key prefix for persisted cache snapshots. Snapshots
 * live under `ANALYTICS_CACHE_STORAGE_KEY + '-' + workspace`; the base
 * string is also the prefix used to enumerate every snapshot.
 */
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
  /** Enumerate existing keys (needed for multi-snapshot persistence). */
  keys?(): string[];
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
 *
 * Pass a `storageKeyFor` resolver to split persistence into multiple
 * snapshots (e.g. one per workspace): each entry is stored under
 * `storageKeyFor(entryKey)` instead of a single shared key, hydration
 * loads every snapshot whose key starts with `storageKey`, and `clear`
 * removes them all. The in-memory cache remains a single map.
 */
export class TtlCache<T> {
  private readonly entries = new Map<string, CacheEntry<T>>();
  private readonly counters = new Map<string, CacheKeyMetrics>();

  /** Entries actually restored from storage during construction. */
  private hydratedCount = 0;
  /** Storage partitions (snapshot keys) that yielded at least one entry. */
  private readonly hydratedFrom = new Set<string>();
  /** Elapsed ms of the construction-time hydration; `null` when none ran. */
  private hydrationMs: number | null = null;

  /**
   * Partition keys this cache wrote or hydrated. Persist/clear prune
   * against this set instead of re-enumerating all of sessionStorage on
   * every write — enumeration happens once at hydration, then writes
   * keep the set current.
   */
  private readonly knownPartitions = new Set<string>();

  constructor(
    private readonly ttlMs: number = ANALYTICS_CACHE_TTL_MS,
    private readonly maxEntries: number = ANALYTICS_CACHE_MAX_ENTRIES,
    private readonly persistence: CachePersistence | null = null,
    private readonly storageKey: string = ANALYTICS_CACHE_STORAGE_KEY,
    private readonly version: number = ANALYTICS_CACHE_VERSION,
    private readonly storageKeyFor: ((entryKey: string) => string) | null = null,
  ) {
    if (persistence) this.hydrate();
  }

  /** Resolve an entry's storage key (base key when no resolver given). */
  private resolveStorageKey(entryKey: string): string {
    return this.storageKeyFor ? this.storageKeyFor(entryKey) : this.storageKey;
  }

  private bump(key: string, field: keyof CacheKeyMetrics): void {
    const m = this.counters.get(key) ?? emptyKeyMetrics();
    m[field] += 1;
    this.counters.set(key, m);
  }

  /** Storage keys that hold snapshots for this cache (single or many). */
  private snapshotKeys(): string[] {
    if (!this.persistence) return [];
    const keys = [this.storageKey];
    try {
      // If the resolver splits entries across multiple keys, hydration
      // and clear must reach every snapshot under the storageKey prefix.
      for (const k of this.persistence.keys?.() ?? []) {
        if (k.startsWith(this.storageKey)) keys.push(k);
      }
    } catch {
      // enumeration unavailable — fall back to the base key only
    }
    return keys;
  }

  /** Load a version-matching snapshot from storage; bad data is ignored. */
  private hydrate(): void {
    const started = performance.now();
    for (const key of this.snapshotKeys()) {
      this.knownPartitions.add(key);
      this.hydrateFrom(key);
    }
    this.hydrationMs = Math.round((performance.now() - started) * 10) / 10;
  }

  private hydrateFrom(storageKey: string): void {
    if (!this.persistence) return;
    let raw: string | null = null;
    try {
      raw = this.persistence.getItem(storageKey);
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
      this.hydratedCount += 1;
      this.hydratedFrom.add(storageKey);
    }
  }

  /** Write the current entries to storage; failures degrade silently. */
  private persist(): void {
    if (!this.persistence) return;
    // Group entries by their resolved storage key so each snapshot only
    // contains that partition's queries.
    const groups = new Map<string, Array<[string, CacheEntry<T>]>>();
    for (const [key, entry] of this.entries) {
      const target = this.resolveStorageKey(key);
      const group = groups.get(target) ?? [];
      group.push([key, entry]);
      groups.set(target, group);
    }
    const written = new Set<string>(groups.keys());
    if (this.storageKeyFor === null) {
      // Single-key cache: always (re)write the base key, even when empty,
      // so invalidating the last entry leaves a clean empty snapshot
      // instead of a stale one from an earlier write.
      written.add(this.storageKey);
    }
    for (const target of written) {
      const snap: PersistedSnapshot = {
        version: this.version,
        savedAt: Date.now(),
        entries: groups.get(target) ?? [],
      };
      try {
        this.persistence.setItem(target, JSON.stringify(snap));
      } catch {
        // Quota exceeded / storage unavailable — the in-memory cache is
        // unaffected; persistence simply lapses for this write.
      }
    }
    // Track every partition just written so future prunes don't need to
    // enumerate all of sessionStorage.
    for (const target of written) {
      this.knownPartitions.add(target);
    }
    // Drop partitions that no longer hold entries so a workspace whose
    // last query was invalidated doesn't leave an orphaned snapshot.
    for (const key of [...this.knownPartitions]) {
      if (written.has(key)) continue;
      try {
        this.persistence.removeItem(key);
        this.knownPartitions.delete(key);
      } catch {
        // storage unavailable — stale snapshot remains, harmless
      }
    }
  }

  /** Read an entry; `undefined` when the key was never stored. */
  get(key: string): CachedValue<T> | undefined {
    const entry = this.entries.get(key);
    if (!entry) {
      this.bump(key, 'misses');
      return undefined;
    }
    // Capture the clock once so the hit/expiry metric and the returned
    // `fresh` flag always agree, even exactly at the TTL boundary.
    const now = Date.now();
    if (now < entry.expiresAt) {
      this.bump(key, 'hits');
    } else {
      this.bump(key, 'expiries');
    }
    return { value: entry.value, fresh: now < entry.expiresAt };
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

  /** Drop every entry, reset all metrics, and remove all snapshots. */
  clear(): void {
    this.entries.clear();
    this.counters.clear();
    if (!this.persistence) return;
    for (const key of [...this.knownPartitions]) {
      try {
        this.persistence.removeItem(key);
      } catch {
        // storage unavailable — in-memory state is already cleared
      }
    }
    this.knownPartitions.clear();
  }

  /** Number of stored entries (fresh or expired). */
  get size(): number {
    return this.entries.size;
  }

  /**
   * How much work construction-time hydration did: the number of
   * entries restored from sessionStorage, from how many snapshot
   * partitions, and the elapsed time. All zeros/`null` when the cache
   * was created without persistence or found nothing to restore.
   */
  hydration(): { restored: number; partitions: number; durationMs: number | null } {
    return {
      restored: this.hydratedCount,
      partitions: this.hydratedFrom.size,
      durationMs: this.hydrationMs,
    };
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

/**
 * sessionStorage adapter. A wrapper (rather than the raw Storage) is
 * required because some environments — notably jsdom — hand out a fresh
 * `Storage` object on every property access, so adding a `keys()` method
 * to it would not survive; the wrapper also keeps the `keys()`
 * enumeration stable across reads.
 */
function sessionStorageOrNull(): CachePersistence | null {
  try {
    const storage = typeof sessionStorage !== 'undefined' ? sessionStorage : null;
    if (!storage) return null;
    return {
      getItem: (k: string) => storage.getItem(k),
      setItem: (k: string, v: string) => storage.setItem(k, v),
      removeItem: (k: string) => storage.removeItem(k),
      keys: () => {
        try {
          // Storage's own enumeration API (key(i)/length) — `Object.keys`
          // does not expose jsdom's Storage entries.
          return Array.from({ length: storage.length }, (_, i) => storage.key(i) ?? '').filter(Boolean);
        } catch {
          return [];
        }
      },
    };
  } catch {
    return null; // some browsers throw on property access in sandboxes
  }
}

/**
 * Persisted snapshot partition for a query key: `retail` / `restaurant`
 * (both query keys embed the workspace), falling back to `shared` for
 * anything unusual.
 */
function cachePartition(entryKey: string): string {
  const parts = entryKey.split(':');
  // card:revenue:retail:daily:... | query:retail:daily:...
  const workspace = parts[0] === 'card' ? parts[2] : parts[1];
  return workspace && (workspace === 'retail' || workspace === 'restaurant') ? workspace : 'shared';
}

/**
 * Shared cache used by the analytics page and its cards. Persisted to
 * sessionStorage as per-workspace snapshots so in-tab navigation back
 * to the page skips refetches for queries still inside their TTL
 * window, and retail/restaurant dashboards never share a snapshot.
 */
export const analyticsDataCache = new TtlCache<unknown>(
  ANALYTICS_CACHE_TTL_MS,
  ANALYTICS_CACHE_MAX_ENTRIES,
  sessionStorageOrNull(),
  ANALYTICS_CACHE_STORAGE_KEY,
  ANALYTICS_CACHE_VERSION,
  (entryKey) => `${ANALYTICS_CACHE_STORAGE_KEY}-${cachePartition(entryKey)}`,
);

// Log hydration cost once, at module load, when a warm session was
// restored. This is the one place we can see how much of the dashboard
// was served from sessionStorage before any refetch: the count of
// restored entries, the partitions they came from, and the elapsed ms.
const hydration = analyticsDataCache.hydration();
if (hydration.restored > 0) {
  console.info(
    `[analytics-cache] hydrated ${hydration.restored} entries from ` +
      `${hydration.partitions} partition(s) in ${hydration.durationMs}ms`,
  );
}

/** Wipe the shared cache (memory + persisted snapshot) — used by tests. */
export function clearAnalyticsCache(): void {
  analyticsDataCache.clear();
}
