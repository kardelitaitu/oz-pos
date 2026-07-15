import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluent } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/sales', () => ({
  listSales: vi.fn(),
  getSale: vi.fn(),
  voidSale: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Cashier', role_name: 'cashier' },
  }),
}));

import VoidOrdersScreen from '@/features/sales/VoidOrdersScreen';
import { listSales, getSale, voidSale } from '@/api/sales';

const mockListSales = listSales as ReturnType<typeof vi.fn>;
const mockGetSale = getSale as ReturnType<typeof vi.fn>;
const mockVoidSale = voidSale as ReturnType<typeof vi.fn>;



const sampleSales = [
  {
    id: 'ORD-001', createdAt: '2026-07-05T10:00:00Z', status: 'Active',
    total: { minor_units: 35000, currency: 'IDR' }, lineCount: 3,
    paymentMethod: 'CASH', userId: 'user-1',
  },
  {
    id: 'ORD-002', createdAt: '2026-07-05T11:00:00Z', status: 'Completed',
    total: { minor_units: 50000, currency: 'IDR' }, lineCount: 2,
    paymentMethod: 'CARD', userId: 'user-1',
  },
  {
    id: 'ORD-003', createdAt: '2026-07-05T12:00:00Z', status: 'Voided',
    total: { minor_units: 12000, currency: 'IDR' }, lineCount: 1,
    paymentMethod: 'CASH', userId: 'user-1',
  },
];

const sampleDetail = {
  id: 'ORD-001', createdAt: '2026-07-05T10:00:00Z', status: 'Active',
  total: { minor_units: 35000, currency: 'IDR' },
  subtotal: { minor_units: 35000, currency: 'IDR' },
  taxTotal: { minor_units: 0, currency: 'IDR' },
  lineCount: 3, paymentMethod: 'CASH', tenderedMinor: 50000,
  userId: 'user-1',
  lines: [
    { id: 'line-1', sku: 'SKU-001', name: 'Indomie Goreng', qty: 2, unit_price: { minor_units: 3500, currency: 'IDR' }, total_minor: 7000, tax_amount: null, tax_rate_id: null },
    { id: 'line-2', sku: 'SKU-002', name: 'Teh Botol', qty: 1, unit_price: { minor_units: 5000, currency: 'IDR' }, total_minor: 5000, tax_amount: null, tax_rate_id: null },
    { id: 'line-3', sku: 'SKU-003', name: 'Nasi Goreng', qty: 1, unit_price: { minor_units: 15000, currency: 'IDR' }, total_minor: 15000, tax_amount: null, tax_rate_id: null },
  ],
};

describe('VoidOrdersScreen', () => {
  it('renders the list view with title', async () => {
    mockListSales.mockResolvedValue([]);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    expect(screen.getByText('Orders')).toBeInTheDocument();
  });

  it('renders loading state initially', async () => {
    mockListSales.mockReturnValue(new Promise(() => {}));
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    expect(screen.getByText(/loading orders/i)).toBeInTheDocument();
  });

  it('renders empty state when no orders', async () => {
    mockListSales.mockResolvedValue([]);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/No orders recorded yet/i)).toBeInTheDocument();
    });
  });

  it('renders orders in the list', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });
    expect(screen.getByText(/ORD-002/)).toBeInTheDocument();
    expect(screen.getByText(/ORD-003/)).toBeInTheDocument();
  });

  it('renders the status filter chips', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText('All')).toBeInTheDocument();
    });
    expect(screen.getAllByText('Active').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Completed').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Voided').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Pending').length).toBeGreaterThanOrEqual(1);
  });

  it('filters orders by status', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });

    await userEvent.click(screen.getAllByText('Voided')[0]!);

    expect(screen.queryByText(/ORD-001/)).not.toBeInTheDocument();
    expect(screen.queryByText(/ORD-002/)).not.toBeInTheDocument();
    expect(screen.getByText(/ORD-003/)).toBeInTheDocument();
  });

  it('opens detail view when View is clicked', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    mockGetSale.mockResolvedValue(sampleDetail);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });

    const viewBtns = screen.getAllByRole('button', { name: /^View|^view/i });
    await userEvent.click(viewBtns[0]!);

    await waitFor(() => {
      expect(screen.getByText(/Indomie Goreng/)).toBeInTheDocument();
    });
    expect(screen.getByText('SKU-001')).toBeInTheDocument();
  });

  it('shows void section for Active orders in detail view', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    mockGetSale.mockResolvedValue(sampleDetail);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });

    const viewBtns = screen.getAllByRole('button', { name: /^view/i });
    await userEvent.click(viewBtns[0]!);

    await waitFor(() => {
      expect(screen.getByText(/Void Order/)).toBeInTheDocument();
    });
  });

  it('renders the void reason select', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    mockGetSale.mockResolvedValue(sampleDetail);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });

    const viewBtns = screen.getAllByRole('button', { name: /^view/i });
    await userEvent.click(viewBtns[0]!);

    await waitFor(() => {
      expect(screen.getByDisplayValue(/Select a reason/i)).toBeInTheDocument();
    });
  });

  it('disables Confirm Void button until a reason is selected', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    mockGetSale.mockResolvedValue(sampleDetail);
    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });

    const viewBtns = screen.getAllByRole('button', { name: /^view/i });
    await userEvent.click(viewBtns[0]!);

    await waitFor(() => {
      const confirmBtn = screen.getByRole('button', { name: /confirm void/i });
      expect(confirmBtn).toBeDisabled();
    });
  });

  it('calls voidSale when Confirm Void is clicked with a reason', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    mockGetSale.mockResolvedValue(sampleDetail);
    mockVoidSale.mockResolvedValue({});
    const user = userEvent.setup();

    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });

    const viewBtns = screen.getAllByRole('button', { name: /^view/i });
    await user.click(viewBtns[0]!);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^cancel$/i })).toBeInTheDocument();
    });

    const reasonSelect = screen.getByDisplayValue(/Select a reason/i);
    await user.selectOptions(reasonSelect, 'cancelled-by-customer');

    const confirmBtn = screen.getByRole('button', { name: /confirm void/i });
    await user.click(confirmBtn);

    await waitFor(() => {
      expect(mockVoidSale).toHaveBeenCalledWith(
        expect.objectContaining({
          saleId: 'ORD-001',
          reason: 'cancelled-by-customer',
        }),
      );
    });
  });

  it('shows error when void fails', async () => {
    mockListSales.mockResolvedValue(sampleSales);
    mockGetSale.mockResolvedValue(sampleDetail);
    mockVoidSale.mockRejectedValue(new Error('Network error'));
    const user = userEvent.setup();

    await renderWithFluent(<VoidOrdersScreen />, salesFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/ORD-001/)).toBeInTheDocument();
    });

    const viewBtns = screen.getAllByRole('button', { name: /^view/i });
    await user.click(viewBtns[0]!);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^cancel$/i })).toBeInTheDocument();
    });

    const reasonSelect = screen.getByDisplayValue(/Select a reason/i);
    await user.selectOptions(reasonSelect, 'cancelled-by-customer');

    const confirmBtn = screen.getByRole('button', { name: /confirm void/i });
    await user.click(confirmBtn);

    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeInTheDocument();
    });
  });
});
