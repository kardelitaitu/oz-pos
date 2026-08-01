//! `useSyncConnection` — lightweight sync server connectivity poller.
//!
//! Polls the cloud sync server's health endpoint every 60 s via the
//! `test_sync_connection` IPC command. Returns a simple state enum so
//! the StatusBar can show a green/red/yellow dot without pulling in
//! the full `useCloudSync` hook (which manages auth, localStorage, and
//! the sync cycle — too heavy for a header indicator).

import { useState, useEffect, useRef } from 'react';
import { testSyncConnection } from '@/api/offline';

/** Connection state to the cloud sync server. */
export type SyncConnectionState = 'checking' | 'connected' | 'disconnected';

/**
 * Return type of the `useSyncConnection` hook.
 *
 * SYNC-12: the hook is deliberately presentation-agnostic — it exposes only
 * raw state and latency, never user-visible label strings. Renderers
 * (StatusBar, login screens) localize at the boundary via Fluent keys, so
 * no hardcoded English (`Checking…` / `Disconnected`) can leak here.
 */
export interface SyncConnectionStatus {
  /** Current connectivity state. */
  state: SyncConnectionState;
  /** Round-trip latency in milliseconds, or null if unknown/offline. */
  latencyMs: number | null;
}

const POLL_INTERVAL_MS = 60_000;

/**
 * Poll the cloud sync server health endpoint on mount and every 60 s.
 *
 * Returns `{ state, latencyMs }` suitable for rendering a connection
 * indicator dot in the StatusBar.
 *
 * - `'checking'` — initial state before the first ping resolves.
 * - `'connected'` — last ping succeeded (`ok: true`).
 * - `'disconnected'` — last ping failed (network error or `ok: false`).
 */
export function useSyncConnection(): SyncConnectionStatus {
  const [state, setState] = useState<SyncConnectionState>('checking');
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;

    async function check() {
      try {
        const result = await testSyncConnection();
        if (!mountedRef.current) return;

        if (result.ok) {
          setState('connected');
          setLatencyMs(result.latencyMs);
        } else {
          setState('disconnected');
          setLatencyMs(null);
        }
      } catch {
        if (!mountedRef.current) return;
        setState('disconnected');
        setLatencyMs(null);
      }
    }

    // Initial check immediately.
    check();

    const interval = setInterval(check, POLL_INTERVAL_MS);
    return () => {
      mountedRef.current = false;
      clearInterval(interval);
    };
  }, []);

  return { state, latencyMs };
}
