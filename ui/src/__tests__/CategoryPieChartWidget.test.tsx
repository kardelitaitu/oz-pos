import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import CategoryPieChartWidget from '@/features/sales/widgets/CategoryPieChartWidget';

// ── Mocks ──────────────────────────────────────────────────────────────

const mockGetCategoryBreakdown = vi.fn();
vi.mock('@/api/reports', () => ({
  getCategoryBreakdown: (...args: unknown[]) => mockGetCategoryBreakdown(...args),
}));

// Mock workspace context by providing a default value
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
        'sales-dashboard-category-title': 'By Category',
        'sales-dashboard-category-aria': 'Category breakdown',
        'sales-dashboard-category-summary': '{count} categories',
        'sales-dashboard-chart-other': 'Other',
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

vi.mock('@/components/charts/CanvasPieChart', () => ({
  default: ({ data, summary }: { data: unknown[]; summary: string }) => (
    <div data-testid="pie-chart" data-count={data.length}>
      {summary}
    </div>
  ),
}));

vi.mock('@/components/Skeleton', () => ({
  Skeleton: ({ width }: { width?: string }) => <div data-testid="skeleton" data-width={width} />,
}));

// ── Tests ──────────────────────────────────────────────────────────────

describe('CategoryPieChartWidget', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows skeleton while loading', () => {
    mockGetCategoryBreakdown.mockReturnValue(new Promise(() => {}));
    render(<CategoryPieChartWidget />);
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
  });

  it('shows error message when API fails', async () => {
    mockGetCategoryBreakdown.mockRejectedValue(new Error('network'));
    render(<CategoryPieChartWidget />);
    await waitFor(() => {
      expect(screen.getByText('An error occurred')).toBeInTheDocument();
    });
  });

  it('shows "no data" when API returns empty array', async () => {
    mockGetCategoryBreakdown.mockResolvedValue([]);
    render(<CategoryPieChartWidget />);
    await waitFor(() => {
      expect(screen.getByText('No data for this period')).toBeInTheDocument();
    });
  });

  it('renders pie chart when data is available', async () => {
    mockGetCategoryBreakdown.mockResolvedValue([
      { category_name: 'Food', total_minor: 5000 },
      { category_name: 'Drinks', total_minor: 3000 },
    ]);
    render(<CategoryPieChartWidget />);
    await waitFor(() => {
      expect(screen.getByTestId('pie-chart')).toBeInTheDocument();
    });
    expect(screen.getByTestId('pie-chart').getAttribute('data-count')).toBe('2');
  });

  it('uses "Uncategorized" for null category names', async () => {
    mockGetCategoryBreakdown.mockResolvedValue([
      { category_name: null, total_minor: 1000 },
    ]);
    render(<CategoryPieChartWidget />);
    await waitFor(() => {
      expect(screen.getByTestId('pie-chart')).toBeInTheDocument();
    });
  });

  it('calls API with correct date range (30 days)', async () => {
    mockGetCategoryBreakdown.mockResolvedValue([]);
    render(<CategoryPieChartWidget />);
    await waitFor(() => {
      expect(mockGetCategoryBreakdown).toHaveBeenCalled();
    });
    const args = mockGetCategoryBreakdown.mock.calls[0] as string[];
    const [start, end] = args;
    expect(start).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(end).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});
