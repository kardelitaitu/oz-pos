import { useEffect, useState, useCallback } from 'react';
import { Localized } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspaceScope } from '@/contexts/WorkspaceContext';
import { getKdsQueue, updateKdsStatus, type KdsOrder, type KdsStatus } from '@/api/kds';
import { KdsTicketCard } from '@/features/kds/components/KdsTicketCard';
import './KdsScreen.css';

const STATUS_ORDER: KdsStatus[] = ['pending', 'preparing', 'ready', 'served'];
const STATUS_LABELS: Record<KdsStatus, string> = {
  pending: 'kds-pending',
  preparing: 'kds-preparing',
  ready: 'kds-ready',
  served: 'kds-served',
  cancelled: 'kds-cancelled',
};

/** KDS (Kitchen Display System) screen — real-time order queue with status advancement (pending, preparing, ready, served) and auto-refresh. */
export default function KdsScreen() {
  const { session } = useAuth();
  const workspaceScope = useWorkspaceScope();
  const userId = session?.user_id ?? '';
  const [orders, setOrders] = useState<KdsOrder[]>([]);
  const [error, setError] = useState<string | null>(null);

  const fetchOrders = useCallback(() => {
    getKdsQueue(userId)
      .then((allOrders) => {
        // ADR #8: Filter out orders whose store_id doesn't match the
        // device-bound store. This is defense-in-depth — the database
        // is already store-scoped, but LAN-broadcast events could carry
        // orders from other stores in future multi-store deployments.
        const activeStoreId = workspaceScope?.storeId;
        if (activeStoreId) {
          const filtered = allOrders.filter((order) => {
            // Keep orders with no store_id (legacy/backward compat)
            // and orders whose store_id matches the active store.
            return !order.store_id || order.store_id === activeStoreId;
          });
          setOrders(filtered);
        } else {
          setOrders(allOrders);
        }
      })
      .catch((e) => setError(e.message ?? String(e)));
  }, [userId, workspaceScope?.storeId]);

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
      await updateKdsStatus(userId, order.id, nextStatus);
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
        <span className="kds-order-count"><Localized id="kds-order-count" vars={{ count: orders.length }}><span>{orders.length} orders</span></Localized></span>
      </div>
      {error && <p className="kds-error">{error}</p>}
      <div className="kds-columns">
        {(['pending', 'preparing', 'ready'] as KdsStatus[]).map((status) => (
          <div key={status} className={`kds-column kds-column--${status}`}>
            <h2 className="kds-column-title">
              <Localized id={STATUS_LABELS[status]}>
                <span>{status}</span>
              </Localized>
              <span className="kds-column-count">{grouped(status).length}</span>
            </h2>
            <div className="kds-tickets">
              {grouped(status).length === 0 ? (
                <p className="kds-empty"><Localized id="kds-no-orders">No orders yet</Localized></p>
              ) : (
                grouped(status).map((order) => (
                  <KdsTicketCard
                    key={order.id}
                    order={order}
                    onAdvance={advanceStatus}
                  />
                ))
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
