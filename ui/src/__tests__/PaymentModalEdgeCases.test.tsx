import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import { act } from 'react';
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

const { invokeMock, defaultImpl } = vi.hoisted(() => {
  // defaultImpl is a PLAIN function (not vi.fn()) used only for restoring
  // the mock implementation in beforeEach/afterEach.
  const impl = (cmd: string) => {
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
  const mock = vi.fn(impl);
  return { invokeMock: mock, defaultImpl: impl };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@/api/customers', () => ({
  listCustomers: vi.fn(),
}));

import { listCustomers } from '@/api/customers';
const mockListCustomers = listCustomers as ReturnType<typeof vi.fn>;

beforeEach(() => {
  invokeMock.mockReset(); // reset calls AND implementation
  invokeMock.mockImplementation(defaultImpl);
  mockListCustomers.mockResolvedValue([
    { id: 'cust-1', name: 'John Doe', phone: '555-0100', email: 'john@example.com' },
    { id: 'cust-2', name: 'Jane Smith', phone: '555-0200', email: 'jane@example.com' },
  ]);
});

afterEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation(defaultImpl);
});

function setProcessingMock() {
  // Make complete_sale never resolve so processing stays true
  invokeMock.mockImplementation((cmd: string): any => {
    if (cmd === 'complete_sale') return new Promise(() => {});
    if (cmd === 'start_sale') return Promise.resolve({ cartId: 'test-cart' });
    if (cmd === 'add_line') return Promise.resolve({ lineId: 'test-line', lineTotal: null });
    if (cmd === 'print_sales_receipt') return Promise.resolve({ printed: true });
    if (cmd === 'get_enabled_features') return Promise.resolve({ features: [] });
    return Promise.resolve({});
  });
}

function setErrorMock() {
  invokeMock.mockImplementation((cmd: string): any => {
    if (cmd === 'complete_sale') return Promise.reject(new Error('Payment gateway timeout'));
    if (cmd === 'start_sale') return Promise.resolve({ cartId: 'test-cart' });
    if (cmd === 'add_line') return Promise.resolve({ lineId: 'test-line', lineTotal: null });
    if (cmd === 'print_sales_receipt') return Promise.resolve({ printed: true });
    if (cmd === 'get_enabled_features') return Promise.resolve({ features: [] });
    return Promise.resolve({});
  });
}

describe('PaymentModal — edge cases', () => {
  // ── Keyboard interaction ──────────────────────────────────────

  it('closes modal when Escape is pressed', async () => {
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

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    }, { timeout: 2000 });
  });

  it('does not close modal via Escape while processing payment', async () => {
    setProcessingMock();

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

    const tenderInput = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(tenderInput, '10');
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    // Complete button should be disabled (loading state)
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^complete$/i })).toBeDisabled();
    });

    // Escape should NOT close while processing
    await userEvent.keyboard('{Escape}');
    expect(onClose).not.toHaveBeenCalled();
  });

  it('disables Cancel button while processing', async () => {
    setProcessingMock();

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

    const tenderInput = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(tenderInput, '10');
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    await waitFor(() => {
      // The footer Cancel button has disabled={processing} — find it by text content
      const cancelBtn = screen.getByRole('button', { name: /^cancel$/i });
      expect(cancelBtn).toBeDisabled();
    });
  });

  // ── Error handling ────────────────────────────────────────────

  it('handles complete sale failure gracefully (processing resets)', async () => {
    setErrorMock();

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

    const tenderInput = screen.getByLabelText(/amount tendered/i);
    await userEvent.type(tenderInput, '10');
    await userEvent.click(screen.getByRole('button', { name: /^complete$/i }));

    // Wait for processing to end (the error is caught, processing set to false)
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^complete$/i })).not.toBeDisabled();
    });

    // Modal should still be open — no done state
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.queryByText(/sale complete/i)).not.toBeInTheDocument();
  });

  // ── Customer search modal ────────────────────────────────────

  it('opens customer search when Select Customer is clicked', async () => {
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

    await userEvent.click(screen.getByText(/select customer/i));

    // The search modal has a unique placeholder on its search input
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/search by name/i)).toBeInTheDocument();
    });
  });

  it('closes customer search when Escape is pressed', async () => {
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

    await userEvent.click(screen.getByText(/select customer/i));
    await screen.findByPlaceholderText(/search by name/i);

    // Focus the search input so keyboard events target the search modal
    const searchInput = screen.getByPlaceholderText(/search by name/i);
    searchInput.focus();
    await userEvent.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/search by name/i)).not.toBeInTheDocument();
    });
  });

  it('closes customer search when Cancel is clicked', async () => {
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

    await userEvent.click(screen.getByText(/select customer/i));
    await screen.findByPlaceholderText(/search by name/i);

    // Scope to the search modal to avoid matching the main modal's Cancel button
    const searchModal = document.querySelector('.payment-customer-search-modal')!;
    const cancelBtn = within(searchModal as HTMLElement).getByRole('button', { name: /^cancel$/i });
    await userEvent.click(cancelBtn);

    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/search by name/i)).not.toBeInTheDocument();
    });
  });

  it('selects a customer from search and shows badge', async () => {
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

    // Open customer search and wait for customer list to load
    await userEvent.click(screen.getByText(/select customer/i));
    await screen.findByPlaceholderText(/search by name/i);

    // Find and click a customer result in the search modal
    const searchModal = document.querySelector('.payment-customer-search-modal')!;
    const customerBtn = (await waitFor(() => {
      const btn = searchModal.querySelector('button.payment-customer-search-item');
      expect(btn).toBeInTheDocument();
      return btn;
    })) as HTMLElement;

    // Use fireEvent.click + act wrapper to ensure React processes the synthetic event
    act(() => { customerBtn.click(); });

    // Customer badge should appear with Change button
    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/search by name/i)).not.toBeInTheDocument();
      // Check badge by class — verifies selectedCustomer state updated correctly
      expect(document.querySelector('.payment-customer-badge')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /change/i })).toBeInTheDocument();
    });
  });
});
