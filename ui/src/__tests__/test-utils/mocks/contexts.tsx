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
}

/**
 * Create a mock `useAuth()` return value. Defaults to a cashier session.
 * Pass overrides for specific test scenarios (e.g. manager, owner).
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
  } = overrides;

  return () => ({
    session: {
      user_id: userId,
      username,
      role_name: roleName,
      token,
      role_id: roleId,
      display_name: displayName,
    },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager,
    isOwner,
  });
}

// ── WorkspaceContext ──────────────────────────────────────────────

/**
 * Create a mock WorkspaceContext provider factory.
 * Defaults to `store-pos` active workspace with empty workspace list.
 */
export function createWorkspaceContextMock() {
  return {
    useWorkspace: () => ({
      activeWorkspace: 'store-pos',
      setActiveWorkspace: vi.fn(),
      availableWorkspaces: [],
      workspaceScreens: [],
      loading: false,
      sessionToken: 'mock-session-token',
    }),
    WorkspaceProvider: ({ children }: { children: ReactNode }) => (
      <>{children}</>
    ),
  };
}
