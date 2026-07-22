import { describe, expect, it, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import shiftsFtl from '@/locales/shifts.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/shifts', () => ({
  listShifts: vi.fn(),
  getActiveShift: vi.fn(),
  openShift: vi.fn(),
  closeShift: vi.fn(),
  getShiftReport: vi.fn(),
  createCashPayout: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Cashier', role_name: 'cashier' },
  }),
}));

vi.mock('@/contexts/CurrencyContext', () => ({
  useCurrency: () => ({ currency: 'IDR', setCurrency: vi.fn(), loading: false }),
}));

import ShiftManagementScreen from '@/features/shifts/ShiftManagementScreen';
import { listShifts, getActiveShift, getShiftReport } from '@/api/shifts';

const mockListShifts = listShifts as ReturnType<typeof vi.fn>;
const mockGetActiveShift = getActiveShift as ReturnType<typeof vi.fn>;
const mockGetShiftReport = getShiftReport as ReturnType<typeof vi.fn>;



const activeShift = {
  id: 'shift-1', userId: 'user-1', terminalId: null,
  openedAt: '2026-07-07T08:00:00.000Z', closedAt: null as string | null,
  openingBalanceMinor: 50000, closingBalanceMinor: null as number | null,
  expectedCashMinor: 55000, cashDifferenceMinor: null as number | null,
  totalSalesMinor: 150000, totalCashMinor: 80000, totalCardMinor: 70000,
  totalOtherMinor: 0, totalVoidsMinor: 0, totalRefundsMinor: 0,
  totalPayoutsMinor: 0, notes: '', status: 'open',
  createdAt: '2026-07-07T08:00:00.000Z', updatedAt: '2026-07-07T08:00:00.000Z',
};

const closedShifts = [
  { ...activeShift, id: 'shift-2', status: 'closed', closedAt: '2026-07-06T20:00:00.000Z',
    closingBalanceMinor: 55000, expectedCashMinor: 54000, cashDifferenceMinor: 1000 } as typeof activeShift,
];

describe('ShiftManagementScreen', () => {
  // ── Rendering ─────────────────────────────────────────────────

  it('renders the title', async () => {
    mockListShifts.mockResolvedValue([]);
    mockGetActiveShift.mockResolvedValue(null);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Shift Management')).toBeInTheDocument();
    });
  });

  it('shows loading skeleton initially', async () => {
    mockListShifts.mockReturnValue(new Promise(() => {}));
    mockGetActiveShift.mockReturnValue(new Promise(() => {}));
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);
    expect(document.querySelector('.shift-mgmt-loading-skeleton')).toBeInTheDocument();
  });

  // ── No active shift ──────────────────────────────────────────

  it('shows no active shift banner when none is active', async () => {
    mockListShifts.mockResolvedValue(closedShifts);
    mockGetActiveShift.mockResolvedValue(null);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('No active shift')).toBeInTheDocument();
    });
    expect(screen.getByText('Open Shift')).toBeInTheDocument();
  });

  it('shows Open Shift button on the no-active banner', async () => {
    mockListShifts.mockResolvedValue([]);
    mockGetActiveShift.mockResolvedValue(null);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Open Shift')).toBeInTheDocument();
    });
  });

  // ── Active shift card ────────────────────────────────────────

  it('shows active shift card when a shift is active', async () => {
    mockListShifts.mockResolvedValue([activeShift]);
    mockGetActiveShift.mockResolvedValue(activeShift);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Active Shift')).toBeInTheDocument();
    });
    expect(screen.getByText('Close Shift')).toBeInTheDocument();
    expect(screen.getByText('Record Payout')).toBeInTheDocument();
  });

  it('shows sales stats on the active shift card', async () => {
    mockListShifts.mockResolvedValue([activeShift]);
    mockGetActiveShift.mockResolvedValue(activeShift);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      // "Sales" appears in multiple labels (Sales, Cash Sales, Card Sales).
      const salesLabel = screen.getAllByText('Sales');
      expect(salesLabel.length).toBeGreaterThanOrEqual(2);
      expect(screen.getByText('Cash Sales')).toBeInTheDocument();
      expect(screen.getByText('Card Sales')).toBeInTheDocument();
    });
  });

  // ── Shift history table ──────────────────────────────────────

  it('shows shift history table with shifts', async () => {
    mockListShifts.mockResolvedValue(closedShifts);
    mockGetActiveShift.mockResolvedValue(null);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Shift History')).toBeInTheDocument();
      // "Closed" appears in both table header and status badge.
      expect(screen.getAllByText('Closed').length).toBeGreaterThanOrEqual(2);
    });
  });

  it('shows empty state when no shifts exist', async () => {
    mockListShifts.mockResolvedValue([]);
    mockGetActiveShift.mockResolvedValue(null);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('No shifts recorded yet.')).toBeInTheDocument();
    });
  });

  // ── Open shift modal ─────────────────────────────────────────

  it('opens the open shift modal when Open Shift is clicked', async () => {
    const user = userEvent.setup();
    mockListShifts.mockResolvedValue([]);
    mockGetActiveShift.mockResolvedValue(null);
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('Open Shift')).toBeInTheDocument();
    });

    // Click the Open Shift button on the banner.
    const openBtns = screen.getAllByText('Open Shift');
    await user.click(openBtns[0]!);

    await waitFor(() => {
      // The modal title should appear.
      expect(screen.getAllByText('Open Shift').length).toBeGreaterThanOrEqual(2);
    });
  });

  // ── Shift detail modal ───────────────────────────────────────

  it('opens detail modal when View button is clicked', async () => {
    const user = userEvent.setup();
    mockListShifts.mockResolvedValue(closedShifts);
    mockGetActiveShift.mockResolvedValue(null);
    mockGetShiftReport.mockResolvedValue({
      paymentBreakdown: [], hourlyBreakdown: [], cashPayouts: [],
      saleCount: 0, voidCount: 0, refundCount: 0,
    });
    renderWithFluentSync(<ShiftManagementScreen />, shiftsFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('View')).toBeInTheDocument();
    });

    await user.click(screen.getByText('View'));

    await waitFor(() => {
      expect(screen.getByText('Shift Details')).toBeInTheDocument();
      // "Status" appears in table header AND detail modal.
      const statusElements = screen.getAllByText('Status');
      expect(statusElements.length).toBeGreaterThanOrEqual(2);
    });
  });
});
