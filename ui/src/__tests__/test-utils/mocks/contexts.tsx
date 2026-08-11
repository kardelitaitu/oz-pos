// ── Shared context mocks ────────────────────────────────────────────
//
// These mock factories are used by RetailPosScreen, PosScreen, AppShell,
// PaymentModal, and other test files that need AuthContext / WorkspaceContext
// providers. Import and use with `createAuthContextMock()` or call the
// factory directly inside a `vi.mock()` block.
//
// Usage:
//   import { createAuthContextMock, createWorkspaceContextMock } from
//     '@/__tests__/test-utils/mocks/contexts';
//
//   vi.mock('@/contexts/AuthContext', () => ({
//     useAuth: createAuthContextMock(),
//   }));

import { vi } from 'vitest';
import type { ReactNode } from 'react';

// ── AuthContext ───────────────────────────────────────────────────

export interface AuthContextOverrides {
  userId?: string;
  username?: string;
  roleName?: string;
  roleId?: string;
  token?: string;
  displayName?: string;
  isManager?: boolean;
  isOwner?: boolean;
  /** Effective permission keys (mirrors the backend registry; empty = none). */
  permissions?: string[];
}

/**
 * Create a mock `useAuth()` return value. Defaults to a cashier session.
 * Pass overrides for specific test scenarios (e.g. manager, owner).
 *
 * The returned function matches the `useAuth` hook signature so it can
 * be used directly in `vi.mock('@/contexts/AuthContext', () => ({
 *   useAuth: createAuthContextMock({ isManager: true }),
 * }))`.
 */
export function createAuthContextMock(overrides: AuthContextOverrides = {}) {
  const {
    userId = 'user-1',
    username = 'testuser',
    roleName = 'cashier',
    roleId = 'role-1',
    token = 'mock-token',
    displayName = 'Kasir Test',
    isManager = false,
    isOwner = false,
    permissions = [],
  } = overrides;

  return () => ({
    session: {
      user_id: userId,
      username,
      role_name: roleName,
      token,
      role_id: roleId,
      display_name: displayName,
      permissions,
    },
    loading: false,
    error: null,
    login: vi.fn(async (_username: string, _pin: string) => {}),
    logout: vi.fn(),
    clearError: vi.fn(),
    swapSession: vi.fn(),
    isManager,
    isOwner,
  });
}

// ── WorkspaceContext ──────────────────────────────────────────────

/**
 * Create a mock WorkspaceContext module factory.
 *
 * Returns the full module shape that `vi.mock('@/contexts/WorkspaceContext')`
 * expects: `{ useWorkspace, useWorkspaceScope, WorkspaceProvider }`.
 *
 * Defaults to `store-pos` active workspace with a mock session token.
 * Components that need `useWorkspaceScope()` will receive non-null defaults.
 */
export function createWorkspaceContextMock() {
  return {
    useWorkspace: () => ({
      activeWorkspace: 'store-pos' as string | null,
      setActiveWorkspace: vi.fn((_key: string | null) => {}),
      activeInstance: null,
      setActiveInstance: vi.fn(),
      availableWorkspaces: [],
      workspaceScreens: [],
      loading: false,
      error: null,
      retry: vi.fn(),
      lastWorkspace: null,
      switchStore: vi.fn((_storeId: string) => {}),
      resolvedStoreId: 'default',
      sessionToken: 'mock-session-token' as string | null,
      swapSessionToken: vi.fn(async (_newUserId: string, _newRoleId: string) => {}),
      terminalId: '',
    }),
    useWorkspaceScope: () => ({
      storeId: 'default',
      instanceId: 'default',
      typeKey: 'store-pos',
    }),
    WorkspaceProvider: ({ children }: { children: ReactNode }) => (
      <>{children}</>
    ),
  };
}
