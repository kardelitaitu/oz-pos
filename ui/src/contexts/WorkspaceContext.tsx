import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import { listWorkspaces, listWorkspaceScreens, type WorkspaceDto } from '@/api/workspaces';
import { useAuth } from '@/contexts/AuthContext';

// ── Fallback workspaces for development ──────────────────────────────

const FALLBACK_WORKSPACES: WorkspaceDto[] = [
  { key: 'restaurant-pos', name: 'Restaurant POS', description: 'Cashier terminal for restaurant ordering with menu categories and table management', icon: 'restaurant' },
  { key: 'store-pos', name: 'Store POS', description: 'Cashier terminal for retail with product lookup, customer management, and loyalty', icon: 'store' },
  { key: 'inventory', name: 'Inventory Management', description: 'Manage products, stock levels, bundles, categories, and inventory reports', icon: 'inventory' },
  { key: 'admin', name: 'Admin', description: 'System settings, staff management, reports, audit logs, and configuration', icon: 'admin' },
];

export interface WorkspaceContextValue {
  activeWorkspace: string | null;
  setActiveWorkspace: (key: string | null) => void;
  availableWorkspaces: WorkspaceDto[];
  workspaceScreens: string[];
  loading: boolean;
}

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth();
  const [activeWorkspace, setActiveWorkspace] = useState<string | null>(null);
  const [availableWorkspaces, setAvailableWorkspaces] = useState<WorkspaceDto[]>([]);
  const [workspaceScreens, setWorkspaceScreensState] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

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
    // Pass user_id so the backend can check for per-user workspace overrides.
    listWorkspaces(roleId, userId || undefined)
      .then((workspaces) => {
        if (workspaces.length > 0) {
          setAvailableWorkspaces(workspaces);
        } else {
          // Empty response from backend — use fallback samples for dev.
          setAvailableWorkspaces(FALLBACK_WORKSPACES);
        }
      })
      .catch((err) => {
        console.warn('WorkspaceContext: failed to list workspaces, using fallback', err);
        setAvailableWorkspaces(FALLBACK_WORKSPACES);
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

  const handleSetActive = useCallback((key: string | null) => {
    setActiveWorkspace(key);
  }, []);

  return (
    <WorkspaceContext.Provider
      value={{
        activeWorkspace,
        setActiveWorkspace: handleSetActive,
        availableWorkspaces,
        workspaceScreens,
        loading,
      }}
    >
      {children}
    </WorkspaceContext.Provider>
  );
}

export function useWorkspace(): WorkspaceContextValue {
  const ctx = useContext(WorkspaceContext);
  if (!ctx) throw new Error('useWorkspace must be used within a WorkspaceProvider');
  return ctx;
}
