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

function isoToday(): string {
  return new Date().toISOString().slice(0, 10);
}

// ── Card definitions ─────────────────────────────────────────────────

interface AnalyticsCard {
  key: string;
  /** `null` = appears in both workspaces */
  workspace: WorkspaceView | null;
  title: string;
  /** `true` = spans all grid columns; default = single cell */
  full?: boolean;
}

const ANALYTICS_CARDS: AnalyticsCard[] = [
  { key: 'heatmap',  workspace: null,         title: 'Peak Hours',             full: true },
  { key: 'revenue',  workspace: null,         title: 'Revenue Overview' },
  { key: 'staff',    workspace: null,         title: 'Staff Performance' },
  { key: 'top-items',workspace: 'retail',     title: 'Top Products' },
  { key: 'top-items',workspace: 'restaurant', title: 'Top Menu Items' },
  { key: 'category', workspace: 'retail',     title: 'Sales by Category' },
  { key: 'tables',   workspace: 'restaurant', title: 'Table Turnover' },
  { key: 'payments', workspace: null,         title: 'Payment Methods' },
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
          {visibleCards.map((card) => (
            <div
              key={`${card.key}-${card.workspace ?? 'shared'}`}
              className={`analytics-card${card.full ? ' analytics-card--full' : ''}`}
            >
              <div className="analytics-card-header">
                <h2 className="analytics-card-title">{card.title}</h2>
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
                ) : (
                  <div className="analytics-card-placeholder">
                    {cardPlaceholder(card.key)}
                    <span className="analytics-card-hint">No data yet</span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </main>

    </div>
  );
}
