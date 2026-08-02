import { invoke } from '@tauri-apps/api/core';
import { recordIpcTiming } from './perf-metrics';

/** Invoke a Tauri command with console timing logs for dev observability. */
export async function loggedInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const start = performance.now();
  if (import.meta.env.DEV) console.log(`[tauri] ${cmd} → started`);
  try {
    const result = await invoke<T>(cmd, args);
    const ms = performance.now() - start;
    if (import.meta.env.DEV) console.log(`[tauri] ${cmd} → succeeded (${Math.round(ms)}ms)`);
    // PERF-06: capture aggregate-only IPC latency (p50/p95/max per command).
    recordIpcTiming(cmd, ms);
    return result;
  } catch (err) {
    const ms = performance.now() - start;
    if (import.meta.env.DEV) console.log(`[tauri] ${cmd} → failed (${Math.round(ms)}ms)`, err);
    recordIpcTiming(cmd, ms);
    throw err;
  }
}
