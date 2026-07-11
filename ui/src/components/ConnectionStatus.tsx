import { useState, useEffect } from 'react';
import './ConnectionStatus.css';

interface ConnectionStatusProps {
  label: string;
  url: string;
  intervalMs?: number;
}

export default function ConnectionStatus({ label, url, intervalMs = 10000 }: ConnectionStatusProps) {
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
        return;
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
        } else if (mounted) {
          setStatus('offline');
          setLatency(null);
        }
      } catch (err) {
        if (mounted) {
          setStatus('offline');
          setLatency(null);
        }
      }
    };

    // Initial check
    checkConnection();
    
    // Polling interval
    const timer = setInterval(checkConnection, intervalMs);
    
    return () => {
      mounted = false;
      clearInterval(timer);
    };
  }, [url, intervalMs]);

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
