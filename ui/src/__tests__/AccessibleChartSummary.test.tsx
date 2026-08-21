// ── AccessibleChartSummary unit tests ─────────────────────────────
//
// Direct unit coverage for the shared A11Y-09 primitive behind every
// canvas chart (CanvasLineChart / CanvasPieChart / CanvasHeatmap).
// The chart-level suites (chartsA11y.test.tsx) prove the integration;
// this file pins the component's own contract:
//   - nothing renders when there is no summary and no data list
//   - summary-only renders when children is empty
//   - list-only renders when summary is undefined (exactOptionalPropertyTypes)
//   - both render when both are present
//   - `hasItems` handles array children with null/undefined holes

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { AccessibleChartSummary } from '@/components/charts/AccessibleChartSummary';

describe('AccessibleChartSummary (A11Y-09)', () => {
  it('renders nothing when there is no summary and no data list', () => {
    const { container } = render(<AccessibleChartSummary summary={undefined} />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId('chart-data-list')).not.toBeInTheDocument();
  });

  it('renders nothing when children is an array of only null/undefined holes', () => {
    const { container } = render(
      <AccessibleChartSummary summary={undefined}>
        {[null, undefined, null]}
      </AccessibleChartSummary>,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders the summary only when children is empty', () => {
    render(
      <AccessibleChartSummary summary="Total 300 across 3 days" />,
    );
    const summary = screen.getByText('Total 300 across 3 days');
    expect(summary).toHaveClass('sr-only');
    expect(screen.queryByTestId('chart-data-list')).not.toBeInTheDocument();
  });

  it('renders the data list only when summary is undefined', () => {
    render(
      <AccessibleChartSummary summary={undefined}>
        <li>Mon: 100</li>
      </AccessibleChartSummary>,
    );
    const list = screen.getByTestId('chart-data-list');
    expect(list).toHaveClass('sr-only');
    expect(list.querySelectorAll('li')).toHaveLength(1);
    expect(screen.getByText('Mon: 100')).toBeInTheDocument();
  });

  it('renders both the summary and the data list when both are present', () => {
    render(
      <AccessibleChartSummary summary="Total 300 across 3 days">
        <li>Mon: 100</li>
        <li>Tue: 200</li>
      </AccessibleChartSummary>,
    );
    expect(screen.getByText('Total 300 across 3 days')).toHaveClass('sr-only');
    const list = screen.getByTestId('chart-data-list');
    expect(list).toHaveClass('sr-only');
    expect(list.querySelectorAll('li')).toHaveLength(2);
  });

  it('treats an array with null holes and valid items as having items', () => {
    render(
      <AccessibleChartSummary summary={undefined}>
        {[null, <li key="1">Sun 9:00 — 100</li>, undefined]}
      </AccessibleChartSummary>,
    );
    const list = screen.getByTestId('chart-data-list');
    expect(list.querySelectorAll('li')).toHaveLength(1);
    expect(screen.getByText('Sun 9:00 — 100')).toBeInTheDocument();
  });

  it('renders a falsy-but-valid single child (e.g. the number 0) as an item', () => {
    render(<AccessibleChartSummary summary={undefined}>{0}</AccessibleChartSummary>);
    const list = screen.getByTestId('chart-data-list');
    expect(list).toHaveClass('sr-only');
    expect(list).toHaveTextContent('0');
  });
});
