import { useEffect, useState, useCallback, useMemo, useRef, Profiler } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { listen } from '@tauri-apps/api/event';
import { usePullToRefresh } from '@/hooks/usePullToRefresh';
import { useKdsOffline } from '@/hooks/useKdsOffline';
import { useWorkspaceScope, useWorkspace } from '@/contexts/WorkspaceContext';
import { getKdsQueueScoped, updateKdsStatusScoped, updateKdsOrderItemsScoped, updateKdsLineItemStatusScoped, getKdsOrderLinesScoped, type KdsOrder, type KdsStatus, type KdsLineItem, type CreateKdsLineItemInput } from '@/api/kds';
import { useKdsPreferences } from '@/features/kds/hooks/useKdsPreferences';
import { useNewTicketSound } from '@/features/kds/hooks/useNewTicketSound';
import { useSound } from '@/frontend/shared/useSound';
import { requiredLocalized, LoadingStatus } from '@/frontend/shared';
import { isEditableTarget } from '@/utils/isEditableTarget';
import { isAnyAriaModalOpen } from '@/utils/modal-guard';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import { KdsLayoutMasonry } from '@/features/kds/KdsLayoutMasonry';
import { KdsHamburgerPanel } from '@/features/kds/KdsHamburgerPanel';
import { type KdsSettings, DEFAULT_SETTINGS } from '@/features/kds/KdsSettingsPanel';
import { KdsHistoryPanel } from '@/features/kds/KdsHistoryPanel';
import { KdsProductPickerModal } from '@/features/kds/components/KdsProductPickerModal';
import type { ProductPickerResult } from '@/features/kds/components/KdsProductPickerModal';
import { KdsDeviceStatusIndicator } from '@/features/kds/components/KdsDeviceStatusIndicator';
import { KdsEnrollmentModal } from '@/features/kds/components/KdsEnrollmentModal';
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
  /** Session token for scoped API calls (e.g., fetching line items). */
  sessionToken: string;
  /** Called when a single line item is tapped to advance its status (TODO 3e). */
  onAdvanceItem?: (item: KdsLineItem) => void;
  /** Called to open the product picker for adding items to a KDS order (TODO 3f). */
  onAddItems?: (orderId: string) => void;
  /** Set of order IDs that just arrived — used for brief highlight animation. */
  newOrderIds: ReadonlySet<string>;
}

/** Keyboard shortcut descriptions for the help popover. */
const SHORTCUTS: { key: string; id: string }[] = [
  { key: '1-9', id: 'kds-shortcut-select' },
  { key: 'Space', id: 'kds-shortcut-advance' },
  { key: '↑↓', id: 'kds-shortcut-navigate' },
  { key: 'Esc', id: 'kds-shortcut-deselect' },
];

