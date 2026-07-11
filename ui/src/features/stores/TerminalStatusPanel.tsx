import { useState, useEffect, useCallback, useRef } from 'react';
import { listTerminals, type TerminalDto } from '@/api/terminals';
import { Card } from '@/components/Card';
import './TerminalStatusPanel.css';

const ONLINE_THRESHOLD_MS = 5 * 60 * 1000;

function formatLastSeen(iso: string | null): string {
  if (!iso) return 'Never';
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 60_000) return 'Just now';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return new Date(iso).toLocaleDateString();
}

function isOnline(lastSeenAt: string | null): boolean {
  if (!lastSeenAt) return false;
  return Date.now() - new Date(lastSeenAt).getTime() < ONLINE_THRESHOLD_MS;
}

interface TerminalStatusPanelProps {
  refreshTrigger?: number;
}

/** Terminal status panel card — displays live online/offline status of all terminals with auto-refresh every 30 seconds. */
export default function TerminalStatusPanel({ refreshTrigger }: TerminalStatusPanelProps) {
  const [terminals, setTerminals] = useState<TerminalDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const load = useCallback(async () => {
    try {
      const data = await listTerminals();
      setTerminals(data);
      setError(null);
    } catch {
      setError('Failed to load terminals');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load, refreshTrigger]);

  useEffect(() => {
    intervalRef.current = setInterval(load, 30_000);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [load]);

  const onlineCount = terminals.filter((t) => isOnline(t.lastSeenAt)).length;

  return (
    <Card
      shadow="sm"
      header={
        <div className="terminal-status-header">
          <span className="terminal-status-title">Terminal Status</span>
          {!loading && (
            <span className="terminal-status-count">
              {onlineCount} / {terminals.length} online
            </span>
          )}
        </div>
      }
    >
      {loading ? (
        <p className="terminal-status-loading">Loading terminals…</p>
      ) : error ? (
        <p className="terminal-status-error">{error}</p>
      ) : terminals.length === 0 ? (
        <p className="terminal-status-empty">No terminals registered.</p>
      ) : (
        <div className="terminal-status-list" role="list" aria-label="Terminal statuses">
          {terminals.map((terminal) => {
            const online = isOnline(terminal.lastSeenAt);
            return (
              <div key={terminal.id} className="terminal-status-item" role="listitem">
                <span
                  className={`terminal-status-dot ${online ? 'terminal-status-dot--online' : 'terminal-status-dot--offline'}`}
                  aria-label={online ? 'Online' : 'Offline'}
                />
                <div className="terminal-status-info">
                  <span className="terminal-status-item-name">{terminal.name}</span>
                  <span className="terminal-status-item-device">{terminal.deviceId}</span>
                </div>
                <span className="terminal-status-item-time">
                  {formatLastSeen(terminal.lastSeenAt)}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}
