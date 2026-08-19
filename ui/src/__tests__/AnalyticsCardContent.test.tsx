import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { AnalyticsCardContent, ExportCsvButton } from '@/features/analytics/AnalyticsCardContent';

// ── Mocks ──────────────────────────────────────────────────────────────

vi.mock('echarts-for-react/lib/core', () => ({
  default: ({ option }: { option?: Record<string, unknown> }) => (
    <div data-testid="echarts" data-keys={option ? Object.keys(option).join(',') : ''} />
  ),
}));

vi.mock('echarts/core', () => ({ use: vi.fn() }));
vi.mock('echarts/charts', () => ({ BarChart: {}, LineChart: {}, PieChart: {} }));
vi.mock('echarts/components', () => ({ GridComponent: {}, LegendComponent: {}, TooltipComponent: {} }));
vi.mock('echarts/renderers', () => ({ CanvasRenderer: {} }));

const mockUseAnalyticsQuery = vi.fn();
vi.mock('@/features/analytics/useAnalyticsQuery', () => ({
  useAnalyticsQuery: (...args: unknown[]) => mockUseAnalyticsQuery(...args),
}));

vi.mock('@/features/analytics/analytics-cache', () => ({
  cardQueryKey: vi.fn(() => ['test-key']),
}));

vi.mock('@/contexts/CurrencyContext', () => ({
  useCurrency: () => ({ currency: 'USD' }),
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string) => {
        const map: Record<string, string> = {
          'analytics-export-csv': 'Export CSV',
          'analytics-card-error-load': 'Failed to load',
          'analytics-card-empty': 'No data',
          'analytics-empty-generic': 'No data available',
        };
        return map[id] || id;
      },
      bundles: [{ locales: ['en-US'] }],
    },
  }),
}));

vi.mock('@/utils/app-error', () => ({
  l10nErrorMessage: () => 'Failed to load',
}));

vi.mock('@/utils/export-csv', () => ({
  downloadCsv: vi.fn(),
}));

// ── Tests: ExportCsvButton ─────────────────────────────────────────────

describe('ExportCsvButton', () => {
  it('renders with aria label', () => {
    render(<ExportCsvButton onClick={vi.fn()} ariaLabel="Export revenue CSV" />);
    expect(screen.getByRole('button', { name: 'Export revenue CSV' })).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const onClick = vi.fn();
    render(<ExportCsvButton onClick={onClick} ariaLabel="Export" />);
    fireEvent.click(screen.getByRole('button', { name: 'Export' }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('renders CSV label text', () => {
    render(<ExportCsvButton onClick={vi.fn()} ariaLabel="Export" />);
    expect(screen.getByText('Export CSV')).toBeInTheDocument();
  });
});

// ── Tests: AnalyticsCardContent ────────────────────────────────────────

describe('AnalyticsCardContent', () => {
  const baseProps = {
    granularity: 'day' as const,
    workspaceView: 'all' as const,
    from: '2026-08-01',
    to: '2026-08-19',
    sessionToken: 'test-token',
    title: 'Revenue',
    expanded: false,
    compare: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows error state when query fails', () => {
    mockUseAnalyticsQuery.mockReturnValue({ data: undefined, isLoading: false, error: new Error('network') });
    render(<AnalyticsCardContent cardKey="revenue" {...baseProps} />);
    expect(screen.getByText('Failed to load')).toBeInTheDocument();
  });

  it('renders aov card with data', () => {
    mockUseAnalyticsQuery.mockReturnValue({
      data: { aov_minor: 1500, total_orders: 20, total_minor: 30000, buckets: [{ label: 'Aug 1', value: 1500 }] },
      isLoading: false,
      error: null,
    });
    render(<AnalyticsCardContent cardKey="aov" {...baseProps} />);
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  it('renders customers card with data', () => {
    mockUseAnalyticsQuery.mockReturnValue({
      data: { new_count: 25, returning_count: 125, total_spend_minor: 50000 },
      isLoading: false,
      error: null,
    });
    render(<AnalyticsCardContent cardKey="customers" {...baseProps} />);
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  it('renders basket card with data', () => {
    mockUseAnalyticsQuery.mockReturnValue({
      data: { avg_line_count: 3.2, sale_count: 50, buckets: [{ label: '1', value: 50 }] },
      isLoading: false,
      error: null,
    });
    render(<AnalyticsCardContent cardKey="basket" {...baseProps} />);
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  it('renders low-stock card with data', () => {
    mockUseAnalyticsQuery.mockReturnValue({
      data: [
        { product_name: 'Widget', current_qty: 2, sku: 'WDG-001', product_id: 'p1' },
        { product_name: 'Gadget', current_qty: 4, sku: 'GDG-001', product_id: 'p2' },
      ],
      isLoading: false,
      error: null,
    });
    const { container } = render(<AnalyticsCardContent cardKey="low-stock" {...baseProps} />);
    expect(container.querySelector('.analytics-card-visual')).toBeInTheDocument();
  });

  it('returns null for unknown card key', () => {
    mockUseAnalyticsQuery.mockReturnValue({ data: null, isLoading: false, error: null });
    const { container } = render(
      <AnalyticsCardContent cardKey="nonexistent" {...baseProps} />,
    );
    expect(container.innerHTML).toBe('');
  });

  it('passes sessionToken to query', () => {
    mockUseAnalyticsQuery.mockReturnValue({ data: null, isLoading: true, error: null });
    render(<AnalyticsCardContent cardKey="revenue" {...baseProps} sessionToken="tok-123" />);
    expect(mockUseAnalyticsQuery).toHaveBeenCalled();
  });

  it('handles compare mode prop', () => {
    mockUseAnalyticsQuery.mockReturnValue({
      data: { aov_minor: 1500, total_orders: 20, total_minor: 30000, buckets: [{ label: 'Aug 1', value: 1500 }] },
      isLoading: false,
      error: null,
    });
    render(<AnalyticsCardContent cardKey="aov" {...baseProps} compare={true} />);
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  it('handles expanded prop', () => {
    mockUseAnalyticsQuery.mockReturnValue({
      data: { aov_minor: 1500, total_orders: 20, total_minor: 30000, buckets: [{ label: 'Aug 1', value: 1500 }] },
      isLoading: false,
      error: null,
    });
    render(<AnalyticsCardContent cardKey="aov" {...baseProps} expanded={true} />);
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });
});
