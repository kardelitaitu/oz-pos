//! `useCloudSync` — shared cloud-sync state and side effects.
//!
//! Owns:
//!   • Non-secret config in `localStorage` (`retail-sync-*` keys)
//!   • Auth token in the secure IPC channel (`sync.auth_token` setting)
//!   • Auto-sync interval lifecycle
//!   • Real sync round-trip via the `sync_run` Tauri command that
//!     flushes the local offline queue (sales, credit, voids, ...) to the
//!     configured cloud server and marks each item as `synced` or `failed`.
//!   • Status pill (`online` / `offline`) derived from the backend result.
//!
//! Designed so the desktop Options screen and the tablet Settings
//! sheet can share one source of truth.

import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  pendingSyncCount,
  syncPull,
  syncRun,
  updateSyncSettings,
} from '@/api/offline';

// ── Public types ───────────────────────────────────────────────────

/** Input shape for adding a notification toast. */
export type ToastInput = {
  message: string;
  type: 'success' | 'error' | 'warning' | 'info';
};

/** Connection status to the cloud sync server. */
export type SyncStatus = 'online' | 'offline';

/** Minimal Fluent-like localisation interface expected by the sync hook. */
export interface L10nLike {
  getString: (key: string, vars?: Record<string, string | number>) => string | null;
}

/** Dependencies injected into the sync hook for side-effect isolation. */
export interface UseCloudSyncDeps {
  addToast: (t: ToastInput) => void;
  l10n: L10nLike;
}

/** Full return type of the `useCloudSync` hook. */
export interface UseCloudSyncReturn {
  // State
  enabled: boolean;
  serverURL: string;
  autoMinutes: number;
  token: string;
  tokenLoaded: boolean;
  lastAt: string | null;
  pending: number;
  syncing: boolean;
  pulling: boolean;
  status: SyncStatus;
  // Setters
  setEnabled: (v: boolean) => void;
  setServerURL: (v: string) => void;
  setAutoMinutes: (v: number) => void;
  setToken: (v: string) => void;
  // Actions
  syncNow: () => Promise<void>;
  pullFromServer: () => Promise<void>;
  testConnection: () => Promise<void>;
  // Persistence
  persist: (userId: string) => Promise<void>;
}

// ── Storage keys ────────────────────────────────────────────────────

const LS_ENABLED = 'retail-sync-enabled';
const LS_SERVER = 'retail-sync-server';
const LS_INTERVAL = 'retail-sync-interval';
const LS_LAST = 'retail-sync-last';
const LS_PENDING = 'retail-sync-pending';
const IPC_TOKEN_KEY = 'sync.auth_token';

// ── Tunables ───────────────────────────────────────────────────────
//
// `sync_run` is a real Tauri command; no in-process latency simulation.
const SIMULATED_LATENCY_TEST_MS = 400;
const MIN_AUTO_INTERVAL_MS = 60_000;

// ── Local helpers ───────────────────────────────────────────────────

function readBoolLS(key: string, fallback: boolean): boolean {
  try {
    return localStorage.getItem(key) === 'true';
  } catch {
    return fallback;
  }
}

