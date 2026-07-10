// ── Terminal Management ───────────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

export interface TerminalDto {
  id: string;
  name: string;
  deviceId: string;
  isActive: boolean;
  lastSeenAt: string | null;
  metadata: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface RegisterTerminalArgs {
  name: string;
  deviceId: string;
  terminalSecret?: string | null;
  metadata?: string | null;
}

export interface UpdateTerminalArgs {
  id: string;
  name: string;
  deviceId: string;
  isActive: boolean;
  metadata?: string | null;
}

export const listTerminals = (): Promise<TerminalDto[]> =>
  invoke<TerminalDto[]>('list_terminals');

/** List terminals (scoped — ADR #7). */
export const listTerminalsScoped = (sessionToken: string): Promise<TerminalDto[]> =>
  invoke<TerminalDto[]>('list_terminals_scoped', { sessionToken });

export const getTerminal = (id: string): Promise<TerminalDto | null> =>
  invoke<TerminalDto | null>('get_terminal', { id });

/** Get a terminal (scoped — ADR #7). */
export const getTerminalScoped = (sessionToken: string, id: string): Promise<TerminalDto | null> =>
  invoke<TerminalDto | null>('get_terminal_scoped', { sessionToken, id });

export const registerTerminal = (userId: string, args: RegisterTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('register_terminal', { userId, args });

/** Register a terminal (scoped — ADR #7). */
export const registerTerminalScoped = (sessionToken: string, args: RegisterTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('register_terminal_scoped', { sessionToken, args });

export const updateTerminal = (userId: string, args: UpdateTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('update_terminal', { userId, args });

/** Update a terminal (scoped — ADR #7). */
export const updateTerminalScoped = (sessionToken: string, args: UpdateTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('update_terminal_scoped', { sessionToken, args });

export const pingTerminal = (id: string): Promise<void> =>
  invoke<void>('ping_terminal', { id });

/** Ping a terminal (scoped — ADR #7). */
export const pingTerminalScoped = (sessionToken: string, id: string): Promise<void> =>
  invoke<void>('ping_terminal_scoped', { sessionToken, id });

export const deleteTerminal = (userId: string, id: string): Promise<void> =>
  invoke('delete_terminal', { userId, id });

/** Delete a terminal (scoped — ADR #7). */
export const deleteTerminalScoped = (sessionToken: string, id: string): Promise<void> =>
  invoke<void>('delete_terminal_scoped', { sessionToken, id });

// ── Feature Overrides ──────────────────────────────────────────────

export interface TerminalFeatureOverride {
  terminalId: string;
  feature: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export const listTerminalOverrides = (terminalId: string): Promise<TerminalFeatureOverride[]> =>
  invoke<TerminalFeatureOverride[]>('list_terminal_overrides', { terminalId });

/** List terminal overrides (scoped — ADR #7). */
export const listTerminalOverridesScoped = (sessionToken: string, terminalId: string): Promise<TerminalFeatureOverride[]> =>
  invoke<TerminalFeatureOverride[]>('list_terminal_overrides_scoped', { sessionToken, terminalId });

export const setTerminalOverride = (
  userId: string,
  terminalId: string,
  feature: string,
  enabled: boolean,
): Promise<void> =>
  invoke<void>('set_terminal_override', { userId, terminalId, feature, enabled });

/** Set terminal override (scoped — ADR #7). */
export const setTerminalOverrideScoped = (
  sessionToken: string,
  terminalId: string,
  feature: string,
  enabled: boolean,
): Promise<void> =>
  invoke<void>('set_terminal_override_scoped', { sessionToken, terminalId, feature, enabled });

export const deleteTerminalOverride = (
  userId: string,
  terminalId: string,
  feature: string,
): Promise<void> =>
  invoke<void>('delete_terminal_override', { userId, terminalId, feature });

/** Delete terminal override (scoped — ADR #7). */
export const deleteTerminalOverrideScoped = (
  sessionToken: string,
  terminalId: string,
  feature: string,
): Promise<void> =>
  invoke<void>('delete_terminal_override_scoped', { sessionToken, terminalId, feature });

// ── Terminal Profiles ───────────────────────────────────────────────

export interface TerminalProfileDto {
  terminalId: string;
  profileType: string;
  lockedScreen: string | null;
  updatedAt: string;
}

export const getTerminalProfile = (
  terminalId: string,
): Promise<TerminalProfileDto | null> =>
  invoke<TerminalProfileDto | null>('get_terminal_profile', { terminalId });

/** Get terminal profile (scoped — ADR #7). */
export const getTerminalProfileScoped = (
  sessionToken: string,
  terminalId: string,
): Promise<TerminalProfileDto | null> =>
  invoke<TerminalProfileDto | null>('get_terminal_profile_scoped', { sessionToken, terminalId });

export const setTerminalProfile = (
  userId: string,
  terminalId: string,
  profileType: string,
  lockedScreen: string | null,
): Promise<void> =>
  invoke<void>('set_terminal_profile', {
    userId,
    args: { terminalId, profileType, lockedScreen },
  });

/** Set terminal profile (scoped — ADR #7). */
export const setTerminalProfileScoped = (
  sessionToken: string,
  terminalId: string,
  profileType: string,
  lockedScreen: string | null,
): Promise<void> =>
  invoke<void>('set_terminal_profile_scoped', {
    sessionToken,
    args: { terminalId, profileType, lockedScreen },
  });

export const listTerminalProfiles = (): Promise<TerminalProfileDto[]> =>
  invoke<TerminalProfileDto[]>('list_terminal_profiles');

/** List terminal profiles (scoped — ADR #7). */
export const listTerminalProfilesScoped = (sessionToken: string): Promise<TerminalProfileDto[]> =>
  invoke<TerminalProfileDto[]>('list_terminal_profiles_scoped', { sessionToken });

export const deleteTerminalProfile = (
  userId: string,
  terminalId: string,
): Promise<void> =>
  invoke<void>('delete_terminal_profile', { userId, terminalId });

/** Delete terminal profile (scoped — ADR #7). */
export const deleteTerminalProfileScoped = (
  sessionToken: string,
  terminalId: string,
): Promise<void> =>
  invoke<void>('delete_terminal_profile_scoped', { sessionToken, terminalId });

// ── Device Binding (ADR #4 Phase 3) ────────────────────────────────

export interface DeviceBindingDto {
  bounded: boolean;
  boundStoreId: string | null;
  boundInstanceId: string | null;
  signatureValid: boolean;
}

/** Get a terminal's device binding and validate its HMAC signature. */
export const getDeviceBinding = (terminalId: string): Promise<DeviceBindingDto> =>
  invoke<DeviceBindingDto>('get_device_binding', { terminalId });

/** Get device binding (scoped — ADR #7). */
export const getDeviceBindingScoped = (sessionToken: string, terminalId: string): Promise<DeviceBindingDto> =>
  invoke<DeviceBindingDto>('get_device_binding_scoped', { sessionToken, terminalId });

/** Set (or update) a terminal's device binding with HMAC signature. */
export const setDeviceBinding = (
  userId: string,
  terminalId: string,
  boundStoreId: string,
  boundInstanceId: string,
): Promise<void> =>
  invoke<void>('set_device_binding', {
    userId,
    args: { terminalId, boundStoreId, boundInstanceId },
  });

/** Set device binding (scoped — ADR #7). */
export const setDeviceBindingScoped = (
  sessionToken: string,
  terminalId: string,
  boundStoreId: string,
  boundInstanceId: string,
): Promise<void> =>
  invoke<void>('set_device_binding_scoped', {
    sessionToken,
    args: { terminalId, boundStoreId, boundInstanceId },
  });

/** Clear a terminal's device binding. */
export const clearDeviceBinding = (
  userId: string,
  terminalId: string,
): Promise<void> =>
  invoke<void>('clear_device_binding', { userId, terminalId });

/** Clear device binding (scoped — ADR #7). */
export const clearDeviceBindingScoped = (
  sessionToken: string,
  terminalId: string,
): Promise<void> =>
  invoke<void>('clear_device_binding_scoped', { sessionToken, terminalId });
