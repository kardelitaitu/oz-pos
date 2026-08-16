import { useState, useEffect, useCallback } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { getUserPreferencesScoped, setUserPreferencesScoped } from '@/api/settings';

/**
 * Retail grid column identifiers (ADR #36 D4). The Cost column is
 * deliberately NOT here — cost is only ever shown in edit surfaces.
 */
export const RETAIL_COLUMNS = [
  'sku',
  'barcode',
  'category',
  'brand',
  'name',
  'rack',
  'stock',
  'price',
  'notes',
] as const;

export type RetailColumn = (typeof RETAIL_COLUMNS)[number];

/** Columns visible by default: the classic retail grid keeps its core. */
export const RETAIL_COLUMN_DEFAULTS: readonly RetailColumn[] = [
  'sku',
  'stock',
  'name',
  'price',
];

export interface RetailColumnPrefs {
  /** Column ids to render, in display order (subset of RETAIL_COLUMNS). */
  visibleColumns: RetailColumn[];
  /** Hide retired (inactive) products from the grid. */
  hideInactive: boolean;
}

const DEFAULTS: RetailColumnPrefs = {
  visibleColumns: [...RETAIL_COLUMN_DEFAULTS],
  hideInactive: false,
};

const STORAGE_KEY_PREFIX = 'oz-retail-cols-';

/** Read retail column prefs from localStorage, or null if missing/invalid. */
function readLocalPrefs(userId: string): RetailColumnPrefs | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_PREFIX + userId);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<RetailColumnPrefs>;
    if (!Array.isArray(parsed.visibleColumns)) return null;
    const valid = parsed.visibleColumns.filter((c): c is RetailColumn =>
      (RETAIL_COLUMNS as readonly string[]).includes(c),
    );
    return {
      visibleColumns: valid.length > 0 ? valid : [...RETAIL_COLUMN_DEFAULTS],
      hideInactive: parsed.hideInactive === true,
    };
  } catch {
    return null;
  }
}

/** Write retail column prefs to localStorage. */
function writeLocalPrefs(userId: string, prefs: RetailColumnPrefs): void {
  try {
    localStorage.setItem(STORAGE_KEY_PREFIX + userId, JSON.stringify(prefs));
  } catch {
    // localStorage may be full or unavailable
  }
}

/**
 * Per-user retail grid column visibility + hide-inactive toggle.
 *
 * Same persistence pattern as `useKdsPreferences` (ADR #36 D4): restore
 * instantly from localStorage, merge with the server copy via the existing
 * `user_preferences` API, write-through on every change.
 */
export function useRetailColumnPrefs(): {
  prefs: RetailColumnPrefs;
  toggleColumn: (col: RetailColumn) => void;
  setHideInactive: (hide: boolean) => void;
  loading: boolean;
} {
  const { session } = useAuth();
  const { sessionToken } = useWorkspace();
  const userId = session?.user_id ?? '';

  const [prefs, setPrefs] = useState<RetailColumnPrefs>(
    () => (userId ? readLocalPrefs(userId) ?? DEFAULTS : DEFAULTS),
  );
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!userId || !sessionToken) {
      setLoading(false);
      return;
    }
    getUserPreferencesScoped(sessionToken)
      .then((raw) => {
        const serverPrefs: RetailColumnPrefs = {
          visibleColumns: parseColumns(raw['retail_visible_columns']),
          hideInactive: raw['retail_hide_inactive'] === 'true',
        };
        setPrefs(serverPrefs);
        writeLocalPrefs(userId, serverPrefs);
      })
      .catch(() => {
        // Server unavailable — keep localStorage defaults (already set).
      })
      .finally(() => setLoading(false));
  }, [userId, sessionToken]);

  const persist = useCallback(
    (patch: Partial<Record<string, string>>) => {
      if (!userId || !sessionToken) return;
      const entries = Object.entries(patch).map(([key, value]) => ({
        key,
        value: value ?? '',
      }));
      setUserPreferencesScoped(sessionToken, entries).catch(() => {
        // Server persistence is best-effort; localStorage already saved.
      });
    },
    [userId, sessionToken],
  );

  const toggleColumn = useCallback(
    (col: RetailColumn) => {
      setPrefs((p) => {
        const visible = p.visibleColumns.includes(col);
        const next: RetailColumnPrefs = {
          ...p,
          visibleColumns: visible
            ? p.visibleColumns.filter((c) => c !== col)
            : [...p.visibleColumns, col],
        };
        writeLocalPrefs(userId, next);
        // Persist the whole column set (single pref key).
        persist({ retail_visible_columns: JSON.stringify(next.visibleColumns) });
        return next;
      });
    },
    [userId, persist],
  );

  const setHideInactive = useCallback(
    (hide: boolean) => {
      setPrefs((p) => {
        const next = { ...p, hideInactive: hide };
        writeLocalPrefs(userId, next);
        return next;
      });
      persist({ retail_hide_inactive: String(hide) });
    },
    [userId, persist],
  );

  return { prefs, toggleColumn, setHideInactive, loading };
}

/** Parse the stored JSON column array, falling back to defaults. */
function parseColumns(raw: string | undefined): RetailColumn[] {
  if (!raw) return [...RETAIL_COLUMN_DEFAULTS];
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [...RETAIL_COLUMN_DEFAULTS];
    const valid = parsed.filter(
      (c): c is RetailColumn =>
        typeof c === 'string' &&
        (RETAIL_COLUMNS as readonly string[]).includes(c),
    );
    return valid.length > 0 ? valid : [...RETAIL_COLUMN_DEFAULTS];
  } catch {
    return [...RETAIL_COLUMN_DEFAULTS];
  }
}
