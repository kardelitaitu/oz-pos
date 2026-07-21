import { loggedInvoke } from '@/utils/logged-invoke';

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
  return loggedInvoke<BootResolution>('resolve_boot_store', { deviceId: deviceId ?? null });
}

// ── Scoped Commands (ADR #7) ───────────────────────────────────────────

/** List workspace instances for the session user within their store. ADR #7. */
export async function listWorkspacesScoped(
  sessionToken: string,
): Promise<WorkspaceDto[]> {
  return loggedInvoke<WorkspaceDto[]>('list_workspaces_scoped', { sessionToken });
}

/** Get a single workspace instance. `is_default` reflects the session user. ADR #7. */
export async function getWorkspaceInstanceScoped(
  sessionToken: string,
  instanceId: string,
): Promise<WorkspaceDto> {
  return loggedInvoke<WorkspaceDto>('get_workspace_instance_scoped', { sessionToken, instanceId });
}

/** Create a new workspace instance (admin). Permission from session. ADR #7. */
export async function createWorkspaceInstanceScoped(
  sessionToken: string,
  req: CreateInstanceRequest,
): Promise<WorkspaceDto> {
  return loggedInvoke<WorkspaceDto>('create_workspace_instance_scoped', { sessionToken, req });
}

/** List screens for a workspace type from the store-scoped database. ADR #7. */
export async function listWorkspaceScreensScoped(
  sessionToken: string,
  typeKey: string,
): Promise<WorkspaceScreenDto[]> {
  return loggedInvoke<WorkspaceScreenDto[]>('list_workspace_screens_scoped', { sessionToken, typeKey });
}

/** Replace all instance assignments for a user. Caller permission from session. ADR #7. */
export async function setUserWorkspaceInstancesScoped(
  sessionToken: string,
  userId: string,
  instanceIds: string[],
  defaultInstanceId?: string,
): Promise<void> {
  return loggedInvoke<void>('set_user_workspace_instances_scoped', {
    sessionToken,
    userId,
    instanceIds,
    defaultInstanceId: defaultInstanceId ?? null,
  });
}

/** Get instance IDs assigned to a user. Permission check from session. ADR #7. */
export async function getUserWorkspaceInstancesScoped(
  sessionToken: string,
  userId: string,
): Promise<string[]> {
  return loggedInvoke<string[]>('get_user_workspace_instances_scoped', { sessionToken, userId });
}

// ── Original Commands (deprecated for multi-store — ADR #7) ────────────

/**
 * @deprecated Use listWorkspacesScoped with session token instead (ADR #7).
 * List workspace instances accessible to the given role and user.
 */
export async function listWorkspaces(
  roleId: string,
  storeId: string,
  userId?: string,
): Promise<WorkspaceDto[]> {
  return loggedInvoke<WorkspaceDto[]>('list_workspaces', {
    roleId,
    storeId,
    userId: userId ?? null,
  });
}

/**
 * @deprecated Use getWorkspaceInstanceScoped with session token instead (ADR #7).
 * Get a single workspace instance by ID.
 */
export async function getWorkspaceInstance(
  instanceId: string,
  userId?: string,
): Promise<WorkspaceDto> {
  return loggedInvoke<WorkspaceDto>('get_workspace_instance', {
    instanceId,
    userId: userId ?? null,
  });
}

/**
 * @deprecated Use createWorkspaceInstanceScoped with session token instead (ADR #7).
 * Create a new workspace instance (admin only).
 */
export async function createWorkspaceInstance(
  req: CreateInstanceRequest,
  callerUserId: string,
): Promise<WorkspaceDto> {
  return loggedInvoke<WorkspaceDto>('create_workspace_instance', {
    req,
    callerUserId,
  });
}

/**
 * @deprecated Use listWorkspaceScreensScoped with session token instead (ADR #7).
 * List screens (nav items) for a given workspace type.
 */
export async function listWorkspaceScreens(
  typeKey: string,
): Promise<WorkspaceScreenDto[]> {
  return loggedInvoke<WorkspaceScreenDto[]>('list_workspace_screens', {
    typeKey,
  });
}

// ── Instance Assignment Commands ────────────────────────────────────────

/**
 * @deprecated Use setUserWorkspaceInstancesScoped with session token instead (ADR #7).
 * Replace all instance assignments for a user.
 */
export async function setUserWorkspaceInstances(
  userId: string,
  instanceIds: string[],
  callerUserId: string,
  defaultInstanceId?: string,
): Promise<void> {
  return loggedInvoke<void>('set_user_workspace_instances', {
    userId,
    instanceIds,
    defaultInstanceId: defaultInstanceId ?? null,
    callerUserId,
  });
}

/**
 * @deprecated Use getUserWorkspaceInstancesScoped with session token instead (ADR #7).
 * Get explicit instance IDs assigned to a user.
 */
export async function getUserWorkspaceInstances(
  userId: string,
): Promise<string[]> {
  return loggedInvoke<string[]>('get_user_workspace_instances', { userId });
}

// ── Legacy Commands (backward compatible, deprecated) ──────────────────

/**
 * @deprecated Use listWorkspacesScoped instead (ADR #7).
 * List all workspace types.
 */
export async function listAllWorkspaces(
  userId: string,
): Promise<WorkspaceTypeDto[]> {
  return loggedInvoke<WorkspaceTypeDto[]>('list_all_workspaces', { userId });
}

/** List all workspace types (scoped — ADR #7). */
export async function listAllWorkspacesScoped(
  sessionToken: string,
): Promise<WorkspaceTypeDto[]> {
  return loggedInvoke<WorkspaceTypeDto[]>('list_all_workspaces_scoped', { sessionToken });
}

/**
 * @deprecated Use setUserWorkspaceInstancesScoped instead (ADR #7).
 * Replace workspace key assignments for a user.
 */
export async function setUserWorkspaces(
  userId: string,
  workspaceKeys: string[],
  callerUserId: string,
): Promise<void> {
  return loggedInvoke<void>('set_user_workspaces', {
    userId,
    workspaceKeys,
    callerUserId,
  });
}

/** Replace workspace key assignments (legacy), caller from session. ADR #7. */
export async function setUserWorkspacesScoped(
  sessionToken: string,
  userId: string,
  workspaceKeys: string[],
): Promise<void> {
  return loggedInvoke<void>('set_user_workspaces_scoped', { sessionToken, userId, workspaceKeys });
}

/**
 * @deprecated Use getUserWorkspaceInstancesScoped instead (ADR #7).
 * Get workspace keys assigned to a user.
 */
export async function getUserWorkspaces(userId: string): Promise<string[]> {
  return loggedInvoke<string[]>('get_user_workspaces', { userId });
}

/** Get workspace keys for a user (legacy), caller from session. ADR #7. */
export async function getUserWorkspacesScoped(
  sessionToken: string,
  userId: string,
): Promise<string[]> {
  return loggedInvoke<string[]>('get_user_workspaces_scoped', { sessionToken, userId });
}
