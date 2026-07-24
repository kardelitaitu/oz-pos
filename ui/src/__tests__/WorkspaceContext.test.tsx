import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { useContext } from 'react';
import { createContext } from 'react';
import type { ReactNode } from 'react';
import {
  WorkspaceProvider,
  useWorkspace,
  useWorkspaceScope,
} from '@/contexts/WorkspaceContext';
import type { LoginSessionDto, CreateSessionResult } from '@/api/staff';
import type { WorkspaceDto } from '@/api/workspaces';

// ── Opt out of the global WorkspaceContext stub ──────────────────────
// The setupFile installs a safe-default mock for useWorkspace and
// useWorkspaceScope so screens render without an explicit provider.
// This file exercises the real provider so it must NOT receive the
// stub. `vi.unmock` is hoisted to the top of the file and removes
// the mocking for this test file's module resolution.
vi.unmock('@/contexts/WorkspaceContext');

// ── Hoisted mock state ────────────────────────────────────────────────

const mocks = vi.hoisted(() => ({
  listWorkspaces: vi.fn(),
  listWorkspaceScreens: vi.fn(),
  resolveBootStore: vi.fn(),
  createSession: vi.fn(),
  destroySession: vi.fn(),
}));

// ── Mock context for AuthContext ───────────────────────────────────────

interface MockAuthCtxValue {
  session: LoginSessionDto | null;
}

const MockAuthCtx = createContext<MockAuthCtxValue>({ session: null });

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => useContext(MockAuthCtx),
}));

// Mock API modules
vi.mock('@/api/workspaces', () => ({
  listWorkspaces: (...args: unknown[]) => mocks.listWorkspaces(...args),
  listWorkspaceScreens: (...args: unknown[]) => mocks.listWorkspaceScreens(...args),
  resolveBootStore: (...args: unknown[]) => mocks.resolveBootStore(...args),
}));

vi.mock('@/api/staff', () => ({
  createSession: (...args: unknown[]) => mocks.createSession(...args),
  destroySession: (...args: unknown[]) => mocks.destroySession(...args),
}));

// ── Test data ─────────────────────────────────────────────────────────

const DEFAULT_SESSION: LoginSessionDto = {
  user_id: 'user-1',
  display_name: 'Alice',
  role_name: 'cashier',
  role_id: 'role-cashier',
};

function makeWorkspace(overrides: Partial<WorkspaceDto> = {}): WorkspaceDto {
  return {
    instance_id: 'inst-restaurant',
    type_key: 'restaurant-pos',
    store_id: 'store-1',
    store_name: 'Main Store',
    name: 'Restaurant POS',
    description: 'Restaurant terminal',
    icon: 'restaurant',
    layout_mode: 'fullscreen',
    colour: null,
    is_default: true,
    ...overrides,
  };
}

const STORE_POS: WorkspaceDto = {
  instance_id: 'inst-store',
  type_key: 'store-pos',
  store_id: 'store-1',
  store_name: 'Main Store',
  name: 'Store POS',
  description: 'Retail terminal',
  icon: 'store',
  layout_mode: 'fullscreen',
  colour: null,
  is_default: false,
};

function makeSessionResult(overrides: Partial<CreateSessionResult> = {}): CreateSessionResult {
  return {
    session_token: 'tok-abc-123',
    context: {
      userId: 'user-1',
      roleId: 'role-cashier',
      storeId: 'store-1',
      instanceId: 'inst-restaurant',
      typeKey: 'restaurant-pos',
      terminalId: '',
    },
    ...overrides,
  };
}

function MockAuthProvider({
  children,
  session,
}: {
  children: ReactNode;
  session: LoginSessionDto | null;
}) {
  return <MockAuthCtx.Provider value={{ session }}>{children}</MockAuthCtx.Provider>;
}

function renderWorkspaceHook(session: LoginSessionDto | null = DEFAULT_SESSION) {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <MockAuthProvider session={session}>
      <WorkspaceProvider>{children}</WorkspaceProvider>
    </MockAuthProvider>
  );
  return renderHook(
    () => ({ workspace: useWorkspace(), scope: useWorkspaceScope() }),
    { wrapper },
  );
}

// ── Helper: flush pending microtasks synchronously ────────────────────
// Replaces waitFor(() => expect(loading).toBe(false)) which polls at
// 50ms intervals. await act(async () => {}) flushes all pending
// microtasks (resolved mock promises + React state updates) in a
// single tick, eliminating the 50ms polling overhead per call.

async function flushAsync() {
  await act(async () => {});
}

// For multi-step async flows where a single flush might not suffice,
// use waitFor with a short interval instead of the default 50ms.
const FAST_WAIT = { interval: 5, timeout: 500 } as const;

// Asserts that loading has completed (loading === false) with fast polling.
// Strictly better than flushAsync() for initial-load waits because it
// preserves the safety assertion: if a future bug makes loading stay true
// forever, the test fails instead of passing with stale state.
// For null-session tests where loading may not transition, use flushAsync().
type HookResult = { current: { workspace: { loading: boolean } } };

