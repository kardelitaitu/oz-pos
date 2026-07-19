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
  const [prefs, setPrefs] = useState<KdsPreferences>(DEFAULTS);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!userId) {
      setLoading(false);
      return;
    }
    getUserPreferences(userId)
      .then((raw) => {
        setPrefs({
          layout: (raw['kds_layout'] as KdsLayout) || DEFAULTS.layout,
          showOrderId: raw['kds_show_order_id'] !== 'false',
          showTableNumber: raw['kds_show_table_number'] !== 'false',
          kdsZone: raw['kds_zone'] ?? DEFAULTS.kdsZone,
        });
      })
      .catch(() => {
        // Fall back to defaults on error
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
      setPrefs((p) => ({ ...p, layout }));
      persist({ kds_layout: layout });
    },
    [persist],
  );

  const setShowOrderId = useCallback(
    (show: boolean) => {
      setPrefs((p) => ({ ...p, showOrderId: show }));
      persist({ kds_show_order_id: String(show) });
    },
    [persist],
  );

  const setShowTableNumber = useCallback(
    (show: boolean) => {
      setPrefs((p) => ({ ...p, showTableNumber: show }));
      persist({ kds_show_table_number: String(show) });
    },
    [persist],
  );

  const setKdsZone = useCallback(
    (zone: string) => {
      setPrefs((p) => ({ ...p, kdsZone: zone }));
      persist({ kds_zone: zone });
    },
    [persist],
  );

  return { prefs, setLayout, setShowOrderId, setShowTableNumber, setKdsZone, loading };
}
