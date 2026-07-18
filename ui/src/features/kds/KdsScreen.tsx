import { useEffect, useState, useCallback } from 'react';
import { Localized } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspaceScope } from '@/contexts/WorkspaceContext';
import { getKdsQueue, updateKdsStatus, type KdsOrder, type KdsStatus } from '@/api/kds';
import { useKdsPreferences, type KdsLayout } from '@/features/kds/hooks/useKdsPreferences';
import { KdsLayoutKanban } from '@/features/kds/KdsLayoutKanban';
import { KdsLayoutFocus } from '@/features/kds/KdsLayoutFocus';
import { KdsLayoutMetro } from '@/features/kds/KdsLayoutMetro';
import { KdsLayoutSwitcher } from '@/features/kds/KdsLayoutSwitcher';
import './KdsScreen.css';

const STATUS_ORDER: KdsStatus[] = ['pending', 'preparing', 'ready', 'served'];

/** Props passed to every KDS layout component. */
export interface KdsLayoutProps {
  orders: KdsOrder[];
  onAdvance: (order: KdsOrder) => void;
  showOrderId: boolean;
  showTableNumber: boolean;
}

const LAYOUT_MAP: Record<KdsLayout, React.ComponentType<KdsLayoutProps>> = {
  kanban: KdsLayoutKanban,
  focus: KdsLayoutFocus,
  metro: KdsLayoutMetro,
};

/** KDS (Kitchen Display System) screen — real-time order queue with switchable layouts and per-user preferences. */
export default function KdsScreen() {
  const { session } = useAuth();
  const workspaceScope = useWorkspaceScope();
  const userId = session?.user_id ?? '';
  const [orders, setOrders] = useState<KdsOrder[]>([]);
  const [error, setError] = useState<string | null>(null);
  const { prefs, setLayout, setShowOrderId, setShowTableNumber, loading: prefsLoading } = useKdsPreferences();

  const fetchOrders = useCallback(() => {
    const zone = prefs.kdsZone || undefined;
    getKdsQueue(userId, zone)
      .then((allOrders) => {
        const activeStoreId = workspaceScope?.storeId;
        if (activeStoreId) {
          const filtered = allOrders.filter((order) =>
            !order.store_id || order.store_id === activeStoreId,
          );
          setOrders(filtered);
        } else {
          setOrders(allOrders);
        }
      })
      .catch((e) => setError(e.message ?? String(e)));
  }, [userId, workspaceScope?.storeId, prefs.kdsZone]);

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

  const LayoutComponent = LAYOUT_MAP[prefs.layout];

  return (
    <div className="kds" role="region" aria-label="Kitchen Display System">
      <div className="kds-header">
        <div className="kds-header-left">
          <h1 className="kds-title"><Localized id="kds-title">Kitchen Display</Localized></h1>
          <span className="kds-order-count"><Localized id="kds-order-count" vars={{ count: orders.length }}><span>{orders.length} orders</span></Localized></span>
        </div>
        <div className="kds-header-right">
          {!prefsLoading && (
            <KdsLayoutSwitcher
              currentLayout={prefs.layout}
              showOrderId={prefs.showOrderId}
              showTableNumber={prefs.showTableNumber}
              onSelectLayout={setLayout}
              onToggleOrderId={setShowOrderId}
              onToggleTableNumber={setShowTableNumber}
            />
          )}
        </div>
      </div>
      {error && <p className="kds-error">{error}</p>}
      {!prefsLoading && (
        <LayoutComponent
          orders={orders}
          onAdvance={advanceStatus}
          showOrderId={prefs.showOrderId}
          showTableNumber={prefs.showTableNumber}
        />
      )}
    </div>
  );
}
