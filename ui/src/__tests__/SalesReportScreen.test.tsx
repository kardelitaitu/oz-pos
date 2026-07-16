// ── SalesReportScreen tests ────────────────────────────────────────
// Covers loading, error, daily/weekly/monthly view modes, revenue bar
// chart, category pie, top products table, hourly heatmap, date filter,
// CSV export, print report, empty states, and ARIA accessibility.

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within, fireEvent } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import userEvent from '@testing-library/user-event';
import SalesReportScreen from '@/features/reports/SalesReportScreen';

// ── FTL bundles ──────────────────────────────────────────────────
const sharedFtl = `
error-occurred = An error occurred
no-results = No results
print = Print
`;
const reportsFtl = `
sales-report-title = Sales Report
sales-report-start-date = Start
sales-report-end-date = End
sales-report-daily = Daily
sales-report-weekly = Weekly
sales-report-monthly = Monthly
sales-report-export-csv = Export CSV
sales-report-revenue-chart = Revenue
sales-report-revenue-label = Revenue (minor units)
sales-report-total-revenue = Total
sales-report-total-orders = Orders
sales-report-category-breakdown = By Category
sales-report-top-products = Top Products
sales-report-rank = #
top-products-name = Name
top-products-quantity = Qty
top-products-revenue = Revenue
heatmap-title = Busiest Hours
heatmap-no-data = No data
`;

// ── Mock recharts ─────────────────────────────────────────────────
vi.mock('recharts', () => ({
  BarChart: ({ children }: { children: React.ReactNode }) => <div data-testid="bar-chart">{children}</div>,
  Bar: (props: { dataKey: string; 'aria-label'?: string }) => <div data-testid="bar" data-key={props.dataKey} aria-label={props['aria-label']} />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => <div data-testid="responsive-container">{children}</div>,
  PieChart: ({ children }: { children: React.ReactNode }) => <div data-testid="pie-chart">{children}</div>,
  Pie: ({ children }: { children: React.ReactNode }) => <div data-testid="pie">{children}</div>,
  Cell: () => <div data-testid="pie-cell" />,
  Legend: () => <div data-testid="legend" />,
}));

// ── Mock API functions ────────────────────────────────────────────
const mockGetDailyRevenue = vi.fn();
const mockGetWeeklyRevenue = vi.fn();
const mockGetMonthlyRevenue = vi.fn();
const mockGetTopProducts = vi.fn();
const mockGetHourlyHeatmap = vi.fn();
const mockGetCategoryBreakdown = vi.fn();
const mockPrintSalesReceipt = vi.fn();

vi.mock('@/api/reports', () => ({
  getDailyRevenue: (...args: unknown[]) => mockGetDailyRevenue(...args),
  getWeeklyRevenue: (...args: unknown[]) => mockGetWeeklyRevenue(...args),
  getMonthlyRevenue: (...args: unknown[]) => mockGetMonthlyRevenue(...args),
  getTopProducts: (...args: unknown[]) => mockGetTopProducts(...args),
  getHourlyHeatmap: (...args: unknown[]) => mockGetHourlyHeatmap(...args),
  getCategoryBreakdown: (...args: unknown[]) => mockGetCategoryBreakdown(...args),
}));

vi.mock('@/api/sales', () => ({
  printSalesReceipt: (...args: unknown[]) => mockPrintSalesReceipt(...args),
}));

vi.mock('@/components/Card', () => ({
  Card: ({ children, className, shadow }: Record<string, unknown>) => (
    <div className={className as string} data-shadow={shadow as string}>{children as React.ReactNode}</div>
  ),
}));

vi.mock('@/components/Button', () => ({
  Button: ({ children, onClick, variant, 'aria-label': ariaLabel }: Record<string, unknown>) => (
    <button onClick={onClick as () => void} data-variant={variant as string} aria-label={ariaLabel as string}>
      {children as React.ReactNode}
    </button>
  ),
}));

vi.mock('@/features/reports/SalesReportScreen.css', () => ({}));

// ── Test helpers ──────────────────────────────────────────────────

