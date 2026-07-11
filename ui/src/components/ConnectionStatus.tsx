import { useState, useEffect } from 'react';
import './ConnectionStatus.css';

interface ConnectionStatusProps {
  label: string;
  url: string;
  minIntervalMs?: number;
  maxIntervalMs?: number;
}

export default function ConnectionStatus({ 
  label, 
  url, 
  minIntervalMs = 30000, 
  maxIntervalMs = 120000 
}: ConnectionStatusProps) {
  const [latency, setLatency] = useState<number | null>(null);
  const [status, setStatus] = useState<'checking' | 'online' | 'offline'>('checking');

  useEffect(() => {
    let mounted = true;
    
    const checkConnection = async () => {
      if (!url) {
        if (mounted) {
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
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 5000);
        
        const res = await fetch(pingUrl, { signal: controller.signal, method: 'GET' });
        clearTimeout(timeoutId);
        
        if (res.ok && mounted) {
          const end = performance.now();
          setLatency(Math.round(end - start));
          setStatus('online');
          return true;
        } else if (mounted) {
          setStatus('offline');
          setLatency(null);
          return false;
        }
      } catch (err) {
        if (mounted) {
          setStatus('offline');
          setLatency(null);
        }
        return false;
      }
      return false;
    };
    
    let timeoutId: ReturnType<typeof setTimeout>;
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
        runCheck();
      }, nextDelay);
    };

    const runCheck = async () => {
      // Don't bother pinging if OS says we are physically offline
      if (!navigator.onLine) {
        if (mounted) {
          setStatus('offline');
          setLatency(null);
        }
        scheduleNextCheck(false);
        return;
      }
      
      const success = await checkConnection();
      scheduleNextCheck(success);
    };

    // Listen for OS network changes for instant reaction
    const handleOnline = () => {
      clearTimeout(timeoutId);
      currentBackoff = 1000; // Fast ping when connection restores
      runCheck();
    };
    
    const handleOffline = () => {
      clearTimeout(timeoutId);
      if (mounted) {
        setStatus('offline');
        setLatency(null);
      }
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // Initial check
    runCheck();
    
    return () => {
      mounted = false;
      clearTimeout(timeoutId);
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
