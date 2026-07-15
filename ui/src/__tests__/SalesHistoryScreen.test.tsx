import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/sales', () => ({
  listSales: vi.fn(),
  getSale: vi.fn(),
  printSalesReceipt: vi.fn(),
  listRefunds: vi.fn(),
  voidSale: vi.fn(),
}));

vi.mock('@/api/staff', () => ({
  listStaff: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Cashier', role_name: 'cashier' },
    isManager: true,
  }),
}));

vi.mock('@/features/sales/RefundModal', () => ({
  default: ({ onClose }: { onClose: () => void }) => (
    <div role="dialog" aria-label="refund-modal">
      <button onClick={onClose}>Close Refund</button>
    </div>
  ),
}));

import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';
import { listSales, getSale, listRefunds } from '@/api/sales';
import { listStaff } from '@/api/staff';

const mockListSales = listSales as ReturnType<typeof vi.fn>;
const mockGetSale = getSale as ReturnType<typeof vi.fn>;
const mockListRefunds = listRefunds as ReturnType<typeof vi.fn>;
const mockListStaff = listStaff as ReturnType<typeof vi.fn>;



const sampleSales = [
  {
    id: 'sale-001-aaaa-bbbb-cccc-ddddeeee', status: 'Completed',
    createdAt: '2026-07-07T10:00:00.000Z', total: { minor_units: 50000, currency: 'IDR' },
    lineCount: 2, paymentMethod: 'cash', userId: 'user-1',
  },
  {
    id: 'sale-002-aaaa-bbbb-cccc-ddddefff', status: 'Pending',
    createdAt: '2026-07-07T11:00:00.000Z', total: { minor_units: 25000, currency: 'IDR' },
    lineCount: 1, paymentMethod: 'card', userId: 'user-2',
  },
  {
    id: 'sale-003-aaaa-bbbb-cccc-ddddaaaa', status: 'Voided',
    createdAt: '2026-07-06T09:00:00.000Z', total: { minor_units: 10000, currency: 'IDR' },
    lineCount: 1, paymentMethod: null, userId: 'user-1',
  },
];

const sampleStaff = [
  { id: 'user-1', display_name: 'Alice' },
  { id: 'user-2', display_name: 'Bob' },
];

const sampleDetail = {
  id: 'sale-001-aaaa-bbbb-cccc-ddddeeee', status: 'Completed',
  createdAt: '2026-07-07T10:00:00.000Z', total: { minor_units: 50000, currency: 'IDR' },
  subtotal: { minor_units: 50000, currency: 'IDR' },
  taxTotal: { minor_units: 0, currency: 'IDR' },
  paymentMethod: 'cash', userId: 'user-1',
  tenderedMinor: 100000,
  lines: [
    { id: 'line-1', sku: 'SKU-001', name: 'Widget', qty: 2,
      unit_price: { minor_units: 25000, currency: 'IDR' },
      total_minor: 50000, tax_amount: null },
  ],
};

