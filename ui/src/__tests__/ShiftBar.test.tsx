import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import inventoryFtl from '@/locales/inventory.ftl?raw';

// ── Mocks ─────────────────────────────────────────────────────────

vi.mock('@/api/inventory', () => ({
  listInventoryLocations: vi.fn(),
  getActiveInventoryShift: vi.fn(),
  startInventoryShift: vi.fn(),
  endInventoryShift: vi.fn(),
  listInventoryTransactions: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Test Cashier', role_name: 'cashier' },
    sessionToken: 'mock-session-token',
  }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    sessionToken: 'mock-session-token',
    currentInstanceId: 'inst-1',
  }),
}));

import ShiftBar from '@/features/inventory/ShiftBar';
import {
  listInventoryLocations,
  getActiveInventoryShift,
  startInventoryShift,
  endInventoryShift,
  listInventoryTransactions,
} from '@/api/inventory';

const mockLocations = listInventoryLocations as ReturnType<typeof vi.fn>;
const mockGetActiveShift = getActiveInventoryShift as ReturnType<typeof vi.fn>;
const mockStartShift = startInventoryShift as ReturnType<typeof vi.fn>;
const mockEndShift = endInventoryShift as ReturnType<typeof vi.fn>;
const mockListTransactions = listInventoryTransactions as ReturnType<typeof vi.fn>;

// ── Test data ─────────────────────────────────────────────────────

const locations = [
  { id: 'loc-1', name: 'Main Warehouse', type: 'warehouse', description: '', is_active: true, created_at: '', updated_at: '' },
  { id: 'loc-2', name: 'Store Front', type: 'store', description: '', is_active: true, created_at: '', updated_at: '' },
];

const activeShift = {
  id: 'shift-active-1',
  user_id: 'user-1',
  location_id: 'loc-1',
  terminal_id: null,
  started_at: new Date(Date.now() - 3600000).toISOString(), // 1 hour ago
  ended_at: null,
  status: 'active' as const,
  notes: 'Evening count',
};

const transactions = [
  { id: 'tx-1', type: 'manual-adjustment', location_id: 'loc-1', staff_id: 'user-1', transfer_id: null, purchase_order_id: null, notes: 'Restock', created_at: new Date(Date.now() - 1800000).toISOString() },
  { id: 'tx-2', type: 'stock-count', location_id: 'loc-1', staff_id: 'user-1', transfer_id: null, purchase_order_id: null, notes: 'Count', created_at: new Date(Date.now() - 900000).toISOString() },
];

describe('ShiftBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLocations.mockResolvedValue(locations);
    mockGetActiveShift.mockResolvedValue(null);
    mockStartShift.mockResolvedValue(activeShift);
    mockEndShift.mockResolvedValue(undefined);
    mockListTransactions.mockResolvedValue([]);
  });

  // ── Empty / Start Form State ──────────────────────────────────

  it('shows start form when no active shift exists', async () => {
    renderWithFluentSync(<ShiftBar />, inventoryFtl);
    await waitFor(() => {
      expect(screen.getByText('Start Inventory Shift')).toBeInTheDocument();
    });
    expect(screen.getByText('Start Shift')).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: /location/i })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: /notes/i })).toBeInTheDocument();
  });

  it('loads locations into the dropdown', async () => {
    renderWithFluentSync(<ShiftBar />, inventoryFtl);
    await waitFor(() => {
      const select = screen.getByRole('combobox', { name: /location/i });
      expect(select).toBeInTheDocument();
      // Should have both location options
      expect(select).toContainHTML('Main Warehouse');
      expect(select).toContainHTML('Store Front');
    });
  });

  it('calls startInventoryShift on form submit', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<ShiftBar />, inventoryFtl);

    await waitFor(() => {
      expect(screen.getByText('Start Shift')).toBeInTheDocument();
    });

    // Select a location
    const select = screen.getByRole('combobox', { name: /location/i });
    await user.selectOptions(select, 'loc-2');

    // Enter notes
    const notesInput = screen.getByRole('textbox', { name: /notes/i });
    await user.type(notesInput, 'Night shift');

    // Submit
    await user.click(screen.getByText('Start Shift'));

    await waitFor(() => {
      expect(mockStartShift).toHaveBeenCalledWith(
        'mock-session-token', 'user-1', 'loc-2', 'Night shift'
      );
    });
  });

  // ── Active Shift State ────────────────────────────────────────

  it('shows active shift info with timer and end button', async () => {
    mockGetActiveShift.mockResolvedValue(activeShift);
    renderWithFluentSync(<ShiftBar />, inventoryFtl);

    await waitFor(() => {
      // Fluent interpolates as "Test Cashier — Main Warehouse — Started 01:00:00"
      expect(screen.getByText(/Main Warehouse/)).toBeInTheDocument();
      expect(screen.getByText(/Test Cashier/)).toBeInTheDocument();
    });
    expect(screen.getByText('End Shift')).toBeInTheDocument();
  });

  it('calls onShiftChange callback when shift is loaded', async () => {
    const onShiftChange = vi.fn();
    mockGetActiveShift.mockResolvedValue(activeShift);
    renderWithFluentSync(<ShiftBar onShiftChange={onShiftChange} />, inventoryFtl);

    await waitFor(() => {
      expect(onShiftChange).toHaveBeenCalledWith(activeShift);
    });
  });

  // ── End Shift Flow ────────────────────────────────────────────

  it('shows summary modal after ending shift', async () => {
    const user = userEvent.setup();
    mockGetActiveShift.mockResolvedValue(activeShift);
    mockListTransactions.mockResolvedValue(transactions);
    renderWithFluentSync(<ShiftBar />, inventoryFtl);

    // Wait for active shift to load
    await waitFor(() => {
      expect(screen.getByText('End Shift')).toBeInTheDocument();
    });

    // Click End Shift
    await user.click(screen.getByText('End Shift'));

    // Verify endInventoryShift was called
    await waitFor(() => {
      expect(mockEndShift).toHaveBeenCalledWith('mock-session-token', activeShift.id);
    });

    // Summary modal should appear
    await waitFor(() => {
      expect(screen.getByText('Shift Summary')).toBeInTheDocument();
      expect(screen.getByText('Transactions performed during this shift:')).toBeInTheDocument();
    });
  });

  it('shows empty state in summary when no transactions exist', async () => {
    const user = userEvent.setup();
    mockGetActiveShift.mockResolvedValue(activeShift);
    mockListTransactions.mockResolvedValue([]);
    renderWithFluentSync(<ShiftBar />, inventoryFtl);

    await waitFor(() => {
      expect(screen.getByText('End Shift')).toBeInTheDocument();
    });

    await user.click(screen.getByText('End Shift'));

    await waitFor(() => {
      expect(screen.getByText('No transactions recorded.')).toBeInTheDocument();
    });
  });

  it('closes summary modal when Close button is clicked', async () => {
    const user = userEvent.setup();
    mockGetActiveShift.mockResolvedValue(activeShift);
    renderWithFluentSync(<ShiftBar />, inventoryFtl);

    await waitFor(() => {
      expect(screen.getByText('End Shift')).toBeInTheDocument();
    });

    await user.click(screen.getByText('End Shift'));

    await waitFor(() => {
      expect(screen.getByText('Shift Summary')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Cancel'));

    await waitFor(() => {
      expect(screen.queryByText('Shift Summary')).not.toBeInTheDocument();
    });
  });
});
