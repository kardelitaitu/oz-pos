import { useCallback, useEffect, useState } from 'react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import type { StockAlertEvent } from '@/api/inventory';
import { getActiveStockAlerts, acknowledgeStockAlert } from '@/api/inventory';

import './StockAlertPanel.css';

interface StockAlertPanelProps {
  /** Inventory location ID to scope alerts to. */
  locationId: string;
  /** Optional interval (ms) for polling; 0 or omit disables polling. */
  pollIntervalMs?: number;
  /** Max alerts to display. */
  maxAlerts?: number;
}

/**
 * StockAlertPanel — ADR-18 §9e-iii dashboard widget.
 *
 * Displays active stock alerts for a given location with product info,
 * severity indicators, and acknowledge buttons. Supports optional polling.
 */
export function StockAlertPanel({
  locationId,
  pollIntervalMs = 30_000,
  maxAlerts = 20,
}: StockAlertPanelProps) {
  const { sessionToken } = useWorkspace();

  const [alerts, setAlerts] = useState<StockAlertEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [acknowledging, setAcknowledging] = useState<Set<string>>(new Set());

  const token = sessionToken ?? '';

  // ── Fetch alerts ──────────────────────────────────────────────────

  const fetchAlerts = useCallback(async () => {
    if (!token || !locationId) return;
    try {
      setError(null);
      const data = await getActiveStockAlerts(token, locationId);
      setAlerts(data.slice(0, maxAlerts));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load alerts');
    } finally {
      setLoading(false);
    }
  }, [token, locationId, maxAlerts]);

  useEffect(() => {
    fetchAlerts();

    if (pollIntervalMs > 0) {
      const interval = setInterval(fetchAlerts, pollIntervalMs);
      return () => clearInterval(interval);
    }
  }, [fetchAlerts, pollIntervalMs]);

  // ── Acknowledge ──────────────────────────────────────────────────

  const handleAcknowledge = useCallback(
    async (alertId: string) => {
      if (!token) return;
      setAcknowledging((prev) => new Set(prev).add(alertId));
      try {
        await acknowledgeStockAlert(token, alertId);
        // Remove from local state immediately for snappy UX
        setAlerts((prev) => prev.filter((a) => a.id !== alertId));
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to acknowledge');
      } finally {
        setAcknowledging((prev) => {
          const next = new Set(prev);
          next.delete(alertId);
          return next;
        });
      }
    },
    [token],
  );

  // ── Severity ─────────────────────────────────────────────────────

  const isCritical = (alert: StockAlertEvent) => alert.current_qty === 0;

  const formatTime = (iso: string) => {
    try {
      const d = new Date(iso);
      const now = new Date();
      const diffMs = now.getTime() - d.getTime();
      const diffMin = Math.floor(diffMs / 60000);
      if (diffMin < 1) return 'Just now';
      if (diffMin < 60) return `${diffMin}m ago`;
      const diffHrs = Math.floor(diffMin / 60);
      if (diffHrs < 24) return `${diffHrs}h ago`;
      return d.toLocaleDateString();
    } catch {
      return iso;
    }
  };

  // ── Loading state ───────────────────────────────────────────────

  if (loading) {
    return (
      <div className="stock-alert-panel" role="region" aria-label="Loading stock alerts">
        <div className="stock-alert-panel-header">
          <span className="stock-alert-panel-title">Stock Alerts</span>
        </div>
        <div className="stock-alert-loading">
          <span>Loading alerts...</span>
        </div>
      </div>
    );
  }

  // ── Error state ─────────────────────────────────────────────────

  if (error && alerts.length === 0) {
    return (
      <div className="stock-alert-panel" role="region" aria-label="Stock alerts">
        <div className="stock-alert-panel-header">
          <span className="stock-alert-panel-title">Stock Alerts</span>
        </div>
        <div className="stock-alert-error" role="alert">
          {error}
        </div>
      </div>
    );
  }

  return (
    <div className="stock-alert-panel" role="region" aria-label="Stock alerts panel">
      {/* Header */}
      <div className="stock-alert-panel-header">
        <span className="stock-alert-panel-title">
          Stock Alerts
        </span>
        {alerts.length > 0 && (
          <span className="stock-alert-panel-badge" aria-label={`${alerts.length} active alerts`}>
            {alerts.length}
          </span>
        )}
      </div>

      {/* Empty state */}
      {alerts.length === 0 && (
        <div className="stock-alert-empty">
          <svg className="stock-alert-empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
          </svg>
          <span className="stock-alert-empty-text">
            No active alerts
          </span>
        </div>
      )}

      {/* Alert list */}
      {alerts.map((alert) => (
        <div
          key={alert.id}
          className={`stock-alert-card ${isCritical(alert) ? 'stock-alert-card--critical' : 'stock-alert-card--warning'}`}
          role="listitem"
        >
          <div className="stock-alert-product-row">
            <span className="stock-alert-sku">{alert.product_sku}</span>
            <span className="stock-alert-product-name" title={alert.product_name}>
              {alert.product_name}
            </span>
          </div>

          <div className="stock-alert-metrics">
            <span className="stock-alert-metric-current">
              <span className={`stock-alert-severity-dot ${isCritical(alert) ? '' : 'stock-alert-severity-dot--warning'}`} />
              <LocalizedText label="Stock">
                <strong>{alert.current_qty}</strong>
              </LocalizedText>
            </span>
            <span className="stock-alert-metric-threshold">
              <LocalizedText label="Threshold:">{alert.threshold}</LocalizedText>
            </span>
          </div>

          <div className="stock-alert-footer">
            <span className="stock-alert-time" title={alert.triggered_at}>
              {formatTime(alert.triggered_at)}
            </span>
            <button
              type="button"
              className="stock-alert-ack-btn"
              onClick={() => handleAcknowledge(alert.id)}
              disabled={acknowledging.has(alert.id)}
              aria-label={`Acknowledge alert for ${alert.product_name}`}
            >
              {acknowledging.has(alert.id) ? '...' : 'Ack'}
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}

/** Inline helper to wrap text in a label span without importing another component. */
function LocalizedText({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <>
      <span className="sr-only">{label}</span>
      {children}
    </>
  );
}
