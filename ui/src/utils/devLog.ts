/**
 * Shared dev-log bus — the ONE pattern for in-app diagnostics.
 *
 * Diagnostics are internal signals (corruption detected and repaired, an
 * expected-but-noteworthy fallback taken, a non-fatal backend hiccup) that
 * belong in the devtools console, never in user-facing copy. Every module
 * that needs to surface one should go through `devLog` instead of calling
 * `console.warn` directly:
 *
 *   devLog.warn('topology', 'restore-time guard dropped 1 dangling wire');
 *
 *  - The line reaches the devtools console as `[topology] ...` (the
 *    existing `[prefix]` convention, kept byte-identical).
 *  - Every entry is ALSO recorded in a bounded buffer (see MAX_DEV_LOG_ENTRIES)
 *    so tests can assert on diagnostics via getDevLog()/clearDevLog()
 *    instead of spying on console internals — the same seam future
 *    diagnostics use, which is what makes the pattern testable.
 *  - The buffer is capped, so a long-running session cannot grow unbounded.
 */

/** Severity of a diagnostic; selects the devtools console method. */
export type DevLogLevel = 'info' | 'warn' | 'error';

/** One recorded diagnostic — the shape tests assert on. */
export interface DevLogEntry {
  level: DevLogLevel;
  /** Becomes the `[source]` bracket prefix on the console line. */
  source: string;
  /** The message body (no prefix — the bus adds `[source] `). */
  message: string;
}

/** Bounded-history cap: keeps the recorder free for any session length. */
const MAX_DEV_LOG_ENTRIES = 100;

let entries: DevLogEntry[] = [];

function emit(level: DevLogLevel, source: string, message: string): void {
  const line = `[${source}] ${message}`;
  if (level === 'error') console.error(line);
  else if (level === 'warn') console.warn(line);
  else console.info(line);
  entries.push({ level, source, message });
  if (entries.length > MAX_DEV_LOG_ENTRIES) {
    entries.splice(0, entries.length - MAX_DEV_LOG_ENTRIES);
  }
}

/** The bus: `devLog.<level>(source, message)`. */
export const devLog: Record<DevLogLevel, (source: string, message: string) => void> = {
  info: (source, message) => emit('info', source, message),
  warn: (source, message) => emit('warn', source, message),
  error: (source, message) => emit('error', source, message),
};

/** Snapshot of everything emitted so far (newest last, ≤ MAX_DEV_LOG_ENTRIES). */
export function getDevLog(): readonly DevLogEntry[] {
  return [...entries];
}

/** Reset the buffer — used between test scopes. */
export function clearDevLog(): void {
  entries = [];
}
