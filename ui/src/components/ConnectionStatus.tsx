import { useState, useEffect, useRef } from 'react';
import './ConnectionStatus.css';

interface ConnectionStatusProps {
  label: string;
  url: string;
  minIntervalMs?: number;
  maxIntervalMs?: number;
}

/**
 * Ping a service URL at intervals and display its reachability,
 * latency, and label. Uses exponential backoff on failure and
 * listens for OS-level `online` / `offline` events.
 */
export default function ConnectionStatus({ 
  label, 
  url, 
  minIntervalMs = 30000, 
  maxIntervalMs = 120000 
}: ConnectionStatusProps) {
  const [latency, setLatency] = useState<number | null>(null);
  const [status, setStatus] = useState<'checking' | 'online' | 'offline'>('checking');
  // ERR-08: active controller + check generation refs so a new check aborts
  // and supersedes an in-flight one, and only the latest generation schedules
  // the next timeout. A slower earlier response can no longer update the
  // indicator after a newer check has begun.
  const controllerRef = useRef<AbortController | null>(null);
  const genRef = useRef(0);

  useEffect(() => {
    let mounted = true;

    // ERR-08: run the check with a per-request AbortController + generation.
    // The generation is bumped before every check so only the latest one may
    // write state or schedule the next timeout; any previous in-flight fetch
    // is aborted via the ref-held controller.
    const checkConnection = async (
      gen: number,
      controller: AbortController,
    ): Promise<boolean> => {
      if (!url) {
        if (mounted && gen === genRef.current) {
          setStatus('offline');
          setLatency(null);
        }
        return false;
      }

      const start = performance.now();
      try {
        // Ping the health endpoint (add /api/health if it's a PocketBase URL, otherwise just ping the root)
        const pingUrl = url.endsWith('/') ? url + 'api/health' : url + '/api/health';

        // Use a short timeout so it doesn't hang forever
        const timeoutId = setTimeout(() => controller.abort(), 5000);
        const res = await fetch(pingUrl, { signal: controller.signal, method: 'GET' });
        clearTimeout(timeoutId);

        // Only the latest generation may update the indicator.
        if (!mounted || gen !== genRef.current) return false;

        if (res.ok) {
          const end = performance.now();
          setLatency(Math.round(end - start));
          setStatus('online');
          return true;
        }
        setStatus('offline');
        setLatency(null);
        return false;
      } catch {
        // Aborted/superseded requests must not clobber a newer check's state.
        if (mounted && gen === genRef.current) {
          setStatus('offline');
          setLatency(null);
        }
        return false;
      }
    };

    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    let currentBackoff = 2000; // Start backoff at 2s

    const scheduleNextCheck = (isSuccess: boolean) => {
      if (!mounted) return;

      let nextDelay = 0;

      if (isSuccess) {
        // If successful, reset backoff and use the standard jittered interval
        currentBackoff = 2000;
        nextDelay = Math.floor(Math.random() * (maxIntervalMs - minIntervalMs + 1)) + minIntervalMs;
      } else {
        // If failed, use exponential backoff (up to 60s max) + small jitter
        const jitter = Math.random() * 1000;
        nextDelay = currentBackoff + jitter;
        currentBackoff = Math.min(currentBackoff * 2, 60000);
      }

      timeoutId = setTimeout(() => {
        void runCheck();
      }, nextDelay);
    };

    const runCheck = async () => {
      // Supersede any in-flight check before starting a new one (ERR-08).
      const gen = ++genRef.current;
      controllerRef.current?.abort();
      const controller = new AbortController();
      controllerRef.current = controller;

      // Don't bother pinging if OS says we are physically offline
      if (!navigator.onLine) {
        if (mounted && gen === genRef.current) {
          setStatus('offline');
          setLatency(null);
        }
        scheduleNextCheck(false);
        return;
      }

      const success = await checkConnection(gen, controller);
      // Only the latest generation schedules the next timeout (ERR-08).
      if (gen === genRef.current) {
        scheduleNextCheck(success);
      }
    };

    // Listen for OS network changes for instant reaction
    const handleOnline = () => {
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      currentBackoff = 1000; // Fast ping when connection restores
      void runCheck();
    };

    const handleOffline = () => {
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      // Supersede any in-flight check so it cannot flip us back online.
      genRef.current += 1;
      controllerRef.current?.abort();
      if (mounted) {
        setStatus('offline');
        setLatency(null);
      }
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // Initial check
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

  // Determine indicator color
  let indicatorClass = 'status-indicator checking';
  if (status === 'online') {
    indicatorClass = latency && latency > 500 ? 'status-indicator warning' : 'status-indicator online';
  } else if (status === 'offline') {
    indicatorClass = 'status-indicator offline';
  }

  const tooltipText = status === 'online' 
    ? `${label}: Online (${latency}ms)`
    : status === 'offline' 
      ? `${label}: Offline`
      : `${label}: Checking...`;

  return (
    <div className="connection-status" title={tooltipText}>
      <span className={indicatorClass} />
      <span className="connection-label">{label}</span>
      {status === 'online' && <span className="connection-latency">{latency}ms</span>}
    </div>
  );
}
