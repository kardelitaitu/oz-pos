import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import salesFtl from '@/locales/sales.ftl?raw';

// Mock the sales API.
vi.mock('@/api/sales', () => ({
  processRefund: vi.fn(),
}));

// Mock AuthContext.
vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Cashier', role_name: 'cashier' },
  }),
}));

import RefundModal from '@/features/sales/RefundModal';
import { processRefund } from '@/api/sales';

const mockProcessRefund = processRefund as ReturnType<typeof vi.fn>;

const refundFtl = `
refund-title = Process Refund
refund-close-aria = Cancel refund
    .aria-label = Cancel refund
refund-sale-id = Sale: { $id }…
refund-sale-total = Total: { $amount }
refund-sale-date = Date: { $date }
refund-items-title = Select Items to Refund
refund-reason-label = Reason *
refund-reason-placeholder = Enter reason…
refund-reason-aria = Refund reason
refund-note-label = Note (internal)
refund-note-placeholder = Internal notes
refund-note-aria = Refund note
refund-total-label = Refund Total
refund-cancel = Cancel
refund-submit = Process Refund
refund-done-title = Refund Processed
refund-done = Done
refund-error = Refund failed
refund-item-aria = Refund { $sku }
    .aria-label = Refund { $sku }
refund-qty-decrease-aria = Decrease refund quantity
    .aria-label = Decrease refund quantity
refund-qty-increase-aria = Increase refund quantity
    .aria-label = Increase refund quantity
refund-dialog-aria = Process refund
    .aria-label = Process refund
refund-done-amount = Refunded: { $amount }
`;

const wrap = (children: React.ReactNode) => withFluent(children, salesFtl, refundFtl);

const mockSale = {
  id: 'sale-abc123456789',
  total: { minor_units: 35000, currency: 'IDR' },
  subtotal: { minor_units: 35000, currency: 'IDR' },
  taxTotal: { minor_units: 0, currency: 'IDR' },
  lineCount: 2,
  status: 'completed',
  paymentMethod: 'CASH',
  tenderedMinor: 50000,
  userId: 'user-1',
  createdAt: '2026-07-05T10:00:00.000Z',
  lines: [
    { id: 'line-1', sku: 'SKU-001', name: 'Indomie Goreng', qty: 2, unit_price: { minor_units: 3500, currency: 'IDR' }, total_minor: 7000, tax_amount: null, tax_rate_id: null },
    { id: 'line-2', sku: 'SKU-002', name: 'Teh Botol', qty: 1, unit_price: { minor_units: 5000, currency: 'IDR' }, total_minor: 5000, tax_amount: null, tax_rate_id: null },
  ],
};

const defaultProps = {
  open: true,
  sale: mockSale,
  onClose: vi.fn(),
  onRefunded: vi.fn(),
};

