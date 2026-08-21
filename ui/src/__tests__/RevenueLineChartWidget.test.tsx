import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import RevenueLineChartWidget from '@/features/sales/widgets/RevenueLineChartWidget';

// ── Mocks ──────────────────────────────────────────────────────────────

const mockGetDailyRevenue = vi.fn();
vi.mock('@/api/reports', () => ({
  getDailyRevenue: (...args: unknown[]) => mockGetDailyRevenue(...args),
}));

vi.mock('@/contexts/WorkspaceContext', () => {
  const ctx = React.createContext({ sessionToken: 'test-token' });
  return { WorkspaceContext: ctx };
});

vi.mock('@/contexts/CurrencyContext', () => ({
  useCurrency: () => ({ currency: 'USD' }),
}));

vi.mock('@fluent/react', () => {
  const stableL10n = {
    getString: (id: string) => {
      const map: Record<string, string> = {
        'sales-dashboard-revenue-title': 'Revenue (14d)',
        'sales-dashboard-revenue-aria': 'Revenue line chart',
        'sales-dashboard-revenue-summary': '{total} over {days} days',
        'sales-dashboard-no-data': 'No data for this period',
        'app-error-generic': 'An error occurred',
      };
      return map[id] || id;
    },
  };
  return {
    Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
    useLocalization: () => ({ l10n: stableL10n }),
  };
});

vi.mock('@/utils/app-error', () => ({
  l10nErrorMessage: () => 'An error occurred',
}));

vi.mock('@/components/charts/CanvasLineChart', () => ({
  default: ({ data, summary }: { data: unknown[]; summary: string }) => (
    <div data-testid="line-chart" data-count={data.length}>
      {summary}
    </div>
  ),
}));

vi.mock('@/components/Skeleton', () => ({
  Skeleton: ({ width }: { width?: string }) => <div data-testid="skeleton" data-width={width} />,
}));

// ── Tests ──────────────────────────────────────────────────────────────

describe('RevenueLineChartWidget', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows skeleton while loading', () => {
    mockGetDailyRevenue.mockReturnValue(new Promise(() => {}));
    render(<RevenueLineChartWidget />);
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
  });

  it('shows error message when API fails', async () => {
    mockGetDailyRevenue.mockRejectedValue(new Error('network'));
    render(<RevenueLineChartWidget />);
    await waitFor(() => {
      expect(screen.getByText('An error occurred')).toBeInTheDocument();
    });
  });

  it('renders chart when data is available', async () => {
    mockGetDailyRevenue.mockResolvedValue([
      { date: '2026-08-01', total_minor: 5000 },
      { date: '2026-08-02', total_minor: 3000 },
    ]);
    render(<RevenueLineChartWidget />);
    await waitFor(() => {
      expect(screen.getByTestId('line-chart')).toBeInTheDocument();
    });
    expect(screen.getByTestId('line-chart').getAttribute('data-count')).toBe('2');
  });

  it('displays total revenue KPI', async () => {
    mockGetDailyRevenue.mockResolvedValue([
      { date: '2026-08-01', total_minor: 5000 },
      { date: '2026-08-02', total_minor: 3000 },
    ]);
    render(<RevenueLineChartWidget />);
    await waitFor(() => {
      expect(screen.getByText('Revenue (14d)')).toBeInTheDocument();
    });
    // $80.00 = (5000 + 3000) / 100
    expect(screen.getByText('$80.00')).toBeInTheDocument();
  });

  it('renders title in success state', async () => {
    mockGetDailyRevenue.mockResolvedValue([
      { date: '2026-08-01', total_minor: 1000 },
    ]);
    render(<RevenueLineChartWidget />);
    await waitFor(() => {
      expect(screen.getByText('Revenue (14d)')).toBeInTheDocument();
    });
  });

  it('calls API with correct date range (14 days)', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    render(<RevenueLineChartWidget />);
    await waitFor(() => {
      expect(mockGetDailyRevenue).toHaveBeenCalled();
    });
    const args = mockGetDailyRevenue.mock.calls[0] as string[];
    const [start, end] = args;
    expect(start).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(end).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('formats $0.00 when no data', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    render(<RevenueLineChartWidget />);
    await waitFor(() => {
      expect(screen.getByText('$0.00')).toBeInTheDocument();
    });
  });
});
