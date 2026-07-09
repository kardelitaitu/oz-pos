/**
 * Hook that provides a `goToWorkspacePicker` function to navigate back
 * to the workspace selection screen from any workspace (POS, KDS,
 * Inventory, Admin, etc.).
 *
 * Usage in any screen component:
 * ```
 * const { goToWorkspacePicker } = useWorkspaceNav();
 * <button onClick={goToWorkspacePicker}>Back to workspaces</button>
 * ```
 *
 * This is a thin wrapper around `useWorkspace().setActiveWorkspace(null)`
 * that avoids importing the full WorkspaceContext in every screen.
 */
import { useCallback } from 'react';
import { useWorkspace } from '@/contexts/WorkspaceContext';

export function useWorkspaceNav() {
  const { setActiveWorkspace } = useWorkspace();

  const goToWorkspacePicker = useCallback(() => {
    setActiveWorkspace(null);
  }, [setActiveWorkspace]);

  return { goToWorkspacePicker };
}
