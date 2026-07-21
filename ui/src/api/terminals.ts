// ── Terminal Management ───────────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

/** A registered POS terminal. */
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

/** Arguments for registering a new POS terminal. */
export interface RegisterTerminalArgs {
  name: string;
  deviceId: string;
  terminalSecret?: string | null;
  metadata?: string | null;
}

/** Arguments for updating an existing terminal. */
export interface UpdateTerminalArgs {
  id: string;
  name: string;
  deviceId: string;
  isActive: boolean;
  metadata?: string | null;
}

/** List all registered terminals. */
export const listTerminals = (): Promise<TerminalDto[]> =>
  loggedInvoke<TerminalDto[]>('list_terminals');

/** List terminals (scoped — ADR #7). */
export const listTerminalsScoped = (sessionToken: string): Promise<TerminalDto[]> =>
  loggedInvoke<TerminalDto[]>('list_terminals_scoped', { sessionToken });

/** Get a single terminal by its identifier. */
export const getTerminal = (id: string): Promise<TerminalDto | null> =>
  loggedInvoke<TerminalDto | null>('get_terminal', { id });

/** Get a terminal (scoped — ADR #7). */
export const getTerminalScoped = (sessionToken: string, id: string): Promise<TerminalDto | null> =>
  loggedInvoke<TerminalDto | null>('get_terminal_scoped', { sessionToken, id });

/** Register a new POS terminal. */
export const registerTerminal = (userId: string, args: RegisterTerminalArgs): Promise<{ id: string }> =>
  loggedInvoke<{ id: string }>('register_terminal', { userId, args });

/** Register a terminal (scoped — ADR #7). */
export const registerTerminalScoped = (sessionToken: string, args: RegisterTerminalArgs): Promise<{ id: string }> =>
  loggedInvoke<{ id: string }>('register_terminal_scoped', { sessionToken, args });

/** Update an existing terminal's details. */
export const updateTerminal = (userId: string, args: UpdateTerminalArgs): Promise<{ id: string }> =>
  loggedInvoke<{ id: string }>('update_terminal', { userId, args });

/** Update a terminal (scoped — ADR #7). */
export const updateTerminalScoped = (sessionToken: string, args: UpdateTerminalArgs): Promise<{ id: string }> =>
  loggedInvoke<{ id: string }>('update_terminal_scoped', { sessionToken, args });

/** Ping a terminal to check it is reachable. */
export const pingTerminal = (id: string): Promise<void> =>
  loggedInvoke<void>('ping_terminal', { id });

/** Ping a terminal (scoped — ADR #7). */
export const pingTerminalScoped = (sessionToken: string, id: string): Promise<void> =>
  loggedInvoke<void>('ping_terminal_scoped', { sessionToken, id });

/** Delete a terminal registration. */
export const deleteTerminal = (userId: string, id: string): Promise<void> =>
  loggedInvoke('delete_terminal', { userId, id });

/** Delete a terminal (scoped — ADR #7). */
export const deleteTerminalScoped = (sessionToken: string, id: string): Promise<void> =>
  loggedInvoke<void>('delete_terminal_scoped', { sessionToken, id });

// ── Feature Overrides ──────────────────────────────────────────────

