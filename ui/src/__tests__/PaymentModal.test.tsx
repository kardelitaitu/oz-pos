import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import PaymentModal from '@/features/sales/PaymentModal';
import type { Money, CartLine, Sku, LineId } from '@/types/domain';

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
  invokeMock: vi.fn().mockResolvedValue({ printed: true }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
  invokeMock.mockResolvedValue({ printed: true });
});

describe('PaymentModal', () => {
  it('renders total due and payment method options when open', () => {
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open={false}
        lineItems={[lineItem()]}
        total={usd(700)}
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('shows change preview for cash payment', async () => {
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
    render(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
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
