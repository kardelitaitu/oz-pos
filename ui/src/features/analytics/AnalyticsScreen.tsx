//! Analytics Screen — layout shell with three flex areas.
//!
//! Top:    back button + title
//! Menu:   workspace selector (row 1) + time granularity buttons (row 2)
//!         + inline custom date range
//! Main:   smart card grid — cards adapt to retail vs restaurant

import { useEffect, useRef, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import './AnalyticsScreen.css';

type WorkspaceView = 'retail' | 'restaurant';
type Granularity = 'daily' | 'weekly' | 'monthly' | 'yearly' | 'custom';

const GRANULARITIES: Granularity[] = ['daily', 'weekly', 'monthly', 'yearly', 'custom'];

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

function isoToday(): string {
  return new Date().toISOString().slice(0, 10);
}

/** Number of days in the current month (28–31). */
export function daysInCurrentMonth(): number {
  const now = new Date();
  return new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
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
  { key: 'heatmap',   workspace: null,         titleKey: 'analytics-card-peak-hours', title: 'Peak Hours',                size: 'wide' },
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
  const [zoomLevel, setZoomLevel] = useState(1);
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const calcTimer = useRef<ReturnType<typeof setTimeout>>();

  const MIN_ZOOM = 0.6;
  const MAX_ZOOM = 1.6;
  const ZOOM_STEP = 0.2;

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

  const zoomIn = () => setZoomLevel((z) => Math.min(MAX_ZOOM, +(z + ZOOM_STEP).toFixed(2)));
  const zoomOut = () => setZoomLevel((z) => Math.max(MIN_ZOOM, +(z - ZOOM_STEP).toFixed(2)));

  // Filter cards visible for the current workspace
  const visibleCards = ANALYTICS_CARDS.filter(
    (c) => c.workspace === null || c.workspace === workspaceView,
  );

  const cardId = (c: AnalyticsCard) => `${c.key}-${c.workspace ?? 'shared'}`;

  // When a card is expanded, only it is shown; otherwise all visible cards
  const displayedCards = expandedKey && visibleCards.some((c) => cardId(c) === expandedKey)
    ? visibleCards.filter((c) => cardId(c) === expandedKey)
    : visibleCards;

  // Smart heatmap — bucket cells change with the selected granularity.
  // Monthly renders one cell per day of the current month (28–31);
  // yearly renders a 12-month × 4-week grid; other granularities are flat.
  // Intensity is a placeholder until real data is wired.
  const renderHeatmap = () => {
    const aria = l10n.getString('analytics-card-peak-hours');
    if (granularity === 'monthly') {
      const days = daysInCurrentMonth();
      return (
        <div className="analytics-heatmap analytics-heatmap--monthly" role="img" aria-label={aria}>
          {Array.from({ length: days }, (_, i) => (
            <div
              key={i}
              className="analytics-heat-cell"
              data-intensity={(i * 37 + 7) % 5}
              title={`Day ${i + 1}`}
            >
              <div className="analytics-heat-block" />
              <span className="analytics-heat-label">{i + 1}</span>
            </div>
          ))}
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
              >
                <Localized id={`analytics-granularity-${g}`}>
                  <span>{g}</span>
                </Localized>
              </button>
            ))}
          </div>

          {granularity === 'custom' && (
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
          )}

          {/* Action buttons — refresh, zoom out, zoom in */}
          <div className="analytics-actions">
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
              className="analytics-action-btn"
              onClick={zoomIn}
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
          </div>
        </div>
      </nav>

      {/* ══════════════════════════════════════════════════════════
          AREA 3 — Main content: smart analytics card grid
          ══════════════════════════════════════════════════════════ */}
      <main className="analytics-main">
        <div className="analytics-grid" style={{ zoom: zoomLevel }}>
          {displayedCards.map((card) => {
            const cid = cardId(card);
            const isExpanded = expandedKey === cid;
            return (
            <div
              key={cid}
              className={`analytics-card${card.size ? ` analytics-card--${card.size}` : ''}${isExpanded ? ' analytics-card--expanded' : ''}`}
            >
              <div className="analytics-card-header">
                <Localized id={card.titleKey}>
                  <h2 className="analytics-card-title">{card.title}</h2>
                </Localized>
                <div className="analytics-card-actions">
                  <button
                    type="button"
                    className="analytics-card-action"
                    onClick={() => setExpandedKey((current) => nextExpandedKey(current, cid))}
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
              <div className="analytics-card-body">
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
            );
          })}
        </div>
      </main>

    </div>
  );
}
