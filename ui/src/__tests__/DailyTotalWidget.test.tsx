import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import DailyTotalWidget from '@/features/sales/widgets/DailyTotalWidget';
import salesFtl from '@/locales/sales.ftl?raw';
import type { DailySummaryRow } from '@/api/sales';

const mockExportDailySummary = vi.fn();

vi.mock('@/api/sales', () => ({
  exportDailySummary: (...args: unknown[]) => mockExportDailySummary(...args),
  exportSalesByHour: vi.fn(),
}));

beforeEach(() => {
  mockExportDailySummary.mockReset();
});

function createRow(overrides: Record<string, unknown> = {}) {
  return {
    date: '2026-07-16',
    total_minor: 0,
    currency: 'USD',
    sale_count: 0,
    line_count: 0,
    ...overrides,
  } as unknown as DailySummaryRow;
}

describe('DailyTotalWidget', () => {
  it('shows loading skeleton initially', () => {
    mockExportDailySummary.mockImplementation(() => new Promise(() => {}));
    const { container } = renderWithFluentSync(<DailyTotalWidget />, salesFtl);

    const skeletons = container.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThanOrEqual(3);
  });

  it('renders KPI values after loading', async () => {
    const rows = [
      createRow({ total_minor: 150000, currency: 'IDR', sale_count: 3, line_count: 12 }),
      createRow({ total_minor: 75000, currency: 'IDR', sale_count: 1, line_count: 5 }),
    ];
    mockExportDailySummary.mockResolvedValue(rows);
    renderWithFluentSync(<DailyTotalWidget />, salesFtl);

    await waitFor(() => {
      expect(screen.getByText((t) => t.includes('IDR'))).toBeTruthy();
    });

    expect(screen.getByText('2')).toBeTruthy();
    expect(screen.getByText('17')).toBeTruthy();
  });

  it('shows zero values when no rows returned', async () => {
    mockExportDailySummary.mockResolvedValue([]);
    renderWithFluentSync(<DailyTotalWidget />, salesFtl);

    await waitFor(() => {
      expect(screen.getByText((t) => /^\$/.test(t))).toBeTruthy();
    });

    const zeros = screen.getAllByText('0');
    expect(zeros.length).toBeGreaterThanOrEqual(2);
  });

  it('falls back to USD when currency is empty string', async () => {
    mockExportDailySummary.mockResolvedValue([createRow({ total_minor: 5000, currency: '' })]);
    const { container } = renderWithFluentSync(<DailyTotalWidget />, salesFtl);

    await waitFor(() => {
      expect(container.querySelector('.reporting-widget-kpi-value--primary')?.textContent)
        .toMatch(/50,00/);
    });
  });

  it('handles API error gracefully', async () => {
    mockExportDailySummary.mockRejectedValue(new Error('API error'));
    renderWithFluentSync(<DailyTotalWidget />, salesFtl);

    await waitFor(() => {
      expect(screen.getByText((t) => /^\$/.test(t))).toBeTruthy();
    });
  });

  it('sets aria-label on the widget', async () => {
    mockExportDailySummary.mockResolvedValue([]);
    renderWithFluentSync(<DailyTotalWidget />, salesFtl);

    const widget = await screen.findByLabelText('Daily sales summary');
    expect(widget).toBeTruthy();
  });
});
