// ── Staff: Login, CRUD ─────────────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

// ── Auth ──────────────────────────────────────────────────────────

export interface StaffLoginArgs {
  username: string;
  pin: string;
}

export interface LoginSessionDto {
  user_id: string;
  display_name: string;
  role_name: string;
  role_id: string;
}

export interface StaffLoginResult {
  session: LoginSessionDto;
}

export const staffLogin = (args: StaffLoginArgs): Promise<StaffLoginResult> =>
  invoke<StaffLoginResult>('staff_login', { args });

// ── Staff Management ──────────────────────────────────────────────

export interface StaffMemberDto {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  role_name: string;
  is_active: boolean;
}

export interface RoleDto {
  id: string;
  name: string;
  description: string;
}

export interface CreateStaffArgs {
  username: string;
  pin: string;
  display_name: string;
  role_id: string;
  /** User ID of the caller (from LoginSession). Used for permission check. */
  caller_user_id: string;
}

export interface UpdateStaffArgs {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  is_active: boolean;
  /** User ID of the caller (from LoginSession). Used for permission check. */
  caller_user_id: string;
}

export const listStaff = (): Promise<StaffMemberDto[]> =>
  invoke<StaffMemberDto[]>('list_staff');

export const listRoles = (): Promise<RoleDto[]> =>
  invoke<RoleDto[]>('list_roles');

export const createStaff = (args: CreateStaffArgs): Promise<StaffMemberDto> =>
  invoke<StaffMemberDto>('create_staff', { args });

export const updateStaff = (args: UpdateStaffArgs): Promise<StaffMemberDto> =>
  invoke<StaffMemberDto>('update_staff', { args });

// ── Session Token (ADR #4 / ADR #7) ───────────────────────────────

/** Arguments for creating a session token after login + workspace selection. */
export interface CreateSessionArgs {
  user_id: string;
  role_id: string;
  store_id: string;
  instance_id: string;
  type_key: string;
  terminal_id: string;
}

/** Session context DTO returned alongside the opaque token. */
export interface SessionContextDto {
  userId: string;
  roleId: string;
  storeId: string;
  instanceId: string;
  typeKey: string;
  terminalId: string;
}

/** Result of create_session — opaque token + resolved context. */
export interface CreateSessionResult {
  session_token: string;
  context: SessionContextDto;
}

/**
 * Create a new session token after authentication and workspace selection.
 *
 * The returned token must be passed to every subsequent Tauri command
 * as the `sessionToken` parameter. The backend resolves the caller's
 * scope (store, instance, type, user, role, terminal) from this token.
 */
export const createSession = (args: CreateSessionArgs): Promise<CreateSessionResult> =>
  invoke<CreateSessionResult>('create_session', { args });

/**
 * Destroy an active session token (logout or store switch).
 *
 * After this call, any command using the old token will fail
 * with InvalidSession.
 */
export const destroySession = (sessionToken: string): Promise<void> =>
  invoke<void>('destroy_session', { sessionToken });
