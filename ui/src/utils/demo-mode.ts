/**
 * Demo-data gate (LOAD-03).
 *
 * Sample/demo catalog data may ONLY be shown when running a development
 * build (`import.meta.env.DEV`) or when explicitly opted in via
 * `VITE_DEMO_MODE=1`. A production build can never surface demo products —
 * even when the live IPC request fails — so a failed load can't be
 * mistaken for live inventory, and cashiers can't select products that
 * are not in the store catalog.
 *
 * Mirror of the perf-metrics `enabled()` gate so recording and exposure
 * stay consistent with the build environment.
 */
export function isDemoMode(): boolean {
  return import.meta.env.DEV || import.meta.env['VITE_DEMO_MODE'] === '1';
}
