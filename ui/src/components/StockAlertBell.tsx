import { useState, useEffect, useCallback } from 'react';
import { getActiveStockAlerts } from '@/api/inventory';
import './StockAlertBell.css';

interface StockAlertBellProps {
  /** Session token for scoped IPC calls. */
  sessionToken: string;
  /** Inventory location ID to scope alerts to. */
  locationId?: string;
  /** Called when the bell is clicked (navigates to products/inventory). */
  onClick: () => void;
}

/**
 * StockAlertBell — global header notification bell for stock alerts.
 *
 * Polls `active_stock_alerts_scoped` every 30s and displays a badge
 * with the count of active alerts. Clicking navigates to the
 * product management screen where the full StockAlertPanel drawer
 * can be opened.
 */
export default function StockAlertBell({
  sessionToken,
  locationId = 'default',
  onClick,
}: StockAlertBellProps) {
  const [count, setCount] = useState(0);

  const fetchCount = useCallback(async () => {
    if (!sessionToken) return;
    try {
      const alerts = await getActiveStockAlerts(sessionToken, locationId);
      setCount(alerts.length);
    } catch {
      // Silently ignore — badge just won't show count.
    }
  }, [sessionToken, locationId]);

  useEffect(() => {
    fetchCount();
    const interval = setInterval(fetchCount, 30_000);
    return () => clearInterval(interval);
  }, [fetchCount]);

  return (
    <button
      type="button"
      className="stock-alert-bell"
      onClick={onClick}
      aria-label={
        count > 0
          ? `${count} active stock alert${count !== 1 ? 's' : ''}`
          : 'No stock alerts'
      }
    >
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        width="20"
        height="20"
        aria-hidden="true"
      >
        <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
        <path d="M13.73 21a2 2 0 0 1-3.46 0" />
      </svg>
      {count > 0 && (
        <span className="stock-alert-bell-badge" aria-hidden="true">
          {count > 99 ? '99+' : count}
        </span>
      )}
    </button>
  );
}
