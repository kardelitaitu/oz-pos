import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
import { StockAlertPanel } from '@/features/inventory/StockAlertPanel';

// ── Mock auth and workspace contexts ───────────────────────────

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Test User', role_name: 'cashier', session_token: 'mock-session-token' },
  }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    sessionToken: 'mock-session-token',
    currentInstanceId: 'inst-1',
    swapSessionToken: vi.fn(),
  }),
}));

// ── Mock API module ──────────────────────────────────────────────

const mockGetActiveStockAlerts = vi.fn();
const mockAcknowledgeStockAlert = vi.fn();

vi.mock('@/api/inventory', () => ({
  getActiveStockAlerts: (...args: unknown[]) => mockGetActiveStockAlerts(...args),
  acknowledgeStockAlert: (...args: unknown[]) => mockAcknowledgeStockAlert(...args),
}));

// ── Test data ────────────────────────────────────────────────────

const mockAlerts = [
  {
    id: 'alert-1',
    threshold_id: 'th-1',
    product_id: 'p-1',
    location_id: 'loc-1',
    current_qty: 0,
    threshold: 10,
    status: 'active' as const,
    triggered_at: new Date(Date.now() - 5 * 60000).toISOString(),
    acknowledged_at: null,
    resolved_at: null,
    acknowledged_by: null,
    product_sku: 'SKU-001',
    product_name: 'Coffee Beans',
  },
  {
    id: 'alert-2',
    threshold_id: 'th-2',
    product_id: 'p-2',
    location_id: 'loc-1',
    current_qty: 3,
    threshold: 5,
    status: 'active' as const,
    triggered_at: new Date(Date.now() - 60 * 60000).toISOString(),
    acknowledged_at: null,
    resolved_at: null,
    acknowledged_by: null,
    product_sku: 'SKU-002',
    product_name: 'Green Tea',
  },
];

describe('StockAlertPanel', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  // ── Alert list ────────────────────────────────────────────────

  it('renders alerts with product info after loading', async () => {
    mockGetActiveStockAlerts.mockResolvedValue(mockAlerts);
    renderWithProviders(<StockAlertPanel locationId="loc-1" pollIntervalMs={0} />);

    // Wait for data to load and find SKU
    const sku001 = await screen.findByText('SKU-001', {}, { timeout: 2000 });
    expect(sku001).toBeInTheDocument();
    expect(screen.getByText('Coffee Beans')).toBeInTheDocument();
    expect(screen.getByText('SKU-002')).toBeInTheDocument();
    expect(screen.getByText('Green Tea')).toBeInTheDocument();
  });

  // ── Badge count ───────────────────────────────────────────────

  it('shows badge with alert count', async () => {
    mockGetActiveStockAlerts.mockResolvedValue(mockAlerts);
    renderWithProviders(<StockAlertPanel locationId="loc-1" pollIntervalMs={0} />);

    await waitFor(() => {
      expect(screen.getByText('2')).toBeInTheDocument();
    });
  });

  // ── Severity classes ──────────────────────────────────────────

  it('marks zero-stock alerts as critical', async () => {
    mockGetActiveStockAlerts.mockResolvedValue(mockAlerts);
    renderWithProviders(<StockAlertPanel locationId="loc-1" pollIntervalMs={0} />);

    await waitFor(() => {
      const cards = screen.getAllByRole('listitem');
      expect(cards.length).toBe(2);
      // First card (alert-1, qty=0) should be critical
      expect(cards[0]!.className).toContain('stock-alert-card--critical');
      // Second card (alert-2, qty=3) should be warning
      expect(cards[1]!.className).toContain('stock-alert-card--warning');
    });
  });

  // ── Empty state ───────────────────────────────────────────────

  it('shows empty state when no alerts', async () => {
    mockGetActiveStockAlerts.mockResolvedValue([]);
    renderWithProviders(<StockAlertPanel locationId="loc-1" pollIntervalMs={0} />);

    await waitFor(() => {
      expect(screen.getByText(/No active alerts/i)).toBeInTheDocument();
    });
  });

  // ── Acknowledge ───────────────────────────────────────────────

  it('calls acknowledgeStockAlert when Ack button is clicked', async () => {
    const user = userEvent.setup();
    mockGetActiveStockAlerts.mockResolvedValue(mockAlerts);
    mockAcknowledgeStockAlert.mockResolvedValue(undefined);
    renderWithProviders(<StockAlertPanel locationId="loc-1" pollIntervalMs={0} />);

    await waitFor(() => {
      expect(screen.getByText('Coffee Beans')).toBeInTheDocument();
    });

    const ackBtns = screen.getAllByRole('button', { name: /acknowledge/i });
    await user.click(ackBtns[0]!);

    expect(mockAcknowledgeStockAlert).toHaveBeenCalledWith(
      expect.any(String),
      'alert-1',
    );
  });

  // ── Error state ───────────────────────────────────────────────

  it('shows error message when fetch fails', async () => {
    mockGetActiveStockAlerts.mockRejectedValue(new Error('Network error'));
    renderWithProviders(<StockAlertPanel locationId="loc-1" pollIntervalMs={0} />);

    await waitFor(() => {
      expect(screen.getByText(/network error/i)).toBeInTheDocument();
    });
  });
});
