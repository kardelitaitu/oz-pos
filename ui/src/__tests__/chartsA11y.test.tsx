//! A11Y-09: chart data accessibility tests.
//!
//! The audit flagged that the canvas charts exposed only a generic label
//! (`Line chart`, `Pie chart`, `Hourly heatmap`) with the underlying data
//! drawn into pixels — inaccessible to screen readers. Phase 3 localised the
//! canvas `aria-label`; this suite pins the A11Y-09 remediation: every chart
//! renders a visually-hidden localized text summary AND a visually-hidden
//! data list of the underlying values, and the behaviour holds for empty,
//! single-point, and populated datasets (the audit's explicit test matrix).

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import CanvasLineChart from '@/components/charts/CanvasLineChart';
import CanvasPieChart from '@/components/charts/CanvasPieChart';
import CanvasHeatmap from '@/components/charts/CanvasHeatmap';

describe('A11Y-09 — CanvasLineChart data accessibility', () => {
  it('renders the localized label on the canvas', () => {
    render(
      <CanvasLineChart
        data={[{ label: 'A', value: 10 }]}
        label="14-day revenue chart"
      />,
    );
    expect(screen.getByRole('img', { name: '14-day revenue chart' })).toBeInTheDocument();
  });

  it('renders the visually-hidden summary with the sr-only utility', () => {
    render(
      <CanvasLineChart
        data={[{ label: 'A', value: 10 }]}
        label="14-day revenue chart"
        summary="$100 total across 14 days"
      />,
    );
    const summary = screen.getByText('$100 total across 14 days');
    expect(summary).toBeInTheDocument();
    // The whole point of the remediation: the summary is visually hidden
    // but present in the accessibility tree.
    expect(summary).toHaveClass('sr-only');
  });

  it('populated dataset: data list exposes every point', () => {
    render(
      <CanvasLineChart
        data={[
          { label: 'Mon', value: 100 },
          { label: 'Tue', value: 200 },
          { label: 'Wed', value: 300 },
        ]}
        label="14-day revenue chart"
      />,
    );
    const list = screen.getByTestId('chart-data-list');
    expect(list).toHaveClass('sr-only');
    expect(list.querySelectorAll('li')).toHaveLength(3);
    expect(screen.getByText('Mon: 100')).toBeInTheDocument();
    expect(screen.getByText('Wed: 300')).toBeInTheDocument();
  });

  it('single-point dataset: data list exposes the single point', () => {
    render(
      <CanvasLineChart
        data={[{ label: 'Today', value: 42 }]}
        label="14-day revenue chart"
      />,
    );
    const list = screen.getByTestId('chart-data-list');
    expect(list.querySelectorAll('li')).toHaveLength(1);
    expect(screen.getByText('Today: 42')).toBeInTheDocument();
  });

  it('empty dataset: no data list is rendered (chart degrades gracefully)', () => {
    render(<CanvasLineChart data={[]} label="14-day revenue chart" />);
    expect(screen.queryByTestId('chart-data-list')).not.toBeInTheDocument();
    // The accessible label is still present for the empty chart.
    expect(screen.getByRole('img', { name: '14-day revenue chart' })).toBeInTheDocument();
  });
});

describe('A11Y-09 — CanvasPieChart data accessibility', () => {
  it('populated dataset: data list exposes every slice', () => {
    render(
      <CanvasPieChart
        data={[
          { name: 'Food', value: 50 },
          { name: 'Drinks', value: 30 },
          { name: 'Other', value: 20 },
        ]}
        label="Category breakdown"
        summary="3 categories"
      />,
    );
    expect(screen.getByText('3 categories')).toBeInTheDocument();
    const list = screen.getByTestId('chart-data-list');
    expect(list.querySelectorAll('li')).toHaveLength(3);
    expect(screen.getByText('Food: 50')).toBeInTheDocument();
    expect(screen.getByText('Drinks: 30')).toBeInTheDocument();
  });

  it('single-point dataset: data list exposes the single slice', () => {
    render(
      <CanvasPieChart
        data={[{ name: 'Food', value: 100 }]}
        label="Category breakdown"
      />,
    );
    const list = screen.getByTestId('chart-data-list');
    expect(list.querySelectorAll('li')).toHaveLength(1);
    expect(screen.getByText('Food: 100')).toBeInTheDocument();
  });

  it('empty dataset: no data list, accessible label still present', () => {
    render(<CanvasPieChart data={[]} label="Category breakdown" />);
    expect(screen.queryByTestId('chart-data-list')).not.toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Category breakdown' })).toBeInTheDocument();
  });
});

describe('A11Y-09 — CanvasHeatmap data accessibility', () => {
  it('populated dataset: data list exposes only non-zero cells with day/hour', () => {
    render(
      <CanvasHeatmap
        data={[
          { dayOfWeek: 0, hour: 9, value: 100 },
          { dayOfWeek: 3, hour: 12, value: 250 },
          { dayOfWeek: 5, hour: 18, value: 0 }, // zero cells are skipped
        ]}
        label="Hourly sales heatmap"
        summary="2 active time slots"
      />,
    );
    expect(screen.getByText('2 active time slots')).toBeInTheDocument();
    const list = screen.getByTestId('chart-data-list');
    expect(list).toHaveClass('sr-only');
    expect(list.querySelectorAll('li')).toHaveLength(2);
    expect(screen.getByText(/Sun 9:00 — 100/)).toBeInTheDocument();
    expect(screen.getByText(/Wed 12:00 — 250/)).toBeInTheDocument();
  });

  it('empty dataset: no data list, accessible label still present', () => {
    render(<CanvasHeatmap data={[]} label="Hourly sales heatmap" />);
    expect(screen.queryByTestId('chart-data-list')).not.toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Hourly sales heatmap' })).toBeInTheDocument();
  });
});