async function waitForLoaded(result: HookResult) {
  await waitFor(() => {
    expect(result.current.workspace.loading).toBe(false);
  }, FAST_WAIT);
}

// ── Setup ──────────────────────────────────────────────────────────────

beforeEach(() => {
  mocks.resolveBootStore.mockResolvedValue({
    is_bound: false,
    store_id: 'store-1',
    instance_id: null,
  });
  mocks.listWorkspaces.mockResolvedValue([makeWorkspace(), STORE_POS]);
  mocks.listWorkspaceScreens.mockResolvedValue([
    { screen_key: 'pos', sort_order: 1 },
    { screen_key: 'orders', sort_order: 2 },
  ]);
  mocks.createSession.mockResolvedValue(makeSessionResult());
  mocks.destroySession.mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
});

// ── Tests ──────────────────────────────────────────────────────────────

describe('WorkspaceContext', () => {
  describe('initial state', () => {
    it('starts with null workspace and no error', () => {
      const { result } = renderWorkspaceHook();

      expect(result.current.workspace.activeWorkspace).toBeNull();
      expect(result.current.workspace.activeInstance).toBeNull();
      expect(result.current.workspace.error).toBeNull();
    });

    it('loads workspaces from API on mount with valid session', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      expect(mocks.resolveBootStore).toHaveBeenCalled();
      expect(mocks.listWorkspaces).toHaveBeenCalledWith('role-cashier', 'store-1', 'user-1');
      expect(result.current.workspace.availableWorkspaces).toHaveLength(2);
    });

    it('does not load workspaces when session is null', async () => {
      const { result } = renderWorkspaceHook(null);

      await flushAsync();

      expect(mocks.listWorkspaces).not.toHaveBeenCalled();
      expect(result.current.workspace.availableWorkspaces).toEqual([]);
    });
  });

  describe('workspace selection', () => {
    it('sets activeWorkspace and syncs activeInstance', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });

      expect(result.current.workspace.activeWorkspace).toBe('restaurant-pos');
      expect(result.current.workspace.activeInstance?.type_key).toBe('restaurant-pos');
      expect(result.current.workspace.lastWorkspace).toBe('restaurant-pos');
    });

    it('sets activeInstance and syncs activeWorkspace', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveInstance(STORE_POS); });

      expect(result.current.workspace.activeWorkspace).toBe('store-pos');
      expect(result.current.workspace.activeInstance?.instance_id).toBe('inst-store');
    });

    it('clears selection when setting null', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      act(() => { result.current.workspace.setActiveWorkspace(null); });

      expect(result.current.workspace.activeWorkspace).toBeNull();
      expect(result.current.workspace.activeInstance).toBeNull();
      expect(result.current.workspace.lastWorkspace).toBeNull();
    });
  });

  describe('workspace screens', () => {
    it('handles race condition: stale listWorkspaceScreens does not overwrite new screens', async () => {
      let resolveScreensA!: (value: unknown) => void;
      const screensAPromise = new Promise((resolve) => { resolveScreensA = resolve; });

      // First call (for A) hangs; second call (for B) resolves immediately
      mocks.listWorkspaceScreens
        .mockImplementationOnce(() => screensAPromise)
        .mockImplementationOnce(() =>
          Promise.resolve([{ screen_key: 'pos', sort_order: 1 }]),
        );

      const { result } = renderWorkspaceHook();
      await waitForLoaded(result);

      // Select workspace A (triggers first listWorkspaceScreens call that hangs)
      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await flushAsync();

      // Switch to B while A's screens are still loading
      act(() => { result.current.workspace.setActiveInstance(STORE_POS); });

      // Wait for B's screens to resolve
      await waitFor(() => {
        expect(result.current.workspace.workspaceScreens).toEqual(['pos']);
      }, FAST_WAIT);

      // Now resolve A's deferred promise — the stale .then() must NOT overwrite
      await act(async () => { resolveScreensA!([{ screen_key: 'orders', sort_order: 1 }]); });

      // B's screens should still be 'pos' (not overwritten by A's 'orders')
      expect(result.current.workspace.workspaceScreens).toEqual(['pos']);
    });

    it('loads screens when an instance is activated', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });

      await waitFor(() => {
        expect(result.current.workspace.workspaceScreens).toEqual(['pos', 'orders']);
      }, FAST_WAIT);
      expect(mocks.listWorkspaceScreens).toHaveBeenCalledWith('restaurant-pos');
    });

    it('clears screens when instance becomes null', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.workspaceScreens.length).toBeGreaterThan(0);
      }, FAST_WAIT);

      act(() => { result.current.workspace.setActiveWorkspace(null); });

      expect(result.current.workspace.workspaceScreens).toEqual([]);
    });
  });

  describe('session token lifecycle', () => {
    it('creates a session token when workspace is selected', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });

      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      }, FAST_WAIT);
    });

    it('destroys old token and creates new one when switching workspace', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      }, FAST_WAIT);

      mocks.createSession.mockResolvedValue(makeSessionResult({ session_token: 'tok-xyz' }));

      act(() => { result.current.workspace.setActiveWorkspace('store-pos'); });

      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-xyz');
      }, FAST_WAIT);
      expect(mocks.destroySession).toHaveBeenCalledWith('tok-abc-123');
    });
  });

  describe('switchStore', () => {
    it('destroys token, clears state, and re-resolves for new store', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      }, FAST_WAIT);

      mocks.listWorkspaces.mockResolvedValue([STORE_POS]);

      act(() => { result.current.workspace.switchStore('store-2'); });

      expect(mocks.destroySession).toHaveBeenCalledWith('tok-abc-123');
      expect(result.current.workspace.activeWorkspace).toBeNull();
      expect(result.current.workspace.resolvedStoreId).toBe('store-2');

      await waitForLoaded(result);
      expect(mocks.listWorkspaces).toHaveBeenCalledWith('role-cashier', 'store-2', 'user-1');
    });
  });

  describe('swapSessionToken (ADR #6)', () => {
    it('preserves workspace and creates token with new user identity', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      }, FAST_WAIT);

      mocks.createSession.mockResolvedValue(makeSessionResult({ session_token: 'tok-swapped' }));

      await act(async () => {
        await result.current.workspace.swapSessionToken('user-2', 'role-manager');
      });

      expect(result.current.workspace.activeWorkspace).toBe('restaurant-pos');
      expect(mocks.destroySession).toHaveBeenCalledWith('tok-abc-123');
      expect(result.current.workspace.sessionToken).toBe('tok-swapped');
    });

    it('is a no-op when no instance is active', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      await act(async () => {
        await result.current.workspace.swapSessionToken('user-2', 'role-manager');
      });

      expect(mocks.destroySession).not.toHaveBeenCalled();
      expect(mocks.createSession).not.toHaveBeenCalled();
    });
  });

  describe('fallback workspaces', () => {
    it('uses fallback data when API returns empty list', async () => {
      mocks.listWorkspaces.mockResolvedValue([]);

      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      expect(result.current.workspace.availableWorkspaces.length).toBeGreaterThan(0);
      expect(result.current.workspace.error).toBeNull();
    });

    it('uses fallback data and sets error when API throws', async () => {
      mocks.listWorkspaces.mockRejectedValue(new Error('IPC unavailable'));

      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      expect(result.current.workspace.availableWorkspaces.length).toBeGreaterThan(0);
      expect(result.current.workspace.error).toBe(
        'Failed to load workspaces from server. Using demo workspaces.',
      );
    });
  });

  describe('retry', () => {
    it('re-fetches workspaces when retry is called', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      mocks.listWorkspaces.mockClear();
      act(() => { result.current.workspace.retry(); });

      expect(result.current.workspace.loading).toBe(true);

      await waitForLoaded(result);
      expect(mocks.listWorkspaces).toHaveBeenCalled();
    });

    it('is a no-op when roleId is empty (no session)', async () => {
      const { result } = renderWorkspaceHook(null);

      await flushAsync();

      mocks.listWorkspaces.mockClear();
      act(() => { result.current.workspace.retry(); });

      expect(mocks.listWorkspaces).not.toHaveBeenCalled();
    });
  });

  describe('useWorkspaceScope', () => {
    it('returns null when no workspace is active', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      expect(result.current.scope).toBeNull();
    });

    it('returns scope derived from active instance', async () => {
      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      act(() => { result.current.workspace.setActiveInstance(makeWorkspace()); });

      expect(result.current.scope).toEqual({
        storeId: 'store-1',
        instanceId: 'inst-restaurant',
        typeKey: 'restaurant-pos',
      });
    });
  });

  describe('boot store resolution', () => {
    it('falls back to default store when resolution fails', async () => {
      mocks.resolveBootStore.mockRejectedValue(new Error('no device binding'));

      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      expect(result.current.workspace.resolvedStoreId).toBe('default');
      expect(mocks.listWorkspaces).toHaveBeenCalledWith('role-cashier', 'default', 'user-1');
    });

    it('resolves store and loads workspaces for that store', async () => {
      mocks.resolveBootStore.mockResolvedValue({
        is_bound: false,
        store_id: 'branch-5',
        instance_id: null,
      });
      mocks.listWorkspaces.mockResolvedValue([{ ...makeWorkspace(), store_id: 'branch-5', store_name: 'Branch 5' }]);

      const { result } = renderWorkspaceHook();

      await waitForLoaded(result);

      expect(result.current.workspace.resolvedStoreId).toBe('branch-5');
      expect(mocks.listWorkspaces).toHaveBeenCalledWith('role-cashier', 'branch-5', 'user-1');
    });
  });
});