describe('SalesHistoryScreen', () => {
  beforeEach(() => {
    mockListStaff.mockResolvedValue(sampleStaff);
  });

  // ── Rendering ─────────────────────────────────────────────────

  it('renders the title', async () => {
    mockListSales.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Sales History')).toBeInTheDocument();
    });
  });

  it('shows loading state', async () => {
    mockListSales.mockReturnValue(new Promise(() => {}));
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);
    expect(screen.getByText(/loading sales/i)).toBeInTheDocument();
  });

  it('shows empty state when no sales exist', async () => {
    mockListSales.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('No sales recorded yet')).toBeInTheDocument();
    });
  });

  // ── Table rendering ──────────────────────────────────────────

  it('displays sales in the table with cashier names', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      // Cashier names appear in table cells AND the cashier filter dropdown.
      expect(screen.getAllByText('Alice').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('Bob').length).toBeGreaterThanOrEqual(1);
    });
    // Status text appears in both filter chips and badges — use getAllByText.
    expect(screen.getAllByText('Completed').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Voided').length).toBeGreaterThanOrEqual(1);
  });

  it('shows status filter chips', async () => {
    mockListSales.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('All')).toBeInTheDocument();
    });
    // "Completed", "Pending", "Voided" appear only in filter chips (no table).
    expect(screen.getByText('Completed')).toBeInTheDocument();
    expect(screen.getByText('Pending')).toBeInTheDocument();
    expect(screen.getByText('Voided')).toBeInTheDocument();
  });

  it('filters sales by status when a filter chip is clicked', async () => {
    const user = userEvent.setup();
    mockListSales.mockResolvedValue(sampleSales);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);

    await waitFor(() => {
      // First sale's truncated ID should appear in the table.
      expect(screen.getByText(/sale-001/)).toBeInTheDocument();
      expect(screen.getByText(/sale-002/)).toBeInTheDocument();
    });

    // Click the "Voided" filter chip.
    const voidedChip = screen.getByRole('radio', { name: /voided/i });
    await user.click(voidedChip);

    // After filtering, only the Voided sale (sale-003) remains.
    // The Completed (sale-001) and Pending (sale-002) sale IDs should be gone.
    await waitFor(() => {
      expect(screen.queryByText(/sale-001/)).not.toBeInTheDocument();
      expect(screen.queryByText(/sale-002/)).not.toBeInTheDocument();
    });
  });

  it('shows search input and cashier dropdown', async () => {
    mockListSales.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
    const searchInputs = screen.getAllByRole('textbox');
    expect(searchInputs.length).toBeGreaterThanOrEqual(1);
  });

  it('shows export CSV button', async () => {
    mockListSales.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('Export CSV')).toBeInTheDocument();
    });
  });

  // ── Detail modal ─────────────────────────────────────────────

  it('opens detail modal when View is clicked', async () => {
    const user = userEvent.setup();
    mockListSales.mockResolvedValue(sampleSales);
    mockGetSale.mockResolvedValue(sampleDetail);
    mockListRefunds.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);

    await waitFor(() => {
      const viewBtns = screen.getAllByText('View');
      expect(viewBtns.length).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByText('View')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Sale Detail')).toBeInTheDocument();
      expect(screen.getByText('Line Items')).toBeInTheDocument();
    });
  });

  it('shows line items in detail modal', async () => {
    const user = userEvent.setup();
    mockListSales.mockResolvedValue([sampleSales[0]!]);
    mockGetSale.mockResolvedValue(sampleDetail);
    mockListRefunds.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('View').length).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByText('View')[0]!);

    await waitFor(() => {
      expect(screen.getByText('SKU-001')).toBeInTheDocument();
      expect(screen.getByText('Widget')).toBeInTheDocument();
    });
  });

  it('shows Reprint Receipt and Refund buttons in detail', async () => {
    const user = userEvent.setup();
    mockListSales.mockResolvedValue([sampleSales[0]!]);
    mockGetSale.mockResolvedValue(sampleDetail);
    mockListRefunds.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('View').length).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByText('View')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Reprint Receipt')).toBeInTheDocument();
      expect(screen.getByText('Refund')).toBeInTheDocument();
    });
  });

  // ── Refund modal ─────────────────────────────────────────────

  it('opens refund modal when Refund is clicked in detail', async () => {
    const user = userEvent.setup();
    mockListSales.mockResolvedValue([sampleSales[0]!]);
    mockGetSale.mockResolvedValue(sampleDetail);
    mockListRefunds.mockResolvedValue([]);
    renderWithFluentSync(<SalesHistoryScreen />, salesFtl, sharedFtl);

    await waitFor(() => {
      expect(screen.getAllByText('View').length).toBeGreaterThan(0);
    });
    await user.click(screen.getAllByText('View')[0]!);

    await waitFor(() => {
      expect(screen.getByText('Refund')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Refund'));

    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: /refund-modal/i })).toBeInTheDocument();
    });
  });
});
