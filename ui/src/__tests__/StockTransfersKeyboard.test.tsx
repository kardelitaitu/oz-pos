// ── StockTransfersScreen keyboard interaction tests ───────────────
//
// Covers: detail/create/receive modals, Escape key closing, X close
// button, Cancel button, modal stacking, form interaction during save.
//
// Uses fireEvent.click for all button clicks (~1ms vs userEvent ~60ms)
// and fireEvent.change for form fields (~1ms vs userEvent.type ~20ms/char).
// Escape key events still use userEvent.keyboard (native listener on
// inner panel ref, not React synthetic event). 8 tests.

import { describe, expect, it, vi } from 'vitest';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import stockTransfersFtl from '@/locales/stock-transfers.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';

vi.mock('@/api/stockTransfers', () => ({
  listStockTransfers: vi.fn(),
  getStockTransfer: vi.fn(),
  sendStockTransfer: vi.fn(),
  cancelStockTransfer: vi.fn(),
  receiveStockTransfer: vi.fn(),
  createStockTransfer: vi.fn(),
}));

vi.mock('@/api/products', () => ({
  listProducts: vi.fn(),
}));

vi.mock('@/api/terminals', () => ({
  listTerminals: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Cashier', role_name: 'cashier' },
  }),
}));

import StockTransfersScreen from '@/features/stock-transfers/StockTransfersScreen';
import { listStockTransfers, getStockTransfer, createStockTransfer } from '@/api/stockTransfers';
import { listProducts } from '@/api/products';
import { listTerminals } from '@/api/terminals';

const mockListTransfers = listStockTransfers as ReturnType<typeof vi.fn>;
const mockGetTransfer = getStockTransfer as ReturnType<typeof vi.fn>;
const mockListProducts = listProducts as ReturnType<typeof vi.fn>;
const mockListTerminals = listTerminals as ReturnType<typeof vi.fn>;

const sampleTransfers = [
  {
    id: 'st-1', transfer_number: 'ST-001', status: 'draft',
    source_location: 'Warehouse A', destination_location: 'Storefront',
    source_terminal_id: null, destination_terminal_id: null,
    notes: '', created_by: 'user-1', received_by: null,
    created_at: '2026-07-01T10:00:00Z', sent_at: null, received_at: null,
    updated_at: '2026-07-01',
  },
  {
    id: 'st-2', transfer_number: 'ST-002', status: 'in_transit',
    source_location: 'Warehouse B', destination_location: 'Store B',
    source_terminal_id: null, destination_terminal_id: null,
    notes: 'Urgent', created_by: 'user-1', received_by: null,
    created_at: '2026-07-02T08:00:00Z', sent_at: '2026-07-02T12:00:00Z', received_at: null,
    updated_at: '2026-07-02',
  },
];

const sampleDetail = {
  transfer: sampleTransfers[0]!,
  lines: [
    { id: 'line-1', transfer_id: 'st-1', sku: 'SKU-001', product_name: 'Widget', qty: 10, received_qty: 0 },
    { id: 'line-2', transfer_id: 'st-1', sku: 'SKU-002', product_name: 'Gadget', qty: 5, received_qty: 0 },
  ],
};

const sampleDetailInTransit = {
  transfer: sampleTransfers[1]!,
  lines: [
    { id: 'line-3', transfer_id: 'st-2', sku: 'SKU-003', product_name: 'Thing', qty: 8, received_qty: 0 },
  ],
};

/* ── Modal helpers (fireEvent.click ~1ms vs userEvent.click ~60ms) ─ */

function setupMocks() {
  mockListProducts.mockResolvedValue([]);
  mockListTerminals.mockResolvedValue([]);
}

async function openDetailModal() {
  setupMocks();
  mockListTransfers.mockResolvedValue(sampleTransfers);
  mockGetTransfer.mockResolvedValue(sampleDetail);
  renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

  await screen.findByText('ST-001');

  // fireEvent.click is safe for <td> with onClick (no label forwarding needed)
  fireEvent.click(screen.getByText('ST-001'));
  // Modal renders sync after fireEvent.click (React 18 auto-batching)
}

