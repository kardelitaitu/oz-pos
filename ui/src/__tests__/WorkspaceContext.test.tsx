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

// ── Hoisted mock state ────────────────────────────────────────────────

const mocks = vi.hoisted(() => ({
  listWorkspaces: vi.fn(),
  listWorkspaceScreens: vi.fn(),
  resolveBootStore: vi.fn(),
  createSession: vi.fn(),
  destroySession: vi.fn(),
}));

// ── Mock context for AuthContext ───────────────────────────────────────
//
// WorkspaceProvider reads `auth.session` via `useAuth()`. We create a
// parallel context so the test wrapper can inject a canned session
// without needing the real AuthProvider.

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

      await waitFor(() => {
        expect(result.current.workspace.loading).toBe(false);
      });

      expect(mocks.resolveBootStore).toHaveBeenCalled();
      expect(mocks.listWorkspaces).toHaveBeenCalledWith('role-cashier', 'store-1', 'user-1');
      expect(result.current.workspace.availableWorkspaces).toHaveLength(2);
    });

    it('does not load workspaces when session is null', async () => {
      const { result } = renderWorkspaceHook(null);

      await waitFor(() => {
        expect(result.current.workspace.loading).toBe(false);
      });

      expect(mocks.listWorkspaces).not.toHaveBeenCalled();
      expect(result.current.workspace.availableWorkspaces).toEqual([]);
    });
  });

  describe('workspace selection', () => {
    it('sets activeWorkspace and syncs activeInstance', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });

      expect(result.current.workspace.activeWorkspace).toBe('restaurant-pos');
      expect(result.current.workspace.activeInstance?.type_key).toBe('restaurant-pos');
      expect(result.current.workspace.lastWorkspace).toBe('restaurant-pos');
    });

    it('sets activeInstance and syncs activeWorkspace', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveInstance(STORE_POS); });

      expect(result.current.workspace.activeWorkspace).toBe('store-pos');
      expect(result.current.workspace.activeInstance?.instance_id).toBe('inst-store');
    });

    it('clears selection when setting null', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      act(() => { result.current.workspace.setActiveWorkspace(null); });

      expect(result.current.workspace.activeWorkspace).toBeNull();
      expect(result.current.workspace.activeInstance).toBeNull();
      expect(result.current.workspace.lastWorkspace).toBeNull();
    });
  });

  describe('workspace screens', () => {
    it('loads screens when an instance is activated', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });

      await waitFor(() => {
        expect(result.current.workspace.workspaceScreens).toEqual(['pos', 'orders']);
      });
      expect(mocks.listWorkspaceScreens).toHaveBeenCalledWith('restaurant-pos');
    });

    it('clears screens when instance becomes null', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.workspaceScreens.length).toBeGreaterThan(0);
      });

      act(() => { result.current.workspace.setActiveWorkspace(null); });

      expect(result.current.workspace.workspaceScreens).toEqual([]);
    });
  });

  describe('session token lifecycle', () => {
    it('creates a session token when workspace is selected', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });

      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      });
    });

    it('destroys old token and creates new one when switching workspace', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      });

      mocks.createSession.mockResolvedValue(makeSessionResult({ session_token: 'tok-xyz' }));

      act(() => { result.current.workspace.setActiveWorkspace('store-pos'); });

      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-xyz');
      });
      expect(mocks.destroySession).toHaveBeenCalledWith('tok-abc-123');
    });
  });

  describe('switchStore', () => {
    it('destroys token, clears state, and re-resolves for new store', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      });

      mocks.listWorkspaces.mockResolvedValue([STORE_POS]);

      act(() => { result.current.workspace.switchStore('store-2'); });

      expect(mocks.destroySession).toHaveBeenCalledWith('tok-abc-123');
      expect(result.current.workspace.activeWorkspace).toBeNull();
      expect(result.current.workspace.resolvedStoreId).toBe('store-2');

      await waitFor(() => {
        expect(result.current.workspace.loading).toBe(false);
      });
      expect(mocks.listWorkspaces).toHaveBeenCalledWith('role-cashier', 'store-2', 'user-1');
    });
  });

  describe('swapSessionToken (ADR #6)', () => {
    it('preserves workspace and creates token with new user identity', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      act(() => { result.current.workspace.setActiveWorkspace('restaurant-pos'); });
      await waitFor(() => {
        expect(result.current.workspace.sessionToken).toBe('tok-abc-123');
      });

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

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

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

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      expect(result.current.workspace.availableWorkspaces.length).toBeGreaterThan(0);
      expect(result.current.workspace.error).toBeNull();
    });

    it('uses fallback data and sets error when API throws', async () => {
      mocks.listWorkspaces.mockRejectedValue(new Error('IPC unavailable'));

      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      expect(result.current.workspace.availableWorkspaces.length).toBeGreaterThan(0);
      expect(result.current.workspace.error).toBe(
        'Failed to load workspaces from server. Using demo workspaces.',
      );
    });
  });

  describe('retry', () => {
    it('re-fetches workspaces when retry is called', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      mocks.listWorkspaces.mockClear();
      act(() => { result.current.workspace.retry(); });

      expect(result.current.workspace.loading).toBe(true);

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));
      expect(mocks.listWorkspaces).toHaveBeenCalled();
    });

    it('is a no-op when roleId is empty (no session)', async () => {
      const { result } = renderWorkspaceHook(null);

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      mocks.listWorkspaces.mockClear();
      act(() => { result.current.workspace.retry(); });

      expect(mocks.listWorkspaces).not.toHaveBeenCalled();
    });
  });

  describe('useWorkspaceScope', () => {
    it('returns null when no workspace is active', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      expect(result.current.scope).toBeNull();
    });

    it('returns scope derived from active instance', async () => {
      const { result } = renderWorkspaceHook();

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

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

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

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

      await waitFor(() => expect(result.current.workspace.loading).toBe(false));

      expect(result.current.workspace.resolvedStoreId).toBe('branch-5');
      expect(mocks.listWorkspaces).toHaveBeenCalledWith('role-cashier', 'branch-5', 'user-1');
    });
  });
});
