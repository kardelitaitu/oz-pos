import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import salesFtl from '@/locales/sales.ftl?raw';
import PaymentModal from '@/features/sales/PaymentModal';
import type { Money, CartLine, Sku, LineId } from '@/types/domain';

function renderWithFluent(ui: React.ReactElement) {
  return render(withFluent(ui, salesFtl));
}

const usd = (minor: number): Money => ({ minor_units: minor, currency: 'USD' });

const lineItem = (overrides: Partial<CartLine> = {}): CartLine => ({
  id: 'line-1' as LineId,
  sku: 'COFFEE' as Sku,
  name: 'Coffee',
  qty: 2,
  unit_price: usd(350),
  ...overrides,
});

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn((cmd: string) => {
    switch (cmd) {
      case 'start_sale':
        return Promise.resolve({ cartId: 'test-cart' });
      case 'add_line':
        return Promise.resolve({ lineId: 'test-line', lineTotal: null });
      case 'complete_sale':
        return Promise.resolve({ saleId: 'sale-1', total: null, lineCount: 1 });
      case 'get_sale':
        return Promise.resolve(null);
      case 'print_sales_receipt':
        return Promise.resolve({ printed: true });
      case 'get_enabled_features':
        return Promise.resolve({ features: [] });
      default:
        return Promise.resolve({});
    }
  }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
});

describe('PaymentModal', () => {
  it('renders total due and payment method options when open', () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText(/Total Due/)).toBeInTheDocument();
    expect(screen.getByText('$7.00')).toBeInTheDocument();
    expect(screen.getByLabelText(/Cash/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Card/)).toBeInTheDocument();
  });

  it('does not render when closed', () => {
    renderWithFluent(
      <PaymentModal
        open={false}
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('shows change preview for cash payment', async () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const input = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(input, '10');

    await waitFor(() => {
      expect(screen.getByText('$3.00')).toBeInTheDocument();
    });
  });

  it('shows insufficient amount warning when tendered < total', async () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const input = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(input, '5');

    await waitFor(() => {
      expect(screen.getByText(/insufficient/i)).toBeInTheDocument();
    });
  });

  it('disables Complete Sale when tendered < total', async () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const input = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(input, '5');

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /complete sale/i })).toBeDisabled();
    });
  });

  it('enables Complete Sale when tendered >= total', async () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const input = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(input, '10');

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /complete sale/i })).not.toBeDisabled();
    });
  });

  it('calls printSalesReceipt on complete', async () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const input = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(input, '10');
    await userEvent.click(screen.getByRole('button', { name: /complete sale/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('print_sales_receipt', expect.any(Object));
    });
  });

  it('calls onComplete after sale done', async () => {
    const onComplete = vi.fn();
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={onComplete}
        onClose={vi.fn()}
      />,
    );

    const input = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(input, '10');
    await userEvent.click(screen.getByRole('button', { name: /complete sale/i }));

    // Wait for the done state to render, then the auto-close timer fires.
    await screen.findByText(/sale complete/i);
    await screen.findByText(/change due/i);

    await waitFor(() => {
      expect(onComplete).toHaveBeenCalled();
    }, { timeout: 5000 });
  });

  it('shows change due in done state for cash', async () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const input = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(input, '10');
    await userEvent.click(screen.getByRole('button', { name: /complete sale/i }));

    expect(await screen.findByText(/sale complete/i)).toBeInTheDocument();
    expect(await screen.findByText(/change due/i)).toBeInTheDocument();
    expect(await screen.findByText('$3.00')).toBeInTheDocument();
  });

  it('shows sale complete state for card and prints receipt', async () => {
    renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByLabelText(/Card/));
    expect(screen.getByRole('button', { name: /complete sale/i })).not.toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: /complete sale/i }));

    // The done state should appear after printSalesReceipt resolves.
    expect(await screen.findByText(/sale complete/i)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('print_sales_receipt', expect.any(Object));
  });
});
