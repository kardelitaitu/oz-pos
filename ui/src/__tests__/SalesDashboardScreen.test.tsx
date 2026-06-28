import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import SalesDashboardScreen from '@/features/sales/SalesDashboardScreen';

const LOCALE_STRINGS = [
  'sales-dashboard-title = Sales Dashboard',
  'sales-dashboard-daily-total = Daily Total',
  'sales-dashboard-total-sales = Total Sales',
  'sales-dashboard-total-items = Total Items',
  'sales-dashboard-hourly-title = Sales by Hour',
  'sales-dashboard-loading = Loading…',
  'sales-dashboard-no-data = No data for today',
].join('\n');

const wrap = (children: React.ReactNode) => {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(LOCALE_STRINGS));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

const SAMPLE_SUMMARY = [
  { sale_id: 'sale-1', total_minor: 1250, currency: 'USD', line_count: 2, status: 'completed', created_at: '2026-06-28T10:00:00Z' },
  { sale_id: 'sale-2', total_minor: 800, currency: 'USD', line_count: 1, status: 'completed', created_at: '2026-06-28T11:00:00Z' },
];

const SAMPLE_HOURLY = [
  { hour: 10, total_minor: 1250, sale_count: 1 },
  { hour: 11, total_minor: 800, sale_count: 1 },
];

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn() as any,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'export_daily_summary') return Promise.resolve(SAMPLE_SUMMARY);
    if (cmd === 'export_sales_by_hour') return Promise.resolve(SAMPLE_HOURLY);
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  });
});

describe('SalesDashboardScreen', () => {
  it('renders title', async () => {
    render(wrap(<SalesDashboardScreen />));
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /sales dashboard/i })).toBeInTheDocument();
    });
  });

  it('shows KPI cards', async () => {
    render(wrap(<SalesDashboardScreen />));
    await waitFor(() => {
      expect(screen.getByText(/daily total/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/total sales/i)).toBeInTheDocument();
    expect(screen.getByText(/total items/i)).toBeInTheDocument();
  });

  it('shows loading state initially', async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    render(wrap(<SalesDashboardScreen />));
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('displays hourly data', async () => {
    render(wrap(<SalesDashboardScreen />));
    await waitFor(() => {
      expect(screen.getByText(/sales by hour/i)).toBeInTheDocument();
    });
    expect(screen.getByLabelText(/sales by hour bar chart/i)).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText('11')).toBeInTheDocument();
  });

  it('shows no data state', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'export_daily_summary') return Promise.resolve([]);
      if (cmd === 'export_sales_by_hour') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(wrap(<SalesDashboardScreen />));
    await waitFor(() => {
      expect(screen.getByText(/no data for today/i)).toBeInTheDocument();
    });
  });

  it('formats currency correctly', async () => {
    render(wrap(<SalesDashboardScreen />));
    await waitFor(() => {
      expect(screen.getByText(/daily total/i)).toBeInTheDocument();
    });
    expect(screen.getByText('$12.50')).toBeInTheDocument();
    expect(screen.getByText('$8.00')).toBeInTheDocument();
  });
});
