import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';
import { getSubscriptionCapabilities, type SubscriptionCapabilities } from '@/api/subscription';

interface SubscriptionContextValue {
  /** The tenant's tier capabilities, or `null` while loading / on failure. */
  caps: SubscriptionCapabilities | null;
  /** True until the first capabilities read settles. */
  loading: boolean;
  /** Re-fetch capabilities (e.g. after license activation/renewal). */
  refresh: () => void;
}

const SubscriptionContext = createContext<SubscriptionContextValue>({
  caps: null,
  loading: true,
  refresh: () => {},
});

/**
 * C2.2: fetches the tenant's subscription capabilities once at app start and
 * shares them with every tier-gated screen (analytics/loyalty locks, QRIS
 * gate, store/terminal/staff limits). The read is local (no network), and a
 * failure degrades to `caps: null` — gates then render open rather than
 * blocking the app.
 */
export function SubscriptionProvider({ children }: { children: ReactNode }) {
  const [caps, setCaps] = useState<SubscriptionCapabilities | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    setLoading(true);
    getSubscriptionCapabilities()
      .then(setCaps)
      .catch(() => setCaps(null))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <SubscriptionContext.Provider value={{ caps, loading, refresh }}>
      {children}
    </SubscriptionContext.Provider>
  );
}

/** Access the tenant's subscription capabilities (C2.2). */
export function useSubscription(): SubscriptionContextValue {
  return useContext(SubscriptionContext);
}
