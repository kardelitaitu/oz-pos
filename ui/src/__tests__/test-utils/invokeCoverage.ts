/**
 * invokeCoverage — makes an unmocked Tauri command LOUD instead of silent.
 *
 * The problem this exists for (R36-02, docs/plans/0.0.36-backlog.md): five test
 * files mock `invoke` with a chain of `if (cmd === 'x') return …` handlers that
 * end in `Promise.reject(new Error('Unknown command: …'))`. When production code
 * calls a command the chain does not handle, the component swallows the
 * rejection, the test still passes, and the assertion is quietly about an error
 * state rather than the data path.
 *
 * Concretely in SalesDashboardScreen.test.tsx: the chain handled
 * `get_category_breakdown`, `get_hourly_heatmap` and `get_daily_revenue`, but
 * ui/src/api/reports.ts invokes the `_scoped` spelling of every one of them —
 * all 24 of its functions do. So three widgets were always rejecting, and
 * "shows KPI cards" was passing while rendering their failure state.
 *
 * Usage in a test file:
 *   beforeEach(() => { resetUnmatchedInvokes(); invokeMock.mockImplementation(fn) })
 *   afterEach(() => assertAllInvokesHandled('SalesDashboardScreen'))
 * and inside the mock's final fallback:
 *   recordUnmatchedInvoke(cmd)
 *   return Promise.reject(new Error(`Unknown command: ${cmd}`))
 *
 * The rejection is kept on purpose: components must still see the same behaviour
 * they see today, so adding this helper cannot change what is being tested. Only
 * the reporting changes — from silent to failing.
 */

let unmatched: string[] = [];

/** Record a command that reached the mock's fallback. Call it, do not inline it. */
export function recordUnmatchedInvoke(cmd: string): void {
  unmatched.push(cmd);
}

/** Clear the record. Call from beforeEach so one test cannot blame another. */
export function resetUnmatchedInvokes(): void {
  unmatched = [];
}

/** Commands seen by the fallback so far, deduped, sorted. Useful in failure text. */
export function getUnmatchedInvokes(): string[] {
  return [...new Set(unmatched)].sort();
}

/**
 * Fail if any command reached the fallback during this test.
 * `context` should name the screen so the message says what to go fix.
 */
export function assertAllInvokesHandled(context: string): void {
  const bad = getUnmatchedInvokes();
  if (bad.length === 0) return;
  throw new Error(
    `${context}: ${bad.length} Tauri command(s) had no mock handler and were ` +
    `rejected at runtime. The component swallowed them, so any assertion that ` +
    `looked green was exercising the error path, not the data path.\n` +
    `  unhandled: ${bad.join(', ')}\n` +
    `  Add a handler for each in the invokeMock.mockImplementation chain. ` +
    `Check the _scoped spelling: ui/src/api/* calls the scoped variant for ` +
    `anything that touches store data.`,
  );
}
