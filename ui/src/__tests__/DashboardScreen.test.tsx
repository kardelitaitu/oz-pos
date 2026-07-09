import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import DashboardScreen from '@/features/reports/DashboardScreen';

// ── FTL bundles ────────────────────────────────────────────────────────
const sharedFtl = `
error-occurred = An error occurred
spinner-label = Loading dashboard
`;

// dashboard keys only exist in id locale, so fallback children are used
const reportsFtl = ``;

// ── mock API functions ─────────────────────────────────────────────────
const mockGetDailyRevenue = vi.fn();
const mockGetTopProducts = vi.fn();
const mockGetLowStockAlerts = vi.fn();

vi.mock('@/api/reports', () => ({
  getDailyRevenue: (...args: unknown[]) => mockGetDailyRevenue(...args),
  getTopProducts: (...args: unknown[]) => mockGetTopProducts(...args),
  getLowStockAlerts: (...args: unknown[]) => mockGetLowStockAlerts(...args),
}));

vi.mock('@/components/Card', () => ({
  Card: ({ children, className, shadow }: Record<string, unknown>) => (
    <div className={className as string} data-shadow={shadow as string}>{children as React.ReactNode}</div>
  ),
}));

vi.mock('@/components/Spinner', () => ({
  Spinner: (props: Record<string, unknown>) => <div data-testid="spinner" aria-label={props['aria-label'] as string} />,
}));

vi.mock('@/features/reports/DashboardScreen.css', () => ({}));

// ── helpers ────────────────────────────────────────────────────────────
const bundle = new FluentBundle('en');
bundle.addResource(new FluentResource(sharedFtl));
bundle.addResource(new FluentResource(reportsFtl));
const l10n = new ReactLocalization([bundle]);

function buildRevenueRow(overrides: Partial<{ date: string; total_minor: number; currency: string; sale_count: number }> = {}) {
  return {
    date: overrides.date ?? '2026-07-07',
    total_minor: overrides.total_minor ?? 150000,
    currency: overrides.currency ?? 'USD',
    sale_count: overrides.sale_count ?? 12,
  };
}

function buildTopProductRow(overrides: Partial<{ product_id: string; sku: string; name: string; total_qty: number; total_minor: number }> = {}) {
  return {
    product_id: overrides.product_id ?? 'prod-1',
    sku: overrides.sku ?? 'SKU001',
    name: overrides.name ?? 'Espresso',
    total_qty: overrides.total_qty ?? 45,
    total_minor: overrides.total_minor ?? 90000,
  };
}

function buildLowStockAlert(overrides: Partial<{ product_id: string; sku: string; name: string; current_qty: number; threshold: number }> = {}) {
  return {
    product_id: overrides.product_id ?? 'prod-lo',
    sku: overrides.sku ?? 'SKU-LOW',
    name: overrides.name ?? 'Milk',
    current_qty: overrides.current_qty ?? 3,
    threshold: overrides.threshold ?? 10,
  };
}

function renderScreen() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <DashboardScreen />
    </LocalizationProvider>,
  );
}

