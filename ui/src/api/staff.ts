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
