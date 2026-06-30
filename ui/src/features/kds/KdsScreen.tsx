import { useEffect, useState, useCallback } from 'react';
import { Localized } from '@fluent/react';
import { getKdsQueue, updateKdsStatus, type KdsOrder, type KdsStatus } from '@/api/kds';
import './KdsScreen.css';

const STATUS_ORDER: KdsStatus[] = ['pending', 'preparing', 'ready', 'served'];
const STATUS_LABELS: Record<KdsStatus, string> = {
  pending: 'Pending',
  preparing: 'Preparing',
  ready: 'Ready',
  served: 'Served',
  cancelled: 'Cancelled',
};

export default function KdsScreen() {
  const [orders, setOrders] = useState<KdsOrder[]>([]);
  const [error, setError] = useState<string | null>(null);

  const fetchOrders = useCallback(() => {
    getKdsQueue()
      .then(setOrders)
      .catch((e) => setError(e.message ?? String(e)));
  }, []);

  useEffect(() => {
    fetchOrders();
    const interval = setInterval(fetchOrders, 15000);
    return () => clearInterval(interval);
  }, [fetchOrders]);

  const advanceStatus = async (order: KdsOrder) => {
    const currentIdx = STATUS_ORDER.indexOf(order.status as KdsStatus);
    if (currentIdx < 0 || currentIdx >= STATUS_ORDER.length - 1) return;
    const nextStatus = STATUS_ORDER[currentIdx + 1]!;
    try {
      await updateKdsStatus(order.id, nextStatus);
      fetchOrders();
    } catch (e) {
      setError(String(e));
    }
  };

  const grouped = (status: KdsStatus) =>
    orders.filter((o) => o.status === status);

  return (
    <div className="kds" role="region" aria-label="Kitchen Display System">
      <div className="kds-header">
        <h1 className="kds-title"><Localized id="kds-title">Kitchen Display</Localized></h1>
        <span className="kds-order-count">{orders.length} orders</span>
      </div>
      {error && <p className="kds-error">{error}</p>}
      <div className="kds-columns">
        {(['pending', 'preparing', 'ready'] as KdsStatus[]).map((status) => (
          <div key={status} className={`kds-column kds-column--${status}`}>
            <h2 className="kds-column-title">
              {STATUS_LABELS[status]}
              <span className="kds-column-count">{grouped(status).length}</span>
            </h2>
            <div className="kds-tickets">
              {grouped(status).length === 0 ? (
                <p className="kds-empty"><Localized id="kds-no-orders">No orders yet</Localized></p>
              ) : (
                grouped(status).map((order) => (
                  <button
                    key={order.id}
                    className="kds-ticket"
                    onClick={() => advanceStatus(order)}
                    aria-label={`Order ${order.display_number}, tap to advance`}
                  >
                    <div className="kds-ticket-header">
                      <span className="kds-ticket-number">#{order.display_number}</span>
                      <span className="kds-ticket-time">{timeAgo(order.received_at)}</span>
                    </div>
                    <span className="kds-ticket-items">{order.items_summary}</span>
                    {order.notes && <span className="kds-ticket-notes">{order.notes}</span>}
                    <span className="kds-ticket-count">
                      <Localized id="kds-items" vars={{ count: order.item_count }}>
                        {`${order.item_count} items`}
                      </Localized>
                    </span>
                  </button>
                ))
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function timeAgo(iso: string): string {
  const minutes = Math.floor((Date.now() - new Date(iso).getTime()) / 60000);
  if (minutes < 1) return 'now';
  return `${minutes}m`;
}
