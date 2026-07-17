import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderInAct } from '@/test-utils/renderInAct';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import { ToastProvider } from '@/frontend/shared/Toast';
import salesFtl from '@/locales/sales.ftl?raw';
import PaymentModal from '@/features/sales/PaymentModal';
import type { Money, CartLine, Sku, LineId } from '@/types/domain';

async function renderWithFluent(ui: React.ReactElement) {
  const wrapped = withFluent(<ToastProvider>{ui}</ToastProvider>, salesFtl);
  await renderInAct(wrapped);
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
      case 'hold_cart':
        return Promise.resolve();
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

describe('PaymentModal — rendering & fast interaction', () => {
  it('renders total due and payment method options when open', async () => {
    await renderWithFluent(
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
    expect(screen.getByText('$ 7,00')).toBeInTheDocument();
    expect(screen.getByLabelText(/Cash/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Card/)).toBeInTheDocument();
  });

  it('does not render when closed', async () => {
    await renderWithFluent(
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
    await renderWithFluent(
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
      expect(screen.getByText('$ 3,00')).toBeInTheDocument();
    });
  });

  it('shows insufficient amount warning when tendered < total', async () => {
    await renderWithFluent(
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
    await renderWithFluent(
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
      expect(screen.getByRole('button', { name: /^complete$/i })).toBeDisabled();
    });
  });

  it('enables Complete Sale when tendered >= total', async () => {
    await renderWithFluent(
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
      expect(screen.getByRole('button', { name: /^complete$/i })).not.toBeDisabled();
    });
  });

  // ── Split payment mode ──

  it('shows split payment UI when toggle is checked', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('checkbox'));

    expect(screen.getByText(/Split Payments/)).toBeInTheDocument();
    expect(screen.getByText(/Split Evenly/)).toBeInTheDocument();
    expect(screen.getByText(/\+ Add Split/)).toBeInTheDocument();
    expect(screen.getByText(/Remaining/)).toBeInTheDocument();
  });

  it('hides split UI when toggle is unchecked', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const toggle = screen.getByRole('checkbox');
    await userEvent.click(toggle);
    await userEvent.click(toggle);

    expect(screen.queryByText(/Split Payments/)).not.toBeInTheDocument();
    expect(screen.getByLabelText(/Cash/)).toBeInTheDocument();
  });

  it('adds a new split row', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('checkbox'));
    expect(screen.getAllByRole('radio', { name: 'Cash' })).toHaveLength(2);

    await userEvent.click(screen.getByText(/\+ Add Split/));
    expect(screen.getAllByRole('radio', { name: 'Cash' })).toHaveLength(3);
  });

  it('removes a split row when remove is clicked', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('checkbox'));
    const removeBtns = screen.getAllByRole('button', { name: /remove split/i });
    expect(removeBtns).toHaveLength(2);

    await userEvent.click(removeBtns[0]!);
    expect(screen.getAllByRole('radio', { name: 'Cash' })).toHaveLength(1);
  });

  it('split evenly distributes total across rows', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('checkbox'));
    await userEvent.click(screen.getByText(/Split Evenly/));

    const splitInputs = screen.getAllByPlaceholderText('0.00') as unknown as HTMLInputElement[];
    expect(splitInputs).toHaveLength(2);
    expect(splitInputs[0]!.value).toBe('3.50');
    expect(splitInputs[1]!.value).toBe('3.50');
  });

  it('shows remaining amount when splits do not cover total', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('checkbox'));
    const splitInputs = screen.getAllByPlaceholderText('0.00') as unknown as HTMLInputElement[];
    await userEvent.type(splitInputs[0]!, '2');

    await waitFor(() => {
      expect(screen.getByText('$ 5,00')).toBeInTheDocument();
    });
  });

  it('enables complete when split amounts sum to total', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('checkbox'));
    const splitInputs = screen.getAllByPlaceholderText('0.00') as unknown as HTMLInputElement[];
    await userEvent.type(splitInputs[0]!, '3.50');
    await userEvent.type(splitInputs[1]!, '3.50');

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^complete$/i })).not.toBeDisabled();
    });
  });

  // ── Other payment method ──

  it('disables complete when other method has no label', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const otherRadio = document.querySelector<HTMLInputElement>('input[type="radio"][value="other"]')!;
    await userEvent.click(otherRadio);

    expect(screen.getByRole('button', { name: /^complete$/i })).toBeDisabled();
  });

  it('enables complete when other method has a label', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const otherRadio = document.querySelector<HTMLInputElement>('input[type="radio"][value="other"]')!;
    await userEvent.click(otherRadio);
    const otherInput = screen.getByPlaceholderText(/^Other/);
    await userEvent.type(otherInput, 'Voucher');

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^complete$/i })).not.toBeDisabled();
    });
  });

  // ── Open Bill ──

  it('shows customer name input for open bill', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const openBillRadio = screen.getByLabelText(/Open Bill/);
    await userEvent.click(openBillRadio);

    expect(screen.getByLabelText(/customer name/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /open bill/i })).toBeInTheDocument();
  });

  it('disables Open Bill complete when customer name is empty', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByLabelText(/Open Bill/));

    expect(screen.getByRole('button', { name: /open bill/i })).toBeDisabled();
  });

  // ── Credit ──

  it('shows customer name input and Credit Sale button for credit', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const creditRadio = screen.getByLabelText(/Credit/);
    await userEvent.click(creditRadio);

    expect(screen.getByLabelText(/customer name/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /credit sale/i })).toBeInTheDocument();
  });

  // ── QRIS ──

  it('shows Pay with QR button for QRIS method', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByLabelText(/QRIS/));

    expect(screen.getByRole('button', { name: /pay with qr/i })).toBeInTheDocument();
  });

  // ── Close button ──

  it('calls onClose after close button click and animation', async () => {
    const onClose = vi.fn();
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={onClose}
      />,
    );

    const closeBtn = screen.getByRole('button', { name: /cancel payment/i });
    await userEvent.click(closeBtn);

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    }, { timeout: 2000 });
  });

  // ── Quick tender presets ──

  it('clicking a quick tender preset sets the tendered amount', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const presetBtn = screen.getByText(/USD 10\.000/);
    await userEvent.click(presetBtn);

    const tenderInput = screen.getByLabelText(/amount tendered/i) as unknown as HTMLInputElement;
    expect(tenderInput.value).toBe('10000.00');
  });

  it('clicking exact tender sets the exact total amount', async () => {
    await renderWithFluent(
      <PaymentModal
        open
        lineItems={[lineItem()]}
        total={usd(700)}
        userId="test-user-id"
        onComplete={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    const exactBtn = screen.getByRole('button', { name: /tend exact amount/i });
    await userEvent.click(exactBtn);

    const tenderInput = screen.getByLabelText(/amount tendered/i) as unknown as HTMLInputElement;
    expect(tenderInput.value).toBe('7.00');
  });
});
