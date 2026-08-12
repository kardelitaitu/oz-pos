//! Analytics Screen — layout shell with three flex areas.
//!
//! Top:    back button + title
//! Menu:   workspace selector (row 1) + time granularity buttons (row 2)
//!         + inline custom date range
//! Main:   smart card grid — cards adapt to retail vs restaurant

import { useEffect, useMemo, useRef, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import './AnalyticsScreen.css';

type WorkspaceView = 'retail' | 'restaurant';
type Granularity = 'daily' | 'weekly' | 'monthly' | 'yearly' | 'custom';

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

// ── Card definitions ─────────────────────────────────────────────────

interface AnalyticsCard {
  key: string;
  /** `null` = appears in both workspaces */
  workspace: WorkspaceView | null;
  /** Fluent message id */
  titleKey: string;
  /** English fallback shown when the Fluent key is missing */
  title: string;
  /** `wide` = span 2 columns; `full` = span all columns; default = single */
  size?: 'wide' | 'full';
}

const ANALYTICS_CARDS: AnalyticsCard[] = [
  // 2×1 wide heatmap
  { key: 'heatmap',   workspace: null,         titleKey: 'analytics-card-heatmap', title: 'Heat Map', size: 'wide' },
  // Shared (both retail and restaurant)
  { key: 'revenue',   workspace: null,         titleKey: 'analytics-card-revenue',    title: 'Revenue Overview' },
  { key: 'aov',       workspace: null,         titleKey: 'analytics-card-aov',        title: 'Average Order Value' },
  { key: 'staff',     workspace: null,         titleKey: 'analytics-card-staff',      title: 'Staff Performance' },
  { key: 'customers', workspace: null,         titleKey: 'analytics-card-customers',  title: 'New vs Returning Customers' },
  { key: 'payments',  workspace: null,         titleKey: 'analytics-card-payments',   title: 'Payment Methods' },
  { key: 'discounts', workspace: null,         titleKey: 'analytics-card-discounts',  title: 'Discounts & Promotions' },
  { key: 'refunds',   workspace: null,         titleKey: 'analytics-card-refunds',    title: 'Refunds & Voids' },
  // Retail-only
  { key: 'top-items', workspace: 'retail',     titleKey: 'analytics-card-top-products', title: 'Top Products' },
  { key: 'category',  workspace: 'retail',     titleKey: 'analytics-card-category',   title: 'Sales by Category' },
  { key: 'basket',    workspace: 'retail',     titleKey: 'analytics-card-basket',     title: 'Average Basket Size' },
  { key: 'inventory', workspace: 'retail',     titleKey: 'analytics-card-inventory',  title: 'Stock Turnover' },
  { key: 'low-stock', workspace: 'retail',     titleKey: 'analytics-card-low-stock',  title: 'Low Stock Alerts' },
  // Restaurant-only
  { key: 'top-items', workspace: 'restaurant', titleKey: 'analytics-card-top-menu',   title: 'Top Menu Items' },
  { key: 'tables',    workspace: 'restaurant', titleKey: 'analytics-card-tables',     title: 'Table Turnover' },
  { key: 'occupancy', workspace: 'restaurant', titleKey: 'analytics-card-occupancy',  title: 'Table Occupancy' },
  { key: 'waitstaff', workspace: 'restaurant', titleKey: 'analytics-card-waitstaff',  title: 'Top Waitstaff' },
  { key: 'voids',     workspace: 'restaurant', titleKey: 'analytics-card-voids',      title: 'Voided Items' },
];

// ── Component ─────────────────────────────────────────────────────────

export default function AnalyticsScreen() {
  const { l10n } = useLocalization();
  const { goToWorkspacePicker } = useWorkspaceNav();

  const [workspaceView, setWorkspaceView] = useState<WorkspaceView>('retail');
  const [granularity, setGranularity] = useState<Granularity>('daily');
  const [customFrom, setCustomFrom] = useState(isoToday());
  const [customTo, setCustomTo] = useState(isoToday());
  const [calculating, setCalculating] = useState(false);
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
  const paletteInputRef = useRef<HTMLInputElement | null>(null);
  const [cardOrder, setCardOrder] = useState<string[]>([]);
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);
  const calcTimer = useRef<ReturnType<typeof setTimeout>>();
  const expandedBodyRef = useRef<HTMLDivElement | null>(null);
  const mainRef = useRef<HTMLElement | null>(null);

  const startRecalculating = useRef<() => void>();
  startRecalculating.current = () => {
    setCalculating(true);
    clearTimeout(calcTimer.current);
    calcTimer.current = setTimeout(() => setCalculating(false), 600);
  };

  // Recalculate when filters change
  useEffect(() => {
    startRecalculating.current?.();
    return () => clearTimeout(calcTimer.current);
  }, [workspaceView, granularity, customFrom, customTo]);

  const zoomIn = () => setZoomLevel((z) => Math.min(ZOOM_MAX, +(z + ZOOM_STEP).toFixed(2)));
  const zoomOut = () => setZoomLevel((z) => Math.max(ZOOM_MIN, +(z - ZOOM_STEP).toFixed(2)));
  const zoomReset = () => setZoomLevel(1);

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
        startRecalculating.current?.();
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
  }, [expandedKey, granularity, workspaceView, calculating]);

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
  };

  const isDefaultOrder = JSON.stringify(cardOrder) === JSON.stringify(defaultOrder);

  const resetLayout = () => {
    try {
      localStorage.removeItem(orderStorageKey);
    } catch {
      /* storage unavailable */
    }
    setCardOrder(defaultOrder);
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
  // Intensity is a placeholder until real data is wired.
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
            {Array.from({ length: 24 }, (_, h) => (
              <div
                key={`${day}-${h}`}
                className="analytics-heat-cell"
                data-intensity={(h * 7 + di * 3 + 5) % 5}
                title={`${day} ${String(h).padStart(2, '0')}:00`}
              >
                <div className="analytics-heat-block" />
              </div>
            ))}
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
          <div
            key={d}
            className="analytics-heat-cell"
            data-intensity={(d * 37 + 7) % 5}
            title={`Day ${d}`}
          >
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
                <div
                  key={week}
                  className="analytics-heat-cell"
                  data-intensity={(mi * 4 + week * 7) % 5}
                  title={`${month} W${week + 1}`}
                >
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
        {labels.map((label, i) => {
          const intensity = (i * 37 + 7) % 5;
          return (
            <div key={label} className="analytics-heat-cell" data-intensity={intensity} title={label}>
              <div className="analytics-heat-block" />
              <span className="analytics-heat-label">{label}</span>
            </div>
          );
        })}
      </div>
    );
  };

  // Render a placeholder chart icon based on card key
  const cardPlaceholder = (key: string) => {
    const icons: Record<string, JSX.Element> = {
      revenue: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
        </svg>
      ),
      staff: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
          <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
      ),
      'top-items': (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <line x1="12" y1="20" x2="12" y2="10" />
          <line x1="18" y1="20" x2="18" y2="4" />
          <line x1="6" y1="20" x2="6" y2="16" />
        </svg>
      ),
      category: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M21.21 15.89A10 10 0 1 1 8 2.83" />
          <path d="M22 12A10 10 0 0 0 12 2v10z" />
        </svg>
      ),
      tables: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <line x1="3" y1="9" x2="21" y2="9" />
          <line x1="9" y1="21" x2="9" y2="9" />
        </svg>
      ),
      payments: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <rect x="1" y="4" width="22" height="16" rx="2" ry="2" />
          <line x1="1" y1="10" x2="23" y2="10" />
        </svg>
      ),
      heatmap: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <rect x="3" y="3" width="7" height="7" />
          <rect x="14" y="3" width="7" height="7" />
          <rect x="3" y="14" width="7" height="7" />
          <rect x="14" y="14" width="7" height="7" />
        </svg>
      ),
      aov: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z" />
          <line x1="7" y1="7" x2="7.01" y2="7" />
        </svg>
      ),
      customers: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
          <circle cx="8.5" cy="7" r="4" />
          <line x1="20" y1="8" x2="20" y2="14" />
          <line x1="23" y1="11" x2="17" y2="11" />
        </svg>
      ),
      discounts: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <line x1="19" y1="5" x2="5" y2="19" />
          <circle cx="6.5" cy="6.5" r="2.5" />
          <circle cx="17.5" cy="17.5" r="2.5" />
        </svg>
      ),
      refunds: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <polyline points="1 4 1 10 7 10" />
          <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
        </svg>
      ),
      basket: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M6 2L3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z" />
          <line x1="3" y1="6" x2="21" y2="6" />
          <path d="M16 10a4 4 0 0 1-8 0" />
        </svg>
      ),
      inventory: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
          <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
          <line x1="12" y1="22.08" x2="12" y2="12" />
        </svg>
      ),
      'low-stock': (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
          <line x1="12" y1="9" x2="12" y2="13" />
          <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
      ),
      occupancy: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      ),
      waitstaff: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
          <circle cx="8.5" cy="7" r="4" />
          <polyline points="17 11 19 13 23 9" />
        </svg>
      ),
      voids: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
          strokeLinecap="round" strokeLinejoin="round" width="32" height="32" aria-hidden="true">
          <circle cx="12" cy="12" r="10" />
          <line x1="15" y1="9" x2="9" y2="15" />
          <line x1="9" y1="9" x2="15" y2="15" />
        </svg>
      ),
    };
    return icons[key] ?? icons['revenue'];
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
              className={`analytics-action-btn${allCollapsed ? ' analytics-action-btn--active' : ''}`}
              onClick={() => {
                const next = !allCollapsed;
                setAllCollapsed(next);
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
              onClick={startRecalculating.current}
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
              onClick={zoomReset}
              aria-label={l10n.getString('analytics-action-zoom-reset-aria')}
              title={l10n.getString('analytics-action-zoom-reset-aria')}
            >
              {Math.round(zoomLevel * 100)}%
            </button>
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

      {/* ══════════════════════════════════════════════════════════
          AREA 3 — Main content: smart analytics card grid
          ══════════════════════════════════════════════════════════ */}
      <main
        className="analytics-main"
        ref={mainRef}
        onScroll={(e) => setShowScrollTop(e.currentTarget.scrollTop > 240)}
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
        </div>

        <div className="analytics-grid" style={{ zoom: zoomLevel }}>
          {orderedCards.map((card) => {
            const cid = cardId(card);
            const isExpanded = expandedKey === cid;
            const isDragging = dragId === cid;
            const isDropTarget = overId === cid;
            return (
            <div
              key={cid}
              draggable={!isExpanded}
              onDragStart={(e) => { setDragId(cid); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
              onDragOver={(e) => { e.preventDefault(); if (overId !== cid) setOverId(cid); }}
              onDragLeave={() => setOverId((o) => (o === cid ? null : o))}
              onDrop={(e) => { e.preventDefault(); reorderCard(dragId ?? '', cid); setDragId(null); setOverId(null); }}
              onDragEnd={() => { setDragId(null); setOverId(null); }}
              className={`analytics-card${card.size ? ` analytics-card--${card.size}` : ''}${isExpanded ? ' analytics-card--expanded' : ''}${allCollapsed ? ' analytics-card--collapsed' : ''}${isDragging ? ' analytics-card--dragging' : ''}${isDropTarget ? ' analytics-card--drop-target' : ''}`}
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
                </div>
              </div>
              <div className="analytics-card-body" ref={isExpanded ? expandedBodyRef : undefined}>
                <div
                  className="analytics-card-content"
                  style={isExpanded ? { transform: `scale(${expandScale})` } : undefined}
                >
                  {calculating ? (
                    <div className="analytics-card-skeleton">
                      <div className="skeleton-bar skeleton-bar--sm" />
                      <div className="skeleton-bar skeleton-bar--lg" />
                      <div className="skeleton-bar skeleton-bar--md" />
                      <div className="skeleton-bar skeleton-bar--lg" />
                      <div className="skeleton-bar skeleton-bar--sm" />
                    </div>
                  ) : card.key === 'heatmap' ? (
                    renderHeatmap()
                  ) : (
                    <div className="analytics-card-placeholder">
                      {cardPlaceholder(card.key)}
                      <span className="analytics-card-hint">No data yet</span>
                    </div>
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
