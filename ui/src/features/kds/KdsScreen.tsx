import { useEffect, useState, useCallback, Profiler } from 'react';
import { Localized } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspaceScope } from '@/contexts/WorkspaceContext';
import { getKdsQueue, updateKdsStatus, type KdsOrder, type KdsStatus } from '@/api/kds';
import { useKdsPreferences, type KdsLayout } from '@/features/kds/hooks/useKdsPreferences';
import { useNewTicketSound } from '@/features/kds/hooks/useNewTicketSound';
import { KdsLayoutKanban } from '@/features/kds/KdsLayoutKanban';
import { KdsLayoutFocus } from '@/features/kds/KdsLayoutFocus';
import { KdsLayoutMetro } from '@/features/kds/KdsLayoutMetro';
import { KdsLayoutSwitcher } from '@/features/kds/KdsLayoutSwitcher';
import { KdsSettingsPanel, type KdsSettings, DEFAULT_SETTINGS } from '@/features/kds/KdsSettingsPanel';
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
  const [settings, setSettings] = useState<KdsSettings>(DEFAULT_SETTINGS);
  const { prefs, setLayout, setShowOrderId, setShowTableNumber, loading: prefsLoading } = useKdsPreferences();

  // P3-2: Chime when new tickets arrive (debounced to max 1 per 5s).
  useNewTicketSound(orders, settings.soundEnabled);

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

  // P2-3: Adaptive polling with dynamic interval based on idle time.
  // Uses recursive setTimeout so the duration recalculates after every
  // fetch. Polls 2s when active, backs off to 10s after 30s idle, 30s
  // after 2min idle. Pauses when the tab is hidden. Idle resets when
  // orders.length changes (via effect dependency re-run).
  useEffect(() => {
    let idleMs = 0;
    let timerId: ReturnType<typeof setTimeout> | null = null;
    let isPaused = document.hidden;

    const getInterval = (idle: number): number => {
      if (idle < 30_000) return 2_000;   // active: 2s
      if (idle < 120_000) return 10_000;  // idle 30s+: 10s
      return 30_000;                        // idle 2min+: 30s
    };

    const clearTimer = () => {
      if (timerId !== null) {
        clearTimeout(timerId);
        timerId = null;
      }
    };

    // Recursive tick — recalculates interval after every fetch
    const tick = () => {
      if (!isPaused) {
        fetchOrders();
      }

      // Advance idle time by the current interval, then schedule next tick
      idleMs += getInterval(idleMs);
      timerId = setTimeout(tick, getInterval(idleMs));
    };

    // Visibility change handler — pause polling when tab hidden
    const onVisibilityChange = () => {
      isPaused = document.hidden;
      if (!isPaused) {
        // Immediately fetch when tab becomes visible, then restart with fresh idle
        clearTimer();
        fetchOrders();
        idleMs = 0;
        timerId = setTimeout(tick, getInterval(0));
      }
    };

    // Initial fetch and start polling
    fetchOrders();
    timerId = setTimeout(tick, getInterval(0));

    // Wire up visibility change listener
    document.addEventListener('visibilitychange', onVisibilityChange);

    return () => {
      clearTimer();
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, [fetchOrders, orders.length]);

  const advanceStatus = useCallback(async (order: KdsOrder) => {
    const currentIdx = STATUS_ORDER.indexOf(order.status as KdsStatus);
    if (currentIdx < 0 || currentIdx >= STATUS_ORDER.length - 1) return;
    const nextStatus = STATUS_ORDER[currentIdx + 1]!;
    try {
      await updateKdsStatus(userId, order.id, nextStatus);
      fetchOrders();
    } catch (e) {
      setError(String(e));
    }
  }, [userId, fetchOrders]);

  const LayoutComponent = LAYOUT_MAP[prefs.layout];

  return (
    <Profiler id="KdsScreen" onRender={(...args) => {
      if (typeof args[2] === 'number' && args[2] > 1) {
        console.debug('[Profiler] KdsScreen', args[1] === 'mount' ? '⚡mount' : '♻update', `${args[2].toFixed(1)}ms`);
      }
    }}>
    <div className="kds" role="region" aria-label="Kitchen Display System">
      <div className="kds-header">
        <div className="kds-header-left">
          <h1 className="kds-title"><Localized id="kds-title">Kitchen Display</Localized></h1>
          <span className="kds-order-count"><Localized id="kds-order-count" vars={{ count: orders.length }}><span>{orders.length} orders</span></Localized></span>
        </div>
        <div className="kds-header-right">
          {!prefsLoading && (<>
            <KdsSettingsPanel
              settings={settings}
              onChangeSound={(v) => setSettings((s) => ({ ...s, soundEnabled: v }))}
              onChangeYellowThreshold={(v) => setSettings((s) => ({ ...s, yellowThresholdMin: v }))}
              onChangeRedThreshold={(v) => setSettings((s) => ({ ...s, redThresholdMin: v }))}
              onChangeAutoAcknowledge={(v) => setSettings((s) => ({ ...s, autoAcknowledge: v }))}
              onChangeDensity={(v) => setSettings((s) => ({ ...s, density: v }))}
            />
            <KdsLayoutSwitcher
              currentLayout={prefs.layout}
              showOrderId={prefs.showOrderId}
              showTableNumber={prefs.showTableNumber}
              onSelectLayout={setLayout}
              onToggleOrderId={setShowOrderId}
              onToggleTableNumber={setShowTableNumber}
            />
          </>)}
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
    </Profiler>
  );
}
