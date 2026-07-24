import { ReactNode } from 'react';
import { render, type RenderOptions } from '@testing-library/react';
import { vi } from 'vitest';
import {
  WorkspaceContext,
  WorkspaceScopeContext,
  type WorkspaceContextValue,
  type WorkspaceScope,
} from '@/contexts/WorkspaceContext';

/** Default session token exposed by the workspace test helper. */
export const MOCK_SESSION_TOKEN = 'mock-session-token';

const defaultWorkspaceValue: WorkspaceContextValue = {
  activeWorkspace: null,
  setActiveWorkspace: vi.fn(),
  activeInstance: null,
  setActiveInstance: vi.fn(),
  availableWorkspaces: [],
  workspaceScreens: [],
  loading: false,
  error: null,
  retry: vi.fn(),
  lastWorkspace: null,
  switchStore: vi.fn(),
  resolvedStoreId: 'default',
  sessionToken: MOCK_SESSION_TOKEN,
  swapSessionToken: vi.fn(),
};

export interface RenderWithWorkspaceOptions extends RenderOptions {
  /** Override specific workspace context values for this render. */
  workspace?: Partial<WorkspaceContextValue>;
  /** Override the workspace scope returned by useWorkspaceScope. */
  scope?: WorkspaceScope | null;
}

const defaultScope: WorkspaceScope | null = null;

/**
 * Render a component inside a lightweight WorkspaceProvider stub.
 *
 * Components that call `useWorkspace()` or `useWorkspaceScope()` crash
 * in unit tests unless they are wrapped in a provider. This helper
 * gives a stub provider with safe defaults so tests can focus on
 * their own logic.
 */
export function renderWithWorkspace(
  ui: ReactNode,
  options: RenderWithWorkspaceOptions = {},
) {
  const { workspace, scope, ...renderOptions } = options;

  const value = { ...defaultWorkspaceValue, ...workspace } as WorkspaceContextValue;
  const scopeValue = scope ?? defaultScope;

  return render(
    <WorkspaceScopeContext.Provider value={scopeValue}>
      <WorkspaceContext.Provider value={value}>{ui}</WorkspaceContext.Provider>
    </WorkspaceScopeContext.Provider>,
    renderOptions,
  );
}
