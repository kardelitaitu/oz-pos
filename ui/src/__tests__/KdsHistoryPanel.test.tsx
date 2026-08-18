import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { KdsHistoryPanel } from '@/features/kds/KdsHistoryPanel';
import kdsFtl from '@/locales/kds.ftl?raw';
import type { KdsOrder } from '@/api/kds';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';

// Mock the API
const mockListKdsOrdersScoped = vi.fn();
vi.mock('@/api/kds', () => ({
  // Forward the arguments so tests can assert the session token + filter.
  listKdsOrdersScoped: (_token: string, _status?: string) => mockListKdsOrdersScoped(_token, _status),
}));

// Mock WorkspaceContext
vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    sessionToken: 'test-session-token',
    activeWorkspace: 'kds',
    setActiveWorkspace: vi.fn(),
    activeInstance: null,
    setActiveInstance: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
    error: null,
    retry: vi.fn(),
    lastWorkspace: null,
    switchStore: vi.fn(),
    resolvedStoreId: 'default',
    swapSessionToken: vi.fn(),
  }),
  useWorkspaceScope: () => ({ storeId: 'store-1', instanceId: 'inst-1', typeKey: 'kds' }),
}));

function makeOrder(overrides: Partial<KdsOrder> = {}): KdsOrder {
  return {
    id: 'order-1',
    sale_id: 'sale-1',
    store_id: 'store-1',
    status: 'served',
    items_summary: 'Latte x2, Croissant x1',
    item_count: 3,
    display_number: 101,
    received_at: new Date('2024-01-15T10:30:00Z').toISOString(),
    started_at: new Date('2024-01-15T10:31:00Z').toISOString(),
    ready_at: new Date('2024-01-15T10:35:00Z').toISOString(),
    served_at: new Date('2024-01-15T10:40:00Z').toISOString(),
    prep_time_seconds: 600,
    kitchen_zone: 'bar',
    notes: 'Extra hot',
    table_number: 'T5',
    priority: false,
    target_instance_id: 'inst-1',
    ...overrides,
  };
}

