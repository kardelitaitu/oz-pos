// ── Staff: Login, Bootstrap, CRUD ──────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

// ── Auth ──────────────────────────────────────────────────────────

/** Arguments for staff login with PIN. */
export interface StaffLoginArgs {
  username: string;
  pin: string;
}

/** A login session with user and role info. */
export interface LoginSessionDto {
  user_id: string;
  display_name: string;
  role_name: string;
  role_id: string;
}

/** Result of a successful staff login. */
export interface StaffLoginResult {
  session: LoginSessionDto;
}

/** Arguments for checking if a username exists. */
export interface CheckUsernameArgs {
  username: string;
}

/** Result of a username existence check. */
export interface CheckUsernameResult {
  found: boolean;
  is_active: boolean;
}

/** Check if a username exists and is active before proceeding to PIN. */
export const checkUsername = (args: CheckUsernameArgs): Promise<CheckUsernameResult> =>
  invoke<CheckUsernameResult>('staff_check_username', { args });

/** Authenticate a staff member with username and PIN. */
export const staffLogin = (args: StaffLoginArgs): Promise<StaffLoginResult> =>
  invoke<StaffLoginResult>('staff_login', { args });

// ── Bootstrap (first-owner, no auth required) ─────────────────────

/** Arguments for bootstrapping the first owner account. */
export interface BootstrapOwnerArgs {
  username: string;
  pin: string;
  display_name: string;
}

/** Result of bootstrapping the first owner account. */
export interface BootstrapOwnerResult {
  session: LoginSessionDto;
}

/**
 * Create the first owner user in a fresh installation.
 *
 * Only succeeds when no staff accounts exist yet. Seeds default roles
 * automatically and returns a login session so the front-end can
 * auto-login immediately.
 */
export const bootstrapOwner = (args: BootstrapOwnerArgs): Promise<BootstrapOwnerResult> =>
  invoke<BootstrapOwnerResult>('bootstrap_owner', { args });

// ── Staff Management ──────────────────────────────────────────────

/** A staff member record. */
export interface StaffMemberDto {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  role_name: string;
  is_active: boolean;
}

/** A role definition with display name and description. */
export interface RoleDto {
  id: string;
  name: string;
  description: string;
}

/** Arguments for creating a new staff member. */
export interface CreateStaffArgs {
  username: string;
  pin: string;
  display_name: string;
  role_id: string;
  /** User ID of the caller (from LoginSession). Used for permission check. */
  caller_user_id: string;
}

/** Arguments for updating an existing staff member. */
export interface UpdateStaffArgs {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  is_active: boolean;
  /** User ID of the caller (from LoginSession). Used for permission check. */
  caller_user_id: string;
}

/** List all staff members. */
export const listStaff = (): Promise<StaffMemberDto[]> =>
  invoke<StaffMemberDto[]>('list_staff');

/** List all roles. */
export const listRoles = (): Promise<RoleDto[]> =>
  invoke<RoleDto[]>('list_roles');

/** Create a new staff member. */
export const createStaff = (args: CreateStaffArgs): Promise<StaffMemberDto> =>
  invoke<StaffMemberDto>('create_staff', { args });

/** Update an existing staff member. */
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
