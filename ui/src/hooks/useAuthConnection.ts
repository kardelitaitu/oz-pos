//! `useAuthConnection` — lightweight auth server connectivity poller.
//!
//! Polls the license server's `/api/health` endpoint via the `test_auth_connection`
//! IPC command (Rust-side HTTP, no CORS). Returns a simple state enum so the
//! StatusBar can show a green/red/yellow dot without pulling in the full
//! license-activation machinery.
//!
//! Mirrors `useSyncConnection` so both indicators use the same polling pattern.

import { useState, useEffect, useRef } from 'react';
import { testAuthConnection } from '@/api/license';

/** Connection state to the auth server. */
export type AuthConnectionState = 'checking' | 'connected' | 'disconnected';

/**
 * Return type of the `useAuthConnection` hook.
 */
export interface AuthConnectionStatus {
  /** Current connectivity state. */
  state: AuthConnectionState;
  /** Round-trip latency in milliseconds, or null if unknown/offline. */
  latencyMs: number | null;
}

const POLL_INTERVAL_MS = 60_000;
const RETRY_INTERVAL_MS = 5_000;

/**
 * Poll the auth server's health endpoint on mount and every 60 s while
 * connected. Retry every 5 s while disconnected so the login screen can
 * recover when the server becomes reachable again.
 *
 * Returns `{ state, latencyMs }` suitable for rendering a connection
 * indicator in the StatusBar.
 *
 * - `'checking'` — initial state before the first ping resolves.
 * - `'connected'` — last ping succeeded (`ok: true`).
 * - `'disconnected'` — last ping failed (network error or `ok: false`).
 */
export function useAuthConnection(): AuthConnectionStatus {
  const [state, setState] = useState<AuthConnectionState>('checking');
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    let timer: number | undefined;

    async function check() {
      let nextDelay = RETRY_INTERVAL_MS;
      try {
        const result = await testAuthConnection();
        if (!mountedRef.current) return;

        if (result.ok) {
          setState('connected');
          setLatencyMs(result.latencyMs);
          nextDelay = POLL_INTERVAL_MS;
        } else {
          setState('disconnected');
          setLatencyMs(null);
        }
      } catch {
        if (!mountedRef.current) return;
        setState('disconnected');
        setLatencyMs(null);
      }

      if (mountedRef.current) {
        timer = window.setTimeout(check, nextDelay);
      }
    }

    // Initial check immediately.
    void check();

    return () => {
      mountedRef.current = false;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, []);

  return { state, latencyMs };
}