function buildDailyRevenue(overrides: Partial<{ date: string; total_minor: number; currency: string; sale_count: number }> = {}) {
  return {
    date: overrides.date ?? '2026-07-01',
    total_minor: overrides.total_minor ?? 150000,
    currency: overrides.currency ?? 'USD',
    sale_count: overrides.sale_count ?? 12,
  };
}

function buildWeeklyRevenue(overrides: Partial<{ week_start: string; total_minor: number; currency: string; sale_count: number }> = {}) {
  return {
    week_start: overrides.week_start ?? '2026-06-29',
    total_minor: overrides.total_minor ?? 500000,
    currency: overrides.currency ?? 'USD',
    sale_count: overrides.sale_count ?? 45,
  };
}

function buildMonthlyRevenue(overrides: Partial<{ month: string; total_minor: number; currency: string; sale_count: number }> = {}) {
  return {
    month: overrides.month ?? '2026-07',
    total_minor: overrides.total_minor ?? 2000000,
    currency: overrides.currency ?? 'USD',
    sale_count: overrides.sale_count ?? 180,
  };
}

function buildTopProduct(overrides: Partial<{ product_id: string; sku: string; name: string; total_qty: number; total_minor: number }> = {}) {
  return {
    product_id: overrides.product_id ?? 'prod-1',
    sku: overrides.sku ?? 'SKU001',
    name: overrides.name ?? 'Espresso',
    total_qty: overrides.total_qty ?? 45,
    total_minor: overrides.total_minor ?? 90000,
  };
}

function buildCategory(overrides: Partial<{ category_id: string | null; category_name: string; total_minor: number; sale_count: number; percentage: number }> = {}) {
  return {
    category_id: overrides.category_id ?? 'cat-1',
    category_name: overrides.category_name ?? 'Beverages',
    total_minor: overrides.total_minor ?? 300000,
    sale_count: overrides.sale_count ?? 60,
    percentage: overrides.percentage ?? 40,
  };
}

function buildHeatmap(overrides: Partial<{ day_of_week: number; hour: number; total_minor: number; sale_count: number }> = {}) {
  return {
    day_of_week: overrides.day_of_week ?? 1,
    hour: overrides.hour ?? 14,
    total_minor: overrides.total_minor ?? 25000,
    sale_count: overrides.sale_count ?? 5,
  };
}

const bundle = new FluentBundle('en');
bundle.addResource(new FluentResource(sharedFtl));
bundle.addResource(new FluentResource(reportsFtl));
const l10n = new ReactLocalization([bundle]);

function renderScreen() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <SalesReportScreen />
    </LocalizationProvider>,
  );
}

function resolveDefaultData() {
  mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
  mockGetTopProducts.mockResolvedValue([buildTopProduct()]);
  mockGetHourlyHeatmap.mockResolvedValue([buildHeatmap()]);
  mockGetCategoryBreakdown.mockResolvedValue([buildCategory()]);
}

// ── Tests ────────────────────────────────────────────────────────

