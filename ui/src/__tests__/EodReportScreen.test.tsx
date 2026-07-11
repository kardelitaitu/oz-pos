import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import salesFtl from '@/locales/sales.ftl?raw';
import shiftsFtl from '@/locales/shifts.ftl?raw';

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

const wrap = (children: React.ReactNode) =>
  withFluent(children, salesFtl, shiftsFtl);

function renderScreen() {
  return render(wrap(<EodReportScreen />));
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

  // -- Payment breakdown: empty state --
  it('shows empty payment data when no payment breakdown', async () => {
    mockEodReport.mockResolvedValue(makeEodReport({ payment_breakdown: [] }));
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Payment Breakdown')).toBeTruthy();
      expect(screen.getByText('No payment data')).toBeTruthy();
    });
  });

  // -- Payment breakdown: ARIA progress bars --
  it('renders payment bars with role progressbar', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      const bars = screen.getAllByRole('progressbar');
      expect(bars.length).toBe(2);
    });
  });

  // -- Hourly chart: empty state --
  it('shows empty hourly data when no hourly breakdown', async () => {
    mockEodReport.mockResolvedValue(makeEodReport({ hourly_breakdown: [] }));
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('No hourly data')).toBeTruthy();
    });
  });

  // -- Summary table --
  it('renders Today\'s Summary section with all rows', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      const summaryEls = screen.getAllByText("Today's Summary");
      expect(summaryEls.length).toBeGreaterThanOrEqual(1);
    });
    expect(screen.getByText('Completed Sales')).toBeTruthy();
    expect(screen.getByText('Voided Sales')).toBeTruthy();
    expect(screen.getByText('Voided Value')).toBeTruthy();
    expect(screen.getByText('Sales with Discounts')).toBeTruthy();
    expect(screen.getByText('Payment Methods Used')).toBeTruthy();
  });

  // -- Date display --
  it('renders the date in the header', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      const dateEl = document.querySelector('.eod-report-date');
      expect(dateEl).toBeTruthy();
      expect(dateEl!.textContent).toBeTruthy();
    });
  });

  // -- Print button --
  it('calls printReceipt when Print button is clicked', async () => {
    mockEodReport.mockResolvedValue(makeEodReport());
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Print')).toBeTruthy();
    });

    const user = userEvent.setup();
    await user.click(screen.getByText('Print'));

    expect(mockPrintReceipt).toHaveBeenCalledWith(
      expect.objectContaining({ body: expect.stringContaining('END-OF-DAY REPORT') }),
    );
  });

  // -- Discounts KPI shows counts --
  it('displays discount count in KPI', async () => {
    mockEodReport.mockResolvedValue(makeEodReport({ discount_count: 3 }));
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getAllByText('3').length).toBeGreaterThanOrEqual(1);
    });
  });

  it('shows "No discounts applied" when discount_count is 0', async () => {
    mockEodReport.mockResolvedValue(makeEodReport({ discount_count: 0 }));
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('No discounts applied')).toBeTruthy();
    });
  });

  // -- Voids KPI --
  it('displays void count in KPI', async () => {
    mockEodReport.mockResolvedValue(makeEodReport({ void_count: 5 }));
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      const voidCounts = screen.getAllByText('5');
      expect(voidCounts.length).toBeGreaterThanOrEqual(1);
    });
  });

  // -- Active shift details --
  it('shows active shift opening balance and sales', async () => {
    const today = new Date().toISOString().slice(0, 10);
    mockEodReport.mockResolvedValue(makeEodReport({ total_sales: 0 }));
    mockListShifts.mockResolvedValue([
      makeShift({
        status: 'open',
        closedAt: null,
        closingBalanceMinor: null,
        openedAt: `${today}T08:00:00.000Z`,
        openingBalanceMinor: 150000,
        totalSalesMinor: 350000,
      }),
    ]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Opening balance')).toBeTruthy();
      expect(screen.getByText('Sales this shift')).toBeTruthy();
    });
  });

  // -- Closed shifts table: diff tags --
  it('shows over/short tags on shift cash differences', async () => {
    const today = new Date().toISOString().slice(0, 10);
    mockEodReport.mockResolvedValue(makeEodReport({ total_sales: 5 }));
    mockListShifts.mockResolvedValue([
      makeShift({
        openedAt: `${today}T08:00:00.000Z`,
        closedAt: `${today}T18:00:00.000Z`,
        cashDifferenceMinor: 50000,
      }),
    ]);
    renderScreen();

    await waitFor(() => {
      const tags = document.querySelectorAll('.eod-report-shift-tag');
      expect(tags.length).toBeGreaterThanOrEqual(1);
    });
  });

  // -- Negative cash diff class --
  it('applies negative diff class for short cash differences', async () => {
    const today = new Date().toISOString().slice(0, 10);
    mockEodReport.mockResolvedValue(makeEodReport({ total_sales: 5 }));
    mockListShifts.mockResolvedValue([
      makeShift({
        openedAt: `${today}T08:00:00.000Z`,
        closedAt: `${today}T18:00:00.000Z`,
        cashDifferenceMinor: -30000,
      }),
    ]);
    renderScreen();

    await waitFor(() => {
      const negDiffs = document.querySelectorAll('.eod-report-shift-diff--negative');
      expect(negDiffs.length).toBeGreaterThanOrEqual(1);
    });
  });

  // -- Cash reconciliation with 2+ closed shifts --
  it('shows cash reconciliation when multiple shifts closed today', async () => {
    const today = new Date().toISOString().slice(0, 10);
    mockEodReport.mockResolvedValue(makeEodReport({ total_sales: 5 }));
    mockListShifts.mockResolvedValue([
      makeShift({
        id: 'shift-1',
        openedAt: `${today}T08:00:00.000Z`,
        closedAt: `${today}T12:00:00.000Z`,
        cashDifferenceMinor: 10000,
      }),
      makeShift({
        id: 'shift-2',
        openedAt: `${today}T12:00:00.000Z`,
        closedAt: `${today}T18:00:00.000Z`,
        cashDifferenceMinor: 20000,
      }),
    ]);
    renderScreen();

    await waitFor(() => {
      expect(screen.getByText('Cash Reconciliation')).toBeTruthy();
      expect(screen.getByText('Total opening')).toBeTruthy();
      expect(screen.getByText('Total counted')).toBeTruthy();
      expect(screen.getByText('Total expected')).toBeTruthy();
      expect(screen.getByText('Net difference')).toBeTruthy();
    });
  });

  // -- Shift totals row --
  it('renders totals row in shift table', async () => {
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
      expect(screen.getByText('Total')).toBeTruthy();
    });
  });

  // -- Print button disabled when no report --
  it('disables Print button when report is null', async () => {
    mockEodReport.mockResolvedValue(null);
    mockListShifts.mockResolvedValue([]);
    renderScreen();

    await waitFor(() => {
      const printBtn = document.querySelector('.eod-report-print-btn');
      expect(printBtn).toBeTruthy();
      expect((printBtn as HTMLButtonElement).disabled).toBe(true);
    });
  });

  // -- Refresh shows spinner/indicator during load --
  it('disables Refresh button while loading', async () => {
    mockEodReport.mockImplementation(() => new Promise(() => {}));
    mockListShifts.mockImplementation(() => new Promise(() => {}));
    renderScreen();

    const refreshBtn = document.querySelector('.eod-report-refresh-btn') as HTMLButtonElement;
    expect(refreshBtn).toBeTruthy();
    expect(refreshBtn.disabled).toBe(true);
  });
});
