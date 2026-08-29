// Vite React Refresh: force full remount on HMR to prevent stale
// WorkspaceContext / WorkspaceScopeContext mismatch.
/// @refresh reset
import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type ReactNode,
} from "react";
import {
  listWorkspaces,
  listWorkspaceScreens,
  resolveBootStore,
  type WorkspaceDto,
} from "@/api/workspaces";
import { createSession, destroySession, refreshPickerTicket } from "@/api/staff";
import { getDeviceId } from "@/api/system";
import { useAuth } from "@/contexts/AuthContext";

// ── Fallback workspaces for development (ADR #4 shape) ──────────────

 
const FALLBACK_WORKSPACES: WorkspaceDto[] = [
  {
    instance_id: "default-restaurant-pos",
    type_key: "restaurant-pos",
    store_id: "default",
    store_name: "Main Store",
    purpose_key: "general",
    name: "Restaurant POS",
    description:
      "Cashier terminal for restaurant ordering with menu categories and table management",
    icon: "restaurant",
    layout_mode: "fullscreen",
    colour: null,
    is_default: false,
  },
  {
    instance_id: "default-store-pos",
    type_key: "store-pos",
    store_id: "default",
    store_name: "Main Store",
    purpose_key: "general",
    name: "Store POS",
    description:
      "Cashier terminal for retail with product lookup, customer management, and loyalty",
    icon: "store",
    layout_mode: "fullscreen",
    colour: null,
    is_default: false,
  },
  {
    instance_id: "default-kds",
    type_key: "kds",
    store_id: "default",
    store_name: "Main Store",
    purpose_key: "general",
    name: "Kitchen Display",
    description:
      "Order queue display for the kitchen — tap tickets to advance their status",
    icon: "kds",
    layout_mode: "fullscreen",
    colour: null,
    is_default: false,
  },
  {
    instance_id: "default-warehouse",
    type_key: "warehouse",
    store_id: "default",
    store_name: "Main Store",
    purpose_key: "stock-control",
    name: "Warehouse",
    description:
      "Manage products, stock levels, bundles, categories, and inventory reports",
    icon: "inventory",
    layout_mode: "sidebar",
    colour: null,
    is_default: false,
  },
  {
    instance_id: "default-admin",
    type_key: "admin",
    store_id: "default",
    store_name: "Main Store",
    purpose_key: "general",
    name: "Admin",
    description:
      "System settings, staff management, reports, audit logs, and configuration",
    icon: "admin",
    layout_mode: "sidebar",
    colour: null,
    is_default: false,
  },
];

// ── Workspace scope context (ADR #4) ────────────────────────────────

/** Resolved workspace scope — derived from the active instance. */
export interface WorkspaceScope {
  storeId: string;
  instanceId: string;
  typeKey: string;
}

/** Exported for test helpers only — always use `useWorkspaceScope` in production code. */
// eslint-disable-next-line react-refresh/only-export-components
export const WorkspaceScopeContext = createContext<WorkspaceScope | null>(null);

/** Access the current workspace scope (storeId, instanceId, typeKey), or null. */
// eslint-disable-next-line react-refresh/only-export-components
export function useWorkspaceScope(): WorkspaceScope | null {
  return useContext(WorkspaceScopeContext);
}

// ── Main workspace context ──────────────────────────────────────────

 
/** Full workspace context value exposed to consumers. */
 
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
  /** ADR #4 Phase 2b: switch to a different store, clearing workspace and re-resolving. */
  switchStore: (storeId: string) => void;
  /** ADR #4 Phase 2b: the currently resolved store ID. */
  resolvedStoreId: string;
  /** ADR #4 / ADR #7: opaque session token for scoped command authorization. */
  sessionToken: string | null;
  /** ADR #22: device/terminal ID for hardware-scoped settings. */
  terminalId: string;
  /**
   * ADR #6: Hot-swap the session token to a new user without resetting
   * the active workspace/instance. Used by FastPINOverlay for shared
   * touchscreen operator switching. Destroys the old token and creates
   * a new one with the same scope (storeId, instanceId, typeKey, terminalId)
   * but the new user's identity.
   */
  swapSessionToken: (newUserId: string, newRoleId: string) => Promise<void>;
}

/** Exported for test helpers only — always use `useWorkspace` in production code. */
// eslint-disable-next-line react-refresh/only-export-components
export const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

/** Default store ID for Phase 1 (single-store mode).
 *  ADR #4 Phase 3: Replaced by dynamic resolution via resolveBootStore().
 *  Kept as fallback when boot resolution fails. */
