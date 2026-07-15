import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import shiftsFtl from '@/locales/shifts.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import EodReportScreen from '@/features/sales/EodReportScreen';

// ── Mocks ────────────────────────────────────────────────────────────

const mockEodReport = vi.fn();
const mockListShifts = vi.fn();
const mockPrintReceipt = vi.fn();

vi.mock('@/api/sales', () => ({
  exportEodReport: (...args: unknown[]) => mockEodReport(...args),
}));

vi.mock('@/api/shifts', () => ({
  listShifts: (...args: unknown[]) => mockListShifts(...args),
}));

vi.mock('@/api/hardware', () => ({
  printReceipt: (...args: unknown[]) => mockPrintReceipt(...args),
}));

// ── Helpers ───────────────────────────────────────────────────────────

function makeEodReport(overrides: Record<string, unknown> = {}) {
  return {
    total_sales: 10,
    total_revenue: 500000,
    currency: 'IDR',
    payment_breakdown: [
      { method: 'cash', count: 5, total: 250000 },
      { method: 'card', count: 5, total: 250000 },
    ],
    void_count: 1,
    void_total: 25000,
    discount_count: 2,
    discount_total: 10000,
    hourly_breakdown: [
      { hour: 8, total_minor: 50000, sale_count: 1 },
      { hour: 14, total_minor: 450000, sale_count: 9 },
    ],
    ...overrides,
  };
}

function makeShift(overrides: Record<string, unknown> = {}) {
  return {
    id: 'shift-1',
    userId: 'user-1',
    terminalId: null,
    openedAt: '2025-07-07T08:00:00.000Z',
    closedAt: '2025-07-07T18:00:00.000Z',
    openingBalanceMinor: 100000,
    closingBalanceMinor: 400000,
    expectedCashMinor: 350000,
    cashDifferenceMinor: 50000,
    totalSalesMinor: 500000,
    totalCashMinor: 250000,
    totalCardMinor: 250000,
    totalOtherMinor: 0,
    totalVoidsMinor: 25000,
    totalRefundsMinor: 0,
    totalPayoutsMinor: 0,
    notes: '',
    status: 'closed',
    createdAt: '2025-07-07T08:00:00.000Z',
    updatedAt: '2025-07-07T18:00:00.000Z',
    ...overrides,
  };
}

function renderScreen() {
  return renderWithFluentSync(<EodReportScreen />, salesFtl, shiftsFtl, sharedFtl);
}

// ── Tests ─────────────────────────────────────────────────────────────

describe('EodReportScreen', () => {
  beforeEach(() => {
    mockEodReport.mockReset();
    mockListShifts.mockReset();
    mockPrintReceipt.mockReset();
  });

  it('renders the title', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('End-of-Day Report')).toBeTruthy();
    });
  });

  it('shows loading state initially', () => {
    mockEodReport.mockImplementation(() => new Promise(() => {}));
    mockListShifts.mockImplementation(() => new Promise(() => {}));
    renderScreen();

    expect(screen.getByText('Loading report…')).toBeTruthy();
  });

  it('shows error state with retry button', async () => {
    mockEodReport.mockRejectedValue(new Error('Network error'));
    mockListShifts.mockRejectedValue(new Error('Network error'));
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Retry')).toBeTruthy();
    });
  });

  it('shows empty state when no report data', async () => {
    mockEodReport.mockResolvedValue(null);
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('No sales data available for today.')).toBeTruthy();
    });
  });

  it('shows KPI cards when report loads', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      // Total Revenue appears in both KPI card and summary table
      const revenueEls = screen.getAllByText('Total Revenue');
      expect(revenueEls.length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('Average Sale')).toBeTruthy();
      expect(screen.getByText('Voids')).toBeTruthy();
      expect(screen.getByText('Discounts Applied')).toBeTruthy();
    });
  });

  it('shows payment breakdown with progress bars', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Payment Breakdown')).toBeTruthy();
      expect(screen.getByText('Cash')).toBeTruthy();
      expect(screen.getByText('Card')).toBeTruthy();
    });

    const bars = document.querySelectorAll('.eod-report-payment-bar');
    expect(bars.length).toBe(2);
  });

  it('shows hourly sales chart', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Sales by Hour')).toBeTruthy();
    });

    const barRows = document.querySelectorAll('.eod-report-hour-bar-row');
    expect(barRows.length).toBe(24);
  });

  it('has a Refresh button', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Refresh')).toBeTruthy();
    });
  });

  it('has a Print button', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Print')).toBeTruthy();
    });
  });

  it('clicks Refresh re-fetches data', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Refresh')).toBeTruthy();
    });

    const user = userEvent.setup();
    await user.click(screen.getByText('Refresh'));

    expect(mockEodReport).toHaveBeenCalledTimes(2);
    expect(mockListShifts).toHaveBeenCalledTimes(2);
  });

  it('shows shift summary when closed shifts exist for today', async () => {
    const today = new Date().toISOString().slice(0, 10);
    mockEodReport.mockResolvedValue(makeEodReport({ total_sales: 5 }));
    mockListShifts.mockResolvedValue([
      makeShift({
        openedAt: `${today}T08:00:00.000Z`,
        closedAt: `${today}T18:00:00.000Z`,
      }),
    ]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Cashier Shifts')).toBeTruthy();
      expect(screen.getByText('Closed Shifts Today')).toBeTruthy();
    });
  });

  it('shows active shift banner when a shift is open', async () => {
    const today = new Date().toISOString().slice(0, 10);
    mockEodReport.mockResolvedValue(makeEodReport({ total_sales: 0 }));
    mockListShifts.mockResolvedValue([
      makeShift({
        status: 'open',
        closedAt: null,
        closingBalanceMinor: null,
        openedAt: `${today}T08:00:00.000Z`,
      }),
    ]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Shift in progress')).toBeTruthy();
    });
  });
});
