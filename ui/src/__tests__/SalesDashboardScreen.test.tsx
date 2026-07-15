import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import SalesDashboardScreen from '@/features/sales/SalesDashboardScreen';
import { registerSalesWidgets } from '@/features/sales/widgets';
import { clearWidgets } from '@/platform/ui/widget-registry';

const SAMPLE_SUMMARY = [
  { sale_id: 'sale-1', total_minor: 1250, currency: 'USD', line_count: 2, status: 'completed', created_at: '2026-06-28T10:00:00Z' },
  { sale_id: 'sale-2', total_minor: 800, currency: 'USD', line_count: 1, status: 'completed', created_at: '2026-06-28T11:00:00Z' },
];

const SAMPLE_HOURLY = [
  { hour: 10, total_minor: 1250, sale_count: 1 },
  { hour: 11, total_minor: 800, sale_count: 1 },
];

const { invokeMock } = vi.hoisted(() => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  invokeMock: vi.fn() as any,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@/hooks/useFeatures', () => ({
  useFeatures: () => ({
    enabled: new Set(['simple-retail']),
    loading: false,
    isEnabled: (key: string) => key === 'simple-retail',
    filterRoutes: (routes: string[]) => routes,
    error: null,
    loaded: true,
  }),
}));

beforeEach(() => {
  clearWidgets();
  registerSalesWidgets();
  invokeMock.mockClear();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'export_daily_summary') return Promise.resolve(SAMPLE_SUMMARY);
    if (cmd === 'export_sales_by_hour') return Promise.resolve(SAMPLE_HOURLY);
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  });
});

describe('SalesDashboardScreen', () => {
  it('renders title', async () => {
    renderWithFluentSync(<SalesDashboardScreen />, salesFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /sales dashboard/i })).toBeInTheDocument();
    });
  });

  it('shows KPI cards', async () => {
    renderWithFluentSync(<SalesDashboardScreen />, salesFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /daily total/i })).toBeInTheDocument();
    });
    expect(screen.getByText(/total sales/i)).toBeInTheDocument();
    expect(screen.getByText(/total items/i)).toBeInTheDocument();
  });

  it('shows loading state initially', async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    renderWithFluentSync(<SalesDashboardScreen />, salesFtl);
    expect(screen.getAllByText(/loading/i).length).toBeGreaterThan(0);
  });

  it('displays hourly data', async () => {
    renderWithFluentSync(<SalesDashboardScreen />, salesFtl);
    await waitFor(() => {
      expect(screen.getByText(/sales by hour/i)).toBeInTheDocument();
    });
    expect(screen.getByRole('list', { name: /hourly sales bars/i })).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText('11')).toBeInTheDocument();
  });

  it('shows no data state', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'export_daily_summary') return Promise.resolve([]);
      if (cmd === 'export_sales_by_hour') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    renderWithFluentSync(<SalesDashboardScreen />, salesFtl);
    await waitFor(() => {
      expect(screen.getByText(/no data for today/i)).toBeInTheDocument();
    });
  });

  it('formats currency correctly', async () => {
    renderWithFluentSync(<SalesDashboardScreen />, salesFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /daily total/i })).toBeInTheDocument();
    });
    // formatMoney uses id-ID locale by default → $ 12,50, $ 8,00
    expect(screen.getByText(/\$ 12,50/)).toBeInTheDocument();
    expect(screen.getByText(/\$ 8,00/)).toBeInTheDocument();
  });
});
