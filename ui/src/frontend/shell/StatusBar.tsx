import { useState, useEffect, useRef } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useLocalization } from '@fluent/react';
import { ping } from '@/api/system';
import './StatusBar.css';

type ConnectionState = 'checking' | 'connected' | 'disconnected';

export default function StatusBar() {
  const { l10n } = useLocalization();
  const { loading: authLoading } = useAuth();
  const [connection, setConnection] = useState<ConnectionState>('checking');
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;

    const check = async () => {
      try {
        const res = await ping();
        if (!mountedRef.current) return;
        setConnection(res === 'pong' ? 'connected' : 'disconnected');
      } catch {
        if (!mountedRef.current) return;
        setConnection(prev => prev === 'checking' ? 'disconnected' : 'disconnected');
      }
      if (mountedRef.current) {
        timer = setTimeout(check, 15000);
      }
    };

    check();
    return () => clearTimeout(timer);
  }, []);

  const hasActivity = authLoading;

  return (
    <div className="status-bar" role="status" aria-live="polite">
      <span
        className={`status-bar-dot status-bar-dot--${connection}`}
        aria-label={l10n.getString(connection === 'connected' ? 'status-bar-connected' : connection === 'disconnected' ? 'status-bar-disconnected' : 'status-bar-checking')}
      />
      {hasActivity && (
        <span className="status-bar-activity">
          <span className="status-bar-spinner" />
          {authLoading && l10n.getString('status-bar-authenticating')}
        </span>
      )}
    </div>
  );
}
