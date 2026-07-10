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

export const getTerminal = (id: string): Promise<TerminalDto | null> =>
  invoke<TerminalDto | null>('get_terminal', { id });

export const registerTerminal = (userId: string, args: RegisterTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('register_terminal', { userId, args });

export const updateTerminal = (userId: string, args: UpdateTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('update_terminal', { userId, args });

export const pingTerminal = (id: string): Promise<void> =>
  invoke<void>('ping_terminal', { id });

export const deleteTerminal = (userId: string, id: string): Promise<void> =>
  invoke('delete_terminal', { userId, id });

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

export const setTerminalOverride = (
  userId: string,
  terminalId: string,
  feature: string,
  enabled: boolean,
): Promise<void> =>
  invoke<void>('set_terminal_override', { userId, terminalId, feature, enabled });

export const deleteTerminalOverride = (
  userId: string,
  terminalId: string,
  feature: string,
): Promise<void> =>
  invoke<void>('delete_terminal_override', { userId, terminalId, feature });

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

export const listTerminalProfiles = (): Promise<TerminalProfileDto[]> =>
  invoke<TerminalProfileDto[]>('list_terminal_profiles');

export const deleteTerminalProfile = (
  userId: string,
  terminalId: string,
): Promise<void> =>
  invoke<void>('delete_terminal_profile', { userId, terminalId });

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

/** Clear a terminal's device binding. */
export const clearDeviceBinding = (
  userId: string,
  terminalId: string,
): Promise<void> =>
  invoke<void>('clear_device_binding', { userId, terminalId });
