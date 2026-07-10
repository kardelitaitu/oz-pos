import { invoke } from '@tauri-apps/api/core';

// ── Workspace Instance DTO (ADR #4 Phase 1) ────────────────────────────

/**
 * Full workspace instance DTO returned by the backend.
 * Contains the resolution chain: store → instance → type.
 */
export interface WorkspaceDto {
  instance_id: string;
  type_key: string;
  store_id: string;
  store_name: string;
  name: string;
  description: string;
  icon: string;
  layout_mode: string;
  colour: string | null;
  is_default: boolean;
}

/** Screen (nav item) within a workspace type. */
export interface WorkspaceScreenDto {
  screen_key: string;
  sort_order: number;
}

/** Request body for creating a workspace instance. */
export interface CreateInstanceRequest {
  id: string;
  type_key: string;
  store_id: string;
  name: string;
  description?: string;
  colour?: string;
}

// ── Legacy DTO (deprecated, kept for backward compat) ───────────────────

/** @deprecated Use WorkspaceDto instead (ADR #4). */
export interface WorkspaceTypeDto {
  key: string;
  name: string;
  description: string;
  icon: string;
}

// ── Boot Resolution (ADR #4 Phase 3) ──────────────────────────────────

/** DTO returned by resolve_boot_store. */
export interface BootResolution {
  is_bound: boolean;
  store_id: string;
  instance_id: string | null;
}

/**
 * Resolve the active store and instance from device binding at boot time.
 *
 * Called once before authentication to determine which store database
 * to open and whether to skip the workspace picker.
 *
 * Resolution:
 * 1. Looks up terminal by device_id (hostname).
 * 2. If terminal has valid HMAC-signed device binding:
 *    - If bound to both store + instance → returns both (skip pickers).
 *    - If bound to store only → returns store (skip store picker).
 * 3. Otherwise → returns the primary store.
 */
export async function resolveBootStore(
  deviceId?: string,
): Promise<BootResolution> {
  return invoke<BootResolution>('resolve_boot_store', { deviceId: deviceId ?? null });
}

// ── Instance Commands (ADR #4 Phase 1) ─────────────────────────────────

/**
 * List workspace instances accessible to the given role and user
 * within a specific store.
 *
 * Resolution order:
 * 1. role-owner → all active instances in store
 * 2. user has explicit assignments → only those instances
 * 3. Otherwise → instances of types in role_workspace_types
 */
export async function listWorkspaces(
  roleId: string,
  storeId: string,
  userId?: string,
): Promise<WorkspaceDto[]> {
  return invoke<WorkspaceDto[]>('list_workspaces', {
    roleId,
    storeId,
    userId: userId ?? null,
  });
}

/**
 * Get a single workspace instance by ID.
 * When userId is provided, is_default reflects the user's default instance.
 */
export async function getWorkspaceInstance(
  instanceId: string,
  userId?: string,
): Promise<WorkspaceDto> {
  return invoke<WorkspaceDto>('get_workspace_instance', {
    instanceId,
    userId: userId ?? null,
  });
}

/**
 * Create a new workspace instance (admin only).
 * Requires staff:update permission.
 */
export async function createWorkspaceInstance(
  req: CreateInstanceRequest,
  callerUserId: string,
): Promise<WorkspaceDto> {
  return invoke<WorkspaceDto>('create_workspace_instance', {
    req,
    callerUserId,
  });
}

/**
 * List screens (nav items) for a given workspace type.
 */
export async function listWorkspaceScreens(
  typeKey: string,
): Promise<WorkspaceScreenDto[]> {
  return invoke<WorkspaceScreenDto[]>('list_workspace_screens', {
    typeKey,
  });
}

// ── Instance Assignment Commands ────────────────────────────────────────

/**
 * Replace all instance assignments for a user.
 * Passing empty instanceIds clears all assignments.
 * Requires staff:update permission.
 */
export async function setUserWorkspaceInstances(
  userId: string,
  instanceIds: string[],
  callerUserId: string,
  defaultInstanceId?: string,
): Promise<void> {
  return invoke<void>('set_user_workspace_instances', {
    userId,
    instanceIds,
    defaultInstanceId: defaultInstanceId ?? null,
    callerUserId,
  });
}

/**
 * Get explicit instance IDs assigned to a user.
 * Requires staff:read permission.
 */
export async function getUserWorkspaceInstances(
  userId: string,
): Promise<string[]> {
  return invoke<string[]>('get_user_workspace_instances', { userId });
}

// ── Legacy Commands (backward compatible, deprecated) ──────────────────

/**
 * @deprecated Use listWorkspaces with storeId instead (ADR #4).
 * List all workspace types.
 */
export async function listAllWorkspaces(
  userId: string,
): Promise<WorkspaceTypeDto[]> {
  return invoke<WorkspaceTypeDto[]>('list_all_workspaces', { userId });
}

/**
 * @deprecated Use setUserWorkspaceInstances with instance IDs instead (ADR #4).
 * Replace workspace key assignments for a user.
 */
export async function setUserWorkspaces(
  userId: string,
  workspaceKeys: string[],
  callerUserId: string,
): Promise<void> {
  return invoke<void>('set_user_workspaces', {
    userId,
    workspaceKeys,
    callerUserId,
  });
}

/**
 * @deprecated Use getUserWorkspaceInstances instead (ADR #4).
 * Get workspace keys assigned to a user.
 */
export async function getUserWorkspaces(userId: string): Promise<string[]> {
  return invoke<string[]>('get_user_workspaces', { userId });
}
