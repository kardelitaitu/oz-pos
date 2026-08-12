//! Analytics Screen — layout shell with three flex areas.
//!
//! Top:    back button + title
//! Menu:   workspace selector (row 1) + time granularity buttons (row 2)
//!         + inline custom date range
//! Main:   smart card grid — cards adapt to retail vs restaurant

import { useEffect, useMemo, useRef, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import { l10nErrorMessage } from '@/utils/app-error';
import { AnalyticsCardContent } from './AnalyticsCardContent';
import { analyticsDataCache, analyticsQueryKey, clearAnalyticsCache, cardQueryKey } from './analytics-cache';
import { buildHeatmapIntensities, loadHeatmapRows, rangeForGranularity } from './analytics-data';
import { clearAnalyticsErrors, useAnalyticsQuery } from './useAnalyticsQuery';
import './AnalyticsScreen.css';

export type WorkspaceView = 'retail' | 'restaurant';
export type Granularity = 'daily' | 'weekly' | 'monthly' | 'yearly' | 'custom';

const GRANULARITIES: Granularity[] = ['daily', 'weekly', 'monthly', 'yearly', 'custom'];

const ZOOM_MIN = 0.6;
const ZOOM_MAX = 1.6;
const ZOOM_STEP = 0.2;

/** Keyboard shortcut metadata — drives both the handler and the help popover. */
const SHORTCUTS: { keys: string; labelKey: string }[] = [
  { keys: '1–5',    labelKey: 'analytics-shortcuts-granularity' },
  { keys: 'R',      labelKey: 'analytics-shortcuts-refresh' },
  { keys: '+ / −',  labelKey: 'analytics-shortcuts-zoom' },
  { keys: '0',      labelKey: 'analytics-shortcuts-zoom-reset' },
  { keys: 'C',      labelKey: 'analytics-shortcuts-collapse' },
  { keys: 'Esc',    labelKey: 'analytics-shortcuts-close' },
];

/**
 * Only one card may be expanded at a time.
 * - clicking the expanded card restores it (`current` → `null`)
 * - expanding when nothing is open sets the new card (`null` → `cid`)
 * - expanding another card while one is open is ignored
 */
export function nextExpandedKey(current: string | null, cid: string): string | null {
  if (current === cid) return null;
  if (current === null) return cid;
  return current;
}

/**
 * Scale factor that enlarges `content` to fill `available` without
 * overflowing either axis, capped at `max`. Returns 1 when the sizes
 * are unknown (e.g. layout not yet measured).
 */
export function smartScale(
  available: { w: number; h: number },
  content: { w: number; h: number },
  max = 4,
): number {
  if (available.w <= 0 || available.h <= 0 || content.w <= 0 || content.h <= 0) return 1;
  return Math.max(1, Math.min(max, Math.min(available.w / content.w, available.h / content.h)));
}

function isoToday(): string {
  return isoDay(new Date());
}

function isoDay(d: Date): string {
  // Local calendar date — `toISOString()` is UTC and can return the
  // previous day for late-evening/early-morning local times, which would
  // make the custom-range default land on the wrong date.
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** Number of days in the current month (28–31). */
export function daysInCurrentMonth(): number {
  const now = new Date();
  return new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
}

/**
 * Calendar layout for the current month. `leading` counts the empty
 * cells before day 1 (Monday-first), `days` the day cells, and
 * `trailing` the empty cells after the last day so the grid always
 * completes whole weeks (leading + days + trailing ≡ 0 mod 7).
 */
export function monthCalendarGrid(): { leading: number; days: number; trailing: number } {
  const now = new Date();
  const year = now.getFullYear();
  const month = now.getMonth();
  const days = new Date(year, month + 1, 0).getDate();
  const leading = (new Date(year, month, 1).getDay() + 6) % 7; // 0 = Monday
  const trailing = (7 - ((leading + days) % 7)) % 7;
  return { leading, days, trailing };
}

// Heatmap time buckets per granularity. Custom falls back to the daily
// week view until a real range is selected.
const HEAT_BUCKETS: Record<Exclude<Granularity, 'custom'>, string[]> = {
  daily: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
  weekly: ['W1', 'W2', 'W3', 'W4'],
  monthly: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'],
  yearly: ['Q1', 'Q2', 'Q3', 'Q4'],
};

function heatLabels(g: Granularity): string[] {
  return g === 'custom' ? HEAT_BUCKETS.daily : HEAT_BUCKETS[g];
}

/**
 * Short, stable label for a cache key in the debug readout:
 * `card:revenue:retail:daily:...` → `revenue`, `query:retail:daily:...` → `query`.
 */
function shortCacheLabel(key: string): string {
  const parts = key.split(':');
  if (parts[0] === 'card' && parts[1]) return parts[1]!;
  return parts[0] ?? key;
}

// ── Card definitions ─────────────────────────────────────────────────

interface AnalyticsCard {
  key: string;
  /** `null` = appears in both workspaces */
  workspace: WorkspaceView | null;
  /** Fluent message id for the title */
  titleKey: string;
  /** English fallback shown when the Fluent key is missing */
  title: string;
  /** Fluent message id for the one-line description (info tooltip) */
  descKey: string;
  /** `wide` = span 2 columns; `full` = span all columns; default = single */
  size?: 'wide' | 'full';
}

const ANALYTICS_CARDS: AnalyticsCard[] = [
  // 2×1 wide heatmap
  { key: 'heatmap',   workspace: null,         titleKey: 'analytics-card-heatmap', title: 'Heat Map', descKey: 'analytics-card-desc-heatmap', size: 'wide' },
  // Shared (both retail and restaurant)
  { key: 'revenue',   workspace: null,         titleKey: 'analytics-card-revenue',    title: 'Revenue Overview', descKey: 'analytics-card-desc-revenue' },
  { key: 'aov',       workspace: null,         titleKey: 'analytics-card-aov',        title: 'Average Order Value', descKey: 'analytics-card-desc-aov' },
  { key: 'staff',     workspace: null,         titleKey: 'analytics-card-staff',      title: 'Staff Performance', descKey: 'analytics-card-desc-staff' },
  { key: 'customers', workspace: null,         titleKey: 'analytics-card-customers',  title: 'New vs Returning Customers', descKey: 'analytics-card-desc-customers' },
  { key: 'payments',  workspace: null,         titleKey: 'analytics-card-payments',   title: 'Payment Methods', descKey: 'analytics-card-desc-payments' },
  { key: 'discounts', workspace: null,         titleKey: 'analytics-card-discounts',  title: 'Discounts & Promotions', descKey: 'analytics-card-desc-discounts' },
  { key: 'refunds',   workspace: null,         titleKey: 'analytics-card-refunds',    title: 'Refunds & Voids', descKey: 'analytics-card-desc-refunds' },
  // Retail-only
  { key: 'top-items', workspace: 'retail',     titleKey: 'analytics-card-top-products', title: 'Top Products', descKey: 'analytics-card-desc-top-products' },
  { key: 'category',  workspace: 'retail',     titleKey: 'analytics-card-category',   title: 'Sales by Category', descKey: 'analytics-card-desc-category' },
  { key: 'basket',    workspace: 'retail',     titleKey: 'analytics-card-basket',     title: 'Average Basket Size', descKey: 'analytics-card-desc-basket' },
  { key: 'inventory', workspace: 'retail',     titleKey: 'analytics-card-inventory',  title: 'Stock Turnover', descKey: 'analytics-card-desc-inventory' },
  { key: 'low-stock', workspace: 'retail',     titleKey: 'analytics-card-low-stock',  title: 'Low Stock Alerts', descKey: 'analytics-card-desc-low-stock' },
  // Restaurant-only
  { key: 'top-items', workspace: 'restaurant', titleKey: 'analytics-card-top-menu',   title: 'Top Menu Items', descKey: 'analytics-card-desc-top-menu' },
  { key: 'tables',    workspace: 'restaurant', titleKey: 'analytics-card-tables',     title: 'Table Turnover', descKey: 'analytics-card-desc-tables' },
  { key: 'occupancy', workspace: 'restaurant', titleKey: 'analytics-card-occupancy',  title: 'Table Occupancy', descKey: 'analytics-card-desc-occupancy' },
  { key: 'waitstaff', workspace: 'restaurant', titleKey: 'analytics-card-waitstaff',  title: 'Top Waitstaff', descKey: 'analytics-card-desc-waitstaff' },
  { key: 'voids',     workspace: 'restaurant', titleKey: 'analytics-card-voids',      title: 'Voided Items', descKey: 'analytics-card-desc-voids' },
];

// ── Component ─────────────────────────────────────────────────────────

export default function AnalyticsScreen() {
  const { l10n } = useLocalization();
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';

  const [workspaceView, setWorkspaceView] = useState<WorkspaceView>('retail');
  const [granularity, setGranularity] = useState<Granularity>('daily');
  const [customFrom, setCustomFrom] = useState(isoToday());
  const [customTo, setCustomTo] = useState(isoToday());
  const [zoomLevel, setZoomLevel] = useState<number>(() => {
    const saved = Number(localStorage.getItem('oz-analytics-zoom'));
    return saved >= ZOOM_MIN && saved <= ZOOM_MAX ? saved : 1;
  });
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const [expandScale, setExpandScale] = useState(1);
  const [showScrollTop, setShowScrollTop] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [allCollapsed, setAllCollapsed] = useState(false);
const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState('');
  const [paletteIndex, setPaletteIndex] = useState(0);
  const [scrollProgress, setScrollProgress] = useState(0);
  const [toasts, setToasts] = useState<{ id: number; message: string }[]>([]);
  const [menuCardId, setMenuCardId] = useState<string | null>(null);
  const [collapsedCards, setCollapsedCards] = useState<Set<string>>(new Set());
  const [zoomPopover, setZoomPopover] = useState(false);
  const [showCacheMetrics, setShowCacheMetrics] = useState(false);
  const [compare, setCompare] = useState(false);
  const [, setMetricsTick] = useState(0);
  const paletteInputRef = useRef<HTMLInputElement | null>(null);
  const toastId = useRef(0);
  const cardRefs = useRef(new Map<string, HTMLDivElement>());
  const [cardOrder, setCardOrder] = useState<string[]>([]);
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);
  const [, setRecalcTick] = useState(0);
  const expandedBodyRef = useRef<HTMLDivElement | null>(null);
  const mainRef = useRef<HTMLElement | null>(null);

  // Inclusive [from, to] window for the current granularity/custom range.
  const dateRange = useMemo(
    () => rangeForGranularity(granularity, customFrom, customTo),
    [granularity, customFrom, customTo],
  );

  /**
   * Kick off a recalculation. `force` (refresh button / R key) always
   * refetches; otherwise an identical query still fresh in the TTL cache
   * renders instantly — switching granularity or workspace back and
   * forth does not refetch identical queries.
   *
   * There is no artificial delay: cards show their own loading skeleton
   * while their IPC query actually resolves, and the heatmap shows its
   * skeleton while its query is in flight. The tick just re-renders the
   * grid so cards re-evaluate their (possibly now-cleared) queries.
   */
  const startRecalculating = useRef<(force?: boolean) => void>();
  startRecalculating.current = (force = false) => {
    const key = analyticsQueryKey(workspaceView, granularity, customFrom, customTo);
    if (force) {
      // Refresh wipes the cached payloads AND the recorded query
      // failures so the data actually recomputes; the TTL-bounded
      // cache refills on the next render.
      clearAnalyticsCache();
      clearAnalyticsErrors();
    } else {
      // Mark the query computed — revisits within the TTL render
      // instantly (card payloads were cached when they rendered).
      analyticsDataCache.set(key, { computedAt: Date.now() });
    }
    setRecalcTick((n) => n + 1);
  };

  // Recalculate when filters change
  useEffect(() => {
    startRecalculating.current?.();
  }, [workspaceView, granularity, customFrom, customTo]);

  const zoomIn = () => setZoomLevel((z) => Math.min(ZOOM_MAX, +(z + ZOOM_STEP).toFixed(2)));
  const zoomOut = () => setZoomLevel((z) => Math.max(ZOOM_MIN, +(z - ZOOM_STEP).toFixed(2)));
  const zoomReset = () => {
    setZoomLevel(1);
    showToast(l10n.getString('analytics-toast-zoom-reset'));
  };

  // Live refresh of the debug cache-metrics readout while it is open.
  useEffect(() => {
    if (!showCacheMetrics) return;
    const id = setInterval(() => setMetricsTick((t) => t + 1), 1000);
    return () => clearInterval(id);
  }, [showCacheMetrics]);

  // Transient toast feedback — auto-dismisses per toast
  const showToast = (message: string) => {
    const id = ++toastId.current;
    setToasts((t) => [...t.slice(-2), { id, message }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 2600);
  };

  // Persist zoom across sessions
  useEffect(() => {
    try {
      localStorage.setItem('oz-analytics-zoom', String(zoomLevel));
    } catch {
      /* storage unavailable */
    }
  }, [zoomLevel]);

  // Keyboard shortcuts (ignored while typing in form fields)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'SELECT' || t.tagName === 'TEXTAREA')) return;
      if (paletteOpen) return;
      const k = e.key;
      if (k >= '1' && k <= '5') {
        setGranularity(GRANULARITIES[Number(k) - 1]!);
      } else if (k === 'r' || k === 'R') {
        startRecalculating.current?.(true);
      } else if (k === '+') {
        zoomIn();
      } else if (k === '-' || k === '_') {
        zoomOut();
      } else if (k === '0') {
        zoomReset();
      } else if (k === 'c' || k === 'C') {
        setAllCollapsed((c) => !c);
      } else if (k === 'Escape') {
        setExpandedKey(null);
        setShowShortcuts(false);
        setMenuCardId(null);
        setZoomPopover(false);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [zoomIn, zoomOut, zoomReset, paletteOpen]);

  // Smart scaling: when a card is expanded, scale its content to fill the
  // available body area (works for any card — heatmap, table, or chart).
  useEffect(() => {
    if (!expandedKey) {
      setExpandScale(1);
      return;
    }
    const body = expandedBodyRef.current;
    const content = body?.querySelector<HTMLElement>('.analytics-card-content');
    if (!body || !content) return;
    setExpandScale(smartScale(
      { w: body.clientWidth, h: body.clientHeight },
      { w: content.offsetWidth, h: content.offsetHeight },
    ));
  }, [expandedKey, granularity, workspaceView]);

  // Filter cards visible for the current workspace
  const visibleCards = ANALYTICS_CARDS.filter(
    (c) => c.workspace === null || c.workspace === workspaceView,
  );

  const cardId = (c: AnalyticsCard) => `${c.key}-${c.workspace ?? 'shared'}`;

  const orderStorageKey = `oz-analytics-card-order-${workspaceView}`;
  const defaultOrder = ANALYTICS_CARDS.map(cardId);

  // Load the saved card order per workspace; merge any new cards at the end
  useEffect(() => {
    let order = defaultOrder;
    try {
      const saved = localStorage.getItem(orderStorageKey);
      if (saved) {
        const parsed = JSON.parse(saved) as string[];
        const known = new Set(defaultOrder);
        const filtered = parsed.filter((id) => known.has(id));
        order = [...filtered, ...defaultOrder.filter((id) => !filtered.includes(id))];
      }
    } catch {
      /* corrupt storage — fall back to default order */
    }
    setCardOrder(order);
  }, [workspaceView]);

  const persistOrder = (order: string[]) => {
    setCardOrder(order);
    try {
      localStorage.setItem(orderStorageKey, JSON.stringify(order));
    } catch {
      /* storage unavailable — keep in-memory order */
    }
  };

  const reorderCard = (from: string, to: string) => {
    if (from === to) return;
    const order = [...cardOrder];
    const i = order.indexOf(from);
    const j = order.indexOf(to);
    if (i < 0 || j < 0) return;
    order.splice(i, 1);
    order.splice(j, 0, from);
    persistOrder(order);
    showToast(l10n.getString('analytics-toast-layout-saved'));
  };

  const moveCard = (id: string, dir: 'up' | 'down' | 'top' | 'bottom') => {
    const order = [...cardOrder];
    const i = order.indexOf(id);
    if (i < 0) return;
    if (dir === 'up' && i > 0) {
      order.splice(i, 1);
      order.splice(i - 1, 0, id);
    } else if (dir === 'down' && i < order.length - 1) {
      order.splice(i, 1);
      order.splice(i + 1, 0, id);
    } else if (dir === 'top' && i > 0) {
      order.splice(i, 1);
      order.unshift(id);
    } else if (dir === 'bottom' && i < order.length - 1) {
      order.splice(i, 1);
      order.push(id);
    } else {
      return;
    }
    persistOrder(order);
    showToast(l10n.getString('analytics-toast-layout-saved'));
  };

  const toggleCardCollapsed = (id: string) => {
    setCollapsedCards((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const isDefaultOrder = JSON.stringify(cardOrder) === JSON.stringify(defaultOrder);

  const resetLayout = () => {
    try {
      localStorage.removeItem(orderStorageKey);
    } catch {
      /* storage unavailable */
    }
    setCardOrder(defaultOrder);
    showToast(l10n.getString('analytics-toast-layout-reset'));
  };

  // ── Command palette (Ctrl/Cmd+K) ──────────────────────────────

  type PaletteItem =
    | { kind: 'workspace'; value: WorkspaceView; label: string; hint: string }
    | { kind: 'granularity'; value: Granularity; label: string; hint: string }
    | { kind: 'action'; value: 'collapse' | 'expand' | 'reset-zoom' | 'reset-layout' | 'home' | 'shortcuts'; label: string; hint: string };

  const paletteItems = useMemo<PaletteItem[]>(() => {
    const items: PaletteItem[] = [
      { kind: 'workspace', value: 'retail', label: l10n.getString('analytics-workspace-retail'), hint: '' },
      { kind: 'workspace', value: 'restaurant', label: l10n.getString('analytics-workspace-restaurant'), hint: '' },
    ];
    GRANULARITIES.forEach((g, i) => {
      items.push({ kind: 'granularity', value: g, label: l10n.getString(`analytics-granularity-${g}`), hint: String(i + 1) });
    });
    items.push(
      { kind: 'action', value: 'collapse', label: l10n.getString('analytics-action-collapse-all-aria'), hint: 'C' },
      { kind: 'action', value: 'expand', label: l10n.getString('analytics-action-expand-all-aria'), hint: 'C' },
      { kind: 'action', value: 'reset-zoom', label: l10n.getString('analytics-action-zoom-reset-aria'), hint: '0' },
      { kind: 'action', value: 'reset-layout', label: l10n.getString('analytics-reset-layout'), hint: '' },
      { kind: 'action', value: 'shortcuts', label: l10n.getString('analytics-shortcuts-title'), hint: '?' },
      { kind: 'action', value: 'home', label: l10n.getString('analytics-palette-home'), hint: '' },
    );
    return items;
  }, [l10n]);

  const q = paletteQuery.trim().toLowerCase();
  const filteredItems = q ? paletteItems.filter((it) => it.label.toLowerCase().includes(q)) : paletteItems;

  const runPaletteItem = (item: PaletteItem) => {
    if (item.kind === 'workspace') {
      setWorkspaceView(item.value);
      setGranularity('daily');
      setExpandedKey(null);
    } else if (item.kind === 'granularity') {
      setGranularity(item.value);
    } else {
      switch (item.value) {
        case 'collapse': setAllCollapsed(true); break;
        case 'expand': setAllCollapsed(false); break;
        case 'reset-zoom': zoomReset(); break;
        case 'reset-layout': resetLayout(); break;
        case 'home': goToWorkspacePicker(); break;
        case 'shortcuts': setShowShortcuts(true); break;
      }
    }
    setPaletteOpen(false);
    setPaletteQuery('');
  };

  const runPaletteRef = useRef(runPaletteItem);
  runPaletteRef.current = runPaletteItem;

  // Ctrl/Cmd+K toggles the palette
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        setPaletteQuery('');
        setPaletteIndex(0);
        setPaletteOpen((o) => !o);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // Keyboard navigation inside the open palette
  useEffect(() => {
    if (!paletteOpen) return;
    const onPaletteKey = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setPaletteIndex((i) => Math.min(i + 1, filteredItems.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setPaletteIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const item = filteredItems[paletteIndex];
        if (item) runPaletteRef.current(item);
      } else if (e.key === 'Escape') {
        setPaletteOpen(false);
        setPaletteQuery('');
      }
    };
    window.addEventListener('keydown', onPaletteKey);
    return () => window.removeEventListener('keydown', onPaletteKey);
  }, [paletteOpen, filteredItems, paletteIndex]);

  // Focus the search input when the palette opens
  useEffect(() => {
    if (paletteOpen) paletteInputRef.current?.focus();
  }, [paletteOpen]);

  // Keep the selection at the top when the query or palette changes
  useEffect(() => {
    setPaletteIndex(0);
  }, [paletteQuery, paletteOpen]);

  const applyRangePreset = (days: number) => {
    const to = new Date();
    const from = new Date();
    from.setDate(from.getDate() - (days - 1));
    setCustomTo(isoDay(to));
    setCustomFrom(isoDay(from));
  };

  // When a card is expanded, only it is shown; otherwise all visible cards
  const displayedCards = expandedKey && visibleCards.some((c) => cardId(c) === expandedKey)
    ? visibleCards.filter((c) => cardId(c) === expandedKey)
    : visibleCards;

  // Apply the user's saved order (falling back to the default when empty)
  const orderedCards = [...displayedCards].sort(
    (a, b) => cardOrder.indexOf(cardId(a)) - cardOrder.indexOf(cardId(b)),
  );

  // Smart heatmap — bucket cells change with the selected granularity.
  // Monthly renders one cell per day of the current month (28–31);
  // yearly renders a 12-month × 4-week grid; other granularities are flat.
  // Intensities come from real revenue rows via the TTL cache.
  const heatmapQuery = useAnalyticsQuery(
    cardQueryKey('heatmap', workspaceView, granularity, dateRange.from, dateRange.to),
    () => loadHeatmapRows({ workspace: workspaceView, granularity, from: dateRange.from, to: dateRange.to, sessionToken }),
  );
  const heatmapIntensities = heatmapQuery.data
    ? buildHeatmapIntensities(granularity, heatmapQuery.data)
    : new Map<string, number>();
  const heatCell = (key: string, label: string, reactKey?: string) => (
    <div
      key={reactKey ?? key}
      className="analytics-heat-cell"
      data-intensity={heatmapIntensities.get(key) ?? 0}
      title={label}
    >
      <div className="analytics-heat-block" />
    </div>
  );

  const renderHeatmap = () => {
    const aria = l10n.getString('analytics-card-heatmap');
    if (granularity === 'weekly') {
      const rows: JSX.Element[] = [
        <div key="header" className="analytics-weekly-row">
          <span className="analytics-heat-label analytics-weekly-day" />
          {Array.from({ length: 24 }, (_, h) => (
            <span key={h} className="analytics-heat-label analytics-weekly-hour">
              {String(h).padStart(2, '0')}
            </span>
          ))}
        </div>,
      ];
      HEAT_BUCKETS.daily.forEach((day, di) => {
        rows.push(
          <div key={day} className="analytics-weekly-row">
            <span className="analytics-heat-label analytics-weekly-day">{day}</span>
            {Array.from({ length: 24 }, (_, h) =>
              heatCell(`${di}:${h}`, `${day} ${String(h).padStart(2, '0')}:00`, `${day}-${h}`),
            )}
          </div>,
        );
      });
      return (
        <div className="analytics-heatmap analytics-heatmap--weekly" role="img" aria-label={aria}>
          {rows}
        </div>
      );
    }
    if (granularity === 'monthly') {
      const { leading, days, trailing } = monthCalendarGrid();
      const cells: JSX.Element[] = [];
      for (let i = 0; i < leading; i++) {
        cells.push(<div key={`lead-${i}`} className="analytics-heat-cell analytics-heat-cell--empty" />);
      }
      for (let d = 1; d <= days; d++) {
        cells.push(
          <div key={d} className="analytics-heat-cell" data-intensity={heatmapIntensities.get(String(d)) ?? 0} title={`Day ${d}`}>
            <div className="analytics-heat-block" />
            <span className="analytics-heat-label">{d}</span>
          </div>,
        );
      }
      for (let i = 0; i < trailing; i++) {
        cells.push(<div key={`trail-${i}`} className="analytics-heat-cell analytics-heat-cell--empty" />);
      }
      return (
        <div className="analytics-heatmap analytics-heatmap--monthly" role="img" aria-label={aria}>
          <div className="analytics-monthly-header">
            {HEAT_BUCKETS.daily.map((d) => (
              <span key={d} className="analytics-heat-label">{d}</span>
            ))}
          </div>
          <div className="analytics-monthly-grid">{cells}</div>
        </div>
      );
    }
    if (granularity === 'yearly') {
      return (
        <div className="analytics-heatmap analytics-heatmap--yearly" role="img" aria-label={aria}>
          {HEAT_BUCKETS.monthly.map((month, mi) => (
            <div className="analytics-heat-column" key={month}>
              <span className="analytics-heat-label">{month}</span>
              {[0, 1, 2, 3].map((week) => (
                <div key={week} className="analytics-heat-cell" data-intensity={heatmapIntensities.get(`${mi}:${week}`) ?? 0} title={`${month} W${week + 1}`}>
                  <div className="analytics-heat-block" />
                </div>
              ))}
            </div>
          ))}
        </div>
      );
    }
    const labels = heatLabels(granularity);
    return (
      <div className="analytics-heatmap" role="img" aria-label={aria}>
        {labels.map((label, i) => (
          <div key={label} className="analytics-heat-cell" data-intensity={heatmapIntensities.get(String(i)) ?? 0} title={label}>
            <div className="analytics-heat-block" />
            <span className="analytics-heat-label">{label}</span>
          </div>
        ))}
      </div>
    );
  };

  return (
    <div className="analytics" role="region" aria-label={l10n.getString('analytics-region-aria')}>

      {/* ══════════════════════════════════════════════════════════
          AREA 1 — Top: back button + title
          ══════════════════════════════════════════════════════════ */}
      <header className="analytics-topbar">
        <button
          type="button"
          className="analytics-back-btn"
          onClick={goToWorkspacePicker}
          aria-label={l10n.getString('analytics-back-aria')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
            strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
            width="18" height="18" aria-hidden="true"
          >
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
        </button>

        <div className="analytics-title-group">
          <Localized id="analytics-title">
            <h1 className="analytics-title">Analytics</h1>
          </Localized>
          <Localized id="analytics-subtitle">
            <p className="analytics-subtitle">Sales, products, and staff performance</p>
          </Localized>
        </div>
      </header>

      {/* ══════════════════════════════════════════════════════════
          AREA 2 — Menu: workspace selector + granularity buttons
          ══════════════════════════════════════════════════════════ */}
      <nav className="analytics-menu">
        {/* Row 1 — workspace selector */}
        <div className="analytics-menu-row">
          <select
            className="analytics-workspace-select-input"
            value={workspaceView}
            onChange={(e) => {
              setWorkspaceView(e.target.value as WorkspaceView);
              setGranularity('daily');
              setExpandedKey(null);
            }}
            aria-label={l10n.getString('analytics-workspace-select-aria')}
          >
            <option value="retail">{l10n.getString('analytics-workspace-retail')}</option>
            <option value="restaurant">{l10n.getString('analytics-workspace-restaurant')}</option>
          </select>
        </div>

        {/* Row 2 — granularity pill buttons + custom date range inline */}
        <div className="analytics-menu-row">
          <div
            className="analytics-granularity"
            role="radiogroup"
            aria-label={l10n.getString('analytics-granularity-aria')}
          >
            {GRANULARITIES.map((g) => (
              <button
                key={g}
                type="button"
                className={`analytics-granularity-btn${granularity === g ? ' analytics-granularity-btn--active' : ''}`}
                onClick={() => setGranularity(g)}
                role="radio"
                aria-checked={granularity === g}
                title={`${l10n.getString(`analytics-granularity-${g}`)} (${GRANULARITIES.indexOf(g) + 1})`}
              >
                <Localized id={`analytics-granularity-${g}`}>
                  <span>{g}</span>
                </Localized>
              </button>
            ))}
          </div>

          {granularity === 'custom' && (
            <>
              <div className="analytics-custom-range">
                <label className="analytics-custom-field">
                  <Localized id="analytics-custom-from">
                    <span className="analytics-custom-label">From</span>
                  </Localized>
                  <input
                    type="date"
                    className="analytics-custom-input"
                    value={customFrom}
                    max={customTo}
                    onChange={(e) => setCustomFrom(e.target.value)}
                    aria-label={l10n.getString('analytics-custom-from')}
                  />
                </label>
                <span className="analytics-custom-sep">—</span>
                <label className="analytics-custom-field">
                  <Localized id="analytics-custom-to">
                    <span className="analytics-custom-label">To</span>
                  </Localized>
                  <input
                    type="date"
                    className="analytics-custom-input"
                    value={customTo}
                    min={customFrom}
                    onChange={(e) => setCustomTo(e.target.value)}
                    aria-label={l10n.getString('analytics-custom-to')}
                  />
                </label>
              </div>
              <div className="analytics-custom-presets" role="group" aria-label={l10n.getString('analytics-range-presets-aria')}>
                {[7, 30, 90, 365].map((days) => (
                  <button
                    key={days}
                    type="button"
                    className="analytics-preset-chip"
                    onClick={() => applyRangePreset(days)}
                    aria-label={l10n.getString(`analytics-range-preset-${days}d`)}
                  >
                    {l10n.getString(`analytics-range-preset-${days}d`)}
                  </button>
                ))}
              </div>
            </>
          )}

          {/* Action buttons — collapse, refresh, zoom out, zoom in */}
          <div className="analytics-actions">
            <button
              type="button"
              className={`analytics-action-btn${compare ? ' analytics-action-btn--active' : ''}`}
              onClick={() => {
                const next = !compare;
                setCompare(next);
                showToast(l10n.getString(next ? 'analytics-toast-compare-on' : 'analytics-toast-compare-off'));
              }}
              aria-pressed={compare}
              aria-label={l10n.getString(compare ? 'analytics-compare-off-aria' : 'analytics-compare-on-aria')}
              title={l10n.getString(compare ? 'analytics-compare-off-aria' : 'analytics-compare-on-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                <path d="M3 7h13" />
                <path d="M3 12h9" />
                <path d="M3 17h5" />
                <polyline points="18 4 22 8 18 12" />
                <polyline points="14 12 18 16 14 20" />
              </svg>
            </button>
            <button
              type="button"
              className={`analytics-action-btn${allCollapsed ? ' analytics-action-btn--active' : ''}`}
              onClick={() => {
                const next = !allCollapsed;
                setAllCollapsed(next);
                showToast(l10n.getString(next ? 'analytics-toast-collapsed' : 'analytics-toast-expanded'));
                // Collapsing all while a card is expanded would otherwise
                // leave the grid showing only that card — restore the grid
                // so the toggle visibly does what its label promises.
                if (next) setExpandedKey(null);
              }}
              aria-label={l10n.getString(allCollapsed ? 'analytics-action-expand-all-aria' : 'analytics-action-collapse-all-aria')}
              title={l10n.getString(allCollapsed ? 'analytics-action-expand-all-aria' : 'analytics-action-collapse-all-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                {allCollapsed ? (
                  <>
                    <path d="M4 14h16" />
                    <path d="M4 18h16" />
                    <path d="M4 6l4 4 4-4" />
                  </>
                ) : (
                  <>
                    <path d="M4 6h16" />
                    <path d="M4 10h16" />
                    <path d="M4 14l4 4 4-4" />
                  </>
                )}
              </svg>
            </button>
            <button
              type="button"
              className="analytics-action-btn"
              onClick={() => {
                startRecalculating.current?.(true);
                showToast(l10n.getString('analytics-toast-refreshing'));
              }}
              aria-label={l10n.getString('analytics-action-refresh-aria')}
              title={l10n.getString('analytics-action-refresh-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
            </button>
            <button
              type="button"
              className="analytics-action-btn"
              onClick={zoomOut}
              disabled={zoomLevel <= ZOOM_MIN}
              aria-label={l10n.getString('analytics-action-zoom-out-aria')}
              title={l10n.getString('analytics-action-zoom-out-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
                <line x1="8" y1="11" x2="14" y2="11" />
              </svg>
            </button>
            <button
              type="button"
              className="analytics-zoom-badge"
              onClick={() => setZoomPopover((o) => !o)}
              aria-label={l10n.getString('analytics-zoom-slider-aria')}
              title={l10n.getString('analytics-zoom-slider-aria')}
            >
              {Math.round(zoomLevel * 100)}%
            </button>
            {zoomPopover && (
              <div className="analytics-zoom-popover" role="dialog" aria-label={l10n.getString('analytics-zoom-slider-aria')}>
                <input
                  type="range"
                  className="analytics-zoom-slider"
                  min={ZOOM_MIN * 100}
                  max={ZOOM_MAX * 100}
                  step={ZOOM_STEP * 100}
                  value={Math.round(zoomLevel * 100)}
                  onChange={(e) => setZoomLevel(Number(e.target.value) / 100)}
                  aria-label={l10n.getString('analytics-zoom-slider-aria')}
                />
                <span className="analytics-zoom-popover-value">{Math.round(zoomLevel * 100)}%</span>
                <button
                  type="button"
                  className="analytics-zoom-reset-btn"
                  onClick={zoomReset}
                >
                  {l10n.getString('analytics-action-zoom-reset-aria')}
                </button>
              </div>
            )}
            <button
              type="button"
              className="analytics-action-btn"
              onClick={zoomIn}
              disabled={zoomLevel >= ZOOM_MAX}
              aria-label={l10n.getString('analytics-action-zoom-in-aria')}
              title={l10n.getString('analytics-action-zoom-in-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
                <line x1="11" y1="8" x2="11" y2="14" />
                <line x1="8" y1="11" x2="14" y2="11" />
              </svg>
            </button>
            <button
              type="button"
              className="analytics-action-btn"
              onClick={() => setShowShortcuts((s) => !s)}
              aria-label={l10n.getString('analytics-shortcuts-aria')}
              title={l10n.getString('analytics-shortcuts-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                <circle cx="12" cy="12" r="10" />
                <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
                <line x1="12" y1="17" x2="12.01" y2="17" />
              </svg>
            </button>

            {showShortcuts && (
              <div className="analytics-shortcuts-popover" role="dialog" aria-label={l10n.getString('analytics-shortcuts-title')}>
                <h3 className="analytics-shortcuts-title">{l10n.getString('analytics-shortcuts-title')}</h3>
                <ul className="analytics-shortcuts-list">
                  {SHORTCUTS.map((s) => (
                    <li key={s.labelKey} className="analytics-shortcuts-item">
                      <kbd className="analytics-shortcuts-keys">{s.keys}</kbd>
                      <span>{l10n.getString(s.labelKey)}</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </div>
      </nav>

      {/* Scroll progress — flush against the menu's bottom edge, tracks
          the main area's scroll position (no gap, no own spacing) */}
      <div className="analytics-scroll-progress" style={{ width: `${scrollProgress * 100}%` }} aria-hidden="true" />

      {/* ══════════════════════════════════════════════════════════
          AREA 3 — Main content: smart analytics card grid
          ══════════════════════════════════════════════════════════ */}
      <main
        className="analytics-main"
        ref={mainRef}
        onScroll={(e) => {
          const el = e.currentTarget;
          setShowScrollTop(el.scrollTop > 240);
          const max = el.scrollHeight - el.clientHeight;
          setScrollProgress(max > 0 ? Math.min(1, el.scrollTop / max) : 0);
        }}
      >
        {/* View status — card count + workspace + time view */}
        <div className="analytics-status">
          <span className="analytics-status-item">
            <Localized id="analytics-status-cards" vars={{ count: String(displayedCards.length) }}>
              <span>{displayedCards.length} cards</span>
            </Localized>
          </span>
          <span className="analytics-status-sep" aria-hidden="true">·</span>
          <span className="analytics-status-item">
            {l10n.getString(workspaceView === 'retail' ? 'analytics-workspace-retail' : 'analytics-workspace-restaurant')}
            <span className="analytics-status-sep" aria-hidden="true">·</span>
            {l10n.getString(`analytics-granularity-${granularity}`)}
          </span>
          {granularity === 'custom' && (
            <>
              <span className="analytics-status-sep" aria-hidden="true">·</span>
              <span className="analytics-status-item">
                <Localized id="analytics-status-range" vars={{ from: customFrom, to: customTo }}>
                  <span>{customFrom} – {customTo}</span>
                </Localized>
              </span>
            </>
          )}
          {!isDefaultOrder && (
            <button
              type="button"
              className="analytics-reset-layout"
              onClick={resetLayout}
            >
              <Localized id="analytics-reset-layout"><span>Reset layout</span></Localized>
            </button>
          )}

          {/* Debug: TTL cache hit/miss/expiry readout per query key */}
          <div className="analytics-cache-metrics">
            <button
              type="button"
              className={`analytics-cache-chip${showCacheMetrics ? ' analytics-cache-chip--open' : ''}`}
              onClick={() => setShowCacheMetrics((o) => !o)}
              aria-expanded={showCacheMetrics}
              aria-label={l10n.getString('analytics-cache-metrics-aria')}
              title={l10n.getString('analytics-cache-metrics-aria')}
            >
              <span className="analytics-cache-chip-dot" aria-hidden="true" />
              <Localized id="analytics-cache-chip"><span>cache</span></Localized>
              <span className="analytics-cache-chip-rate">
                {(() => {
                  const { totals } = analyticsDataCache.metrics();
                  return totals.hitRate === null ? '–' : `${Math.round(totals.hitRate * 100)}%`;
                })()}
              </span>
            </button>
            {showCacheMetrics && (
              <div className="analytics-cache-popover" role="dialog" aria-label={l10n.getString('analytics-cache-metrics-aria')}>
                <div className="analytics-cache-popover-head">
                  <div className="analytics-cache-popover-meta">
                    <h3 className="analytics-cache-popover-title">
                      <Localized id="analytics-cache-popover-title"><span>Cache metrics</span></Localized>
                    </h3>
                    {(() => {
                      const { totals } = analyticsDataCache.metrics();
                      const rate = totals.hitRate === null ? '–' : `${Math.round(totals.hitRate * 100)}%`;
                      return (
                        <span className="analytics-cache-popover-summary">
                          <Localized
                            id="analytics-cache-summary"
                            vars={{
                              rate,
                              hits: String(totals.hits),
                              misses: String(totals.misses),
                              expiries: String(totals.expiries),
                            }}
                          >
                            <span>{rate} · {totals.hits} hits · {totals.misses} misses · {totals.expiries} expired</span>
                          </Localized>
                        </span>
                      );
                    })()}
                  </div>
                  <button
                    type="button"
                    className="analytics-cache-clear-btn"
                    onClick={() => {
                      clearAnalyticsCache();
                      setMetricsTick((t) => t + 1);
                      showToast(l10n.getString('analytics-toast-cache-cleared'));
                    }}
                    aria-label={l10n.getString('analytics-cache-clear-aria')}
                    title={l10n.getString('analytics-cache-clear-aria')}
                  >
                    <Localized id="analytics-cache-clear"><span>Clear cache</span></Localized>
                  </button>
                </div>
                <table className="analytics-cache-table">
                  <thead>
                    <tr>
                      <th><Localized id="analytics-cache-col-key"><span>key</span></Localized></th>
                      <th><Localized id="analytics-cache-col-hits"><span>hits</span></Localized></th>
                      <th><Localized id="analytics-cache-col-misses"><span>misses</span></Localized></th>
                      <th><Localized id="analytics-cache-col-expiries"><span>expired</span></Localized></th>
                      <th><Localized id="analytics-cache-col-evictions"><span>evicted</span></Localized></th>
                    </tr>
                  </thead>
                  <tbody>
                    {(() => {
                      const { perKey } = analyticsDataCache.metrics();
                      const rows = [...perKey.entries()].sort((a, b) => {
                        const readsB = b[1].hits + b[1].misses + b[1].expiries;
                        const readsA = a[1].hits + a[1].misses + a[1].expiries;
                        return readsB - readsA;
                      });
                      if (rows.length === 0) {
                        return (
                          <tr>
                            <td colSpan={5} className="analytics-cache-empty">
                              <Localized id="analytics-cache-empty"><span>No queries yet</span></Localized>
                            </td>
                          </tr>
                        );
                      }
                      return rows.map(([key, m]) => (
                        <tr key={key} title={key}>
                          <td className="analytics-cache-key">{shortCacheLabel(key)}</td>
                          <td>{m.hits}</td>
                          <td>{m.misses}</td>
                          <td>{m.expiries}</td>
                          <td>{m.evictions}</td>
                        </tr>
                      ));
                    })()}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>

        <div className="analytics-grid" style={{ zoom: zoomLevel }}>
          {orderedCards.map((card) => {
            const cid = cardId(card);
            const isExpanded = expandedKey === cid;
            const isCollapsed = !isExpanded && (allCollapsed || collapsedCards.has(cid));
            const isDragging = dragId === cid;
            const isDropTarget = overId === cid;
            const menuOpen = menuCardId === cid;
            const idx = cardOrder.indexOf(cid);
            const isFirst = idx === 0;
            const isLast = idx === cardOrder.length - 1;
            return (
            <div
              key={cid}
              ref={(el) => { if (el) cardRefs.current.set(cid, el); else cardRefs.current.delete(cid); }}
              role="button"
              tabIndex={0}
              draggable={!isExpanded}
              aria-label={card.title}
              onKeyDown={(e) => {
                const idx = orderedCards.findIndex((c) => cardId(c) === cid);
                if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
                  e.preventDefault();
                  const next = orderedCards[idx + 1];
                  if (next) cardRefs.current.get(cardId(next))?.focus();
                } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
                  e.preventDefault();
                  const prev = orderedCards[idx - 1];
                  if (prev) cardRefs.current.get(cardId(prev))?.focus();
                } else if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setExpandedKey((current) => nextExpandedKey(current, cid));
                }
              }}
              onDragStart={(e) => { setDragId(cid); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
              onDragOver={(e) => { e.preventDefault(); if (overId !== cid) setOverId(cid); }}
              onDragLeave={() => setOverId((o) => (o === cid ? null : o))}
              onDrop={(e) => { e.preventDefault(); reorderCard(dragId ?? '', cid); setDragId(null); setOverId(null); }}
              onDragEnd={() => { setDragId(null); setOverId(null); }}
              className={`analytics-card${card.size ? ` analytics-card--${card.size}` : ''}${isExpanded ? ' analytics-card--expanded' : ''}${isCollapsed ? ' analytics-card--collapsed' : ''}${isDragging ? ' analytics-card--dragging' : ''}${isDropTarget ? ' analytics-card--drop-target' : ''}`}
            >
              <div className="analytics-card-header">
                <span className="analytics-card-grip" aria-hidden="true">
                  <svg viewBox="0 0 24 24" fill="currentColor" width="12" height="12">
                    <circle cx="9" cy="5" r="1.4" /><circle cx="15" cy="5" r="1.4" />
                    <circle cx="9" cy="12" r="1.4" /><circle cx="15" cy="12" r="1.4" />
                    <circle cx="9" cy="19" r="1.4" /><circle cx="15" cy="19" r="1.4" />
                  </svg>
                </span>
                <Localized id={card.titleKey}>
                  <h2 className="analytics-card-title">{card.title}</h2>
                </Localized>
                <div className="analytics-card-actions">
                  <button
                    type="button"
                    className="analytics-card-action analytics-card-info"
                    onClick={(e) => e.stopPropagation()}
                    aria-label={l10n.getString(card.descKey)}
                    title={l10n.getString(card.descKey)}
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                      strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                      <circle cx="12" cy="12" r="10" />
                      <line x1="12" y1="16" x2="12" y2="12" />
                      <line x1="12" y1="8" x2="12.01" y2="8" />
                    </svg>
                  </button>
                  <button
                    type="button"
                    className="analytics-card-action"
                    onClick={() => setExpandedKey((current) => {
                      const next = nextExpandedKey(current, cid);
                      // Expanding a card while in compact mode shows the card
                      // in full; collapse-all and expand are mutually exclusive.
                      if (next) setAllCollapsed(false);
                      return next;
                    })}
                    aria-label={l10n.getString(isExpanded ? 'analytics-card-restore-aria' : 'analytics-card-expand-aria')}
                    title={l10n.getString(isExpanded ? 'analytics-card-restore-aria' : 'analytics-card-expand-aria')}
                  >
                    {isExpanded ? (
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                        strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                        <polyline points="4 14 10 14 10 20" />
                        <polyline points="20 10 14 10 14 4" />
                        <line x1="14" y1="10" x2="21" y2="3" />
                        <line x1="3" y1="21" x2="10" y2="14" />
                      </svg>
                    ) : (
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                        strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                        <polyline points="15 3 21 3 21 9" />
                        <polyline points="9 21 3 21 3 15" />
                        <line x1="21" y1="3" x2="14" y2="10" />
                        <line x1="3" y1="21" x2="10" y2="14" />
                      </svg>
                    )}
                  </button>
                  <button
                    type="button"
                    className={`analytics-card-action${menuOpen ? ' analytics-card-action--active' : ''}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      setMenuCardId(menuOpen ? null : cid);
                    }}
                    aria-label={l10n.getString('analytics-card-menu-aria')}
                    aria-haspopup="menu"
                    aria-expanded={menuOpen}
                    title={l10n.getString('analytics-card-menu-aria')}
                  >
                    <svg viewBox="0 0 24 24" fill="currentColor" width="14" height="14" aria-hidden="true">
                      <circle cx="5" cy="12" r="1.6" />
                      <circle cx="12" cy="12" r="1.6" />
                      <circle cx="19" cy="12" r="1.6" />
                    </svg>
                  </button>
                  {menuOpen && (
                    <div className="analytics-card-menu" role="menu" aria-label={l10n.getString('analytics-card-menu-aria')}>
                      <button type="button" role="menuitem" disabled={isFirst}
                        onClick={() => { moveCard(cid, 'up'); setMenuCardId(null); }}>
                        {l10n.getString('analytics-menu-move-up')}
                      </button>
                      <button type="button" role="menuitem" disabled={isLast}
                        onClick={() => { moveCard(cid, 'down'); setMenuCardId(null); }}>
                        {l10n.getString('analytics-menu-move-down')}
                      </button>
                      <button type="button" role="menuitem" disabled={isFirst}
                        onClick={() => { moveCard(cid, 'top'); setMenuCardId(null); }}>
                        {l10n.getString('analytics-menu-move-top')}
                      </button>
                      <button type="button" role="menuitem" disabled={isLast}
                        onClick={() => { moveCard(cid, 'bottom'); setMenuCardId(null); }}>
                        {l10n.getString('analytics-menu-move-bottom')}
                      </button>
                      <div className="analytics-card-menu-sep" role="separator" />
                      <button type="button" role="menuitem"
                        onClick={() => {
                          setExpandedKey((current) => nextExpandedKey(current, cid));
                          setMenuCardId(null);
                        }}>
                        {l10n.getString(isExpanded ? 'analytics-card-restore-aria' : 'analytics-card-expand-aria')}
                      </button>
                      <button type="button" role="menuitem"
                        onClick={() => { toggleCardCollapsed(cid); setMenuCardId(null); }}>
                        {l10n.getString(isCollapsed ? 'analytics-menu-show-card' : 'analytics-menu-collapse-card')}
                      </button>
                    </div>
                  )}
                </div>
              </div>
              <div className="analytics-card-body" ref={isExpanded ? expandedBodyRef : undefined}>
                <div
                  className="analytics-card-content"
                  style={isExpanded ? { transform: `scale(${expandScale})` } : undefined}
                >
                  {card.key === 'heatmap' ? (
                    heatmapQuery.status === 'loading' ? (
                      <div className="analytics-card-skeleton analytics-heat-skeleton">
                        {Array.from({ length: 28 }, (_, i) => (
                          <div key={i} className="skeleton-bar skeleton-heat-block" />
                        ))}
                      </div>
                    ) : heatmapQuery.status === 'error' ? (
                      <div className="analytics-card-error" role="alert">
                        <span className="analytics-card-error-icon" aria-hidden="true">⚠</span>
                        <span className="analytics-card-error-text">
                          {l10nErrorMessage(heatmapQuery.error, l10n, 'analytics-card-error-load')}
                        </span>
                      </div>
                    ) : (
                      <>
                        {renderHeatmap()}
                        <div
                          className="analytics-heat-scale"
                          role="group"
                          aria-label={l10n.getString('analytics-heat-scale-aria')}
                        >
                          <span className="analytics-heat-scale-label">{l10n.getString('analytics-heat-scale-low')}</span>
                          {[0, 1, 2, 3, 4].map((i) => (
                            <span key={i} className="analytics-heat-cell analytics-heat-scale-swatch" data-intensity={i} aria-hidden="true">
                              <div className="analytics-heat-block" />
                            </span>
                          ))}
                          <span className="analytics-heat-scale-label">{l10n.getString('analytics-heat-scale-high')}</span>
                        </div>
                      </>
                    )
                  ) : (
                    <AnalyticsCardContent
                      cardKey={card.key}
                      granularity={granularity}
                      workspaceView={workspaceView}
                      from={dateRange.from}
                      to={dateRange.to}
                      sessionToken={sessionToken}
                      title={l10n.getString(card.titleKey)}
                      expanded={isExpanded}
                      compare={compare}
                    />
                  )}
                </div>
              </div>
            </div>
            );
          })}
        </div>

        {/* Scroll-to-top — appears after scrolling the grid */}
        {showScrollTop && (
          <button
            type="button"
            className="analytics-scroll-top"
            onClick={() => mainRef.current?.scrollTo({ top: 0, behavior: 'smooth' })}
            aria-label={l10n.getString('analytics-scroll-top-aria')}
            title={l10n.getString('analytics-scroll-top-aria')}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
              strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
              <polyline points="18 15 12 9 6 15" />
            </svg>
          </button>
        )}
      </main>

      {/* Outside-click backdrop for the per-card options menu */}
      {menuCardId && (
        <div
          className="analytics-menu-backdrop"
          role="presentation"
          tabIndex={-1}
          onClick={(e) => { if (e.target === e.currentTarget) setMenuCardId(null); }}
        />
      )}

      {/* Transient action feedback toasts */}
      {toasts.length > 0 && (
        <div className="analytics-toasts" role="status" aria-live="polite">
          {toasts.map((t) => (
            <div key={t.id} className="analytics-toast">{t.message}</div>
          ))}
        </div>
      )}

      {/* Command palette overlay (Ctrl/Cmd+K) */}
      {paletteOpen && (
        <div
          className="analytics-palette-backdrop"
          role="presentation"
          tabIndex={-1}
          onClick={(e) => { if (e.target === e.currentTarget) { setPaletteOpen(false); setPaletteQuery(''); } }}
        >
          <div
            className="analytics-palette"
            role="dialog"
            aria-label={l10n.getString('analytics-palette-aria')}
          >
            <input
              ref={paletteInputRef}
              type="text"
              className="analytics-palette-input"
              value={paletteQuery}
              onChange={(e) => setPaletteQuery(e.target.value)}
              placeholder={l10n.getString('analytics-palette-placeholder')}
              aria-label={l10n.getString('analytics-palette-placeholder')}
            />
            <ul className="analytics-palette-list" role="listbox" aria-label={l10n.getString('analytics-palette-aria')}>
              {filteredItems.length === 0 ? (
                <li className="analytics-palette-empty">{l10n.getString('analytics-palette-empty')}</li>
              ) : (
                filteredItems.map((item, i) => (
                  <li key={`${item.kind}-${item.value}`}>
                    <button
                      type="button"
                      className={`analytics-palette-item${i === paletteIndex ? ' analytics-palette-item--active' : ''}`}
                      onMouseEnter={() => setPaletteIndex(i)}
                      onClick={() => runPaletteItem(item)}
                    >
                      <span>{item.label}</span>
                      {item.hint && <kbd className="analytics-palette-hint">{item.hint}</kbd>}
                    </button>
                  </li>
                ))
              )}
            </ul>
          </div>
        </div>
      )}

    </div>
  );
}
