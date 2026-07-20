import { useState, useEffect, useRef } from 'react';

// ── Types ─────────────────────────────────────────────────────────────

/** SLA threshold level for a KDS ticket (P3-1 progressive escalation). */
export type SlaLevel = 'green' | 'yellow' | 'red';

/** Return type of useTicketSla. */
export interface TicketSlaResult {
  /** Elapsed seconds since the ticket was received. */
  elapsedSeconds: number;
  /** SLA threshold level. */
  level: SlaLevel;
  /** True when the ticket is ≥15 min overdue (red background + urgent badge). */
  urgent: boolean;
  /** Human-readable elapsed time string (e.g. "5m 30s"). */
  display: string;
}

// ── Constants ─────────────────────────────────────────────────────────

/** Green threshold: < 300 seconds (5 minutes). */
const GREEN_MAX = 300;

/** Yellow threshold: < 600 seconds (10 minutes). */
const YELLOW_MAX = 600;

/** Red-urgent threshold: ≥ 900 seconds (15 minutes). Adds bg + badge. */
const RED_URGENT = 900;

/** Tick interval in milliseconds (every second). */
const TICK_MS = 1000;

// ── Helpers ───────────────────────────────────────────────────────────

/** Compute the SLA level from elapsed seconds (P3-1 progressive thresholds). */
function computeLevel(elapsed: number): SlaLevel {
  if (elapsed < GREEN_MAX) return 'green';
  if (elapsed < YELLOW_MAX) return 'yellow';
  return 'red';
}

/** Format elapsed seconds into a short display string like "5m 30s". */
function formatElapsed(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  if (m === 0) return `${s}s`;
  if (s === 0) return `${m}m`;
  return `${m}m ${s}s`;
}

// ── Hook ──────────────────────────────────────────────────────────────

/**
 * `useTicketSla` — computes elapsed seconds and returns an SLA
 * threshold level for a KDS ticket given its `created_at` ISO-8601
 * timestamp.
 *
 * Progressive escalation thresholds (P3-1):
 * - **Green**:     < 5 min (300 s) — on time
 * - **Yellow**:    5–10 min (300–600 s) — amber border + pulse
 * - **Red**:       10–15 min (600–900 s) — red border + shake animation
 * - **Red urgent**: ≥ 15 min (≥ 900 s, `urgent: true`) — red bg + badge
 *
 * The `urgent` flag indicates the ticket has exceeded the 15-minute
 * threshold and needs urgent visual escalation (red background + badge).
 *
 * Updates every second via `setInterval`. Automatically cleans up
 * the interval on unmount or when `createdAt` changes.
 */
export function useTicketSla(createdAt: string): TicketSlaResult {
  // Store the parsed epoch in a ref so we don't re-parse on every tick.
  const createdAtMs = useRef(Date.now());

  // Update the ref whenever createdAt changes.
  if (createdAt) {
    createdAtMs.current = new Date(createdAt).getTime();
  }

  const compute = (): TicketSlaResult => {
    const elapsed = Math.max(0, Math.floor((Date.now() - createdAtMs.current) / 1000));
    const level = computeLevel(elapsed);
    const urgent = elapsed >= RED_URGENT;
    return { elapsedSeconds: elapsed, level, urgent, display: formatElapsed(elapsed) };
  };

  const [result, setResult] = useState<TicketSlaResult>(compute);

  useEffect(() => {
    // Recompute immediately if createdAt changes.
    setResult(compute());

    const tick = () => setResult(compute());
    const interval = setInterval(tick, TICK_MS);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [createdAt]);

  return result;
}