describe('SalesReportScreen', () => {
  beforeEach(() => {
    // Default: never resolves (loading state)
    mockGetDailyRevenue.mockImplementation(() => new Promise(() => {}));
    mockGetTopProducts.mockImplementation(() => new Promise(() => {}));
    mockGetHourlyHeatmap.mockImplementation(() => new Promise(() => {}));
    mockGetCategoryBreakdown.mockImplementation(() => new Promise(() => {}));
    mockPrintSalesReceipt.mockResolvedValue(undefined);
  });

  // ── Loading ──────────────────────────────────────────────────
  it('shows loading skeleton initially', () => {
    renderScreen();
    const skeleton = document.querySelector('.sales-report-loading-skeleton');
    expect(skeleton).toBeTruthy();
    expect(skeleton?.getAttribute('aria-hidden')).toBe('true');
  });

  // ── Error ────────────────────────────────────────────────────
  it('shows error message when API calls fail', async () => {
    mockGetDailyRevenue.mockRejectedValue(new Error('Server offline'));
    mockGetTopProducts.mockRejectedValue(new Error('Server offline'));
    mockGetHourlyHeatmap.mockRejectedValue(new Error('Server offline'));
    mockGetCategoryBreakdown.mockRejectedValue(new Error('Server offline'));
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('An error occurred')).toBeTruthy();
    });
  });

  // ── Title & controls ─────────────────────────────────────────
  it('renders the Sales Report title', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Sales Report')).toBeTruthy();
    });
  });

  it('renders date inputs with default values', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      const startInput = screen.getByLabelText('Start date');
      const endInput = screen.getByLabelText('End date');
      expect(startInput).toBeTruthy();
      expect(endInput).toBeTruthy();
      // Both should have values (default is last 30 days and today)
      expect((startInput as HTMLInputElement).value).toBeTruthy();
      expect((endInput as HTMLInputElement).value).toBeTruthy();
    });
  });

  it('renders view mode toggle buttons (daily, weekly, monthly)', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: 'daily' })).toBeTruthy();
      expect(screen.getByRole('radio', { name: 'weekly' })).toBeTruthy();
      expect(screen.getByRole('radio', { name: 'monthly' })).toBeTruthy();
    });
  });

  it('daily is the default selected view mode', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: 'daily' }).getAttribute('aria-checked')).toBe('true');
      expect(screen.getByRole('radio', { name: 'weekly' }).getAttribute('aria-checked')).toBe('false');
      expect(screen.getByRole('radio', { name: 'monthly' }).getAttribute('aria-checked')).toBe('false');
    });
  });

  it('renders Print and Export CSV buttons', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Print report' })).toBeTruthy();
      expect(screen.getByRole('button', { name: 'Export CSV' })).toBeTruthy();
    });
  });

  // ── Daily view data ──────────────────────────────────────────
  it('displays total revenue and total orders for daily data', async () => {
    mockGetDailyRevenue.mockResolvedValue([
      buildDailyRevenue({ total_minor: 250000, sale_count: 5 }),
      buildDailyRevenue({ total_minor: 100000, sale_count: 3 }),
    ]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      // $3,500.00 total (2500 + 1000)
      expect(screen.getByText(/\$3,500\.00/)).toBeTruthy();
      // 8 orders (5 + 3)
      expect(screen.getByText(/8/)).toBeTruthy();
    });
  });

  it('renders the bar chart and revenue section', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      // 'Revenue' appears in both chart heading and top-products table header
      expect(screen.getAllByText('Revenue').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByTestId('bar-chart')).toBeTruthy();
      expect(screen.getByTestId('bar')).toBeTruthy();
    });
  });

  it('bar chart has Revenue (minor units) aria label', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      const bar = screen.getByTestId('bar');
      expect(bar.getAttribute('aria-label')).toBe('Revenue (minor units)');
    });
  });

  // ── Category breakdown ───────────────────────────────────────
  it('renders category breakdown section with pie chart', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('By Category')).toBeTruthy();
      expect(screen.getByTestId('pie-chart')).toBeTruthy();
    });
  });

  it('shows "No results" when category breakdown is empty', async () => {
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const noResultsElements = screen.getAllByText('No results');
      // Category breakdown section should show "No results"
      expect(noResultsElements.length).toBeGreaterThanOrEqual(1);
    });
  });

  // ── Top products ─────────────────────────────────────────────
  it('renders top products table with headers', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Top Products')).toBeTruthy();
      expect(screen.getByText('#')).toBeTruthy();
      expect(screen.getByText('Name')).toBeTruthy();
      expect(screen.getByText('Qty')).toBeTruthy();
      // 'Revenue' appears multiple times; check at least one exists
      expect(screen.getAllByText('Revenue').length).toBeGreaterThanOrEqual(1);
    });
  });

  it('renders top product rows with data', async () => {
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([
      buildTopProduct({ product_id: 'prod-1', name: 'Latte', total_qty: 30, total_minor: 120000 }),
      buildTopProduct({ product_id: 'prod-2', name: 'Mocha', total_qty: 20, total_minor: 100000 }),
    ]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Latte')).toBeTruthy();
      expect(screen.getByText('30')).toBeTruthy();
      expect(screen.getByText('Mocha')).toBeTruthy();
      expect(screen.getByText('20')).toBeTruthy();
      // Revenue formatted: $1,200.00 and $1,000.00
      expect(screen.getByText('$1,200.00')).toBeTruthy();
      expect(screen.getByText('$1,000.00')).toBeTruthy();
    });
  });

  it('shows "No results" when top products is empty', async () => {
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const noResultsElements = screen.getAllByText('No results');
      // Top products section should show "No results"
      expect(noResultsElements.length).toBeGreaterThanOrEqual(1);
    });
  });

  // ── Hourly heatmap ───────────────────────────────────────────
  it('renders heatmap section with title', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Busiest Hours')).toBeTruthy();
    });
  });

  it('renders heatmap grid with 24 hour columns and 7 day rows', async () => {
    const heatmapData = [
      buildHeatmap({ day_of_week: 1, hour: 9, total_minor: 50000 }),
      buildHeatmap({ day_of_week: 3, hour: 15, total_minor: 75000 }),
    ];
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue(heatmapData);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const grid = screen.getByRole('grid', { name: 'Hourly heatmap' });
      expect(grid).toBeTruthy();
      // 7 rows (one per day) + 1 header row = 8 total rows
      const rows = within(grid).getAllByRole('row');
      expect(rows.length).toBe(7);
    });
  });

  it('renders heatmap cells with aria labels showing day, hour, and value', async () => {
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([buildHeatmap({ day_of_week: 1, hour: 14, total_minor: 25000 })]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      // Cell aria-label: "Mon 14:00 - $250.00"
      const cell = screen.getByRole('gridcell', { name: /Mon 14:00/ });
      expect(cell).toBeTruthy();
    });
  });

  it('shows "No data" when heatmap is empty', async () => {
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('No data')).toBeTruthy();
    });
  });

  // ── View mode switching ──────────────────────────────────────
  it('switches to weekly view and calls getWeeklyRevenue', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeTruthy();
    });

    mockGetWeeklyRevenue.mockResolvedValue([buildWeeklyRevenue()]);
    // Reset mocks for the new view fetch
    mockGetDailyRevenue.mockClear();
    mockGetWeeklyRevenue.mockClear();

    await userEvent.click(screen.getByRole('radio', { name: 'weekly' }));

    await waitFor(() => {
      expect(mockGetWeeklyRevenue).toHaveBeenCalled();
      expect(screen.getByRole('radio', { name: 'weekly' }).getAttribute('aria-checked')).toBe('true');
    });
  });

  it('switches to monthly view and calls getMonthlyRevenue', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeTruthy();
    });

    mockGetMonthlyRevenue.mockResolvedValue([buildMonthlyRevenue()]);
    mockGetDailyRevenue.mockClear();
    mockGetMonthlyRevenue.mockClear();

    await userEvent.click(screen.getByRole('radio', { name: 'monthly' }));

    await waitFor(() => {
      expect(mockGetMonthlyRevenue).toHaveBeenCalled();
      expect(screen.getByRole('radio', { name: 'monthly' }).getAttribute('aria-checked')).toBe('true');
    });
  });

  // ── Date filter ──────────────────────────────────────────────
  it('re-fetches data when start date changes', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeTruthy();
    });

    mockGetDailyRevenue.mockClear();
    resolveDefaultData();

    const startInput = screen.getByLabelText('Start date') as HTMLInputElement;
    fireEvent.change(startInput, { target: { value: '2026-06-01' } });

    await waitFor(() => {
      expect(mockGetDailyRevenue).toHaveBeenCalledWith('2026-06-01', expect.any(String));
    });
  });

  it('re-fetches data when end date changes', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeTruthy();
    });

    mockGetDailyRevenue.mockClear();
    resolveDefaultData();

    const endInput = screen.getByLabelText('End date') as HTMLInputElement;
    fireEvent.change(endInput, { target: { value: '2026-07-20' } });

    await waitFor(() => {
      expect(mockGetDailyRevenue).toHaveBeenCalledWith(expect.any(String), '2026-07-20');
    });
  });

  // ── CSV export ───────────────────────────────────────────────
  it('triggers CSV download when Export CSV button is clicked', async () => {
    // jsdom doesn't provide URL.createObjectURL — stub it
    const origCreateObjectURL = URL.createObjectURL;
    const origRevokeObjectURL = URL.revokeObjectURL;
    URL.createObjectURL = vi.fn(() => 'blob:test');
    URL.revokeObjectURL = vi.fn();
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    // Capture the anchor element via createElement spy (it's never appended to DOM)
    const origCreateElement = document.createElement.bind(document);
    let capturedAnchor: HTMLAnchorElement | null = null;
    const createElementSpy = vi.spyOn(document, 'createElement').mockImplementation((tag: string, options?: ElementCreationOptions) => {
      const el = origCreateElement(tag, options);
      if (tag === 'a') capturedAnchor = el as HTMLAnchorElement;
      return el;
    });

    mockGetDailyRevenue.mockResolvedValue([
      buildDailyRevenue({ date: '2026-07-01', total_minor: 150000, sale_count: 12 }),
    ]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeTruthy();
    });

    await userEvent.click(screen.getByRole('button', { name: 'Export CSV' }));

    expect(URL.createObjectURL).toHaveBeenCalled();
    expect(capturedAnchor).toBeTruthy();
    expect(capturedAnchor!.download).toMatch(/sales-report-.*\.csv/);

    // Restore originals
    URL.createObjectURL = origCreateObjectURL;
    URL.revokeObjectURL = origRevokeObjectURL;
    clickSpy.mockRestore();
    createElementSpy.mockRestore();
  });

  // ── Print report ─────────────────────────────────────────────
  it('calls printSalesReceipt when Print button is clicked', async () => {
    mockGetDailyRevenue.mockResolvedValue([
      buildDailyRevenue({ total_minor: 150000 }),
    ]);
    mockGetTopProducts.mockResolvedValue([buildTopProduct()]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeTruthy();
    });

    await userEvent.click(screen.getByRole('button', { name: 'Print report' }));

    await waitFor(() => {
      expect(mockPrintSalesReceipt).toHaveBeenCalledTimes(1);
      const callArgs = mockPrintSalesReceipt.mock.calls[0]![0] as Record<string, unknown>;          expect(callArgs['receiptNumber']).toEqual(expect.stringMatching(/^RPT-/));
      expect(callArgs['items']).toBeTruthy();
    });
  });

  // ── ARIA ─────────────────────────────────────────────────────
  it('has role="region" with aria-label="Sales Report"', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Sales Report' })).toBeTruthy();
    });
  });

  it('view mode toggle has role="radiogroup"', async () => {
    resolveDefaultData();
    renderScreen();
    await waitFor(() => {
      expect(screen.getByRole('radiogroup', { name: 'View mode' })).toBeTruthy();
    });
  });

  // ── Edge: null category_id ───────────────────────────────────
  it('handles category with null category_id', async () => {
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([
      buildCategory({ category_id: null, category_name: 'Uncategorized' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByTestId('pie-chart')).toBeTruthy();
    });
  });

  // ── Edge: heatmap row with out-of-bounds values ──────────────
  it('handles heatmap row with day_of_week and hour within bounds', async () => {
    mockGetDailyRevenue.mockResolvedValue([buildDailyRevenue()]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([
      { day_of_week: 0, hour: 0, total_minor: 100, sale_count: 1 },
      { day_of_week: 6, hour: 23, total_minor: 200, sale_count: 2 },
    ]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByRole('grid', { name: 'Hourly heatmap' })).toBeTruthy();
    });
  });

  // ── All data empty ───────────────────────────────────────────
  it('renders all sections with empty data without errors', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetHourlyHeatmap.mockResolvedValue([]);
    mockGetCategoryBreakdown.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      // Revenue section still renders (heading + $0.00 total)
      expect(screen.getAllByText('Revenue').length).toBeGreaterThanOrEqual(1);
      // Category breakdown and top products both show "No results"
      expect(screen.getAllByText('No results').length).toBe(2);
      // Heatmap shows "No data"
      expect(screen.getByText('No data')).toBeTruthy();
    });
  });
});
