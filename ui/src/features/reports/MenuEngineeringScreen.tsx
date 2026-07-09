import { useEffect, useState, useMemo, useCallback } from 'react';
import { Localized } from '@fluent/react';
import {
  ScatterChart,
  Scatter,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
  ReferenceArea,
  CartesianGrid,
  Legend,
} from 'recharts';
import {
  getMenuEngineering,
  type MenuEngineeringRow,
  type MenuEngineeringResult,
  type MenuQuadrant,
} from '@/api/reports';
import { Card } from '@/components/Card';
import { Spinner } from '@/components/Spinner';
import './MenuEngineeringScreen.css';

const QUADRANT_META: Record<
  MenuQuadrant,
  { label: string; color: string; bgColor: string; icon: string }
> = {
  Star: {
    label: 'Star',
    color: '#10b981',
    bgColor: 'rgba(16, 185, 129, 0.12)',
    icon: '★',
  },
  Plowhorse: {
    label: 'Plowhorse',
    color: '#f59e0b',
    bgColor: 'rgba(245, 158, 11, 0.12)',
    icon: '▲',
  },
  Puzzle: {
    label: 'Puzzle',
    color: '#3b82f6',
    bgColor: 'rgba(59, 130, 246, 0.12)',
    icon: '◆',
  },
  Dog: {
    label: 'Dog',
    color: '#ef4444',
    bgColor: 'rgba(239, 68, 68, 0.12)',
    icon: '▼',
  },
};

function classifyQuadrant(
  volume: number,
  margin: number,
  medianVolume: number,
  medianMargin: number,
): MenuQuadrant {
  const volumeHigh = volume >= medianVolume;
  const marginHigh = margin >= medianMargin;
  if (volumeHigh && marginHigh) return 'Star';
  if (volumeHigh && !marginHigh) return 'Plowhorse';
  if (!volumeHigh && marginHigh) return 'Puzzle';
  return 'Dog';
}

function fmtCurrency(minor: number): string {
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
  }).format(minor / 100);
}

function fmtCompact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function monthAgo(): string {
  const d = new Date();
  d.setDate(d.getDate() - 30);
  return d.toISOString().slice(0, 10);
}

/** Custom scatter dot shape — colored circle with outline. */
function ScatterDot(props: Record<string, unknown>) {
  const { cx, cy, fill } = props as { cx: number; cy: number; fill: string };
  if (cx == null || cy == null) return null;
  return (
    <circle
      cx={cx}
      cy={cy}
      r={6}
      fill={fill}
      stroke="rgba(255,255,255,0.8)"
      strokeWidth={1.5}
      className="menu-eng-scatter-dot"
    />
  );
}

/** Custom tooltip for the scatter chart. */
function ScatterTooltip({
  active,
  payload,
}: {
  active?: boolean;
  payload?: Array<{ payload: MenuEngineeringRow }>;
}) {
  if (!active || !payload || payload.length === 0) return null;
  const row = payload[0]!.payload;

  return (
    <div className="menu-eng-tooltip">
      <strong className="menu-eng-tooltip-name">{row.name}</strong>
      <div className="menu-eng-tooltip-grid">
        <span>Volume:</span>
        <span>{row.total_volume}</span>
        <span>Revenue:</span>
        <span>{fmtCurrency(row.total_revenue_minor)}</span>
        <span>Margin:</span>
        <span>{fmtCurrency(row.total_margin_minor)}</span>
        <span>Price:</span>
        <span>{fmtCurrency(row.unit_price_minor)}</span>
        <span>Cost:</span>
        <span>{fmtCurrency(row.unit_cost_minor)}</span>
      </div>
    </div>
  );
}