function readStringLS(key: string, fallback: string): string {
  try {
    return localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

function readIntLS(key: string, fallback: number): number {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return fallback;
    const n = Number(raw);
    return Number.isFinite(n) ? n : fallback;
  } catch {
    return fallback;
  }
}

function t(
  l10n: L10nLike,
  key: string,
  fallback: string,
  vars?: Record<string, string | number>,
): string {
  // Fluent's `getString` returns `null` when the key is missing; fall
  // back to a default string so non-localised shells still render.
  return l10n.getString(key, vars) ?? fallback;
}

// ── Hook ────────────────────────────────────────────────────────────

/**
 * Shared cloud-sync state and side-effects for the desktop Options screen
 * and tablet Settings sheet.
 *
 * Owns non-secret config in `localStorage`, the auth token in the secure
 * IPC channel, the auto-sync interval lifecycle, and the real sync
 * round-trip to the configured cloud server. Returns setters, actions
 * (`syncNow`, `pullFromServer`, `testConnection`), and a `persist`
 * function that flushes everything to the backend.
 */
export function useCloudSync(deps: UseCloudSyncDeps): UseCloudSyncReturn {
  const { addToast, l10n } = deps;

  // Initial state — lazy from localStorage so reloads preserve config.
  const [enabled, setEnabled] = useState<boolean>(() => readBoolLS(LS_ENABLED, false));
  const [serverURL, setServerURL] = useState<string>(() => readStringLS(LS_SERVER, ''));
  const [autoMinutes, setAutoMinutes] = useState<number>(() => readIntLS(LS_INTERVAL, 0));
  const [token, setToken] = useState<string>('');
  const [tokenLoaded, setTokenLoaded] = useState<boolean>(false);
  const [lastAt, setLastAt] = useState<string | null>(() => {
    const raw = readStringLS(LS_LAST, '');
    return raw.length > 0 ? raw : null;
  });
  const [pending, setPending] = useState<number>(() => readIntLS(LS_PENDING, 0));
  const [syncing, setSyncing] = useState<boolean>(false);
  const [pulling, setPulling] = useState<boolean>(false);
  const [status, setStatus] = useState<SyncStatus>('offline');

  // Load auth token from secure IPC once on mount.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const tok: string | null = await invoke('get_setting', { key: IPC_TOKEN_KEY });
        if (!cancelled && tok) setToken(tok);
      } catch {
        /* IPC may not be available yet; load still completes so the UI un-blocks */
      }
      if (!cancelled) setTokenLoaded(true);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Pull the authoritative pending count from the offline queue so the
  // UI reflects whatever the background sync daemon has accumulated.
  const refreshPendingCount = useCallback(async (): Promise<void> => {
    try {
      const count = await pendingSyncCount();
      setPending(count);
      try {
        localStorage.setItem(LS_PENDING, String(count));
      } catch {
        /* localStorage unavailable */
      }
    } catch {
      /* IPC may fail in tests; keep the lazy value. */
    }
  }, []);

  useEffect(() => {
    void refreshPendingCount();
  }, [refreshPendingCount]);

  // In-flight ref so the auto-sync interval can't double-fire a sync_run
  // while another round-trip is already mid-flight (the React state would
  // be stale inside the memoised callback).
  const syncingRef = useRef(false);
  const pullingRef = useRef(false);

  const syncNow = useCallback(async (): Promise<void> => {
    // Guard against overlapping runs (auto-sync interval can fire while
    // a manual sync is still in flight).
    if (syncingRef.current) return;
    if (!serverURL.trim()) {
      addToast({
        message: t(l10n, 'settings-sync-toast-fail', 'Sync failed — check server URL and token'),
        type: 'error',
      });
      return;
    }
    syncingRef.current = true;
    setSyncing(true);
    try {
      // Real round-trip via `sync_run` Tauri command — iterates the
      // pending offline queue (sales, credit, voids, ...) and POSTs each
      // to the configured cloud server. Items are then marked `synced` or
      // `failed` in the local DB.
      const result = await syncRun();
      const now = new Date().toLocaleString();
      setLastAt(now);
      setStatus(result.error ? 'offline' : 'online');
      try {
        localStorage.setItem(LS_LAST, now);
      } catch {
        /* localStorage may be disabled (e.g. private mode) */
      }
      await refreshPendingCount();

      if (result.synced > 0 && !result.error) {
        addToast({
          message: t(l10n, 'settings-sync-toast-success', 'Sync completed successfully'),
          type: 'success',
        });
      } else if (result.failed > 0) {
        addToast({
          message:
            result.error ??
            t(l10n, 'settings-sync-toast-fail', 'Sync failed — check server URL and token'),
          type: 'error',
        });
      } else if (result.error) {
        addToast({ message: result.error, type: 'error' });
      }
    } catch {
      setStatus('offline');
      addToast({
        message: t(l10n, 'settings-sync-toast-fail', 'Sync failed — check server URL and token'),
        type: 'error',
      });
    } finally {
      syncingRef.current = false;
      setSyncing(false);
    }
  }, [serverURL, addToast, l10n, refreshPendingCount]);

  // `syncNow` is closed over `serverURL` so we hold a ref so the
  // auto-sync interval always invokes the freshest version without
  // tearing down and recreating the timer on every URL change.
  const syncNowRef = useRef<() => Promise<void>>(syncNow);
  useEffect(() => {
    syncNowRef.current = syncNow;
  }, [syncNow]);

  // Auto-sync loop — fires whenever `enabled` and `autoMinutes`
  // both become positive; cleans up on reduce/disable.
  useEffect(() => {
    if (!enabled || autoMinutes <= 0) return;
    const ms = Math.max(MIN_AUTO_INTERVAL_MS, autoMinutes * 60_000);
    const handle = window.setInterval(() => {
      // Fire-and-forget — we don't block the timer on the network.
      void syncNowRef.current();
    }, ms);
    return () => window.clearInterval(handle);
  }, [enabled, autoMinutes]);

  const testConnection = useCallback(async (): Promise<void> => {
    setSyncing(true);
    try {
      await new Promise((resolve) => window.setTimeout(resolve, SIMULATED_LATENCY_TEST_MS));
      const reachable = serverURL.trim().length > 0;
      setStatus(reachable ? 'online' : 'offline');
      addToast({
        message: reachable
          ? t(l10n, 'settings-sync-toast-test-success', 'Connection test passed')
          : t(l10n, 'settings-sync-toast-test-fail', 'Could not reach server'),
        type: reachable ? 'success' : 'error',
      });
    } catch {
      setStatus('offline');
      addToast({
        message: t(l10n, 'settings-sync-toast-test-fail', 'Could not reach server'),
        type: 'error',
      });
    } finally {
      setSyncing(false);
    }
  }, [serverURL, addToast, l10n]);

  /**
   * Persist everything. Non-secret config is mirrored to localStorage
   * (UI hydration) AND routed into the backend settings DB via the
   * `update_sync_settings` IPC — the latter is what `sync_run` reads
   * from to know the server URL / API key / enabled flag. The auth
   * token is also kept in the secure `set_setting` channel for
   * backward compatibility. Errors on individual channels are
   * swallowed so a partial save is still surfaced elsewhere instead
   * of failing the parent Save flow.
   */
  const persist = useCallback(
    async (userId: string): Promise<void> => {
      try {
        localStorage.setItem(LS_ENABLED, String(enabled));
        localStorage.setItem(LS_SERVER, serverURL);
        localStorage.setItem(LS_INTERVAL, String(autoMinutes));
      } catch {
        /* localStorage unavailable */
      }
      try {
        await updateSyncSettings({
          serverUrl: serverURL.trim() || null,
          apiKey: token.trim() || null,
          enabled,
        });
      } catch {
        /* IPC may fail in tests; non-secret still in localStorage */
      }
      // Only persist the token when the user actually typed one;
      // an empty value would overwrite a token previously saved
      // via the Settings page (which now also writes here).
      if (token.trim()) {
        try {
          await invoke('set_setting', { key: IPC_TOKEN_KEY, value: token, user_id: userId });
        } catch {
          /* settings DB unavailable */
        }
      }
    },
    [enabled, serverURL, autoMinutes, token],
  );

  /**
   * Pull a server snapshot (products, tax rates, users) and overwrite
   * the local cache. The caller is expected to have shown a
   * `window.confirm` dialog before invoking this action.
   *
   * The action shares the same `serverURL.trim()` guard as `syncNow`
   * and the same in-flight ref pattern so it can never overlap itself
   * (a second click while a pull is in flight is a no-op).
   */
  const pullFromServer = useCallback(async (): Promise<void> => {
    if (pullingRef.current) return;
    if (!serverURL.trim()) {
      addToast({
        message: t(l10n, 'settings-sync-pull-toast-fail', 'Pull failed — check server URL and token'),
        type: 'error',
      });
      return;
    }
    pullingRef.current = true;
    setPulling(true);
    try {
      const result = await syncPull();
      const total = result.productsPulled + result.taxRatesPulled + result.usersPulled;
      setStatus(result.error ? 'offline' : 'online');
      if (result.error) {
        addToast({
          message: result.error,
          type: 'error',
        });
        return;
      }
      if (total > 0) {
        addToast({
          message: t(
            l10n,
            'settings-sync-pull-toast-success',
            'Pulled { $products } products, { $tax_rates } tax rates, { $users } users from server',
            {
              products: result.productsPulled,
              tax_rates: result.taxRatesPulled,
              users: result.usersPulled,
            },
          ),
          type: 'success',
        });
      } else {
        addToast({
          message: t(
            l10n,
            'settings-sync-pull-toast-empty',
            'Server snapshot was empty — nothing to pull',
          ),
          type: 'info',
        });
      }
    } catch {
      setStatus('offline');
      addToast({
        message: t(l10n, 'settings-sync-pull-toast-fail', 'Pull failed — check server URL and token'),
        type: 'error',
      });
    } finally {
      pullingRef.current = false;
      setPulling(false);
    }
  }, [serverURL, addToast, l10n]);

  return {
    enabled,
    serverURL,
    autoMinutes,
    token,
    tokenLoaded,
    lastAt,
    pending,
    syncing,
    pulling,
    status,
    setEnabled,
    setServerURL,
    setAutoMinutes,
    setToken,
    syncNow,
    pullFromServer,
    testConnection,
    persist,
  };
}
