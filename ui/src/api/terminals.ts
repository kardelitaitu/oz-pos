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

export const registerTerminal = (args: RegisterTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('register_terminal', { args });

export const updateTerminal = (args: UpdateTerminalArgs): Promise<{ id: string }> =>
  invoke<{ id: string }>('update_terminal', { args });

export const pingTerminal = (id: string): Promise<void> =>
  invoke<void>('ping_terminal', { id });

export const deleteTerminal = (id: string): Promise<void> =>
  invoke('delete_terminal', { id });

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
  terminalId: string,
  feature: string,
  enabled: boolean,
): Promise<void> =>
  invoke<void>('set_terminal_override', { terminalId, feature, enabled });

export const deleteTerminalOverride = (
  terminalId: string,
  feature: string,
): Promise<void> =>
  invoke<void>('delete_terminal_override', { terminalId, feature });