async function openCreateModal() {
  setupMocks();
  mockListTransfers.mockResolvedValue(sampleTransfers);
  mockGetTransfer.mockResolvedValue(sampleDetail);
  renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

  await screen.findByText('ST-001');

  fireEvent.click(screen.getByRole('button', { name: /new transfer/i }));
}

async function openReceiveModal() {
  setupMocks();
  mockListTransfers.mockResolvedValue([sampleTransfers[1]!]); // in_transit
  mockGetTransfer.mockResolvedValue(sampleDetailInTransit);
  renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

  await screen.findByText('ST-002');

  fireEvent.click(screen.getByText('ST-002'));

  const receiveBtn = await screen.findByRole('button', { name: /receive transfer/i });
  fireEvent.click(receiveBtn);
}

// ── Form field helper (fireEvent.change ~1ms vs userEvent.type ~20ms/char) ─

function fillField(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

describe('StockTransfersScreen — modal keyboard interaction', () => {
  // ── Detail modal: Escape key ──────────────────────────────────

  it('closes detail modal when Escape is pressed', async () => {
    await openDetailModal();

    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('opens and closes detail modal with X close button', async () => {
    await openDetailModal();

    const dialog = screen.getByRole('dialog');
    const xBtn = dialog.querySelector('.stock-transfers-modal-close') as HTMLElement;
    fireEvent.click(xBtn);

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  // ── Create modal: Escape key ─────────────────────────────────

  it('closes create modal when Escape is pressed', async () => {
    await openCreateModal();

    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('opens and closes create modal with X close button', async () => {
    await openCreateModal();

    const dialog = screen.getByRole('dialog');
    const xBtn = dialog.querySelector('.stock-transfers-modal-close') as HTMLElement;
    fireEvent.click(xBtn);

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  // ── Receive modal: Escape key ────────────────────────────────

  it('closes receive modal when Escape is pressed', async () => {
    await openReceiveModal();

    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByText(/enter the quantity/i)).not.toBeInTheDocument();
    });
  });

  it('closes receive modal with Cancel button', async () => {
    await openReceiveModal();

    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));

    await waitFor(() => {
      expect(screen.queryByText(/enter the quantity/i)).not.toBeInTheDocument();
    });
  });

  // ── Create modal: form interaction ──────────────────────────

  it('blocks Escape when creating a transfer', async () => {
    vi.mocked(createStockTransfer).mockReturnValue(new Promise(() => {}));
    await openCreateModal();

    // Fill in required fields with fireEvent.change (saves ~20ms/char)
    fillField('Source', 'Warehouse A');
    fillField('Destination', 'Store B');

    // Add a line item
    fireEvent.click(screen.getByRole('button', { name: /add line/i }));

    fillField('SKU', 'SKU-001');

    const qtyInput = screen.getByLabelText('Qty');
    fireEvent.change(qtyInput, { target: { value: '5' } });

    // Click Create Transfer (modal enters saving state)
    fireEvent.click(screen.getByRole('button', { name: /create transfer/i }));

    // Escape should NOT close the modal during save
    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    // Modal should still be open
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  // ── Multiple modals: correct one closes ────────────────────

  it('closes only the top-most modal when Escape is pressed', async () => {
    setupMocks();
    mockListTransfers.mockResolvedValue([sampleTransfers[1]!]); // in_transit
    mockGetTransfer.mockResolvedValue(sampleDetailInTransit);
    renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

    await screen.findByText('ST-002');
    fireEvent.click(screen.getByText('ST-002'));

    const receiveBtn = await screen.findByRole('button', { name: /receive transfer/i });
    fireEvent.click(receiveBtn);
    await screen.findByText(/enter the quantity/i);

    // Escape should close receive modal (topmost)
    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByText(/enter the quantity/i)).not.toBeInTheDocument();
    });

    // Detail modal should still be open
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });
});
