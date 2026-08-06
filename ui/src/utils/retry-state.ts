// ui/src/utils/retry-state.ts
//
// Shared async-load state contract (ERR-09).
//
// Screens historically hand-rolled `loading` + `error` booleans, which let
// retry/refresh flows drift: some cleared current data on every reload
// (blank/skeleton transitions), others preserved rows with no "refreshing"
// indicator. This module standardizes the derived phase:
//
//   idle       — nothing loaded yet, not loading, no data
//   loading    — initial load in flight, no data to show yet
//   refreshing — a reload/retry in flight WITH data already visible (rows
//                must stay on screen; only the indicator changes)
//   success    — last load succeeded (data present)
//   error      — last load failed AND no usable data is present (retry is
//                the primary recovery)
//   staleError — last load failed but previous data remains visible (a
//                non-blocking error notice accompanies the stale rows)
//
// Retry/refresh semantics (documented contract):
//   - Retry after an initial failure resets pagination and filters: the
//     first page is reloaded from scratch (callers pass `{ reset: true }`).
//   - Refresh while data is visible preserves the current rows, pagination
//     cursor, and filter values; it must NOT blank the screen.
//   - The phase must be surfaced in accessible status text (aria-live) so
//     assistive tech hears the retry intent.

/** Derived async phase for a list/detail load. */
export type AsyncPhase =
  | 'idle'
  | 'loading'
  | 'refreshing'
  | 'success'
  | 'error'
  | 'staleError';

export interface AsyncPhaseInput {
  /** True while any load/refresh is in flight. */
  loading: boolean;
  /** True when the last load failed. */
  error: boolean;
  /** True when usable data is currently displayed. */
  hasData: boolean;
}

/**
 * Derive the standardized async phase from raw loading/error/data flags.
 *
 * The key rule (ERR-09): when a reload is in flight and data is already on
 * screen the phase is `refreshing`, not `loading` — callers must preserve
 * the existing rows and only surface a "refreshing" status.
 */
export function deriveAsyncPhase({
  loading,
  error,
  hasData,
}: AsyncPhaseInput): AsyncPhase {
  if (loading) {
    return hasData ? 'refreshing' : 'loading';
  }
  if (error) {
    return hasData ? 'staleError' : 'error';
  }
  return hasData ? 'success' : 'idle';
}
