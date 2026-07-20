import { useState, useEffect, useCallback } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { getUserPreferences, setUserPreferences } from '@/api/settings';

export type KdsLayout = 'kanban' | 'focus' | 'metro';

export interface KdsPreferences {
  layout: KdsLayout;
  showOrderId: boolean;
  showTableNumber: boolean;
  /** Kitchen zone filter (empty string = no zone / all orders). */
  kdsZone: string;
}

const DEFAULTS: KdsPreferences = {
  layout: 'kanban',
  showOrderId: true,
  showTableNumber: true,
  kdsZone: '',
};

const STORAGE_KEY_PREFIX = 'oz-kds-prefs-';

/** Read KDS preferences from localStorage, or null if missing/invalid. */
function readLocalPrefs(userId: string): KdsPreferences | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_PREFIX + userId);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<KdsPreferences>;
    // Validate — ensure all required fields are present.
    if (!parsed.layout || !['kanban', 'focus', 'metro'].includes(parsed.layout)) return null;
    return { ...DEFAULTS, ...parsed };
  } catch {
    return null;
  }
}

/** Write KDS preferences to localStorage. */
function writeLocalPrefs(userId: string, prefs: KdsPreferences): void {
  try {
    localStorage.setItem(STORAGE_KEY_PREFIX + userId, JSON.stringify(prefs));
  } catch {
    // localStorage may be full or unavailable
  }
}

export function useKdsPreferences(): {
  prefs: KdsPreferences;
  setLayout: (layout: KdsLayout) => void;
  setShowOrderId: (show: boolean) => void;
  setShowTableNumber: (show: boolean) => void;
  setKdsZone: (zone: string) => void;
  loading: boolean;
} {
  const { session } = useAuth();
  const userId = session?.user_id ?? '';

  // Initialize from localStorage first (instant restore), fall back to defaults.
  const [prefs, setPrefs] = useState<KdsPreferences>(
    () => (userId ? readLocalPrefs(userId) ?? DEFAULTS : DEFAULTS),
  );
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!userId) {
      setLoading(false);
      return;
    }
    // Fetch from server and merge.
    getUserPreferences(userId)
      .then((raw) => {
        const serverPrefs: KdsPreferences = {
          layout: (raw['kds_layout'] as KdsLayout) || DEFAULTS.layout,
          showOrderId: raw['kds_show_order_id'] !== 'false',
          showTableNumber: raw['kds_show_table_number'] !== 'false',
          kdsZone: raw['kds_zone'] ?? DEFAULTS.kdsZone,
        };
        setPrefs(serverPrefs);
        writeLocalPrefs(userId, serverPrefs);
      })
      .catch(() => {
        // Server unavailable — keep localStorage defaults (already set).
      })
      .finally(() => setLoading(false));
  }, [userId]);

  const persist = useCallback(
    (patch: Partial<Record<string, string>>) => {
      if (!userId) return;
      const entries = Object.entries(patch).map(([key, value]) => ({
        key,
        value: value ?? '',
      }));
      setUserPreferences(userId, entries).catch(() => {});
    },
    [userId],
  );

  const setLayout = useCallback(
    (layout: KdsLayout) => {
      setPrefs((p) => {
        const next = { ...p, layout };
        writeLocalPrefs(userId, next);
        return next;
      });
      persist({ kds_layout: layout });
    },
    [userId, persist],
  );

  const setShowOrderId = useCallback(
    (show: boolean) => {
      setPrefs((p) => {
        const next = { ...p, showOrderId: show };
        writeLocalPrefs(userId, next);
        return next;
      });
      persist({ kds_show_order_id: String(show) });
    },
    [userId, persist],
  );

  const setShowTableNumber = useCallback(
    (show: boolean) => {
      setPrefs((p) => {
        const next = { ...p, showTableNumber: show };
        writeLocalPrefs(userId, next);
        return next;
      });
      persist({ kds_show_table_number: String(show) });
    },
    [userId, persist],
  );

  const setKdsZone = useCallback(
    (zone: string) => {
      setPrefs((p) => {
        const next = { ...p, kdsZone: zone };
        writeLocalPrefs(userId, next);
        return next;
      });
      persist({ kds_zone: zone });
    },
    [userId, persist],
  );

  return { prefs, setLayout, setShowOrderId, setShowTableNumber, setKdsZone, loading };
}
