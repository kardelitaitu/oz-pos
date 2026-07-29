import { useEffect, useState, useCallback, useMemo, useRef, Profiler } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { listen } from '@tauri-apps/api/event';
import { usePullToRefresh } from '@/hooks/usePullToRefresh';
import { useWorkspaceScope, useWorkspace } from '@/contexts/WorkspaceContext';
import { getKdsQueueScoped, updateKdsStatusScoped, updateKdsOrderItemsScoped, type KdsOrder, type KdsStatus } from '@/api/kds';
import { useKdsPreferences, type KdsLayout } from '@/features/kds/hooks/useKdsPreferences';
import { useNewTicketSound } from '@/features/kds/hooks/useNewTicketSound';
import { useSound } from '@/frontend/shared/useSound';
import { KdsLayoutKanban } from '@/features/kds/KdsLayoutKanban';
import { KdsLayoutFocus } from '@/features/kds/KdsLayoutFocus';
import { KdsLayoutMetro } from '@/features/kds/KdsLayoutMetro';
import { KdsLayoutSwitcher } from '@/features/kds/KdsLayoutSwitcher';
import { KdsSettingsPanel, type KdsSettings, DEFAULT_SETTINGS } from '@/features/kds/KdsSettingsPanel';
import { KdsHistoryPanel } from '@/features/kds/KdsHistoryPanel';
import './KdsScreen.css';

const STATUS_ORDER: KdsStatus[] = ['pending', 'preparing', 'ready', 'served'];

/** Props passed to every KDS layout component. */
export interface KdsLayoutProps {
  orders: KdsOrder[];
  onAdvance: (order: KdsOrder) => void;
  showOrderId: boolean;
  showTableNumber: boolean;
  /** Currently keyboard-selected order ID (highlighted card). */
  selectedOrderId: string | null;
  /** Called when the items on a ticket are edited. */
  onSaveItems?: (orderId: string, itemsSummary: string, itemCount: number) => void;
}

const LAYOUT_MAP: Record<KdsLayout, React.ComponentType<KdsLayoutProps>> = {
  kanban: KdsLayoutKanban,
  focus: KdsLayoutFocus,
  metro: KdsLayoutMetro,
};