describe('KdsHistoryPanel', () => {
  beforeEach(() => {
    mockListKdsOrdersScoped.mockResolvedValue([]);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Initial render', () => {
    it('renders the filter tabs for Served and Cancelled', async () => {
      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      // Filter tabs should be visible
      expect(screen.getByRole('tablist')).toBeInTheDocument();
      expect(screen.getByRole('tab', { name: /served/i })).toBeInTheDocument();
      expect(screen.getByRole('tab', { name: /cancelled/i })).toBeInTheDocument();
    });

    it('shows loading state on initial load', async () => {
      let resolveFn: (value: KdsOrder[]) => void;
      mockListKdsOrdersScoped.mockImplementation(() => new Promise((resolve) => { resolveFn = resolve; }));

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      // Loading state should be shown
      expect(screen.getByRole('status')).toBeInTheDocument();
      expect(screen.getByText(/loading history/i)).toBeInTheDocument();

      // Resolve the promise
      await act(async () => {
        resolveFn!([makeOrder()]);
        await Promise.resolve();
      });
    });
  });

  describe('Filter tabs', () => {
    it('defaults to Served tab selected', async () => {
      mockListKdsOrdersScoped.mockResolvedValue([makeOrder({ status: 'served' })]);

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());

      const servedTab = screen.getByRole('tab', { name: /served/i });
      expect(servedTab).toHaveAttribute('aria-selected', 'true');

      const cancelledTab = screen.getByRole('tab', { name: /cancelled/i });
      expect(cancelledTab).toHaveAttribute('aria-selected', 'false');
    });

    it('switches to Cancelled tab when clicked', async () => {
      const servedOrders = [makeOrder({ id: 'served-1', status: 'served' })];
      const cancelledOrders = [makeOrder({ id: 'cancelled-1', status: 'cancelled' })];

      mockListKdsOrdersScoped
        .mockResolvedValueOnce(servedOrders)
        .mockResolvedValueOnce(cancelledOrders);

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());

      const user = userEvent.setup();
      const cancelledTab = screen.getByRole('tab', { name: /cancelled/i });
      await user.click(cancelledTab);

      // Wait for the filter change to trigger a new fetch
      await waitFor(() => {
        expect(mockListKdsOrdersScoped).toHaveBeenCalledWith('test-session-token', 'cancelled');
      });
    });
  });

  describe('Order display', () => {
    it('renders order cards with correct information', async () => {
      const orders = [
        makeOrder({
          id: 'order-1',
          display_number: 101,
          items_summary: 'Latte x2, Croissant x1',
          table_number: 'T5',
          status: 'served',
          received_at: new Date('2024-01-15T10:30:00Z').toISOString(),
          served_at: new Date('2024-01-15T10:40:00Z').toISOString(),
          notes: 'Extra hot',
        }),
        makeOrder({
          id: 'order-2',
          display_number: 102,
          items_summary: 'Espresso x1',
          table_number: null,
          status: 'served',
          received_at: new Date('2024-01-15T11:00:00Z').toISOString(),
          served_at: new Date('2024-01-15T11:05:00Z').toISOString(),
          notes: '',
        }),
      ];

      mockListKdsOrdersScoped.mockResolvedValue(orders);

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());

      // Check order cards are rendered
      expect(screen.getByText('#101')).toBeInTheDocument();
      expect(screen.getByText('#102')).toBeInTheDocument();
      expect(screen.getByText('Latte x2, Croissant x1')).toBeInTheDocument();
      expect(screen.getByText('Espresso x1')).toBeInTheDocument();
      expect(screen.getByText('T5')).toBeInTheDocument();
      expect(screen.getByText('Extra hot')).toBeInTheDocument();
      // The "Served" tab plus the per-card status badges.
      expect(screen.getAllByText(/served/i).length).toBeGreaterThanOrEqual(2);
      // One "Received" timestamp per card.
      expect(screen.getAllByText(/received/i).length).toBeGreaterThanOrEqual(2);
    });

    it('shows empty state when no orders', async () => {
      mockListKdsOrdersScoped.mockResolvedValue([]);

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());

      expect(screen.getByText(/no completed orders yet/i)).toBeInTheDocument();
    });
  });

  describe('Error handling', () => {
    it('shows error message with retry button on fetch failure', async () => {
      mockListKdsOrdersScoped.mockRejectedValue(new Error('Network error'));

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeInTheDocument();
        expect(screen.getByText(/failed to load order history/i)).toBeInTheDocument();
        expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
      });
    });

    it('retries fetch when retry button is clicked', async () => {
      mockListKdsOrdersScoped
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce([makeOrder()]);

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeInTheDocument();
      });

      const user = userEvent.setup();
      const retryBtn = screen.getByRole('button', { name: /retry/i });
      await user.click(retryBtn);

      // The alert clears synchronously on retry, but the refetched list
      // renders only after the promise resolves — wait for both.
      await waitFor(() => {
        expect(screen.queryByRole('alert')).not.toBeInTheDocument();
        expect(screen.getByText('#101')).toBeInTheDocument();
      });
    });
  });

  describe('Refreshing state', () => {
    it('shows refreshing indicator when changing filter with existing orders', async () => {
      const servedOrders = [makeOrder({ id: 'served-1', status: 'served' })];
      const cancelledOrders = [makeOrder({ id: 'cancelled-1', status: 'cancelled' })];

      mockListKdsOrdersScoped
        .mockResolvedValueOnce(servedOrders)
        .mockImplementationOnce(() => new Promise((resolve) => {
          setTimeout(() => resolve(cancelledOrders), 100);
        }));

      renderWithFluentSync(<KdsHistoryPanel />, kdsFtl);

      await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());

      const user = userEvent.setup();
      const cancelledTab = screen.getByRole('tab', { name: /cancelled/i });
      await user.click(cancelledTab);

      // Should show refreshing indicator while fetching (the effect runs
      // asynchronously after the click, so wait for it).
      await waitFor(() => expect(screen.getByRole('status')).toBeInTheDocument());
      expect(screen.getByText(/loading history/i)).toBeInTheDocument();

      // The in-flight fetch resolves after ~100ms of real time.
      await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
    });
  });
});