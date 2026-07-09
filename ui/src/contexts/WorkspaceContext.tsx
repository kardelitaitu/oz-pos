import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import { listWorkspaces, listWorkspaceScreens, type WorkspaceDto } from '@/api/workspaces';
import { useAuth } from '@/contexts/AuthContext';

// ── Fallback workspaces for development ──────────────────────────────

// eslint-disable-next-line react-refresh/only-export-components
const FALLBACK_WORKSPACES: WorkspaceDto[] = [
  { key: 'restaurant-pos', name: 'Restaurant POS', description: 'Cashier terminal for restaurant ordering with menu categories and table management', icon: 'restaurant' },
  { key: 'store-pos', name: 'Store POS', description: 'Cashier terminal for retail with product lookup, customer management, and loyalty', icon: 'store' },
  { key: 'kds', name: 'Kitchen Display', description: 'Order queue display for the kitchen — tap tickets to advance their status', icon: 'kds' },
  { key: 'inventory', name: 'Inventory Management', description: 'Manage products, stock levels, bundles, categories, and inventory reports', icon: 'inventory' },
  { key: 'admin', name: 'Admin', description: 'System settings, staff management, reports, audit logs, and configuration', icon: 'admin' },
];

// eslint-disable-next-line react-refresh/only-export-components
export interface WorkspaceContextValue {
  activeWorkspace: string | null;
  setActiveWorkspace: (key: string | null) => void;
  availableWorkspaces: WorkspaceDto[];
  workspaceScreens: string[];
  loading: boolean;
  error: string | null;
  retry: () => void;
  /** The most recently active workspace key — persists even after switching back to the picker. */
  lastWorkspace: string | null;
}

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth();
  const [activeWorkspace, setActiveWorkspace] = useState<string | null>(null);
  const [availableWorkspaces, setAvailableWorkspaces] = useState<WorkspaceDto[]>([]);
  const [workspaceScreens, setWorkspaceScreensState] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const roleId = session?.role_id ?? '';
  const userId = session?.user_id ?? '';

  // Reset workspace selection on login/logout so the user always
  // sees the workspace picker after authentication.
  useEffect(() => {
    setActiveWorkspace(null);
  }, [session]);

  useEffect(() => {
    if (!roleId) {
      setAvailableWorkspaces([]);
      setWorkspaceScreensState([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    // Pass user_id so the backend can check for per-user workspace overrides.
    listWorkspaces(roleId, userId || undefined)
      .then((workspaces) => {
        if (workspaces.length > 0) {
          setAvailableWorkspaces(workspaces);
          setError(null);
        } else {
          // Empty response from backend — use fallback samples for dev.
          setAvailableWorkspaces(FALLBACK_WORKSPACES);
          setError(null);
        }
      })
      .catch((err) => {
        console.warn('WorkspaceContext: failed to list workspaces, using fallback', err);
        setAvailableWorkspaces(FALLBACK_WORKSPACES);
        setError('Failed to load workspaces from server. Using demo workspaces.');
      })
      .finally(() => setLoading(false));
  }, [roleId, userId]);

  useEffect(() => {
    if (!activeWorkspace) {
      setWorkspaceScreensState([]);
      return;
    }
    listWorkspaceScreens(activeWorkspace)
      .then((screens) => {
        if (screens.length > 0) {
          setWorkspaceScreensState(screens.map((s) => s.screen_key));
        } else {
          // No screens from backend — licence all nav items.
          setWorkspaceScreensState([]);
        }
      })
      .catch(() => setWorkspaceScreensState([]));
  }, [activeWorkspace]);

  const [lastWorkspace, setLastWorkspace] = useState<string | null>(null);

  const handleSetActive = useCallback((key: string | null) => {
    if (key) {
      // Track the most recently entered workspace so the picker
      // can show an active indicator when the user switches back.
      setLastWorkspace(key);
    }
    setActiveWorkspace(key);
  }, []);

  const retry = useCallback(() => {
    if (!roleId) return;
    setLoading(true);
    setError(null);
    listWorkspaces(roleId, userId || undefined)
      .then((workspaces) => {
        if (workspaces.length > 0) {
          setAvailableWorkspaces(workspaces);
        } else {
          setAvailableWorkspaces(FALLBACK_WORKSPACES);
        }
        setError(null);
      })
      .catch((err) => {
        console.warn('WorkspaceContext: retry failed', err);
        setAvailableWorkspaces(FALLBACK_WORKSPACES);
        setError('Failed to load workspaces from server. Using demo workspaces.');
      })
      .finally(() => setLoading(false));
  }, [roleId, userId]);

  return (
    <WorkspaceContext.Provider
      value={{
        activeWorkspace,
        setActiveWorkspace: handleSetActive,
        availableWorkspaces,
        workspaceScreens,
        loading,
        error,
        retry,
        lastWorkspace,
      }}
    >
      {children}
    </WorkspaceContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function useWorkspace(): WorkspaceContextValue {
  const ctx = useContext(WorkspaceContext);
  if (!ctx) throw new Error('useWorkspace must be used within a WorkspaceProvider');
  return ctx;
}
