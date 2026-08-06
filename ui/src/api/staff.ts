// ── Staff: Login, Bootstrap, CRUD ──────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

// ── Auth ──────────────────────────────────────────────────────────

/** Arguments for staff login with PIN. */
export interface StaffLoginArgs {
  username: string;
  pin: string;
  /** Optional device/terminal id for per-device abuse controls (STAFF-07). */
  device_id?: string;
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
  /**
   * Short-lived ticket for the pre-session workspace picker (audit/06).
   * Passed to listWorkspaces / listWorkspaceScreens until createSession
   * returns the opaque session token.
   */
  picker_ticket: string;
}

/** Arguments for checking if a username exists. */
export interface CheckUsernameArgs {
  username: string;
}

/**
 * Result of the uniform username pre-check (STAFF-06).
 *
 * Always `{ proceed: true }` — the pre-check never reveals whether an
 * account exists or is active, so it cannot be used to enumerate staff.
 */
export interface CheckUsernameResult {
  proceed: boolean;
}

/** Check if a username exists and is active before proceeding to PIN. */
export const checkUsername = (args: CheckUsernameArgs): Promise<CheckUsernameResult> =>
  loggedInvoke<CheckUsernameResult>('staff_check_username', { args });

/** Authenticate a staff member with username and PIN. */
export const staffLogin = (args: StaffLoginArgs): Promise<StaffLoginResult> =>
  loggedInvoke<StaffLoginResult>('staff_login', { args });

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
  /** Short-lived ticket for the pre-session workspace picker (audit/06). */
  picker_ticket: string;
}

/**
 * Create the first owner user in a fresh installation.
 *
 * Only succeeds when no staff accounts exist yet. Seeds default roles
 * automatically and returns a login session so the front-end can
 * auto-login immediately.
 */
export const bootstrapOwner = (args: BootstrapOwnerArgs): Promise<BootstrapOwnerResult> =>
  loggedInvoke<BootstrapOwnerResult>('bootstrap_owner', { args });

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

// ── Session-scoped Staff Management (ADR #7 · audit/06 STAFF-01) ───
//
// These are the secure replacements. The caller identity is resolved from
// the session token on the backend — the args carry NO caller_user_id.

/** Arguments for creating a staff member via a session-scoped command. */
export interface CreateStaffScopedArgs {
  username: string;
  pin: string;
  display_name: string;
  role_id: string;
}

/** Arguments for updating a staff member via a session-scoped command. */
export interface UpdateStaffScopedArgs {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  is_active: boolean;
  /** STAFF-03: optional new PIN; hashed server-side. Omit to keep current. */
  pin?: string;
  /**
   * STAFF-05: workspace key assignment applied atomically with the profile
   * update (single IPC call). Omit to leave workspace assignments untouched.
   */
  workspace_keys?: string[];
}

/** List all staff members (caller resolved from session token). */
export const listStaffScoped = (sessionToken: string): Promise<StaffMemberDto[]> =>
  loggedInvoke<StaffMemberDto[]>('list_staff_scoped', { sessionToken });

/** List all roles (caller resolved from session token). */
export const listRolesScoped = (sessionToken: string): Promise<RoleDto[]> =>
  loggedInvoke<RoleDto[]>('list_roles_scoped', { sessionToken });

/** Create a new staff member (caller resolved from session token). */
export const createStaffScoped = (
  sessionToken: string,
  args: CreateStaffScopedArgs,
): Promise<StaffMemberDto> =>
  loggedInvoke<StaffMemberDto>('create_staff_scoped', { sessionToken, args });

/** Update an existing staff member (caller resolved from session token). */
export const updateStaffScoped = (
  sessionToken: string,
  args: UpdateStaffScopedArgs,
): Promise<StaffMemberDto> =>
  loggedInvoke<StaffMemberDto>('update_staff_scoped', { sessionToken, args });

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
  loggedInvoke<CreateSessionResult>('create_session', { args });

/**
 * Destroy an active session token (logout or store switch).
 *
 * After this call, any command using the old token will fail
 * with InvalidSession.
 */
export const destroySession = (sessionToken: string): Promise<void> =>
  loggedInvoke<void>('destroy_session', { sessionToken });
