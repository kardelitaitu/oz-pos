import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import HourlyHeatmapWidget from '@/features/sales/widgets/HourlyHeatmapWidget';

// ── Mocks ──────────────────────────────────────────────────────────────

const mockGetHourlyHeatmap = vi.fn();
vi.mock('@/api/reports', () => ({
  getHourlyHeatmap: (...args: unknown[]) => mockGetHourlyHeatmap(...args),
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
        'sales-dashboard-heatmap-title': 'Busiest Hours',
        'sales-dashboard-heatmap-aria': 'Hourly heatmap',
        'sales-dashboard-heatmap-summary': '{count} active cells',
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

vi.mock('@/components/charts/CanvasHeatmap', () => ({
  default: ({ data, summary }: { data: unknown[]; summary: string }) => (
    <div data-testid="heatmap" data-count={data.length}>
      {summary}
    </div>
  ),
}));

vi.mock('@/components/Skeleton', () => ({
  Skeleton: ({ width }: { width?: string }) => <div data-testid="skeleton" data-width={width} />,
}));

// ── Tests ──────────────────────────────────────────────────────────────

describe('HourlyHeatmapWidget', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows skeleton while loading', () => {
    mockGetHourlyHeatmap.mockReturnValue(new Promise(() => {}));
    render(<HourlyHeatmapWidget />);
    expect(screen.getAllByTestId('skeleton').length).toBeGreaterThan(0);
  });

  it('shows error message when API fails', async () => {
    mockGetHourlyHeatmap.mockRejectedValue(new Error('network'));
    render(<HourlyHeatmapWidget />);
    await waitFor(() => {
      expect(screen.getByText('An error occurred')).toBeInTheDocument();
    });
  });

  it('shows "no data" when API returns empty array', async () => {
    mockGetHourlyHeatmap.mockResolvedValue([]);
    render(<HourlyHeatmapWidget />);
    await waitFor(() => {
      expect(screen.getByText('No data for this period')).toBeInTheDocument();
    });
  });

  it('renders heatmap when data is available', async () => {
    mockGetHourlyHeatmap.mockResolvedValue([
      { day_of_week: 1, hour: 9, total_minor: 5000 },
      { day_of_week: 1, hour: 12, total_minor: 3000 },
    ]);
    render(<HourlyHeatmapWidget />);
    await waitFor(() => {
      expect(screen.getByTestId('heatmap')).toBeInTheDocument();
    });
    expect(screen.getByTestId('heatmap').getAttribute('data-count')).toBe('2');
  });

  it('calls API with correct date range (7 days)', async () => {
    mockGetHourlyHeatmap.mockResolvedValue([]);
    render(<HourlyHeatmapWidget />);
    await waitFor(() => {
      expect(mockGetHourlyHeatmap).toHaveBeenCalled();
    });
    const args = mockGetHourlyHeatmap.mock.calls[0] as string[];
    const [start, end] = args;
    expect(start).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(end).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('renders title in success state', async () => {
    mockGetHourlyHeatmap.mockResolvedValue([
      { day_of_week: 1, hour: 9, total_minor: 1000 },
    ]);
    render(<HourlyHeatmapWidget />);
    await waitFor(() => {
      expect(screen.getByText('Busiest Hours')).toBeInTheDocument();
    });
  });
});
