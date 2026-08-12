//! Analytics Screen — layout shell with three flex areas.
//!
//! Top:    back button + title
//! Menu:   workspace selector (row 1) + time granularity buttons (row 2)
//!         + optional date range popup when "Custom" is selected
//! Main:   placeholder for charts, KPIs, and data tables

import { useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import './AnalyticsScreen.css';

type WorkspaceView = 'retail' | 'restaurant';
type Granularity = 'daily' | 'weekly' | 'monthly' | 'yearly' | 'custom';

const GRANULARITIES: Granularity[] = ['daily', 'weekly', 'monthly', 'yearly', 'custom'];

function isoToday(): string {
  return new Date().toISOString().slice(0, 10);
}

export default function AnalyticsScreen() {
  const { l10n } = useLocalization();
  const { goToWorkspacePicker } = useWorkspaceNav();

  const [workspaceView, setWorkspaceView] = useState<WorkspaceView>('retail');
  const [granularity, setGranularity] = useState<Granularity>('daily');
  const [customFrom, setCustomFrom] = useState(isoToday());
  const [customTo, setCustomTo] = useState(isoToday());

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

        {/* Row 2 — granularity pill buttons */}
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
        </div>

        {/* Row 3 — custom date range popup (visible only when Custom is active) */}
        {granularity === 'custom' && (
          <div className="analytics-menu-row">
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
          </div>
        )}
      </nav>

      {/* ══════════════════════════════════════════════════════════
          AREA 3 — Main content: charts, KPIs, and data tables
          ══════════════════════════════════════════════════════════ */}
      <main className="analytics-main">
        <div className="analytics-placeholder">
          <div className="analytics-placeholder-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
              strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"
              width="48" height="48" aria-hidden="true"
            >
              <line x1="18" y1="20" x2="18" y2="10" />
              <line x1="12" y1="20" x2="12" y2="4" />
              <line x1="6" y1="20" x2="6" y2="14" />
            </svg>
          </div>
          <p className="analytics-placeholder-text">
            Charts and data will appear here.
            Select a workspace and time range to begin.
          </p>
        </div>
      </main>

    </div>
  );
}