// ── tests ──────────────────────────────────────────────────────────────
describe('DashboardScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default: never resolves (loading state)
    mockGetDailyRevenue.mockImplementation(() => new Promise(() => {}));
    mockGetTopProducts.mockImplementation(() => new Promise(() => {}));
    mockGetLowStockAlerts.mockImplementation(() => new Promise(() => {}));
  });

  // ── Loading ────────────────────────────────────────────────────────
  it('shows loading spinner initially', () => {
    renderScreen();
    expect(screen.getByTestId('spinner')).toBeTruthy();
    expect(screen.getByTestId('spinner').getAttribute('aria-label')).toBe('Loading dashboard');
  });

  // ── Error ──────────────────────────────────────────────────────────
  it('shows error message when all API calls fail', async () => {
    const error = new Error('Server offline');
    mockGetDailyRevenue.mockRejectedValue(error);
    mockGetTopProducts.mockRejectedValue(error);
    mockGetLowStockAlerts.mockRejectedValue(error);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('An error occurred')).toBeTruthy();
    });
  });

  // ── Title ──────────────────────────────────────────────────────────
  it('renders the Dashboard title', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Dashboard')).toBeTruthy();
    });
  });

  // ── KPI cards ──────────────────────────────────────────────────────
  it('shows KPI labels: Today Revenue, Orders Today, Top Product', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText("Today's Revenue")).toBeTruthy();
      expect(screen.getByText('Orders Today')).toBeTruthy();
      expect(screen.getByText('Top Product')).toBeTruthy();
    });
  });

  it('displays formatted revenue and order count in KPIs', async () => {
    const revenue = [buildRevenueRow({ total_minor: 250000, sale_count: 5 })];
    mockGetDailyRevenue.mockResolvedValue(revenue);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      // $2,500.00 appears in both KPI and weekly bar — use getAllByText
      const amounts = screen.getAllByText('$2,500.00');
      expect(amounts.length).toBeGreaterThanOrEqual(1);
      // Order count (5) appears only once
      expect(screen.getByText('5')).toBeTruthy();
    });
  });

  it('shows top product name or dash when none', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('-')).toBeTruthy();
    });
  });

  it('shows top product name when available', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([buildTopProductRow({ name: 'Latte' })]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Latte')).toBeTruthy();
    });
  });

  // ── Weekly revenue chart ───────────────────────────────────────────
  it('renders weekly revenue section with "Revenue This Week" heading', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Revenue This Week')).toBeTruthy();
    });
  });

  it('renders weekly bar rows with date labels and progress bars', async () => {
    const weekData = [
      buildRevenueRow({ date: '2026-07-01', total_minor: 100000 }),
      buildRevenueRow({ date: '2026-07-02', total_minor: 200000 }),
    ];
    // first call = today, second call = weekly
    mockGetDailyRevenue
      .mockResolvedValueOnce([])          // getDailyRevenue(today, today)
      .mockResolvedValueOnce(weekData);   // getDailyRevenue(weekAgo, today)
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('07-01')).toBeTruthy();
      expect(screen.getByText('07-02')).toBeTruthy();
      const bars = document.querySelectorAll('.dashboard-weekly-bar');
      expect(bars.length).toBe(2);
    });
  });

  it('weekly chart bars have role="img" with aria-label describing value', async () => {
    const weekData = [buildRevenueRow({ date: '2026-07-01', total_minor: 100000 })];
    mockGetDailyRevenue
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(weekData);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const bar = screen.getByRole('img');
      expect(bar).toBeTruthy();
      expect(bar.getAttribute('aria-label')).toMatch(/1,000\.00|1\.000/);
    });
  });

  // ── Low stock alerts ───────────────────────────────────────────────
  it('renders "Low Stock Alerts" section heading', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Low Stock Alerts')).toBeTruthy();
    });
  });

  it('shows "No sales data yet today" when low stock is empty', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('No sales data yet today')).toBeTruthy();
    });
  });

  it('renders low stock items with name and quantity', async () => {
    const alerts = [
      buildLowStockAlert({ name: 'Milk', current_qty: 2 }),
      buildLowStockAlert({ name: 'Sugar', current_qty: 5 }),
    ];
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue(alerts);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Milk')).toBeTruthy();
      expect(screen.getByText('2 left')).toBeTruthy();
      expect(screen.getByText('Sugar')).toBeTruthy();
      expect(screen.getByText('5 left')).toBeTruthy();
    });
  });

  it('low stock list has aria-label', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([buildLowStockAlert()]);
    renderScreen();
    await waitFor(() => {
      const list = screen.getByRole('list', { name: 'Low stock alerts' });
      expect(list).toBeTruthy();
    });
  });

  // ── ARIA ───────────────────────────────────────────────────────────
  it('has role="region" with aria-label="Dashboard" on container', async () => {
    mockGetDailyRevenue.mockResolvedValue([]);
    mockGetTopProducts.mockResolvedValue([]);
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Dashboard' })).toBeTruthy();
    });
  });
});
