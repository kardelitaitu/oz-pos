import { describe, expect, it, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
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

/* ── Helper: setup + open modal ──────────────────────────────── */

function setupMocks() {
  mockListProducts.mockResolvedValue([]);
  mockListTerminals.mockResolvedValue([]);
}

async function openDetailModal(user: ReturnType<typeof userEvent.setup>) {
  setupMocks();
  mockListTransfers.mockResolvedValue(sampleTransfers);
  mockGetTransfer.mockResolvedValue(sampleDetail);
  renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

  await waitFor(() => expect(screen.getByText('ST-001')).toBeInTheDocument());

  // Click transfer number to open detail
  await user.click(screen.getByText('ST-001'));

  await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
}

async function openCreateModal(user: ReturnType<typeof userEvent.setup>) {
  setupMocks();
  mockListTransfers.mockResolvedValue(sampleTransfers);
  mockGetTransfer.mockResolvedValue(sampleDetail);
  renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

  await waitFor(() => expect(screen.getByText('ST-001')).toBeInTheDocument());

  await user.click(screen.getByRole('button', { name: /new transfer/i }));
  await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
}

async function openReceiveModal(user: ReturnType<typeof userEvent.setup>) {
  setupMocks();
  mockListTransfers.mockResolvedValue([sampleTransfers[1]!]); // in_transit
  mockGetTransfer.mockResolvedValue(sampleDetailInTransit);
  renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

  await waitFor(() => expect(screen.getByText('ST-002')).toBeInTheDocument());

  // Open detail for in_transit transfer
  await user.click(screen.getByText('ST-002'));
  await waitFor(() => expect(screen.getByRole('button', { name: /receive transfer/i })).toBeInTheDocument());

  // Open receive modal
  await user.click(screen.getByRole('button', { name: /receive transfer/i }));
  await waitFor(() => expect(screen.getByText(/enter the quantity/i)).toBeInTheDocument());
}

describe('StockTransfersScreen — modal keyboard interaction', () => {
  // ── Detail modal: Escape key ──────────────────────────────────

  it('closes detail modal when Escape is pressed', async () => {
    const user = userEvent.setup();
    await openDetailModal(user);

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('opens and closes detail modal with X close button', async () => {
    const user = userEvent.setup();
    await openDetailModal(user);

    const dialog = screen.getByRole('dialog');
    const xBtn = dialog.querySelector('.stock-transfers-modal-close') as HTMLElement;
    await user.click(xBtn);

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  // ── Create modal: Escape key ─────────────────────────────────

  it('closes create modal when Escape is pressed', async () => {
    const user = userEvent.setup();
    await openCreateModal(user);

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('opens and closes create modal with X close button', async () => {
    const user = userEvent.setup();
    await openCreateModal(user);

    const dialog = screen.getByRole('dialog');
    const xBtn = dialog.querySelector('.stock-transfers-modal-close') as HTMLElement;
    await user.click(xBtn);

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  // ── Receive modal: Escape key ────────────────────────────────

  it('closes receive modal when Escape is pressed', async () => {
    const user = userEvent.setup();
    await openReceiveModal(user);

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByText(/enter the quantity/i)).not.toBeInTheDocument();
    });
  });

  it('closes receive modal with Cancel button', async () => {
    const user = userEvent.setup();
    await openReceiveModal(user);

    await user.click(screen.getByRole('button', { name: /cancel/i }));

    await waitFor(() => {
      expect(screen.queryByText(/enter the quantity/i)).not.toBeInTheDocument();
    });
  });

  // ── Create modal: form interaction ──────────────────────────

  it('blocks Escape when creating a transfer', async () => {
    const user = userEvent.setup();
    vi.mocked(createStockTransfer).mockReturnValue(new Promise(() => {}));
    await openCreateModal(user);

    // Fill in required fields
    const sourceInput = screen.getByLabelText('Source');
    await user.type(sourceInput, 'Warehouse A');

    const destInput = screen.getByLabelText('Destination');
    await user.type(destInput, 'Store B');

    // Add a line item first (createLines starts empty)
    await user.click(screen.getByRole('button', { name: /add line/i }));

    const skuInputs = screen.getAllByLabelText('SKU');
    await user.type(skuInputs[0]!, 'SKU-001');

    const qtyInput = screen.getByLabelText('Qty');
    await user.type(qtyInput, '5');

    // Click Create Transfer (modal enters saving state)
    await user.click(screen.getByRole('button', { name: /create transfer/i }));

    // Escape should NOT close the modal during save
    await user.keyboard('{Escape}');

    // Modal should still be open
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  // ── Multiple modals: correct one closes ────────────────────

  it('closes only the top-most modal when Escape is pressed', async () => {
    const user = userEvent.setup();
    setupMocks();
    mockListTransfers.mockResolvedValue([sampleTransfers[1]!]); // in_transit
    mockGetTransfer.mockResolvedValue(sampleDetailInTransit);
    renderWithFluentSync(<StockTransfersScreen />, stockTransfersFtl, sharedFtl);

    await waitFor(() => expect(screen.getByText('ST-002')).toBeInTheDocument());

    // Open detail modal
    await user.click(screen.getByText('ST-002'));
    await screen.findByRole('dialog');

    // Open receive modal on top of detail
    await user.click(screen.getByRole('button', { name: /receive transfer/i }));
    await waitFor(() => expect(screen.getByText(/enter the quantity/i)).toBeInTheDocument());

    // Escape should close receive modal (topmost)
    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByText(/enter the quantity/i)).not.toBeInTheDocument();
    });

    // Detail modal should still be open
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });
});
