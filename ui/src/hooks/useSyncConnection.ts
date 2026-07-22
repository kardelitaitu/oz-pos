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

/** Return type of the `useSyncConnection` hook. */
export interface SyncConnectionStatus {
  /** Current connectivity state. */
  state: SyncConnectionState;
  /** Round-trip latency in milliseconds, or null if unknown/offline. */
  latencyMs: number | null;
  /** Human-readable status string from the server, e.g. "Connected (12ms)". */
  label: string;
}

const POLL_INTERVAL_MS = 60_000;

/**
 * Poll the cloud sync server health endpoint on mount and every 60 s.
 *
 * Returns `{ state, latencyMs, label }` suitable for rendering a
 * connection indicator dot in the StatusBar.
 *
 * - `'checking'` — initial state before the first ping resolves.
 * - `'connected'` — last ping succeeded (`ok: true`).
 * - `'disconnected'` — last ping failed (network error or `ok: false`).
 */
export function useSyncConnection(): SyncConnectionStatus {
  const [state, setState] = useState<SyncConnectionState>('checking');
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const [label, setLabel] = useState<string>('Checking…');
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
          setLabel(result.status);
        } else {
          setState('disconnected');
          setLatencyMs(null);
          setLabel(result.status);
        }
      } catch {
        if (!mountedRef.current) return;
        setState('disconnected');
        setLatencyMs(null);
        setLabel('Disconnected');
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

  return { state, latencyMs, label };
}