/** A feature override applied to a specific terminal. */
export interface TerminalFeatureOverride {
  terminalId: string;
  feature: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

/** List feature overrides for a terminal. */
export const listTerminalOverrides = (terminalId: string): Promise<TerminalFeatureOverride[]> =>
  loggedInvoke<TerminalFeatureOverride[]>('list_terminal_overrides', { terminalId });

/** List terminal overrides (scoped — ADR #7). */
export const listTerminalOverridesScoped = (sessionToken: string, terminalId: string): Promise<TerminalFeatureOverride[]> =>
  loggedInvoke<TerminalFeatureOverride[]>('list_terminal_overrides_scoped', { sessionToken, terminalId });

/** Enable or disable a feature override for a terminal. */
export const setTerminalOverride = (
  userId: string,
  terminalId: string,
  feature: string,
  enabled: boolean,
): Promise<void> =>
  loggedInvoke<void>('set_terminal_override', { userId, terminalId, feature, enabled });

/** Set terminal override (scoped — ADR #7). */
export const setTerminalOverrideScoped = (
  sessionToken: string,
  terminalId: string,
  feature: string,
  enabled: boolean,
): Promise<void> =>
  loggedInvoke<void>('set_terminal_override_scoped', { sessionToken, terminalId, feature, enabled });

/** Remove a feature override from a terminal. */
export const deleteTerminalOverride = (
  userId: string,
  terminalId: string,
  feature: string,
): Promise<void> =>
  loggedInvoke<void>('delete_terminal_override', { userId, terminalId, feature });

/** Delete terminal override (scoped — ADR #7). */
export const deleteTerminalOverrideScoped = (
  sessionToken: string,
  terminalId: string,
  feature: string,
): Promise<void> =>
  loggedInvoke<void>('delete_terminal_override_scoped', { sessionToken, terminalId, feature });

// ── Terminal Profiles ───────────────────────────────────────────────

/** A terminal profile defining its locked screen and profile type. */
export interface TerminalProfileDto {
  terminalId: string;
  profileType: string;
  lockedScreen: string | null;
  updatedAt: string;
}

/** Get the profile for a terminal. */
export const getTerminalProfile = (
  terminalId: string,
): Promise<TerminalProfileDto | null> =>
  loggedInvoke<TerminalProfileDto | null>('get_terminal_profile', { terminalId });

/** Get terminal profile (scoped — ADR #7). */
export const getTerminalProfileScoped = (
  sessionToken: string,
  terminalId: string,
): Promise<TerminalProfileDto | null> =>
  loggedInvoke<TerminalProfileDto | null>('get_terminal_profile_scoped', { sessionToken, terminalId });

/** Set or update a terminal's profile. */
export const setTerminalProfile = (
  userId: string,
  terminalId: string,
  profileType: string,
  lockedScreen: string | null,
): Promise<void> =>
  loggedInvoke<void>('set_terminal_profile', {
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
  loggedInvoke<void>('set_terminal_profile_scoped', {
    sessionToken,
    args: { terminalId, profileType, lockedScreen },
  });

/** List all terminal profiles. */
export const listTerminalProfiles = (): Promise<TerminalProfileDto[]> =>
  loggedInvoke<TerminalProfileDto[]>('list_terminal_profiles');

/** List terminal profiles (scoped — ADR #7). */
export const listTerminalProfilesScoped = (sessionToken: string): Promise<TerminalProfileDto[]> =>
  loggedInvoke<TerminalProfileDto[]>('list_terminal_profiles_scoped', { sessionToken });

/** Delete a terminal's profile. */
export const deleteTerminalProfile = (
  userId: string,
  terminalId: string,
): Promise<void> =>
  loggedInvoke<void>('delete_terminal_profile', { userId, terminalId });

/** Delete terminal profile (scoped — ADR #7). */
export const deleteTerminalProfileScoped = (
  sessionToken: string,
  terminalId: string,
): Promise<void> =>
  loggedInvoke<void>('delete_terminal_profile_scoped', { sessionToken, terminalId });

// ── Device Binding (ADR #4 Phase 3) ────────────────────────────────

/** Device binding status with HMAC signature validation result. */
export interface DeviceBindingDto {
  bounded: boolean;
  boundStoreId: string | null;
  boundInstanceId: string | null;
  signatureValid: boolean;
}

/** Get a terminal's device binding and validate its HMAC signature. */
export const getDeviceBinding = (terminalId: string): Promise<DeviceBindingDto> =>
  loggedInvoke<DeviceBindingDto>('get_device_binding', { terminalId });

/** Get device binding (scoped — ADR #7). */
export const getDeviceBindingScoped = (sessionToken: string, terminalId: string): Promise<DeviceBindingDto> =>
  loggedInvoke<DeviceBindingDto>('get_device_binding_scoped', { sessionToken, terminalId });

/** Set (or update) a terminal's device binding with HMAC signature. */
export const setDeviceBinding = (
  userId: string,
  terminalId: string,
  boundStoreId: string,
  boundInstanceId: string,
): Promise<void> =>
  loggedInvoke<void>('set_device_binding', {
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
  loggedInvoke<void>('set_device_binding_scoped', {
    sessionToken,
    args: { terminalId, boundStoreId, boundInstanceId },
  });

/** Clear a terminal's device binding. */
export const clearDeviceBinding = (
  userId: string,
  terminalId: string,
): Promise<void> =>
  loggedInvoke<void>('clear_device_binding', { userId, terminalId });

/** Clear device binding (scoped — ADR #7). */
export const clearDeviceBindingScoped = (
  sessionToken: string,
  terminalId: string,
): Promise<void> =>
  loggedInvoke<void>('clear_device_binding_scoped', { sessionToken, terminalId });
