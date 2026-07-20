import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import inventoryFtl from '@/locales/inventory.ftl?raw';

// ── Mocks ─────────────────────────────────────────────────────────

vi.mock('@/api/stockTransfers', () => ({
  listStockTransfers: vi.fn(),
  getStockTransferLines: vi.fn(),
  cancelStockTransfer: vi.fn(),
}));

import TransitAuditScreen from '@/features/inventory/TransitAuditScreen';
import { listStockTransfers, getStockTransferLines, cancelStockTransfer } from '@/api/stockTransfers';

const mockListTransfers = listStockTransfers as ReturnType<typeof vi.fn>;
const mockGetLines = getStockTransferLines as ReturnType<typeof vi.fn>;
const mockCancelTransfer = cancelStockTransfer as ReturnType<typeof vi.fn>;

// ── Test data ─────────────────────────────────────────────────────

const recentTransfer = {
  id: 'tr-1',
  transfer_number: 'TXN-001',
  status: 'in_transit',
  source_location: 'Warehouse A',
  destination_location: 'Store Front',
  source_terminal_id: null,
  destination_terminal_id: null,
  notes: '',
  created_by: 'user-1',
  received_by: null,
  created_at: new Date(Date.now() - 3600000).toISOString(), // 1 hour ago
  sent_at: new Date(Date.now() - 3600000).toISOString(),
  received_at: null,
  updated_at: new Date(Date.now() - 3600000).toISOString(),
};

const overdueTransfer = {
  id: 'tr-2',
  transfer_number: 'TXN-002',
  status: 'in_transit',
  source_location: 'Warehouse B',
  destination_location: 'Branch 2',
  source_terminal_id: null,
  destination_terminal_id: null,
  notes: '',
  created_by: 'user-1',
  received_by: null,
  created_at: new Date(Date.now() - 48 * 3600000).toISOString(), // 48 hours ago
  sent_at: new Date(Date.now() - 48 * 3600000).toISOString(),
  received_at: null,
  updated_at: new Date(Date.now() - 48 * 3600000).toISOString(),
};

const lines = [
  { id: 'line-1', transfer_id: 'tr-1', sku: 'SKU-001', product_name: 'Coffee Beans', qty: 10, received_qty: 0 },
  { id: 'line-2', transfer_id: 'tr-1', sku: 'SKU-002', product_name: 'Green Tea', qty: 5, received_qty: 0 },
];

describe('TransitAuditScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListTransfers.mockResolvedValue([recentTransfer, overdueTransfer]);
    mockGetLines.mockResolvedValue(lines);
    mockCancelTransfer.mockResolvedValue({
      ...recentTransfer,
      status: 'cancelled',
    });
    window.confirm = vi.fn(() => true);
  });

  // ── Rendering ─────────────────────────────────────────────────

  it('renders the title', async () => {
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);
    await waitFor(() => {
      expect(screen.getByText('Transit Stock Audit')).toBeInTheDocument();
    });
  });

  it('shows loading state initially', () => {
    mockListTransfers.mockReturnValue(new Promise(() => {}));
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);
    // inv-loading = "Loading products…" in inventory.ftl
    expect(screen.getByText(/Loading products/)).toBeInTheDocument();
  });

  // ── Transfer cards ────────────────────────────────────────────

  it('renders transfer cards with meta info', async () => {
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);
    await waitFor(() => {
      expect(screen.getByText('TXN-001')).toBeInTheDocument();
    });
    expect(screen.getByText('TXN-002')).toBeInTheDocument();
    expect(screen.getByText('Warehouse A')).toBeInTheDocument();
    expect(screen.getByText('Warehouse B')).toBeInTheDocument();
  });

  it('renders line items in the table', async () => {
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);
    await waitFor(() => {
      // SKU-001 appears in both transfer cards
      expect(screen.getAllByText('SKU-001').length).toBeGreaterThanOrEqual(2);
    });
    // Coffee Beans, Green Tea, and quantities appear in BOTH transfer cards
    expect(screen.getAllByText('Coffee Beans').length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText('10').length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText('Green Tea').length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText('5').length).toBeGreaterThanOrEqual(2);
  });

  // ── Overdue detection ─────────────────────────────────────────

  it('marks overdue transfers with overdue class (48h+)', async () => {
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);
    await waitFor(() => {
      expect(screen.getByText('TXN-001')).toBeInTheDocument();
    });

    // Overdue card should show "OVERDUE" text via the CSS ::before pseudo-element
    // DOM check: the overdue card has class 'overdue'
    const cards = document.querySelectorAll('.transit-card');
    expect(cards.length).toBe(2);

    const cardEntries = Array.from(cards);
    // The component renders transfers in the order: [recentTransfer, overdueTransfer]
    // cardEntries[0] = recent (1h old, NOT overdue)
    // cardEntries[1] = overdue (48h old, IS overdue)
    expect(cardEntries[0]!.className).not.toContain('overdue');
    expect(cardEntries[1]!.className).toContain('overdue');
  });

  // ── Reverse transfer ──────────────────────────────────────────

  it('calls cancelStockTransfer when Reverse Transfer is clicked', async () => {
    const user = userEvent.setup();
    window.confirm = vi.fn(() => true);
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);

    await waitFor(() => {
      expect(screen.getByText('TXN-001')).toBeInTheDocument();
    });

    const reverseBtns = screen.getAllByText('Reverse Transfer');
    await user.click(reverseBtns[0]!);

    await waitFor(() => {
      expect(window.confirm).toHaveBeenCalled();
      expect(mockCancelTransfer).toHaveBeenCalled();
    });
  });

  it('does not cancel when confirm is dismissed', async () => {
    const user = userEvent.setup();
    window.confirm = vi.fn(() => false);
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);

    await waitFor(() => {
      expect(screen.getByText('TXN-001')).toBeInTheDocument();
    });

    const reverseBtns = screen.getAllByText('Reverse Transfer');
    await user.click(reverseBtns[0]!);

    expect(mockCancelTransfer).not.toHaveBeenCalled();
  });

  // ── Empty state ───────────────────────────────────────────────

  it('shows empty state when no transfers in transit', async () => {
    mockListTransfers.mockResolvedValue([]);
    renderWithProvidersSync(<TransitAuditScreen />, inventoryFtl);

    await waitFor(() => {
      // inv-transit-no-overdue = "No overdue transit items." in inventory.ftl
      expect(screen.getByText(/No overdue transit/)).toBeInTheDocument();
    });
  });
});
