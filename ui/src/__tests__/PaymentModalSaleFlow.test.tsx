// ── PaymentModal sale flow tests ───────────────────────────────────
//
// Covers: full sale completion flow (start_sale → add_line →
// complete_sale → get_sale → print_sales_receipt). These tests are
// the heaviest in PaymentModal (~2-3s each) due to IPC round-trips.
// Extracted to enable parallel execution with fast rendering tests.
// 4 tests.

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
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
  invokeMock: vi.fn((cmd: string): Promise<unknown> => {
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

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    activeWorkspace: null,
    sessionToken: 'mock-token',
    swapSessionToken: vi.fn(),
    workspaces: [],
    loading: false,
  }),
  WorkspaceProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
});

describe('PaymentModal — sale flow', () => {
  it('calls printSalesReceipt on complete', async () => {
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
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    const printBtn = await screen.findByRole('button', { name: /Print Receipt/i });
    await userEvent.click(printBtn);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('print_sales_receipt', expect.any(Object));
    });
  });

  it('calls onComplete after sale done', async () => {
    const onComplete = vi.fn();
    await renderWithFluent(
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
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    const printBtn = await screen.findByRole('button', { name: /Print Receipt/i });
    await userEvent.click(printBtn);

    await waitFor(() => {
      expect(onComplete).toHaveBeenCalled();
    }, { timeout: 5000 });
  });

  it('shows change due in done state for cash', async () => {
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
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    expect(await screen.findByRole('region', { name: /Receipt Preview/i })).toBeInTheDocument();
    expect(await screen.findByText(/CHANGE:/i)).toBeInTheDocument();
    expect(await screen.findByText('$ 3,00')).toBeInTheDocument();
  });

  it('shows sale complete state for card and prints receipt', async () => {
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

    await userEvent.click(screen.getByLabelText(/Card/));
    expect(screen.getByRole('button', { name: /^complete$/i })).not.toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    const printBtn = await screen.findByRole('button', { name: /Print Receipt/i });
    await userEvent.click(printBtn);

    expect(invokeMock).toHaveBeenCalledWith('print_sales_receipt', expect.any(Object));
  });
});

// ── Shortfall resolution integration ────────────────────────────────

const defaultInvokeImpl = (cmd: string) => {
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
};

describe('PaymentModal — shortfall resolution', () => {
  afterEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(defaultInvokeImpl as (cmd: string) => Promise<unknown>);
  });

  it('shows StockShortfallDialog when completeSale fails with PartialStockResult', async () => {
    const shortfallPayload = {
      requiresResolution: true,
      shortfalls: [
        {
          sku: 'COFFEE',
          productName: 'Coffee',
          requestedQty: 5,
          primaryQtyAvailable: 2,
          deficit: 3,
          primaryLocationId: 'main',
          alternatives: [
            { locationId: 'alt-1', locationName: 'Warehouse', qtyAvailable: 10 },
          ],
        },
      ],
    };

    invokeMock.mockImplementation((cmd: string): Promise<unknown> => {
      if (cmd === 'complete_sale') {
        return Promise.reject(
          new Error(JSON.stringify(shortfallPayload)),
        );
      }
      return defaultInvokeImpl(cmd) as Promise<unknown>;
    });

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

    await userEvent.click(screen.getByLabelText(/Card/));
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    await waitFor(() => {
      expect(screen.getByText('Insufficient Stock')).toBeInTheDocument();
    });
    expect(screen.getByText('#COFFEE')).toBeInTheDocument();
    expect(screen.getByText('Coffee')).toBeInTheDocument();
    expect(screen.getByText('Confirm & Continue')).toBeInTheDocument();
    expect(screen.getByText('Cancel Sale')).toBeInTheDocument();
  });
});
