//! `useHealthLatency` — ping a service health endpoint and report latency.
//!
//! Polls `{url}/api/health` at configurable intervals with exponential
//! backoff on failure. Returns a state enum and round-trip latency so
//! the StatusBar can color icons by latency thresholds.

import { useState, useEffect, useRef } from 'react';

export type HealthState = 'checking' | 'online' | 'offline';

export interface HealthLatencyInfo {
  state: HealthState;
  latencyMs: number | null;
}

/**
 * Ping a health endpoint at regular intervals and report latency.
 *
 * - `checking` — initial state before the first ping resolves.
 * - `online` — last ping returned HTTP 200.
 * - `offline` — last ping failed (network error or non-200).
 *
 * On failure the hook retries with exponential backoff (2 s → 60 s max).
 * Listens for OS `online` / `offline` events for instant reaction.
 */
export function useHealthLatency(
  url: string,
  minIntervalMs = 30_000,
  maxIntervalMs = 120_000,
): HealthLatencyInfo {
  const [state, setState] = useState<HealthState>('checking');
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const controllerRef = useRef<AbortController | null>(null);
  const genRef = useRef(0);

  useEffect(() => {
    if (!url) {
      setState('offline');
      setLatencyMs(null);
      return;
    }

    let mounted = true;
    let timeoutId: number | undefined;
    let currentBackoff = 2000;

    const checkConnection = async (gen: number): Promise<boolean> => {
      const pingUrl = url.endsWith('/') ? url + 'api/health' : url + '/api/health';
      const start = performance.now();

      // Supersede any in-flight check.
      genRef.current += 1;
      controllerRef.current?.abort();
      const controller = new AbortController();
      controllerRef.current = controller;

      try {
        const timeoutId = window.setTimeout(() => controller.abort(), 5000);
        const res = await fetch(pingUrl, { signal: controller.signal, method: 'GET' });
        clearTimeout(timeoutId);

        if (!mounted || gen !== genRef.current) return false;

        if (res.ok) {
          setLatencyMs(Math.round(performance.now() - start));
          setState('online');
          return true;
        }
        setState('offline');
        setLatencyMs(null);
        return false;
      } catch {
        if (mounted && gen === genRef.current) {
          setState('offline');
          setLatencyMs(null);
        }
        return false;
      }
    };

    const scheduleNext = (success: boolean) => {
      if (!mounted) return;
      let delay: number;
      if (success) {
        currentBackoff = 2000;
        delay = Math.floor(Math.random() * (maxIntervalMs - minIntervalMs + 1)) + minIntervalMs;
      } else {
        delay = currentBackoff + Math.random() * 1000;
        currentBackoff = Math.min(currentBackoff * 2, 60_000);
      }
      timeoutId = window.setTimeout(() => { void runCheck(); }, delay);
    };

    const runCheck = () => {
      const gen = ++genRef.current;
      if (!navigator.onLine) {
        if (mounted && gen === genRef.current) {
          setState('offline');
          setLatencyMs(null);
        }
        scheduleNext(false);
        return;
      }
      checkConnection(gen).then(success => {
        if (gen === genRef.current) scheduleNext(success);
      });
    };

    const handleOnline = () => {
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      currentBackoff = 1000;
      void runCheck();
    };

    const handleOffline = () => {
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      genRef.current += 1;
      controllerRef.current?.abort();
      if (mounted) {
        setState('offline');
        setLatencyMs(null);
      }
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    void runCheck();

    return () => {
      mounted = false;
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      genRef.current += 1;
      controllerRef.current?.abort();
      controllerRef.current = null;
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [url, minIntervalMs, maxIntervalMs]);

  return { state, latencyMs };
}