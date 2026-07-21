// ── System: ping, version info ───────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

/** Placeholder type for the ping command (returns "pong" as a string). */
export interface PingResult {
  // ping returns "pong" as a string
}

/** Application version and build information. */
export interface VersionInfo {
  name: string;
  version: string;
  rustVersion: string;
  target: string;
}

/** Ping the backend to verify connectivity. Returns "pong" on success. */
export const ping = (): Promise<string> => invoke<string>('ping');

/** Get the application version and build details. */
export const getVersion = (): Promise<VersionInfo> =>
  invoke<VersionInfo>('version');

/** Get the local IP address of the device. */
export const getLocalIp = (): Promise<string> =>
  invoke<string>('get_local_ip');

/** Get the stable device identifier (hostname) for terminal binding. */
export const getDeviceId = (): Promise<string> =>
  invoke<string>('get_device_id');