describe('RefundModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders null when not open', () => {
    const { container } = render(wrap(<RefundModal {...defaultProps} open={false} />));
    expect(container.innerHTML).toBe('');
  });

  it('renders the refund form title and sale info', () => {
    render(wrap(<RefundModal {...defaultProps} />));
    // The heading "Process Refund" appears in multiple elements (dialog aria-label + h2).
    const headings = screen.getAllByText('Process Refund');
    expect(headings.length).toBeGreaterThanOrEqual(1);
    // Fluent bidi markers wrap variables, so use a function matcher.
    expect(screen.getByText((content: string) => content.includes('Sale:') && content.includes('sale-abc'))).toBeInTheDocument();
    expect(screen.getByText('Select Items to Refund')).toBeInTheDocument();
  });

  it('shows sale line items with SKU and name', () => {
    render(wrap(<RefundModal {...defaultProps} />));
    expect(screen.getByText('SKU-001')).toBeInTheDocument();
    expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    expect(screen.getByText('SKU-002')).toBeInTheDocument();
    expect(screen.getByText('Teh Botol')).toBeInTheDocument();
  });

  it('renders refund details fields', () => {
    render(wrap(<RefundModal {...defaultProps} />));
    expect(screen.getByText('Reason *')).toBeInTheDocument();
    expect(screen.getByText('Note (internal)')).toBeInTheDocument();
  });

  it('shows Process Refund button disabled when no items selected', () => {
    render(wrap(<RefundModal {...defaultProps} />));
    const btns = screen.getAllByRole('button', { name: /process refund/i });
    expect(btns.length).toBeGreaterThanOrEqual(1);
    expect(btns[0]).toBeDisabled();
  });

  it('selects a line item when its checkbox is clicked', async () => {
    render(wrap(<RefundModal {...defaultProps} />));
    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes).toHaveLength(2);

    await userEvent.click(checkboxes[0]!);
    // Should be selected.
    expect(checkboxes[0]).toBeChecked();
  });

  it('shows qty controls when a line is selected', async () => {
    render(wrap(<RefundModal {...defaultProps} />));
    const checkbox = screen.getAllByRole('checkbox')[0]!;
    await userEvent.click(checkbox);

    // Qty controls should appear: decrement, value, increment.
    expect(screen.getByText('2')).toBeInTheDocument(); // maxQty displayed
    expect(screen.getByRole('button', { name: /decrease refund quantity/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /increase refund quantity/i })).toBeInTheDocument();
  });

  it('decrements selected qty when decrease button clicked', async () => {
    render(wrap(<RefundModal {...defaultProps} />));
    const checkbox = screen.getAllByRole('checkbox')[0]!;
    await userEvent.click(checkbox);

    // Initial qty = maxQty = 2
    expect(screen.getByText('2')).toBeInTheDocument();

    const decBtn = screen.getByRole('button', { name: /decrease refund quantity/i });
    await userEvent.click(decBtn);
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', async () => {
    const onClose = vi.fn();
    render(wrap(<RefundModal {...defaultProps} onClose={onClose} />));
    const closeBtns = screen.getAllByRole('button', { name: /cancel refund/i });
    await userEvent.click(closeBtns[0]!);
    await vi.waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it('calls processRefund with correct args on submit', async () => {
    mockProcessRefund.mockResolvedValueOnce({ refundId: 'ref-1', totalMinor: 7000 });
    render(wrap(<RefundModal {...defaultProps} />));

    // Select first line item.
    await userEvent.click(screen.getAllByRole('checkbox')[0]!);

    // Enter reason into the reason input (first text input).
    const reasonInput = screen.getAllByRole('textbox')[0]!;
    await userEvent.type(reasonInput, 'Customer returned');

    // Process.
    const submitBtns = screen.getAllByRole('button', { name: /process refund/i });
    await userEvent.click(submitBtns[submitBtns.length - 1]!);

    expect(mockProcessRefund).toHaveBeenCalledTimes(1);
    expect(mockProcessRefund).toHaveBeenCalledWith(
      expect.objectContaining({
        saleId: 'sale-abc123456789',
        reason: 'Customer returned',
        userId: 'user-1',
        lines: expect.arrayContaining([
          expect.objectContaining({ sku: 'SKU-001', qty: 2 }),
        ]),
      }),
    );
  });

  it('shows success state after refund', async () => {
    mockProcessRefund.mockResolvedValueOnce({ refundId: 'ref-done', totalMinor: 12000 });
    render(wrap(<RefundModal {...defaultProps} />));

    await userEvent.click(screen.getAllByRole('checkbox')[0]!);
    await userEvent.type(screen.getAllByRole('textbox')[0]!, 'Defective');
    const submitBtns = screen.getAllByRole('button', { name: /process refund/i });
    await userEvent.click(submitBtns[submitBtns.length - 1]!);

    await vi.waitFor(() => {
      expect(screen.getByText('Refund Processed')).toBeInTheDocument();
    });
  });

  it('shows error when processRefund fails', async () => {
    mockProcessRefund.mockRejectedValueOnce(new Error('Network down'));
    render(wrap(<RefundModal {...defaultProps} />));

    await userEvent.click(screen.getAllByRole('checkbox')[0]!);
    await userEvent.type(screen.getAllByRole('textbox')[0]!, 'Broken');
    const submitBtns = screen.getAllByRole('button', { name: /process refund/i });
    await userEvent.click(submitBtns[submitBtns.length - 1]!);

    await vi.waitFor(() => {
      expect(screen.getByText('Network down')).toBeInTheDocument();
    });
  });

  it('calls onRefunded and onClose when Done is clicked after success', async () => {
    mockProcessRefund.mockResolvedValueOnce({ refundId: 'ref-ok', totalMinor: 0 });
    const onRefunded = vi.fn();
    const onClose = vi.fn();

    render(wrap(<RefundModal {...defaultProps} onRefunded={onRefunded} onClose={onClose} />));

    await userEvent.click(screen.getAllByRole('checkbox')[0]!);
    await userEvent.type(screen.getAllByRole('textbox')[0]!, 'Return');
    const submitBtns = screen.getAllByRole('button', { name: /process refund/i });
    await userEvent.click(submitBtns[submitBtns.length - 1]!);

    await vi.waitFor(() => {
      expect(screen.getByText('Refund Processed')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /done/i }));
    expect(onRefunded).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it('shows refund total row', () => {
    render(wrap(<RefundModal {...defaultProps} />));
    expect(screen.getByText('Refund Total')).toBeInTheDocument();
  });

  it('deselects a line item when clicked again', async () => {
    render(wrap(<RefundModal {...defaultProps} />));
    const checkbox = screen.getAllByRole('checkbox')[0]!;
    await userEvent.click(checkbox);
    expect(checkbox).toBeChecked();
    await userEvent.click(checkbox);
    expect(checkbox).not.toBeChecked();
  });
});
