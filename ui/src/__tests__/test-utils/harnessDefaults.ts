/**
 * The session token the global WorkspaceContext mock hands to every test.
 *
 * Lives here rather than being inlined in test-setup.ts so a test can assert
 * against the real value instead of hardcoding it. That distinction matters:
 * before R36-07, seven components read the raw context object (which the harness
 * does NOT stub) and therefore received an empty token, and several tests had
 * baked `''` into their expectations -- e.g.
 * `expect(mockGetLowStockAlerts).toHaveBeenCalledWith(10, '')`. Those assertions
 * were documenting the defect, and passed only because the token was wrong.
 */
export const HARNESS_SESSION_TOKEN = 'mock-session-token';
