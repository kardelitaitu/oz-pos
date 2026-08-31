import { useEffect } from 'react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useCurrency } from '@/contexts/CurrencyContext';

/**
 * Bridge between the workspace session and the currency bootstrap
 * (CurrencyContext reload). CurrencyProvider sits ABOVE
 * WorkspaceProvider — it must render the login/setup screens with a
 * display currency before any session exists — so it can never observe
 * a store switch by itself. This component renders below
 * WorkspaceProvider and pushes each new session token into
 * `refresh()`, so the per-store scoped default (CUR-03) reaches every
 * `useCurrency` consumer without a page reload. Renders nothing.
 */
export default function CurrencyWorkspaceSync() {
  const { sessionToken } = useWorkspace();
  const { refresh } = useCurrency();

  useEffect(() => {
    if (!sessionToken) return;
    void refresh(sessionToken);
  }, [sessionToken, refresh]);

  return null;
}
