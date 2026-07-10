import { createContext, useContext, useState, useCallback, useEffect, useMemo, type ReactNode } from 'react';
import { listWorkspaces, listWorkspaceScreens, type WorkspaceDto } from '@/api/workspaces';
import { useAuth } from '@/contexts/AuthContext';

// ── Fallback workspaces for development (ADR #4 shape) ──────────────

// eslint-disable-next-line react-refresh/only-export-components
const FALLBACK_WORKSPACES: WorkspaceDto[] = [
  { instance_id: 'default-restaurant-pos', type_key: 'restaurant-pos', store_id: 'default', store_name: 'Main Store', name: 'Restaurant POS', description: 'Cashier terminal for restaurant ordering with menu categories and table management', icon: 'restaurant', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-store-pos', type_key: 'store-pos', store_id: 'default', store_name: 'Main Store', name: 'Store POS', description: 'Cashier terminal for retail with product lookup, customer management, and loyalty', icon: 'store', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-kds', type_key: 'kds', store_id: 'default', store_name: 'Main Store', name: 'Kitchen Display', description: 'Order queue display for the kitchen — tap tickets to advance their status', icon: 'kds', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-inventory', type_key: 'inventory', store_id: 'default', store_name: 'Main Store', name: 'Inventory Management', description: 'Manage products, stock levels, bundles, categories, and inventory reports', icon: 'inventory', layout_mode: 'sidebar', colour: null, is_default: false },
  { instance_id: 'default-admin', type_key: 'admin', store_id: 'default', store_name: 'Main Store', name: 'Admin', description: 'System settings, staff management, reports, audit logs, and configuration', icon: 'admin', layout_mode: 'sidebar', colour: null, is_default: false },
];

// ── Workspace scope context (ADR #4) ────────────────────────────────

/** Resolved workspace scope — derived from the active instance. */
export interface WorkspaceScope {
  storeId: string;
  instanceId: string;
  typeKey: string;
}

const WorkspaceScopeContext = createContext<WorkspaceScope | null>(null);

// eslint-disable-next-line react-refresh/only-export-components
export function useWorkspaceScope(): WorkspaceScope | null {
  return useContext(WorkspaceScopeContext);
}

// ── Main workspace context ──────────────────────────────────────────

// eslint-disable-next-line react-refresh/only-export-components
export interface WorkspaceContextValue {
  /** Workspace type key (backward compat). Same as activeInstance?.type_key. */
  activeWorkspace: string | null;
  setActiveWorkspace: (key: string | null) => void;
  /** ADR #4: the full instance DTO, or null when no workspace is active. */
  activeInstance: WorkspaceDto | null;
  /** ADR #4: set the active instance directly (also updates activeWorkspace). */
  setActiveInstance: (instance: WorkspaceDto | null) => void;
  /** @deprecated Alias for activeInstance, kept for backward compat. Use activeInstance instead. */
  availableWorkspaces: WorkspaceDto[];
  workspaceScreens: string[];
  loading: boolean;
  error: string | null;
  retry: () => void;
  /** The most recently active workspace key — persists even after switching back to the picker. */
  lastWorkspace: string | null;
}

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

/** Default store ID for Phase 1 (single-store mode).
 *  TODO(ADR #4 Phase 2): Derive from device binding or user preference. */
const DEFAULT_STORE_ID = 'default';

export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth();
  // Standalone state — not derived from activeInstance, so it works
  // even before availableWorkspaces is loaded (no race condition).
  const [activeWorkspace, setActiveWorkspace] = useState<string | null>(null);
  const [activeInstance, setActiveInstance] = useState<WorkspaceDto | null>(null);
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
    setActiveInstance(null);
  }, [session]);

  // Sync activeInstance from activeWorkspace whenever the list changes
  // or the key changes. This handles the race condition where
  // setActiveWorkspace is called before the async list resolves.
  useEffect(() => {
    if (activeWorkspace && availableWorkspaces.length > 0) {
      const instance = availableWorkspaces.find((i) => i.type_key === activeWorkspace);
      setActiveInstance(instance ?? null);
    } else if (!activeWorkspace) {
      setActiveInstance(null);
    }
  }, [activeWorkspace, availableWorkspaces]);

  useEffect(() => {
    if (!roleId) {
      setAvailableWorkspaces([]);
      setWorkspaceScreensState([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    // ADR #4: listWorkspaces now requires storeId. Phase 1 uses 'default'.
    listWorkspaces(roleId, DEFAULT_STORE_ID, userId || undefined)
      .then((workspaces) => {
        if (workspaces.length > 0) {
          setAvailableWorkspaces(workspaces);
          setError(null);
        } else {
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
    if (!activeInstance) {
      setWorkspaceScreensState([]);
      return;
    }
    listWorkspaceScreens(activeInstance.type_key)
      .then((screens) => {
        if (screens.length > 0) {
          setWorkspaceScreensState(screens.map((s) => s.screen_key));
        } else {
          setWorkspaceScreensState([]);
        }
      })
      .catch(() => setWorkspaceScreensState([]));
  }, [activeInstance]);

  const [lastWorkspace, setLastWorkspace] = useState<string | null>(null);

  // Backward-compat: sets the type_key string directly.
  const handleSetActive = useCallback((key: string | null) => {
    if (key) {
      setLastWorkspace(key);
    }
    setActiveWorkspace(key);
    // activeInstance syncs via useEffect above
  }, []);

  // ADR #4: set active instance directly.
  const handleSetActiveInstance = useCallback((instance: WorkspaceDto | null) => {
    if (instance) {
      setLastWorkspace(instance.type_key);
      setActiveWorkspace(instance.type_key);
    } else {
      setActiveWorkspace(null);
    }
    setActiveInstance(instance);
  }, []);

  const retry = useCallback(() => {
    if (!roleId) return;
    setLoading(true);
    setError(null);
    listWorkspaces(roleId, DEFAULT_STORE_ID, userId || undefined)
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

  // Derived scope from active instance
  const scope: WorkspaceScope | null = useMemo(
    () =>
      activeInstance
        ? {
            storeId: activeInstance.store_id,
            instanceId: activeInstance.instance_id,
            typeKey: activeInstance.type_key,
          }
        : null,
    [activeInstance],
  );

  return (
    <WorkspaceScopeContext.Provider value={scope}>
      <WorkspaceContext.Provider
        value={{
          activeWorkspace,
          setActiveWorkspace: handleSetActive,
          activeInstance,
          setActiveInstance: handleSetActiveInstance,
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
    </WorkspaceScopeContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function useWorkspace(): WorkspaceContextValue {
  const ctx = useContext(WorkspaceContext);
  if (!ctx) throw new Error('useWorkspace must be used within a WorkspaceProvider');
  return ctx;
}