export default function MenuEngineeringScreen() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [startDate, setStartDate] = useState(monthAgo());
  const [endDate, setEndDate] = useState(today());
  const [result, setResult] = useState<MenuEngineeringResult | null>(null);
  const [hoveredRow, setHoveredRow] = useState<string | null>(null);

  const fetchData = useCallback(() => {
    setLoading(true);
    setError(null);

    getMenuEngineering(startDate, endDate)
      .then((res) => {
        setResult(res);
      })
      .catch((e) => {
        setError(e.message ?? String(e));
      })
      .finally(() => {
        setLoading(false);
      });
  }, [startDate, endDate]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // Annotate rows with quadrant classification and metadata.
  const rowsWithMeta = useMemo(() => {
    if (!result) return [];
    return result.rows.map((row) => {
      const quadrant = classifyQuadrant(
        row.total_volume,
        row.total_margin_minor,
        result.median_volume,
        result.median_margin,
      );
      return { ...row, quadrant, quadrantMeta: QUADRANT_META[quadrant] };
    });
  }, [result]);

  // Group data into quadrants for scatter chart.
  const scatterData = useMemo(() => {
    const groups: Record<MenuQuadrant, MenuEngineeringRow[]> = {
      Star: [],
      Plowhorse: [],
      Puzzle: [],
      Dog: [],
    };
    for (const row of result?.rows ?? []) {
      const q = classifyQuadrant(
        row.total_volume,
        row.total_margin_minor,
        result?.median_volume ?? 0,
        result?.median_margin ?? 0,
      );
      groups[q].push(row);
    }
    return groups;
  }, [result]);

  // Count of each quadrant.
  const quadrantCounts = useMemo(() => {
    const counts: Record<string, number> = {
      Star: 0,
      Plowhorse: 0,
      Puzzle: 0,
      Dog: 0,
    };
    for (const q of rowsWithMeta.map((r) => r.quadrant)) {
      counts[q] = (counts[q] ?? 0) + 1;
    }
    return counts;
  }, [rowsWithMeta]);

  // Compute max axis values for scatter plot (avoids Infinity issues).
  const maxVolume = useMemo(() => {
    if (!result?.rows?.length) return 100;
    return Math.max(...result.rows.map((r) => r.total_volume), result.median_volume) * 1.15;
  }, [result]);

  const maxMargin = useMemo(() => {
    if (!result?.rows?.length) return 10000;
    return Math.max(...result.rows.map((r) => r.total_margin_minor), result.median_margin) * 1.15;
  }, [result]);

  // Recommendation text for a quadrant.
  function recommendation(q: MenuQuadrant): string {
    switch (q) {
      case 'Star':
        return 'Promote Star — high volume & high margin. Feature prominently.';
      case 'Plowhorse':
        return 'Increase Price on Plowhorse — high volume but low margin. Raise price or reduce cost.';
      case 'Puzzle':
        return 'Reposition Puzzle — low volume but high margin. Improve visibility or bundle.';
      case 'Dog':
        return 'Remove Dog — low volume & low margin. Consider delisting.';
    }
  }

  if (loading) {
    return (
      <div className="menu-eng" role="region" aria-label="Menu Engineering Report">
        <Spinner aria-label="Loading menu engineering report" />
      </div>
    );
  }

  const totalRevenue = result?.rows.reduce((s, r) => s + r.total_revenue_minor, 0) ?? 0;
  const totalMargin = result?.rows.reduce((s, r) => s + r.total_margin_minor, 0) ?? 0;
  const totalProducts = result?.rows.length ?? 0;

  return (
    <div className="menu-eng" role="region" aria-label="Menu Engineering Report">
      {/* ── Header ────────────────────────────────────── */}
      <div className="menu-eng-header">
        <Localized id="menu-eng-title">
          <h1 className="menu-eng-title">Menu Engineering</h1>
        </Localized>

        <div className="menu-eng-controls">
          <label htmlFor="me-start-date" className="menu-eng-label">
            <Localized id="sales-report-start-date">Start</Localized>
          </label>
          <input
            id="me-start-date"
            type="date"
            value={startDate}
            onChange={(e) => setStartDate(e.target.value)}
            className="menu-eng-input"
            aria-label="Start date"
          />

          <label htmlFor="me-end-date" className="menu-eng-label">
            <Localized id="sales-report-end-date">End</Localized>
          </label>
          <input
            id="me-end-date"
            type="date"
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            className="menu-eng-input"
            aria-label="End date"
          />
        </div>
      </div>

      {error && (
        <p className="menu-eng-error">
          <Localized id="error-occurred">
            <span>An error occurred</span>
          </Localized>
          : {error}
        </p>
      )}

      {/* ── Summary KPI cards ─────────────────────────── */}
      <div className="menu-eng-kpis">
        <Card shadow="sm" className="menu-eng-kpi">
          <span className="menu-eng-kpi-label">
            <Localized id="menu-eng-products">Products</Localized>
          </span>
          <span className="menu-eng-kpi-value">{totalProducts}</span>
        </Card>
        <Card shadow="sm" className="menu-eng-kpi">
          <span className="menu-eng-kpi-label">
            <Localized id="menu-eng-total-revenue">Total Revenue</Localized>
          </span>
          <span className="menu-eng-kpi-value">{fmtCurrency(totalRevenue)}</span>
        </Card>
        <Card shadow="sm" className="menu-eng-kpi">
          <span className="menu-eng-kpi-label">
            <Localized id="menu-eng-total-margin">Total Margin</Localized>
          </span>
          <span className="menu-eng-kpi-value">{fmtCurrency(totalMargin)}</span>
        </Card>
        <Card shadow="sm" className="menu-eng-kpi">
          <span className="menu-eng-kpi-label">
            <Localized id="menu-eng-margin-rate">Margin Rate</Localized>
          </span>
          <span className="menu-eng-kpi-value">
            {totalRevenue > 0
              ? `${((totalMargin / totalRevenue) * 100).toFixed(1)}%`
              : '—'}
          </span>
        </Card>
      </div>

      {/* ── Quadrant Summary Cards ──────────────────── */}
      <div className="menu-eng-quadrant-cards">
        {(['Star', 'Plowhorse', 'Puzzle', 'Dog'] as MenuQuadrant[]).map((q) => {
          const meta = QUADRANT_META[q];
          const count = quadrantCounts[q] ?? 0;
          return (
            <div
              key={q}
              className="menu-eng-quadrant-card"
              style={{
                borderLeftColor: meta.color,
                background: meta.bgColor,
              }}
            >
              <span className="menu-eng-quadrant-icon" style={{ color: meta.color }}>
                {meta.icon}
              </span>
              <div className="menu-eng-quadrant-info">
                <span className="menu-eng-quadrant-label">
                  <Localized id={`menu-eng-${q.toLowerCase()}`}>{meta.label}</Localized>
                </span>
                <span className="menu-eng-quadrant-count">{count}</span>
              </div>
              <span className="menu-eng-quadrant-recs">{recommendation(q)}</span>
            </div>
          );
        })}
      </div>

      {/* ── Scatter Plot ──────────────────────────────── */}
      <Card shadow="sm" className="menu-eng-chart-card">
        <Localized id="menu-eng-scatter-title">
          <h2 className="menu-eng-section-title">Volume vs. Margin Matrix</h2>
        </Localized>
        {result && (
          <ResponsiveContainer width="100%" height={400}>
            <ScatterChart
              margin={{ top: 20, right: 20, bottom: 20, left: 60 }}
            >
              <CartesianGrid strokeDasharray="3 3" opacity={0.3} />

              {/* Quadrant background areas */}
              <ReferenceArea
                x1={result.median_volume}
                x2={maxVolume}
                y1={result.median_margin}
                y2={maxMargin}
                fill={QUADRANT_META.Star.color}
                fillOpacity={0.04}
              />
              <ReferenceArea
                x1={0}
                x2={result.median_volume}
                y1={result.median_margin}
                y2={maxMargin}
                fill={QUADRANT_META.Puzzle.color}
                fillOpacity={0.04}
              />
              <ReferenceArea
                x1={result.median_volume}
                x2={maxVolume}
                y1={0}
                y2={result.median_margin}
                fill={QUADRANT_META.Plowhorse.color}
                fillOpacity={0.04}
              />
              <ReferenceArea
                x1={0}
                x2={result.median_volume}
                y1={0}
                y2={result.median_margin}
                fill={QUADRANT_META.Dog.color}
                fillOpacity={0.04}
              />

              {/* Median reference lines */}
              <ReferenceLine
                x={result.median_volume}
                stroke="#94a3b8"
                strokeDasharray="6 3"
                strokeWidth={1.5}
                label={
                  <Localized id="menu-eng-median-volume">
                    <text fill="#94a3b8" fontSize={11}>
                      Median Volume
                    </text>
                  </Localized>
                }
              />
              <ReferenceLine
                y={result.median_margin}
                stroke="#94a3b8"
                strokeDasharray="6 3"
                strokeWidth={1.5}
                label={
                  <Localized id="menu-eng-median-margin">
                    <text fill="#94a3b8" fontSize={11}>
                      Median Margin
                    </text>
                  </Localized>
                }
              />

              <XAxis
                dataKey="total_volume"
                type="number"
                tick={{ fontSize: 11 }}
                label={{
                  value: 'Volume (units sold)',
                  position: 'bottom',
                  offset: -5,
                  style: { fontSize: 12, fill: '#64748b' },
                }}
              />
              <YAxis
                type="number"
                tick={{ fontSize: 11 }}
                tickFormatter={(v: number) => fmtCompact(v)}
                label={{
                  value: 'Total Margin',
                  angle: -90,
                  position: 'insideLeft',
                  offset: -45,
                  style: { fontSize: 12, fill: '#64748b' },
                }}
              />
              <Tooltip content={<ScatterTooltip />} cursor={{ strokeDasharray: '3 3' }} />
              <Legend
                formatter={(value: string) => (
                  <span style={{ color: QUADRANT_META[value as MenuQuadrant]?.color }}>
                    {value}
                  </span>
                )}
              />

              {(Object.keys(scatterData) as MenuQuadrant[]).map((q) => {
                const data =
                  scatterData[q]?.map((row) => ({
                    ...row,
                    x: row.total_volume,
                    y: row.total_margin_minor,
                  })) ?? [];
                if (data.length === 0) return null;

                return (
                  <Scatter
                    key={q}
                    name={q}
                    data={data}
                    fill={QUADRANT_META[q].color}
                    shape={<ScatterDot />}
                  />
                );
              })}
            </ScatterChart>
          </ResponsiveContainer>
        )}

        <div className="menu-eng-chart-legend">
          <span style={{ color: QUADRANT_META.Star.color }}>
            ● Star (high vol, high margin)
          </span>
          <span style={{ color: QUADRANT_META.Plowhorse.color }}>
            ▲ Plowhorse (high vol, low margin)
          </span>
          <span style={{ color: QUADRANT_META.Puzzle.color }}>
            ◆ Puzzle (low vol, high margin)
          </span>
          <span style={{ color: QUADRANT_META.Dog.color }}>
            ▼ Dog (low vol, low margin)
          </span>
        </div>
      </Card>

      {/* ── Data table ────────────────────────────────── */}
      <Card shadow="sm" className="menu-eng-chart-card">
        <Localized id="menu-eng-table-title">
          <h2 className="menu-eng-section-title">Product Breakdown</h2>
        </Localized>

        {rowsWithMeta.length === 0 ? (
          <p className="menu-eng-no-data">
            <Localized id="no-results">
              <span>No results</span>
            </Localized>
          </p>
        ) : (
          <div className="menu-eng-table" role="table" aria-label="Menu engineering product breakdown">
            <div className="menu-eng-table-header" role="row">
              <span role="columnheader">#</span>
              <span role="columnheader">
                <Localized id="top-products-name">Name</Localized>
              </span>
              <span role="columnheader">SKU</span>
              <span role="columnheader">
                <Localized id="top-products-quantity">Qty</Localized>
              </span>
              <span role="columnheader">
                <Localized id="top-products-revenue">Revenue</Localized>
              </span>
              <span role="columnheader">Margin</span>
              <span role="columnheader">Margin/Unit</span>
              <span role="columnheader">
                <Localized id="menu-eng-quadrant">Quadrant</Localized>
              </span>
              <span role="columnheader">
                <Localized id="menu-eng-recommendation">Recommendation</Localized>
              </span>
            </div>

            {rowsWithMeta.map((row, i) => (
              <div
                key={row.product_id}
                className={`menu-eng-table-row ${hoveredRow === row.product_id ? 'is-hovered' : ''}`}
                role="row"
                onMouseEnter={() => setHoveredRow(row.product_id)}
                onMouseLeave={() => setHoveredRow(null)}
                tabIndex={0}
                aria-label={`${row.name}: ${row.quadrantMeta.label}`}
              >
                <span role="cell">{i + 1}</span>
                <span role="cell" className="menu-eng-table-name">
                  {row.name}
                </span>
                <span role="cell" className="menu-eng-table-sku">
                  {row.sku}
                </span>
                <span role="cell">{row.total_volume}</span>
                <span role="cell" className="menu-eng-table-mono">
                  {fmtCurrency(row.total_revenue_minor)}
                </span>
                <span role="cell" className="menu-eng-table-mono">
                  {fmtCurrency(row.total_margin_minor)}
                </span>
                <span role="cell" className="menu-eng-table-mono">
                  {fmtCurrency(row.margin_per_unit)}
                </span>
                <span role="cell">
                  <span
                    className="menu-eng-badge"
                    style={{
                      background: row.quadrantMeta.bgColor,
                      color: row.quadrantMeta.color,
                      border: `1px solid ${row.quadrantMeta.color}`,
                    }}
                  >
                    {row.quadrantMeta.icon} {row.quadrantMeta.label}
                  </span>
                </span>
                <span role="cell" className="menu-eng-table-recs">
                  {recommendation(row.quadrant)}
                </span>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