/** KDS (Kitchen Display System) screen — real-time order queue in a single masonry view, with Open/Completed tabs and per-user preferences. */
export default function KdsScreen() {
  const workspaceScope = useWorkspaceScope();
  const { l10n } = useLocalization();
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { sessionToken: rawToken, terminalId } = useWorkspace();
  const sessionToken = rawToken || '';
  const [orders, setOrders] = useState<KdsOrder[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedOrderId, setSelectedOrderId] = useState<string | null>(null);
  const [settings, setSettings] = useState<KdsSettings>(DEFAULT_SETTINGS);
  /** Open vs Completed view — the prototype's primary tab navigation. */
  const [activeTab, setActiveTab] = useState<'open' | 'completed'>('open');
  const [initialLoading, setInitialLoading] = useState(true);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const shortcutsBtnRef = useRef<HTMLButtonElement>(null);
  const shortcutsRef = useRef<HTMLDivElement>(null);
  // KEY-07: ARIA tabs pattern — zone chips get roving tabindex + arrow keys.
  const zoneTabRefs = useRef<Array<HTMLButtonElement | null>>([]);
  // Open/Completed tab indicator: measured from the track + active tab.
  const tabsTrackRef = useRef<HTMLDivElement>(null);
  const tabOpenRef = useRef<HTMLButtonElement>(null);
  const tabCompletedRef = useRef<HTMLButtonElement>(null);
  const [tabIndicator, setTabIndicator] = useState<{ left: number; width: number }>({ left: 3, width: 0 });
  // 3f: Product picker state — which order is being edited.
  const [pickerOrderId, setPickerOrderId] = useState<string | null>(null);
  // KDS device enrollment modal state.
  const [showEnrollment, setShowEnrollment] = useState(false);
  // Re-entry guard for the picker confirm: the merge is async and the modal
  // stays open until it resolves, so a fast double-tap would fire the merge
  // twice and duplicate the items on the ticket. Pinned by KdsScreen.test.tsx
  // (deferred-promise double-tap). The state twin drives the modal's
  // disabled Confirm for visual feedback; the ref keeps the guard immune to
  // render timing between two rapid taps.
  const pickerSavingRef = useRef(false);
  const [pickerSaving, setPickerSaving] = useState(false);
  const { prefs, setShowOrderId, setShowTableNumber, setAutoAcknowledge, setKdsZone, loading: prefsLoading } = useKdsPreferences();

  // Track previous order IDs for new-ticket arrival animation.
  const prevOrderIdsRef = useRef(new Set<string>());
  const [newOrderIds, setNewOrderIds] = useState<Set<string>>(new Set());
  const arrivalTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Close shortcuts popover on Escape or outside click.
  useEffect(() => {
    if (!showShortcuts) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setShowShortcuts(false);
    };
    const handleClickOutside = (e: MouseEvent) => {
      if (
        shortcutsRef.current && !shortcutsRef.current.contains(e.target as Node) &&
        shortcutsBtnRef.current && !shortcutsBtnRef.current.contains(e.target as Node)
      ) {
        setShowShortcuts(false);
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [showShortcuts]);

  // 3b: Offline resilience — cache, retry queue, optimistic updates.
  // OFF-07: the hook namespaces all localStorage by store scope so switching
  // stores on a shared terminal never leaks orders or queued mutations.
  const {
    online, pendingQueueLength, deadLetterLength,
    wrapFetch, wrapUpdate, retryPending, clearDeadLetter,
    forceRetryCounter, storageUnavailable,
  } = useKdsOffline(workspaceScope?.storeId);

  // P3-2: Chime when new tickets arrive (debounced to max 1 per 5s).
  useNewTicketSound(orders, settings.soundEnabled);
  const { speak } = useSound();

  const fetchOrders = useCallback(async () => {
    const zone = prefs.kdsZone || undefined;
    const { orders: fetchedOrders, fromCache } = await wrapFetch(() =>
      getKdsQueueScoped(sessionToken, zone),
    );
    const activeStoreId = workspaceScope?.storeId;
    let filtered = fetchedOrders;
    if (activeStoreId) {
      filtered = fetchedOrders.filter((order) =>
        !order.store_id || order.store_id === activeStoreId,
      );
    }
    // A cancelled ticket is terminal history — it must never surface on
    // the active kitchen board (it would only show in the history panel).
    filtered = filtered.filter((order) => order.status !== 'cancelled');

    // Track new ticket IDs for arrival animation.
    const currentIds = new Set(filtered.map((o) => o.id));
    const arrivedIds = new Set<string>();
    for (const id of currentIds) {
      if (!prevOrderIdsRef.current.has(id)) {
        arrivedIds.add(id);
      }
    }
    prevOrderIdsRef.current = currentIds;
    if (arrivedIds.size > 0) {
      setNewOrderIds(arrivedIds);
      // Clear the arrival highlight after 3s.
      if (arrivalTimerRef.current !== null) clearTimeout(arrivalTimerRef.current);
      arrivalTimerRef.current = setTimeout(() => setNewOrderIds(new Set()), 3000);
    }

    setOrders(filtered);
    setInitialLoading(false);

    // On reconnect (fetch succeeded, not from cache), flush pending queue.
    if (!fromCache && pendingQueueLength > 0) {
      retryPending(async (action) => {
        try {
          await updateKdsStatusScoped(sessionToken, action.orderId, action.targetStatus);
          // Voice callout if reconnected action targeted 'ready'.
          if (action.targetStatus === 'ready') {
            speak(`${requiredLocalized(l10n, 'kds-order-up-tts')} ${requiredLocalized(l10n, 'kds-ready-tts')}!`);
          }
          return true;
        } catch {
          return false;
        }
      });
    }
  }, [sessionToken, workspaceScope?.storeId, prefs.kdsZone, wrapFetch, pendingQueueLength, retryPending, speak, l10n]);

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
      if (arrivalTimerRef.current !== null) clearTimeout(arrivalTimerRef.current);
    };
  }, [fetchOrders]);

  const clearError = useCallback(() => setError(null), []);

  const advanceStatus = useCallback(async (order: KdsOrder) => {
    const currentIdx = STATUS_ORDER.indexOf(order.status as KdsStatus);
    if (currentIdx < 0 || currentIdx >= STATUS_ORDER.length - 1) return;
    const nextStatus = STATUS_ORDER[currentIdx + 1]!;

    // 3b: Offline-aware status update — queue on failure + optimistic local update.
    const ok = await wrapUpdate(order.id, nextStatus, () =>
      updateKdsStatusScoped(sessionToken, order.id, nextStatus),
    );

    if (ok) {
      // 3d: Voice callout when a ticket hits 'ready' — "Order 42 up!"
      if (nextStatus === 'ready') {
        speak(`${requiredLocalized(l10n, 'kds-order-up-tts')} ${order.display_number} ${requiredLocalized(l10n, 'kds-ready-tts')}!`);
      }
      // No manual fetchOrders() — the kds:orders-changed event triggers a refresh.
    } else {
      // Optimistic update: advance locally so the kitchen can keep working.
      setOrders((prev) => prev.map((o) =>
        o.id === order.id ? { ...o, status: nextStatus } : o,
      ));
      // Show a user-friendly banner instead of raw error.
      setError(requiredLocalized(l10n, 'kds-offline-queued-update'));
    }
  }, [sessionToken, speak, l10n, wrapUpdate]);

  // ── Per-item status advance (TODO 3e) ──────────────────────────
  const advanceItemStatus = useCallback(async (item: KdsLineItem) => {
    const ITEM_STATUS_ORDER: KdsStatus[] = ['pending', 'preparing', 'ready', 'served'];
    const currentIdx = ITEM_STATUS_ORDER.indexOf(item.item_status as KdsStatus);
    if (currentIdx < 0 || currentIdx >= ITEM_STATUS_ORDER.length - 1) return;
    const nextStatus = ITEM_STATUS_ORDER[currentIdx + 1]!;

    try {
      await updateKdsLineItemStatusScoped(sessionToken, item.id, nextStatus);
      // 3d: Voice callout when an item hits 'ready'.
      if (nextStatus === 'ready') {
        speak(`${requiredLocalized(l10n, 'kds-order-up-tts')} ${item.display_name} ${requiredLocalized(l10n, 'kds-ready-tts')}!`);
      }
    } catch {
      // Silent — the backend will emit kds:orders-changed on next poll.
    }
  }, [sessionToken, speak, l10n]);

  // OFF-04: reconnect is a first-class transition. When the OS fires an
  // `online` event the hook bumps forceRetryCounter; we turn that into a
  // bounded fetch/probe cycle. The probe succeeds only if the backend is
  // reachable, and the post-fetch flush replays any queued actions.
  const reconnectRef = useRef(false);
  useEffect(() => {
    if (forceRetryCounter === 0) return;
    // Serialize: skip if a fetch/retry is already in flight.
    if (reconnectRef.current) return;
    reconnectRef.current = true;
    void fetchOrders().finally(() => {
      reconnectRef.current = false;
    });
  }, [forceRetryCounter, fetchOrders]);

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

  // KEY-07: managed screen-level listener with editable + modal guards.
  // Previously the handler was bound to the root element, so shortcuts stopped
  // working whenever focus left the region. Binding to `document` (with guards)
  // keeps 1-9/Arrow/Space/Escape working regardless of where focus lands, and
  // the KDS component unmounting removes the listener.
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Guard: never intercept while the user is typing in an editable target.
      if (isEditableTarget(e.target)) return;
      // Guard: never intercept while a modal owns the keyboard.
      if (isAnyAriaModalOpen()) return;

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

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
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

  // KEY-07: ARIA tabs pattern — ArrowLeft/ArrowRight/Home/End move between the
  // zone chips (roving tabindex: the selected chip keeps tabIndex 0, others -1),
  // and the chip reached by arrow keys becomes the active zone filter.
  const handleZoneTablistKeyDown = useCallback((e: React.KeyboardEvent) => {
    const chips = zoneTabRefs.current;
    if (!chips || chips.length === 0) return;
    const current = chips.findIndex((c) => c === document.activeElement);
    let next = -1;
    if (e.key === 'ArrowRight') {
      next = current < 0 ? 0 : (current + 1) % chips.length;
    } else if (e.key === 'ArrowLeft') {
      next = current < 0 ? chips.length - 1 : (current - 1 + chips.length) % chips.length;
    } else if (e.key === 'Home') {
      next = 0;
    } else if (e.key === 'End') {
      next = chips.length - 1;
    }
    if (next < 0) return;
    e.preventDefault();
    chips[next]?.focus();
    // chip 0 = "All" (zone ''), chips 1..n = zones[0..n-1]
    setKdsZone(next === 0 ? '' : (zones[next - 1] ?? ''));
  }, [zones, setKdsZone]);

  // Open/Completed tab indicator: measure the active tab button inside
  // the track and slide the blue pill to it (prototype .kds-tab-indicator).
  useEffect(() => {
    const track = tabsTrackRef.current;
    const tab = activeTab === 'open' ? tabOpenRef.current : tabCompletedRef.current;
    if (!track || !tab) return;
    setTabIndicator({
      left: tab.offsetLeft - track.offsetLeft,
      width: tab.offsetWidth,
    });
  }, [activeTab]);

  // ── Initial loading skeleton ──────────────────────────────────
  const renderContent = () => {
    if (initialLoading) {
      // LOAD-05: the skeleton columns are decorative; the localized
      // status line (role=status) is what screen readers announce.
      return (
        <LoadingStatus className="kds-loading-container" label={requiredLocalized(l10n, 'kds-loading')}>
            <div className="kds-loading-columns">
              {['pending', 'preparing', 'ready'].map((status) => (
                <div key={status} className="kds-loading-column">
                  <div className="kds-loading-header" />
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="kds-loading-card">
                      <div className="kds-loading-line kds-loading-line--short" />
                      <div className="kds-loading-line kds-loading-line--long" />
                      <div className="kds-loading-line kds-loading-line--medium" />
                    </div>
                  ))}
                </div>
              ))}
            </div>
          </LoadingStatus>
        );
    }

    if (activeTab === 'completed') {
      return <KdsHistoryPanel />;
    }

    return (
      <div {...pullRefreshProps}>
        <KdsLayoutMasonry
          orders={orders}
          onAdvance={advanceStatus}
          showOrderId={prefs.showOrderId}
          showTableNumber={prefs.showTableNumber}
          selectedOrderId={selectedOrderId}
          sessionToken={sessionToken}
          onSaveItems={async (orderId, itemsSummary, itemCount) => {
            try {
              await updateKdsOrderItemsScoped(sessionToken, { id: orderId, items_summary: itemsSummary, item_count: itemCount });
            } catch (e) {
              setError(String(e));
            }
          }}
          onAdvanceItem={advanceItemStatus}
          onAddItems={setPickerOrderId}
          newOrderIds={newOrderIds}
        />
      </div>
    );
  };

  return (
    <Profiler id="KdsScreen" onRender={(...args) => {
      if (typeof args[2] === 'number' && args[2] > 1) {
        console.debug('[Profiler] KdsScreen', args[1] === 'mount' ? '⚡mount' : '♻update', `${args[2].toFixed(1)}ms`);
      }
    }}>
    <div ref={kdsRef} className="kds" tabIndex={-1} role="region" aria-label={requiredLocalized(l10n, 'kds-screen-aria')}>
      <div className="kds-header">
        <div className="kds-header-left">
          <button
            className="kds-back-btn"
            onClick={goToWorkspacePicker}
            aria-label={requiredLocalized(l10n, 'kds-back-aria')}
            data-testid="kds-topbar-back"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M15 19l-7-7 7-7" /></svg>
          </button>
          <h1 className="kds-title"><Localized id="kds-title">Kitchen Display</Localized></h1>
          <span className="kds-order-count"><Localized id="kds-order-count" vars={{ count: orders.length }}><span>{orders.length} orders</span></Localized></span>
        </div>

        {/* Open/Completed tabs — prototype .kds-tabs */}
        <div className="kds-tabs" ref={tabsTrackRef} role="tablist" aria-label={requiredLocalized(l10n, 'kds-tablist-aria')}>
          <span className="kds-tab-indicator" style={{ left: tabIndicator.left, width: tabIndicator.width }} />
          <button
            ref={tabOpenRef}
            className={`kds-tab${activeTab === 'open' ? ' active' : ''}`}
            onClick={() => setActiveTab('open')}
            role="tab"
            aria-selected={activeTab === 'open'}
            data-testid="kds-tab-open"
          >
            <Localized id="kds-tab-open"><span>Open</span></Localized>
            <span className="kds-tab-count">{orders.length}</span>
          </button>
          <button
            ref={tabCompletedRef}
            className={`kds-tab${activeTab === 'completed' ? ' active' : ''}`}
            onClick={() => setActiveTab('completed')}
            role="tab"
            aria-selected={activeTab === 'completed'}
            data-testid="kds-tab-completed"
          >
            <Localized id="kds-tab-completed"><span>Completed</span></Localized>
          </button>
        </div>

        <div className="kds-header-right">
          {/* Shortcut help button */}
          <button
            ref={shortcutsBtnRef}
            className="kds-shortcuts-btn"
            onClick={() => setShowShortcuts((p) => !p)}
            aria-label={requiredLocalized(l10n, 'kds-shortcuts-aria')}
            aria-expanded={showShortcuts}
            aria-controls="kds-shortcuts-popover"
          >
            <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16" aria-hidden="true">
              <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-3a1 1 0 00-.867.5 1 1 0 11-1.731-1A3 3 0 0113 8a3.001 3.001 0 01-2 2.83V11a1 1 0 11-2 0v-1a1 1 0 011-1 1 1 0 100-2zm0 8a1 1 0 100-2 1 1 0 000 2z" clipRule="evenodd" />
            </svg>
          </button>
          {showShortcuts && (
            <div
              id="kds-shortcuts-popover"
              ref={shortcutsRef}
              className="kds-shortcuts-popover"
              role="region"
              aria-label={requiredLocalized(l10n, 'kds-shortcuts-label')}
            >
              {SHORTCUTS.map((s) => (
                <div key={s.id} className="kds-shortcut-row">
                  <kbd className="kds-shortcut-key">{s.key}</kbd>
                  <span className="kds-shortcut-desc">{requiredLocalized(l10n, s.id)}</span>
                </div>
              ))}
            </div>
          )}
          {/* Device status indicator */}
          <KdsDeviceStatusIndicator sessionToken={sessionToken} />
          {/* Enroll new KDS device button */}
          <button
            className="kds-enroll-btn"
            onClick={() => setShowEnrollment(true)}
            aria-label={requiredLocalized(l10n, 'kds-enrollment-title')}
          >
            <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16" aria-hidden="true">
              <path fillRule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clipRule="evenodd" />
            </svg>
          </button>
          {/* Hamburger settings panel — only when prefs loaded */}
          {!prefsLoading && (
            <KdsHamburgerPanel
              settings={{ ...settings, autoAcknowledge: prefs.autoAcknowledge }}
              onChangeSound={(v) => setSettings((s) => ({ ...s, soundEnabled: v }))}
              onChangeYellowThreshold={(v) => setSettings((s) => ({ ...s, yellowThresholdMin: v }))}
              onChangeRedThreshold={(v) => setSettings((s) => ({ ...s, redThresholdMin: v }))}
              onChangeAutoAcknowledge={(v) => setAutoAcknowledge(v)}
              onChangeDensity={(v) => setSettings((s) => ({ ...s, density: v }))}
              showOrderId={prefs.showOrderId}
              showTableNumber={prefs.showTableNumber}
              onToggleOrderId={setShowOrderId}
              onToggleTableNumber={setShowTableNumber}
            />
          )}
        </div>
      </div>

      {/* ── Zone chips — secondary filter row below the header ────── */}
      {zones.length > 0 && (
        <div className="kds-zone-chips" role="tablist" aria-label={requiredLocalized(l10n, 'kds-zone-filter-aria')} onKeyDown={handleZoneTablistKeyDown} tabIndex={0}>
          <button
            className={`kds-zone-chip${!prefs.kdsZone ? ' kds-zone-chip--active' : ''}`}
            onClick={() => setKdsZone('')}
            role="tab"
            aria-selected={!prefs.kdsZone}
            tabIndex={!prefs.kdsZone ? 0 : -1}
            ref={(el) => { zoneTabRefs.current[0] = el; }}
          >
            <Localized id="kds-zone-all">All</Localized>
          </button>
          {zones.map((zone, i) => (
            <button
              key={zone}
              className={`kds-zone-chip${prefs.kdsZone === zone ? ' kds-zone-chip--active' : ''}`}
              onClick={() => setKdsZone(zone)}
              role="tab"
              aria-selected={prefs.kdsZone === zone}
              tabIndex={prefs.kdsZone === zone ? 0 : -1}
              ref={(el) => { zoneTabRefs.current[i + 1] = el; }}
            >
              {zone}
            </button>
          ))}
        </div>
      )}

      {/* ── Error banner (dismissible + retry) ──────────────────── */}
      {error && (
        <div className="kds-error-banner" role="alert">
          <span className="kds-error-banner-text">{error}</span>
          <button
            className="kds-error-retry-btn"
            onClick={() => {
              clearError();
              fetchOrders();
            }}
            aria-label={requiredLocalized(l10n, 'kds-error-retry-aria')}
          >
            <Localized id="kds-offline-retry">Retry</Localized>
          </button>
          <button
            className="kds-error-dismiss-btn"
            onClick={clearError}
            aria-label={requiredLocalized(l10n, 'kds-error-dismiss-aria')}
          >
            &times;
          </button>
        </div>
      )}

      {/* OFF-08: local persistence is unavailable — queued actions are not durable */}
      {storageUnavailable && (
        <div className="kds-offline-banner kds-offline-banner--storage" role="alert">
          <span className="kds-offline-banner-text">
            {requiredLocalized(l10n, 'kds-offline-storage-unavailable')}
          </span>
        </div>
      )}

      {/* OFF-05: actions that exhausted retries and need operator attention */}
      {deadLetterLength > 0 && (
        <div className="kds-offline-banner kds-offline-banner--deadletter" role="alert">
          <span className="kds-offline-banner-text">
            {requiredLocalized(l10n, 'kds-offline-dead-letter', { count: String(deadLetterLength) })}
          </span>
          <button
            className="kds-offline-retry-btn"
            onClick={() => {
              // Re-queue dead-letter actions: clear the dead list so the next
              // fetch/retry cycle picks up the operator intent; then flush.
              clearDeadLetter();
              retryPending(async (action) => {
                try {
                  await updateKdsStatusScoped(sessionToken, action.orderId, action.targetStatus);
                  return true;
                } catch {
                  return false;
                }
              });
            }}
            aria-label={requiredLocalized(l10n, 'kds-offline-retry-aria')}
          >
            <Localized id="kds-offline-retry">Retry</Localized>
          </button>
          <button
            className="kds-offline-dismiss-btn"
            onClick={clearDeadLetter}
            aria-label={requiredLocalized(l10n, 'kds-offline-dead-letter-clear-aria')}
          >
            &times;
          </button>
        </div>
      )}

      {/* 3b: Offline banner — shown when backend is unreachable or actions are queued */}
      {!online && (
        <div className="kds-offline-banner" role="alert">
          <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16" aria-hidden="true">
            <path fillRule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-.47.81c-.54.5-1.1 1.36-1.1 2.52V8l4.89-4.89c-.04-.26-.14-.52-.34-.73zM5.99 5.58l-2.84 2.84a1.532 1.532 0 000 2.16l7.29 7.29c.39.39 1.02.39 1.41 0l2.84-2.84-5.99-5.99-2.71-2.76v.3zm10.02 2.46l2.13 2.13a1.532 1.532 0 010 2.16l-2.13 2.13a.5.5 0 01-.71-.71l2.13-2.13a.532.532 0 000-.75l-2.13-2.13a.5.5 0 01.71-.71zm-5.02 5.32a1.25 1.25 0 110-2.5 1.25 1.25 0 010 2.5z" clipRule="evenodd" />
          </svg>
          <span className="kds-offline-banner-text">
            {pendingQueueLength > 0
              ? requiredLocalized(l10n, 'kds-offline-queued', { count: pendingQueueLength })
              : requiredLocalized(l10n, 'kds-offline-label')}
          </span>
          {pendingQueueLength > 0 && (
            <button
              className="kds-offline-retry-btn"
              onClick={() => {
                retryPending(async (action) => {
                  try {
                    await updateKdsStatusScoped(sessionToken, action.orderId, action.targetStatus);
                    return true;
                  } catch {
                    return false;
                  }
                });
                // The backend will emit kds:orders-changed on success,
                // which triggers fetchOrders via the event listener.
              }}
              aria-label={requiredLocalized(l10n, 'kds-offline-retry-aria')}
            >
              <Localized id="kds-offline-retry">Retry</Localized>
            </button>
          )}
          <button
            className="kds-offline-dismiss-btn"
            onClick={() => setError(null)}
            aria-label={requiredLocalized(l10n, 'kds-offline-dismiss-aria')}
          >
            &times;
          </button>
        </div>
      )}

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

      {/* ── Main content: loading skeleton, history panel, or layout ── */}
      {renderContent()}

      {/* 3f: Product picker modal for adding items mid-preparation */}
      <KdsProductPickerModal
        orderId={pickerOrderId ?? ''}
        sessionToken={sessionToken}
        isOpen={pickerOrderId !== null}
        pending={pickerSaving}
        onConfirm={async (result: ProductPickerResult) => {
          // Ignore re-entry while a confirm merge is in flight (double-tap).
          if (pickerSavingRef.current) return;
          pickerSavingRef.current = true;
          setPickerSaving(true);
          try {
            // Re-fetch existing line items to merge with new ones.
            const existing = await getKdsOrderLinesScoped(sessionToken, result.orderId);
            const mergedItems: CreateKdsLineItemInput[] = [
              ...existing.map((item) => ({
                sku: item.sku,
                display_name: item.display_name,
                qty: item.qty,
                course: item.course,
                modifiers: item.modifiers,
              })),
              ...result.items,
            ];
            await updateKdsOrderItemsScoped(sessionToken, {
              id: result.orderId,
              items_summary: '', // ignored — will be re-derived from line_items
              item_count: 0,     // ignored — will be re-derived from line_items
              line_items: mergedItems,
            });
          } catch (e) {
            setError(String(e));
          } finally {
            pickerSavingRef.current = false;
            setPickerSaving(false);
            setPickerOrderId(null);
          }
        }}
        onClose={() => setPickerOrderId(null)}
      />

      {/* KDS device enrollment modal */}
      <KdsEnrollmentModal
        sessionToken={sessionToken}
        restaurantPosId={terminalId}
        isOpen={showEnrollment}
        onEnrolled={() => {
          setShowEnrollment(false);
        }}
        onClose={() => setShowEnrollment(false)}
      />
    </div>
    </Profiler>
  );
}
