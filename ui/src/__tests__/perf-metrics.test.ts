import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import {
  recordIpcTiming,
  recordMark,
  getPerfReport,
  resetPerfMetrics,
  installPerfProbe,
} from '@/utils/perf-metrics';

/**
 * PERF-06 — unit tests for the aggregate-only runtime metrics module.
 *
 * Verifies: bounded sampling, percentile math, aggregate-only output
 * (no payloads), mark capture, reset, and the window probe install.
 */
describe('perf-metrics (PERF-06)', () => {
  beforeEach(() => {
    resetPerfMetrics();
  });

  it('computes p50/p95/max aggregates per command', () => {
    // 50 samples (under the 64-sample ring cap): 1..50 (ms).
    // p50 = sorted[ceil(50/100*50)-1] = sorted[24] = 25.
    // p95 = sorted[ceil(95/100*50)-1] = sorted[47] = 48.
    for (let i = 1; i <= 50; i++) {
      recordIpcTiming('list_products_scoped', i);
    }
    const report = getPerfReport();
    const agg = report.ipc.find((r) => r.command === 'list_products_scoped');
    expect(agg).toBeDefined();
    expect(agg!.count).toBe(50);
    expect(agg!.max).toBe(50);
    expect(agg!.p95).toBeGreaterThanOrEqual(47);
    expect(agg!.p95).toBeLessThanOrEqual(49);
    expect(agg!.p50).toBeGreaterThanOrEqual(24);
    expect(agg!.p50).toBeLessThanOrEqual(26);
  });

  it('bounded the sample ring buffer per command', () => {
    for (let i = 0; i < 1000; i++) {
      recordIpcTiming('get_active_shift_scoped', i);
    }
    const report = getPerfReport();
    const agg = report.ipc.find((r) => r.command === 'get_active_shift_scoped');
    expect(agg!.count).toBeLessThanOrEqual(64);
    expect(agg!.max).toBe(999); // most recent 64 samples are 936..999
  });

  it('separates aggregates per command (no cross-talk)', () => {
    recordIpcTiming('a', 10);
    recordIpcTiming('b', 500);
    recordIpcTiming('a', 20);
    const report = getPerfReport();
    const a = report.ipc.find((r) => r.command === 'a')!;
    const b = report.ipc.find((r) => r.command === 'b')!;
    expect(a.count).toBe(2);
    expect(a.max).toBe(20);
    expect(b.count).toBe(1);
    expect(b.max).toBe(500);
    // Sorted by p95 descending — b (500ms) before a.
    expect(report.ipc[0]!.command).toBe('b');
  });

  it('records named marks with monotonic timestamps', () => {
    recordMark('oz:shell-ready');
    recordMark('oz:pos-interactive');
    const report = getPerfReport();
    expect(report.marks.map((m) => m.name)).toEqual([
      'oz:shell-ready',
      'oz:pos-interactive',
    ]);
    expect(report.marks[0]!.time).toBeLessThanOrEqual(report.marks[1]!.time);
  });

  it('ignores non-finite IPC durations', () => {
    recordIpcTiming('x', Number.NaN);
    recordIpcTiming('x', Number.POSITIVE_INFINITY);
    expect(getPerfReport().ipc).toHaveLength(0);
  });

  it('reset clears all collected data', () => {
    recordIpcTiming('a', 1);
    recordMark('oz:shell-ready');
    resetPerfMetrics();
    const report = getPerfReport();
    expect(report.ipc).toHaveLength(0);
    expect(report.marks).toHaveLength(0);
  });

  it('installPerfProbe exposes the aggregate report on window in dev', () => {
    installPerfProbe();
    // In dev (vitest jsdom, import.meta.env.DEV) the probe is installed.
    if (import.meta.env.DEV) {
      expect(window.__OZ_PERF__).toBeDefined();
      expect(typeof window.__OZ_PERF__!.getPerfReport).toBe('function');
    }
  });

  it('installPerfProbe does NOT expose the probe in non-dev builds', () => {
    // Simulate a production build: DEV=false and VITE_PERF_METRICS unset.
    vi.stubEnv('DEV', false);
    vi.stubEnv('VITE_PERF_METRICS', undefined);
    delete window.__OZ_PERF__;
    installPerfProbe();
    expect(window.__OZ_PERF__).toBeUndefined();
    vi.unstubAllEnvs();
  });

  it('installPerfProbe DOES expose the probe in prod when VITE_PERF_METRICS=1', () => {
    vi.stubEnv('DEV', false);
    vi.stubEnv('VITE_PERF_METRICS', '1');
    delete window.__OZ_PERF__;
    installPerfProbe();
    expect(window.__OZ_PERF__).toBeDefined();
    vi.unstubAllEnvs();
  });

  afterEach(() => {
    delete window.__OZ_PERF__;
    vi.unstubAllEnvs();
  });
});