const DEFAULT_STORE_ID = "default";

/**
 * Provides workspace state to the entire app tree.
 *
 * Loads available workspaces on mount, resolves the boot store via
 * ADR #4 Phase 3, manages the active workspace/instance selection,
 * creates session tokens (ADR #4 / ADR #7), and supports hot-swap
 * session token switching (ADR #6).
 */
export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const { session, pickerTicket, updatePickerTicket } = useAuth();
  const updatePickerTicketFn = updatePickerTicket ?? (() => {});
  // Standalone state — not derived from activeInstance, so it works
  // even before availableWorkspaces is loaded (no race condition).
  const [activeWorkspace, setActiveWorkspace] = useState<string | null>(null);
  const [activeInstance, setActiveInstance] = useState<WorkspaceDto | null>(
    null,
  );
  const [availableWorkspaces, setAvailableWorkspaces] = useState<
    WorkspaceDto[]
  >([]);
  const [workspaceScreens, setWorkspaceScreensState] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);


  // ADR #4 Phase 3: Dynamically resolved store ID from device binding or primary store.
  const [resolvedStoreId, setResolvedStoreId] =
    useState<string>(DEFAULT_STORE_ID);

  // ADR #4 / ADR #7: Opaque session token created by create_session command.
  const [sessionToken, setSessionToken] = useState<string | null>(null);
  const sessionTokenRef = useRef(sessionToken);
  sessionTokenRef.current = sessionToken;

  // ADR #22: Device/terminal ID resolved once on mount.
  const [terminalId, setTerminalId] = useState('');
  useEffect(() => {
    getDeviceId().then(setTerminalId).catch(() => setTerminalId(''));
  }, []);

  // ADR #6: Stable ref for the active instance so swapSessionToken
  // can read it without depending on the state (keeps the callback
  // reference stable).
  const activeInstanceRef = useRef(activeInstance);
  activeInstanceRef.current = activeInstance;

  // ADR #6: Guard to prevent the token-creation effect from also
  // creating a token during a hot-swap (swapSessionToken handles it).
  const isHotSwappingRef = useRef(false);

  // Reset workspace selection on login/logout so the user always
  // sees the workspace picker after authentication.
  // Uses a ref for sessionToken to avoid the effect re-firing when
  // sessionToken changes (which would clear the workspace immediately
  // after selection).
  //
  // ADR #6: Only reset on null↔non-null transitions (login/logout),
  // not on value-to-value changes (hot-swap via FastPINOverlay).
  const prevSessionRef = useRef(session);
  useEffect(() => {
    const prev = prevSessionRef.current;
    prevSessionRef.current = session;

    // Hot-swap: session changed from one user to another — don't reset.
    if (prev && session) return;

    // Fresh login (null → session) or logout (session → null): reset.
    setActiveWorkspace(null);
    setActiveInstance(null);
    const token = sessionTokenRef.current;
    if (token) {
      destroySession(token).catch(() => {});
      setSessionToken(null);
    }
  }, [session]);

  // Sync activeInstance from activeWorkspace whenever the list changes
  // or the key changes. This handles the race condition where
  // setActiveWorkspace is called before the async list resolves.
  useEffect(() => {
    if (activeWorkspace && availableWorkspaces.length > 0) {
      const instance = availableWorkspaces.find(
        (i) => i.type_key === activeWorkspace,
      );
      setActiveInstance(instance ?? null);
    } else if (!activeWorkspace) {
      setActiveInstance(null);
    }
  }, [activeWorkspace, availableWorkspaces]);

  // ── Shared workspace-fetching logic ─────────────────────────────────

  const fetchWorkspaces = useCallback(
    async (storeId: string, cancelled: () => boolean) => {
      // audit-open-findings: the picker ticket binds the caller server-side; the
      // backend derives the real role from the ticket, so the UI never
      // sends a role/user claim that could be forged.
      if (!pickerTicket) return;
      try {
        const workspaces = await listWorkspaces(pickerTicket, storeId);
        if (!cancelled()) {
          if (workspaces.length > 0) {
            setAvailableWorkspaces(workspaces);
            setError(null);
          } else {
            setAvailableWorkspaces(FALLBACK_WORKSPACES);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled()) {
          console.warn(
            "WorkspaceContext: failed to list workspaces, using fallback",
            err,
          );
          setAvailableWorkspaces(FALLBACK_WORKSPACES);
          setError(
            "Failed to load workspaces from server. Using demo workspaces.",
          );
        }
      }
    },
    [pickerTicket],
  );

  // ADR #4 Phase 2b: Switch to a different store.
  // Destroys the current session token, clears workspace, re-resolves for new store.
  // Uses sessionTokenRef to keep the callback reference stable.
  const switchStore = useCallback(
    (storeId: string) => {
      const token = sessionTokenRef.current;
      if (token) {
        destroySession(token).catch(() => {});
        setSessionToken(null);
      }
      setActiveWorkspace(null);
      setActiveInstance(null);
      setWorkspaceScreensState([]);
      setResolvedStoreId(storeId);
      setLoading(true);
      setError(null);
      fetchWorkspaces(storeId, () => false).finally(() => setLoading(false));
    },
    [fetchWorkspaces],
  );

  // ADR #6: Hot-swap the session token to a new user without resetting
  // the workspace. Used by FastPINOverlay for shared touchscreen
  // operator switching.
  //
  // Destroys the old token and creates a new one with the same scope
  // (storeId, instanceId, typeKey, terminalId) but the new user's
  // identity. The active workspace and instance are preserved.
  //
  // Sets isHotSwappingRef to prevent the [activeInstance, session]
  // effect from also creating a token during the swap (race condition).
  const swapSessionToken = useCallback(
    async (newUserId: string, newRoleId: string) => {
      const instance = activeInstanceRef.current;
      if (!instance) return;

      isHotSwappingRef.current = true;
      try {
        // Refresh the picker ticket using the old session before
        // destroying it — the hot-swap user needs a fresh ticket
        // bound to THEIR identity (not the previous user's).
        let ticket = pickerTicket ?? "";
        const prev = sessionTokenRef.current;
        if (prev) {
          try {
            const refreshed = await refreshPickerTicket(prev);
            ticket = refreshed.picker_ticket;
            updatePickerTicketFn(ticket);
          } catch {
            // Refresh failed — use the existing ticket.
          }

          await destroySession(prev).catch(() => {});
          setSessionToken(null);
        }

        // Create a new token with the same scope but new user.
        const result = await createSession({
          user_id: newUserId,
          role_id: newRoleId,
          store_id: instance.store_id,
          instance_id: instance.instance_id,
          type_key: instance.type_key,
          terminal_id: await getDeviceId().catch(() => ""),
          picker_ticket: ticket,
        });

        setSessionToken(result.session_token);
      } finally {
        isHotSwappingRef.current = false;
      }
    },
    [], // stable — reads from refs
  );

  // ADR #4 Phase 3: Resolve the boot store first, then load workspaces.
  // This is called once on mount (or when the picker ticket changes).
  useEffect(() => {
    if (!pickerTicket) {
      setAvailableWorkspaces([]);
      setWorkspaceScreensState([]);
      setLoading(false);
      return;
    }

    let cancelled = false;

    async function boot() {
      setLoading(true);
      setError(null);

      // Step 1: Resolve the store from device binding or primary store.
      let storeId = DEFAULT_STORE_ID;
      try {
        // ADR #4 Phase 3: pass the device id so a bound terminal
        // auto-boots into its store+instance instead of the primary.
        const deviceId = await getDeviceId().catch(() => "");
        const resolution = await resolveBootStore(deviceId || undefined);
        storeId = resolution.store_id || DEFAULT_STORE_ID;
        if (!cancelled) {
          setResolvedStoreId(storeId);
        }
      } catch (err) {
        console.warn(
          "WorkspaceContext: boot store resolution failed, using default",
          err,
        );
        if (!cancelled) {
          setResolvedStoreId(DEFAULT_STORE_ID);
        }
      }

      // Step 2: Load workspace instances for the resolved store.
      await fetchWorkspaces(storeId, () => cancelled);

      if (!cancelled) {
        setLoading(false);
      }
    }

    boot();

    return () => {
      cancelled = true;
    };
  }, [pickerTicket, fetchWorkspaces]);

  useEffect(() => {
    let cancelled = false;
    if (!activeInstance) {
      setWorkspaceScreensState([]);
      return;
    }
    // audit-open-findings: the picker ticket binds the caller server-side — the screen
    // list is only readable by a genuinely-authenticated user.
    listWorkspaceScreens(
      pickerTicket ?? "",
      activeInstance.type_key,
      activeInstance.store_id,
    )
      .then((screens) => {
        if (cancelled) return;
        if (screens.length > 0) {
          setWorkspaceScreensState(screens.map((s) => s.screen_key));
        } else {
          setWorkspaceScreensState([]);
        }
      })
      .catch(() => {
        if (!cancelled) setWorkspaceScreensState([]);
      });
    return () => { cancelled = true; };
  }, [activeInstance, pickerTicket]);

  const [lastWorkspace, setLastWorkspace] = useState<string | null>(null);

  // ADR #4 / ADR #7: Create a session token when an instance is activated.
  // This effect fires after activeInstance changes (set by handleSetActiveInstance
  // or the useEffect that syncs from activeWorkspace).
  //
  // ADR #6: Skips token creation when isHotSwappingRef is set, because
  // swapSessionToken handles token lifecycle during a hot-swap.
  useEffect(() => {
    if (!activeInstance || !session?.user_id) return;
    if (isHotSwappingRef.current) return; // ADR #6: swapSessionToken handles this

    let cancelled = false;

    // Resolve device ID for terminal binding (ADR #7), then refresh
    // the picker ticket and create session.
    (async () => {
      const deviceId = await getDeviceId().catch(() => "");
      if (cancelled) return;

      // ADR #6 re-entry: if a previous session exists (user pressed
      // Back from KDS), refresh the picker ticket BEFORE destroying
      // the old session. The ticket has a 5-minute TTL; if the user
      // spent longer than that at the workspace picker, the original
      // ticket from login is stale. The refresh re-mints a fresh
      // ticket using the still-valid session token.
      let ticket = pickerTicket ?? "";
      const prevToken = sessionTokenRef.current;
      if (prevToken) {
        try {
          const refreshed = await refreshPickerTicket(prevToken);
          ticket = refreshed.picker_ticket;
          updatePickerTicketFn(ticket);
        } catch {
          // Refresh failed (session expired?) — proceed with the
          // original ticket; createSession will reject if it's stale.
        }
      }

      if (cancelled) return;

      // Destroy any previous token before creating a new one.
      if (prevToken) {
        destroySession(prevToken).catch(() => {});
        setSessionToken(null);
      }

      createSession({
        user_id: session.user_id,
        role_id: session.role_id,
        store_id: activeInstance.store_id,
        instance_id: activeInstance.instance_id,
        type_key: activeInstance.type_key,
        terminal_id: deviceId,
        picker_ticket: ticket,
      })
        .then((result) => {
          if (!cancelled) {
            setSessionToken(result.session_token);
          }
        })
        .catch((err) => {
          if (!cancelled) {
            console.warn("WorkspaceContext: failed to create session token", err);
          }
        });
    })();

    return () => {
      cancelled = true;
    };
  }, [activeInstance, session]);


  // Backward-compat: sets the type_key string directly.
  // lastWorkspace persists even when returning to the workspace picker
  // (key === null) so the last-used card keeps its active indicator —
  // matching the interface contract above. It is cleared only on a fresh
  // login/logout: the reset effect also calls setActiveInstance(null),
  // which routes through handleSetActiveInstance and clears it.
  const handleSetActive = useCallback((key: string | null) => {
    if (key) setLastWorkspace(key);
    setActiveWorkspace(key);
    // activeInstance syncs via useEffect above
  }, []);

  // ADR #4: set active instance directly.
  // Always updates lastWorkspace — even with null — so the active
  // card visual is cleared when returning to the workspace picker.
  const handleSetActiveInstance = useCallback(
    (instance: WorkspaceDto | null) => {
      if (instance) {
        setActiveWorkspace(instance.type_key);
        setLastWorkspace(instance.type_key);
      } else {
        setActiveWorkspace(null);
        setLastWorkspace(null);
      }
      setActiveInstance(instance);
    },
    [],
  );

  const retry = useCallback(() => {
    if (!pickerTicket) return;
    setLoading(true);
    setError(null);
    fetchWorkspaces(resolvedStoreId, () => false).finally(() =>
      setLoading(false),
    );
  }, [pickerTicket, resolvedStoreId, fetchWorkspaces]);

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
    <WorkspaceScopeContext.Provider value={scope}>        <WorkspaceContext.Provider
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
          switchStore,
          resolvedStoreId,
          sessionToken,
          swapSessionToken,
          terminalId,
        }}
      >
        {children}
      </WorkspaceContext.Provider>
    </WorkspaceScopeContext.Provider>
  );
}

/** Access the workspace context. Must be used within a `<WorkspaceProvider>`. */
// eslint-disable-next-line react-refresh/only-export-components
export function useWorkspace(): WorkspaceContextValue {
  const ctx = useContext(WorkspaceContext);
  if (!ctx)
    throw new Error("useWorkspace must be used within a WorkspaceProvider");
  return ctx;
}
