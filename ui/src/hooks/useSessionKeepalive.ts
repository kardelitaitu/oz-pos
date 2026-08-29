//! Keep the current session alive while a long-lived screen is mounted.
//!
//! Sessions carry a TTL (`session.ttl_seconds`, default 24h). A dashboard
//! left open — analytics, reports — would otherwise expire silently while
//! the user reads it, and every later command would fail with
//! `InvalidSession` ("Your session has expired. Please sign in again.").
//!
//! This hook pings `session_keepalive` periodically, which extends the
//! session's `expires_at` on the backend (desktop + tablet). It pauses
//! while the document is hidden (tab switched away, window minimized) and
//! only pings when a valid token is present. Failures are swallowed: a
//! genuinely expired session surfaces through the real command that fails,
//! not this heartbeat — which is best-effort by design.

import { useEffect, useRef } from 'react';
import { loggedInvoke } from '@/utils/logged-invoke';

/** How often to extend the session TTL while the screen is visible. */
const KEEPALIVE_INTERVAL_MS = 10 * 60 * 1000; // 10 minutes

/**
 * Ping `session_keepalive` every {@link KEEPALIVE_INTERVAL_MS} while the
 * document is visible, extending the current session's expiry.
 *
 * @param sessionToken The active session token (from `useWorkspace`).
 *   Pass `null`/`undefined`/empty to disable the heartbeat.
 */
export function useSessionKeepalive(sessionToken: string | null | undefined): void {
  const tokenRef = useRef(sessionToken);
  tokenRef.current = sessionToken;

  useEffect(() => {
    if (!sessionToken) return;

    let cancelled = false;

    const ping = async (): Promise<void> => {
      const token = tokenRef.current;
      if (cancelled || !token) return;
      try {
        await loggedInvoke('session_keepalive', { sessionToken: token });
      } catch {
        // Best-effort heartbeat: a dead session is reported by the
        // command that actually fails, not by this background ping.
      }
    };

    const onVisibility = (): void => {
      if (document.visibilityState === 'visible') {
        // Resume pinging on return to the tab — and immediately touch the
        // session so an idle-background period never carries the session
        // past its TTL without a fresh extension.
        void ping();
      }
    };

    // Touch immediately (covers screens mounted mid-session), then on a
    // fixed cadence while the document is visible. `visibilitychange`
    // handles the pause/resume lifecycle.
    void ping();
    const timer = window.setInterval(() => {
      if (document.visibilityState === 'visible') void ping();
    }, KEEPALIVE_INTERVAL_MS);
    document.addEventListener('visibilitychange', onVisibility);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, [sessionToken]);
}
