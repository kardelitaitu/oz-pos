import { invoke } from '@tauri-apps/api/core';

export interface WorkspaceDto {
  key: string;
  name: string;
  description: string;
  icon: string;
}

export interface WorkspaceScreenDto {
  screen_key: string;
  sort_order: number;
}

/**
 * List workspaces accessible to the given role, with optional per-user override.
 * When `userId` is provided and the user has explicit workspace assignments,
 * those replace the role-level defaults.
 */
export async function listWorkspaces(roleId: string, userId?: string): Promise<WorkspaceDto[]> {
  return invoke<WorkspaceDto[]>('list_workspaces', { roleId, userId });
}

/**
 * List ALL workspaces in the system (for admin dropdowns).
 * Requires staff:read permission.
 */
export async function listAllWorkspaces(userId: string): Promise<WorkspaceDto[]> {
  return invoke<WorkspaceDto[]>('list_all_workspaces', { userId });
}

/**
 * Replace all workspace assignments for a user.
 * Passing empty array clears all assignments (falls back to role defaults).
 * Requires staff:update permission.
 */
export async function setUserWorkspaces(
  userId: string,
  workspaceKeys: string[],
  callerUserId: string,
): Promise<void> {
  return invoke<void>('set_user_workspaces', { userId, workspaceKeys, callerUserId });
}

/**
 * Get explicit workspace keys assigned to a user.
 * Returns empty array when user has no custom assignments.
 */
export async function getUserWorkspaces(userId: string): Promise<string[]> {
  return invoke<string[]>('get_user_workspaces', { userId });
}

export async function listWorkspaceScreens(workspaceKey: string): Promise<WorkspaceScreenDto[]> {
  return invoke<WorkspaceScreenDto[]>('list_workspace_screens', { workspaceKey });
}
