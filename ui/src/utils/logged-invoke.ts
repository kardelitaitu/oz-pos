import { invoke } from '@tauri-apps/api/core';

/** Invoke a Tauri command with console timing logs for dev observability. */
export async function loggedInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const start = performance.now();
  if (import.meta.env.DEV) console.log(`[tauri] ${cmd} → started`);
  try {
    const result = await invoke<T>(cmd, args);
    if (import.meta.env.DEV) console.log(`[tauri] ${cmd} → succeeded (${Math.round(performance.now() - start)}ms)`);
    return result;
  } catch (err) {
    if (import.meta.env.DEV) console.log(`[tauri] ${cmd} → failed (${Math.round(performance.now() - start)}ms)`, err);
    throw err;
  }
}