/** KDS (Kitchen Display System) screen — real-time order queue with switchable layouts and per-user preferences. */
export default function KdsScreen() {
  const workspaceScope = useWorkspaceScope();
  const { l10n } = useLocalization();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';
  const [orders, setOrders] = useState<KdsOrder[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedOrderId, setSelectedOrderId] = useState<string | null>(null);
  const [settings, setSettings] = useState<KdsSettings>(DEFAULT_SETTINGS);
  const [showHistory, setShowHistory] = useState(false);
  const { prefs, setLayout, setShowOrderId, setShowTableNumber, setAutoAcknowledge, setKdsZone, loading: prefsLoading } = useKdsPreferences();

  // P3-2: Chime when new tickets arrive (debounced to max 1 per 5s).
  useNewTicketSound(orders, settings.soundEnabled);
  const { speak } = useSound();

  const fetchOrders = useCallback(() => {
    const zone = prefs.kdsZone || undefined;
    getKdsQueueScoped(sessionToken, zone)
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
  }, [sessionToken, workspaceScope?.storeId, prefs.kdsZone]);

  // 1a: Real-time push via Tauri events — replaces adaptive polling.
  // Listens for kds:orders-changed emitted by the Rust backend after
  // order creation or status updates. Falls back to re-fetch on tab
  // visibility change to catch any events missed while hidden.
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    // Initial fetch on mount.
    fetchOrders();

    // Subscribe to real-time KDS order changes (push, not poll).
    listen<null>('kds:orders-changed', () => {
      fetchOrders();
    }).then((fn) => { unlisten = fn; });

    // Visibility change fallback — re-fetch when tab becomes visible
    // to catch any events missed while the tab was hidden.
    const onVisibilityChange = () => {
      if (!document.hidden) {
        fetchOrders();
      }
    };
    document.addEventListener('visibilitychange', onVisibilityChange);

    return () => {
      if (unlisten) unlisten();
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, [fetchOrders]);

  const advanceStatus = useCallback(async (order: KdsOrder) => {
    const currentIdx = STATUS_ORDER.indexOf(order.status as KdsStatus);
    if (currentIdx < 0 || currentIdx >= STATUS_ORDER.length - 1) return;
    const nextStatus = STATUS_ORDER[currentIdx + 1]!;
    try {
      await updateKdsStatusScoped(sessionToken, order.id, nextStatus);
      // 3d: Voice callout when a ticket hits 'ready' — "Order 42 up!"
      if (nextStatus === 'ready') {
        speak(`${l10n.getString('kds-order-up-tts') || 'Order'} ${order.display_number} ${l10n.getString('kds-ready-tts') || 'up'}!`);
      }
      // No manual fetchOrders() — the kds:orders-changed event triggers a refresh.
    } catch (e) {
      setError(String(e));
    }
  }, [sessionToken, speak, l10n]);

  // 1c: Auto-acknowledge — when enabled, advance pending tickets to
  // preparing after acknowledgeDelayMin minutes without manual tap.
  // Must be placed AFTER advanceStatus declaration to avoid TDZ errors.
  useEffect(() => {
    if (!prefs.autoAcknowledge || prefs.acknowledgeDelayMin <= 0) return;

    const delayMs = prefs.acknowledgeDelayMin * 60 * 1000;
    const now = Date.now();

    for (const order of orders) {
      if (order.status !== 'pending') continue;
      if (!order.received_at) continue;

      const receivedAt = new Date(order.received_at).getTime();
      if (isNaN(receivedAt)) continue;

      if (now - receivedAt >= delayMs) {
        // Fire-and-forget — advance silently without awaiting.
        advanceStatus(order);
      }
    }
  }, [orders, prefs.autoAcknowledge, prefs.acknowledgeDelayMin, advanceStatus]);

  // 2d: Keyboard shortcuts — number keys to select, Space to advance, Arrows/Escape to navigate.
  const kdsRef = useRef<HTMLDivElement>(null);
  const selectedRef = useRef(selectedOrderId);
  selectedRef.current = selectedOrderId;

  // Auto-focus the container on mount so keyboard shortcuts work immediately.
  useEffect(() => {
    kdsRef.current?.focus();
  }, []);

  useEffect(() => {
    const el = kdsRef.current;
    if (!el) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't intercept when focus is inside an input/textarea.
      const tag = (e.target as HTMLElement).tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

      if (e.key >= '1' && e.key <= '9') {
        e.preventDefault();
        const idx = parseInt(e.key, 10) - 1;
        if (idx < orders.length) {
          setSelectedOrderId(orders[idx]!.id);
        }
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedOrderId((prev) => {
          const currentIdx = prev ? orders.findIndex((o) => o.id === prev) : -1;
          const nextIdx = Math.min(currentIdx + 1, orders.length - 1);
          return nextIdx >= 0 ? orders[nextIdx]!.id : null;
        });
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedOrderId((prev) => {
          const currentIdx = prev ? orders.findIndex((o) => o.id === prev) : orders.length;
          const nextIdx = Math.max(currentIdx - 1, 0);
          return orders.length > 0 ? orders[nextIdx]!.id : null;
        });
      } else if (e.key === ' ' && selectedRef.current) {
        // Skip if a ticket button already has focus (its onClick will handle advance).
        if ((e.target as HTMLElement).closest('.kds-ticket')) return;
        e.preventDefault();
        const selected = orders.find((o) => o.id === selectedRef.current);
        if (selected) {
          advanceStatus(selected);
        }
      } else if (e.key === 'Escape') {
        setSelectedOrderId(null);
      }
    };

    el.addEventListener('keydown', handleKeyDown);
    return () => el.removeEventListener('keydown', handleKeyDown);
  }, [orders, advanceStatus]);

  // P7-3: Pull-to-refresh gesture on KDS ticket board
  const { containerProps: pullRefreshProps, state: pullState, pullDistance } = usePullToRefresh({
    onRefresh: fetchOrders,
  });

  // 3a: Extract unique kitchen zones from orders for the zone-switching chips.
  const zones = useMemo(() => {
    const zoneSet = new Set<string>();
    for (const order of orders) {
      if (order.kitchen_zone) zoneSet.add(order.kitchen_zone);
    }
    return [...zoneSet].sort();
  }, [orders]);

  const LayoutComponent = LAYOUT_MAP[prefs.layout];

  return (
    <Profiler id="KdsScreen" onRender={(...args) => {
      if (typeof args[2] === 'number' && args[2] > 1) {
        console.debug('[Profiler] KdsScreen', args[1] === 'mount' ? '⚡mount' : '♻update', `${args[2].toFixed(1)}ms`);
      }
    }}>
    <div ref={kdsRef} className="kds" tabIndex={-1} role="region" aria-label={l10n.getString('kds-screen-aria') || 'Kitchen Display System'}>
      <div className="kds-header">
        <div className="kds-header-left">
          <h1 className="kds-title"><Localized id="kds-title">Kitchen Display</Localized></h1>
          <span className="kds-order-count"><Localized id="kds-order-count" vars={{ count: orders.length }}><span>{orders.length} orders</span></Localized></span>
        </div>
        <div className="kds-header-center">
          {zones.length > 0 && (
            <div className="kds-zone-chips" role="tablist" aria-label={l10n.getString('kds-zone-filter-aria') || 'Filter by kitchen zone'}>
              <button
                className={`kds-zone-chip${!prefs.kdsZone ? ' kds-zone-chip--active' : ''}`}
                onClick={() => setKdsZone('')}
                role="tab"
                aria-selected={!prefs.kdsZone}
              >
                <Localized id="kds-zone-all">All</Localized>
              </button>
              {zones.map((zone) => (
                <button
                  key={zone}
                  className={`kds-zone-chip${prefs.kdsZone === zone ? ' kds-zone-chip--active' : ''}`}
                  onClick={() => setKdsZone(zone)}
                  role="tab"
                  aria-selected={prefs.kdsZone === zone}
                >
                  {zone}
                </button>
              ))}
            </div>
          )}
        </div>
        <div className="kds-header-right">
          {!prefsLoading && (<>
            <KdsSettingsPanel
              settings={{ ...settings, autoAcknowledge: prefs.autoAcknowledge }}
              onChangeSound={(v) => setSettings((s) => ({ ...s, soundEnabled: v }))}
              onChangeYellowThreshold={(v) => setSettings((s) => ({ ...s, yellowThresholdMin: v }))}
              onChangeRedThreshold={(v) => setSettings((s) => ({ ...s, redThresholdMin: v }))}
              onChangeAutoAcknowledge={(v) => setAutoAcknowledge(v)}
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
      {/* P7-3: Pull-to-refresh indicator */}
      {pullState !== 'idle' && (
        <div
          className="kds-pull-indicator"
          style={{
            transform: `translateY(${pullDistance}px)`,
            opacity: Math.min(1, pullDistance / 60),
          }}
        >
          {pullState === 'loading' && <span className="kds-refresh-spinner" />}
          {pullState === 'pulling' && <Localized id="kds-pull-to-refresh">Pull down to refresh</Localized>}
          {pullState === 'ready' && <Localized id="kds-release-to-refresh">Release to refresh</Localized>}
        </div>
      )}
      {/* 2b: History/recall toggle button */}
      {!prefsLoading && (
        <button
          className={`kds-history-toggle${showHistory ? ' kds-history-toggle--active' : ''}`}
          onClick={() => setShowHistory((p) => !p)}
          aria-label={l10n.getString('kds-history-toggle-aria') || 'Toggle order history'}
          aria-pressed={showHistory}
          title={l10n.getString('kds-history-toggle-title') || 'Order history'}
        >
          <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16" aria-hidden="true">
            <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z" clipRule="evenodd" />
          </svg>
        </button>
      )}
      {!prefsLoading && !showHistory && (
        <div {...pullRefreshProps}>
          <LayoutComponent
            orders={orders}
            onAdvance={advanceStatus}
            showOrderId={prefs.showOrderId}              showTableNumber={prefs.showTableNumber}
              selectedOrderId={selectedOrderId}
              onSaveItems={async (orderId, itemsSummary, itemCount) => {
                try {
                  await updateKdsOrderItemsScoped(sessionToken, { id: orderId, items_summary: itemsSummary, item_count: itemCount });
                } catch (e) {
                  setError(String(e));
                }
              }}
            />
          </div>
      )}
      {!prefsLoading && showHistory && (
        <KdsHistoryPanel />
      )}
    </div>
    </Profiler>
  );
}
