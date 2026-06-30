// ── System: ping, version info ───────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

export interface PingResult {
  // ping returns "pong" as a string
}

export interface VersionInfo {
  name: string;
  version: string;
  rustVersion: string;
  target: string;
}

export const ping = (): Promise<string> => invoke<string>('ping');

export const getVersion = (): Promise<VersionInfo> =>
  invoke<VersionInfo>('version');
