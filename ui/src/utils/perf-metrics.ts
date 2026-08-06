/**
 * PERF-06 — Lightweight runtime performance metrics.
 *
 * Collects ONLY aggregate timings: per-command IPC counts/percentiles and
 * named point-in-time marks. Never captures payloads, request bodies, user
 * data, or raw samples beyond a bounded ring buffer per command — so the
 * collected data is safe to ship in dev/CI traces and safe for the
 * Playwright perf smoke suite to read (PERF-10).
 *
 * The probe is installed on `window.__OZ_PERF__` in dev builds (or when
 * `VITE_PERF_METRICS=1`) so automated checks can assert budgets against
 * real runtime numbers instead of build artifacts only.
 */

/** Bounded samples kept per command for percentile computation. */
const MAX_SAMPLES_PER_COMMAND = 64;

const ipcSamples = new Map<string, number[]>();
const marks: Array<{ name: string; time: number }> = [];

/**
 * Collection gate: dev builds always; production only when explicitly
 * opted in via `VITE_PERF_METRICS=1`. Mirrors the window-probe gate so
 * recording and exposure are always consistent.
 */
function enabled(): boolean {
  return import.meta.env.DEV || import.meta.env['VITE_PERF_METRICS'] === '1';
}

/** Record the wall-clock duration of one IPC round trip (aggregate only). */
export function recordIpcTiming(command: string, ms: number): void {
  if (!enabled() || !Number.isFinite(ms)) return;
  let samples = ipcSamples.get(command);
  if (!samples) {
    samples = [];
    ipcSamples.set(command, samples);
  }
  samples.push(ms);
  if (samples.length > MAX_SAMPLES_PER_COMMAND) {
    samples.shift();
  }
}

/** Record a named point-in-time mark (e.g. shell mount, POS interactive). */
export function recordMark(name: string): void {
  if (!enabled()) return;
  marks.push({ name, time: performance.now() });
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1);
  return sorted[Math.max(0, idx)] ?? 0;
}

export interface IpcAggregate {
  command: string;
  count: number;
  p50: number;
  p95: number;
  max: number;
}

export interface PerfReport {
  marks: Array<{ name: string; time: number }>;
  ipc: IpcAggregate[];
}

/** Snapshot the current aggregates, sorted by p95 descending (no payloads). */
export function getPerfReport(): PerfReport {
  const ipc: IpcAggregate[] = [];
  for (const [command, samples] of ipcSamples) {
    const sorted = [...samples].sort((a, b) => a - b);
    ipc.push({
      command,
      count: samples.length,
      p50: percentile(sorted, 50),
      p95: percentile(sorted, 95),
      max: sorted[sorted.length - 1] ?? 0,
    });
  }
  ipc.sort((a, b) => b.p95 - a.p95);
  return { marks: [...marks], ipc };
}

/** Clear all collected data (used by unit tests and between CI runs). */
export function resetPerfMetrics(): void {
  ipcSamples.clear();
  marks.length = 0;
}

declare global {
  interface Window {
    /** Aggregate-only runtime perf probe installed in dev / opt-in builds. */
    __OZ_PERF__?: { getPerfReport: () => PerfReport };
  }
}

/**
 * Install the window probe. Enabled in dev always, and in production when
 * `VITE_PERF_METRICS=1` — aggregates are safe, but the window surface is
 * still opt-in for prod builds.
 */
export function installPerfProbe(): void {
  if (enabled()) {
    window.__OZ_PERF__ = { getPerfReport };
  }
}
