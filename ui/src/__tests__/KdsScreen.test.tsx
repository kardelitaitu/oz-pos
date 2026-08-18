import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import KdsScreen from '@/features/kds/KdsScreen';
import kdsFtl from '@/locales/kds.ftl?raw';
import type { KdsOrder } from '@/api/kds';

const { mockGetKdsQueue, mockUpdateKdsStatus, mockListKdsOrdersScoped, mockGetKdsOrderLines, mockUpdateKdsOrderItems, mockListProducts, mockUseTicketSla, mockPlayAlert, mockSpeak, mockUseWorkspaceScope } = vi.hoisted(() => ({
  mockGetKdsQueue: vi.fn(),
  mockUpdateKdsStatus: vi.fn(),
  mockListKdsOrdersScoped: vi.fn().mockResolvedValue([]),
  mockGetKdsOrderLines: vi.fn().mockResolvedValue([]),
  mockUpdateKdsOrderItems: vi.fn(),
  mockListProducts: vi.fn().mockResolvedValue([]),
  mockUseTicketSla: vi.fn((): { level: 'green' | 'yellow' | 'red'; elapsedSeconds: number; display: string } => ({
    level: 'green',
    elapsedSeconds: 120,
    display: '2m 0s',
  })),
  mockPlayAlert: vi.fn(),
  mockSpeak: vi.fn(),
  mockUseWorkspaceScope: vi.fn<() => { storeId: string; instanceId: string; typeKey: string } | null>(() => null),
}));

vi.mock('@/api/kds', () => ({
  getKdsQueue: async (_userId: string, _kdsZone?: string) => {
    const orders = await mockGetKdsQueue();
    if (!_kdsZone) {
      return orders;
    }
    return orders.filter(order => order['kitchen_zone'] === _kdsZone);
  },
  getKdsQueueScoped: async (_token: string, _kdsZone?: string) => {
    const orders = await mockGetKdsQueue();
    if (!_kdsZone) {
      return orders;
    }
    return orders.filter(order => order['kitchen_zone'] === _kdsZone);
  },
  listKdsOrdersScoped: (_token: string, _status: string) => mockListKdsOrdersScoped(),
  updateKdsStatus: (_userId: string, id: string, status: string) =>
    mockUpdateKdsStatus(id, status),
  updateKdsStatusScoped: (
    _token: string,
    id: string,
    status: string,
  ) => mockUpdateKdsStatus(id, status),
  getKdsOrderLinesScoped: (_token: string, _orderId: string) => mockGetKdsOrderLines(),
  updateKdsOrderItemsScoped: (_token: string, args: unknown) =>
    mockUpdateKdsOrderItems(args),
}));

vi.mock('@/api/products', () => ({
  listProductsScoped: () => mockListProducts(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({ session: { user_id: 'user-1', display_name: 'Alice', role_name: 'cashier' } }),
}));

vi.mock('@/features/kds/hooks/useTicketSla', () => ({
  useTicketSla: (..._args: unknown[]) => mockUseTicketSla(),
}));

vi.mock('@/frontend/shared/useSound', () => ({
  useSound: () => ({
    playAlert: mockPlayAlert,
    playBeep: vi.fn(),
    playError: vi.fn(),
    playSuccess: vi.fn(),
    speak: mockSpeak,
    setSoundEnabled: vi.fn(),
  }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspaceScope: () => mockUseWorkspaceScope(),
  useWorkspace: () => ({
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
    sessionToken: null,
    swapSessionToken: vi.fn(),
  }),
}));

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(kdsFtl));
const l10n = new ReactLocalization([bundle]);

function renderScreen() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <KdsScreen />
    </LocalizationProvider>,
  );
}

function makeOrder(overrides: Partial<KdsOrder> = {}): KdsOrder {
  return {
    id: 'o-1',
    sale_id: 's-1',
    store_id: null,
    status: 'pending',
    items_summary: 'Burger x1, Fries x1',
    item_count: 2,
    display_number: 101,
    received_at: new Date().toISOString(),
    started_at: null,
    ready_at: null,
    served_at: null,
    prep_time_seconds: 0,
    kitchen_zone: null,
    notes: '',
    table_number: null,
    priority: false,
    ...overrides,
  };
}

describe('KdsScreen', () => {
  beforeEach(() => {
    mockGetKdsQueue.mockResolvedValue([]);
    mockSpeak.mockClear();
  });

  it('renders the title', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('Kitchen Display')).toBeDefined());
  });

  it('shows three columns: Pending, Preparing, Ready', async () => {
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Pending')).toBeDefined();
      expect(screen.getByText('Preparing')).toBeDefined();
      expect(screen.getByText('Ready')).toBeDefined();
    });
  });

  it('shows order count in the header', async () => {
    mockGetKdsQueue.mockResolvedValue([makeOrder(), makeOrder({ id: 'o-2' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Kitchen Display')).toBeDefined());
    const countEl = document.querySelector('.kds-order-count');
    expect(countEl).toBeDefined();
    // Fluent renders "2 orders" with Bidi chars, match pattern
    expect(countEl?.textContent).toMatch(/2/);
  });

  it('shows empty state in each column when no orders', async () => {
    renderScreen();
    await waitFor(() => {
      const empties = screen.getAllByText('No orders yet');
      // Three columns, each with an empty message
      expect(empties.length).toBe(3);
    });
  });

  it('renders tickets in the correct column by status', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101, items_summary: 'Burger' }),
      makeOrder({ id: 'o-2', status: 'preparing', display_number: 102, items_summary: 'Fries' }),
      makeOrder({ id: 'o-3', status: 'ready', display_number: 103, items_summary: 'Drink' }),
    ]);
    renderScreen();
    await waitFor(() => {
      // Ticket numbers rendered as #display_number
      expect(screen.getByText('#101')).toBeDefined();
      expect(screen.getByText('#102')).toBeDefined();
      expect(screen.getByText('#103')).toBeDefined();
    });
  });

  it('shows items summary on each ticket', async () => {
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Burger x1, Fries x1')).toBeDefined());
  });

  it('shows item count on each ticket', async () => {
    mockGetKdsQueue.mockResolvedValue([makeOrder({ item_count: 3 })]);
    renderScreen();
    await waitFor(() => {
      // Fluent Localized wraps the text in <span>, so use a custom matcher
      const countEl = document.querySelector('.kds-ticket-count');
      expect(countEl?.textContent).toMatch(/3/);
      expect(countEl?.textContent).toMatch(/items/);
    });
  });

  it('shows notes on ticket when present', async () => {
    mockGetKdsQueue.mockResolvedValue([makeOrder({ notes: 'No onions' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('No onions')).toBeDefined());
  });

  it('advances pending order to preparing on click', async () => {
    mockGetKdsQueue.mockResolvedValue([makeOrder({ status: 'pending' })]);
    mockUpdateKdsStatus.mockResolvedValue({});

    renderScreen();
    await waitFor(() => expect(screen.getByText('Burger x1, Fries x1')).toBeDefined());

    const ticket = document.querySelector('.kds-ticket')!;
    await userEvent.click(ticket);

    await waitFor(() =>
      expect(mockUpdateKdsStatus).toHaveBeenCalledWith('o-1', 'preparing'),
    );
  });

  it('advances preparing order to ready on click', async () => {
    mockGetKdsQueue.mockResolvedValue([makeOrder({ status: 'preparing' })]);
    mockUpdateKdsStatus.mockResolvedValue({});

    renderScreen();
    await waitFor(() => expect(screen.getByText('Burger x1, Fries x1')).toBeDefined());

    const ticket = document.querySelector('.kds-ticket')!;
    await userEvent.click(ticket);

    await waitFor(() =>
      expect(mockUpdateKdsStatus).toHaveBeenCalledWith('o-1', 'ready'),
    );
  });

  it('shows offline banner when getKdsQueue fails', async () => {
    mockGetKdsQueue.mockRejectedValue(new Error('Network down'));
    renderScreen();
    // The offline banner should show instead of the raw error.
    await waitFor(() => {
      expect(document.querySelector('.kds-offline-banner')).not.toBeNull();
    });
  });

  it('shows time ago on tickets', async () => {
    const recentTime = new Date(Date.now() - 5 * 60000).toISOString(); // 5 min ago
    mockGetKdsQueue.mockResolvedValue([makeOrder({ received_at: recentTime })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Burger x1, Fries x1')).toBeDefined());
    const timeCell = document.querySelector('.kds-ticket-time');
    // timeAgo should show "5m" for 5 minutes ago
    expect(timeCell?.textContent).toMatch(/m/);
  });

  it('shows column counts', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending' }),
      makeOrder({ id: 'o-2', status: 'pending' }),
    ]);
    renderScreen();
    await waitFor(() => {
      const counts = document.querySelectorAll('.kds-column-count');
      expect(counts.length).toBe(3);
      // Pending column should show count of 2
      expect(counts[0]?.textContent).toBe('2');
    });
  });

  it('has aria-label on the KDS region', async () => {
    renderScreen();
    await waitFor(() =>
      expect(screen.getByRole('region', { name: 'Kitchen Display System' })).toBeDefined(),
    );
  });

  it('does not render cancelled orders in any column', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'cancelled', display_number: 999, items_summary: 'Cancel Item' }),
    ]);
    renderScreen();
    await waitFor(() => {
      // Cancelled tickets are terminal history — the board is truly empty
      // (never surfaces on the kitchen board, history panel only).
      const empties = screen.getAllByText('No orders yet');
      expect(empties.length).toBe(3);
    });
    // Cancelled order should not be visible
    expect(screen.queryByText('#999')).toBeNull();
    expect(screen.queryByText('Cancel Item')).toBeNull();
  });

  // ── SLA class tests ──────────────────────────────────────────────────

  it('applies green SLA class by default', async () => {
    mockUseTicketSla.mockReturnValue({ level: 'green', elapsedSeconds: 120, display: '2m 0s' });
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    renderScreen();
    await waitFor(() => {
      const ticket = document.querySelector('.kds-ticket');
      expect(ticket).not.toBeNull();
      expect(ticket?.classList.contains('kds-ticket--green')).toBe(true);
    });
  });

  it('applies yellow SLA class', async () => {
    mockUseTicketSla.mockReturnValue({ level: 'yellow', elapsedSeconds: 720, display: '12m 0s' });
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    renderScreen();
    await waitFor(() => {
      const ticket = document.querySelector('.kds-ticket');
      expect(ticket?.classList.contains('kds-ticket--yellow')).toBe(true);
    });
  });

  it('applies red SLA class', async () => {
    mockUseTicketSla.mockReturnValue({ level: 'red', elapsedSeconds: 1200, display: '20m 0s' });
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    renderScreen();
    await waitFor(() => {
      const ticket = document.querySelector('.kds-ticket');
      expect(ticket?.classList.contains('kds-ticket--red')).toBe(true);
    });
  });

  it('shows SLA display string instead of timeAgo', async () => {
    mockUseTicketSla.mockReturnValue({ level: 'green', elapsedSeconds: 300, display: '5m 0s' });
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    renderScreen();
    await waitFor(() => {
      const timeCell = document.querySelector('.kds-ticket-time');
      expect(timeCell?.textContent).toBe('5m 0s');
    });
  });

  it('does not fire playAlert on initial render with red ticket (no transition)', async () => {
    // From the code: prevLevel starts null, so on first render it doesn't play
    mockUseTicketSla.mockReturnValue({ level: 'red', elapsedSeconds: 1200, display: '20m 0s' });
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    renderScreen();
    await waitFor(() => {
      expect(document.querySelector('.kds-ticket--red')).not.toBeNull();
    });
    expect(mockPlayAlert).not.toHaveBeenCalled();
  });

  it('applies color class on time element matching SLA level', async () => {
    mockUseTicketSla.mockReturnValue({ level: 'yellow', elapsedSeconds: 720, display: '12m 0s' });
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    renderScreen();
    await waitFor(() => {
      const timeCell = document.querySelector('.kds-ticket-time');
      expect(timeCell?.classList.contains('kds-ticket-time--yellow')).toBe(true);
    });
  });

  // ── ADR #8: store_id filtering tests ────────────────────────────────

  it('passes through orders with null store_id when scope is set (legacy compat)', async () => {
    mockUseWorkspaceScope.mockReturnValue({ storeId: 'store-downtown', instanceId: 'i-1', typeKey: 'kds' });
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', store_id: null, items_summary: 'Legacy Order' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Legacy Order')).toBeDefined();
    });
  });

  it('passes through orders whose store_id matches the active scope', async () => {
    mockUseWorkspaceScope.mockReturnValue({ storeId: 'store-downtown', instanceId: 'i-1', typeKey: 'kds' });
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', store_id: 'store-downtown', items_summary: 'Downtown Order' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Downtown Order')).toBeDefined();
    });
  });

  it('filters out orders whose store_id does not match the active scope', async () => {
    mockUseWorkspaceScope.mockReturnValue({ storeId: 'store-downtown', instanceId: 'i-1', typeKey: 'kds' });
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', store_id: 'store-mall', items_summary: 'Mall Order' }),
    ]);
    renderScreen();
    await waitFor(() => {
      // The Mall order should be filtered out — all columns show empty state
      const empties = screen.getAllByText('No orders yet');
      expect(empties.length).toBe(3);
    });
    expect(screen.queryByText('Mall Order')).toBeNull();
  });

  it('passes through all orders when workspace scope is null (no filtering)', async () => {
    mockUseWorkspaceScope.mockReturnValue(null);
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', store_id: 'store-downtown', items_summary: 'DT Order' }),
      makeOrder({ id: 'o-2', store_id: 'store-mall', items_summary: 'Mall Order' }),
      makeOrder({ id: 'o-3', store_id: null, items_summary: 'Legacy Order' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('DT Order')).toBeDefined();
      expect(screen.getByText('Mall Order')).toBeDefined();
      expect(screen.getByText('Legacy Order')).toBeDefined();
    });
  });

  it('filters mixed orders — keeps matching and legacy, drops mismatched', async () => {
    mockUseWorkspaceScope.mockReturnValue({ storeId: 'store-downtown', instanceId: 'i-1', typeKey: 'kds' });
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', store_id: 'store-downtown', items_summary: 'DT Order' }),
      makeOrder({ id: 'o-2', store_id: 'store-mall', items_summary: 'Mall Order' }),
      makeOrder({ id: 'o-3', store_id: null, items_summary: 'Legacy Order' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('DT Order')).toBeDefined();
      expect(screen.getByText('Legacy Order')).toBeDefined();
    });
    // Mall order should be filtered out
    expect(screen.queryByText('Mall Order')).toBeNull();
    // Header count should show 2 orders
    const countEl = document.querySelector('.kds-order-count');
    expect(countEl?.textContent).toMatch(/2/);
  });

  // ── 2d: Keyboard shortcuts tests ─────────────────────────────────

  it('selects the first order when pressing key 1', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101 }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102 }),
    ]);
    mockUpdateKdsStatus.mockResolvedValue({});
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    const kds = document.querySelector('.kds')!;
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: '1', bubbles: true }));

    // First ticket should have selected class
    await waitFor(() => {
      const tickets = document.querySelectorAll('.kds-ticket');
      expect(tickets[0]?.classList.contains('kds-ticket--selected')).toBe(true);
      expect(tickets[1]?.classList.contains('kds-ticket--selected')).toBe(false);
    });
  });

  it('selects the second order when pressing key 2', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101 }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102 }),
    ]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    const kds = document.querySelector('.kds')!;
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: '2', bubbles: true }));

    await waitFor(() => {
      const tickets = document.querySelectorAll('.kds-ticket');
      expect(tickets[0]?.classList.contains('kds-ticket--selected')).toBe(false);
      expect(tickets[1]?.classList.contains('kds-ticket--selected')).toBe(true);
    });
  });

  it('advances selected order when pressing Space', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending' }),
    ]);
    mockUpdateKdsStatus.mockResolvedValue({});
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    const kds = document.querySelector('.kds')!;
    // Select the first order
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: '1', bubbles: true }));
    // Then advance it with Space
    await waitFor(() => {
      expect(document.querySelector('.kds-ticket--selected')).not.toBeNull();
    });
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));

    await waitFor(() =>
      expect(mockUpdateKdsStatus).toHaveBeenCalledWith('o-1', 'preparing'),
    );
  });

  it('deselects on Escape key', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending' }),
    ]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    const kds = document.querySelector('.kds')!;
    // Select first order then deselect
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: '1', bubbles: true }));
    await waitFor(() => {
      expect(document.querySelector('.kds-ticket--selected')).not.toBeNull();
    });
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await waitFor(() => {
      expect(document.querySelector('.kds-ticket--selected')).toBeNull();
    });
  });

  // ── 3a: Zone-switching tests ───────────────────────────────────

  it('shows zone chips when orders have kitchen zones', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101, kitchen_zone: 'Grill' }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102, kitchen_zone: 'Fry' }),
    ]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    // Should show "All" chip and both zone chips
    expect(screen.getByText('All')).toBeDefined();
    expect(screen.getByText('Grill')).toBeDefined();
    expect(screen.getByText('Fry')).toBeDefined();
  });

  it('hides zone chips when no orders have kitchen zones', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', kitchen_zone: null }),
    ]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    // "All" chip should NOT be shown since the zone bar is hidden entirely
    expect(screen.queryByText('All')).toBeNull();
  });

  it('activates clicked zone chip', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101, kitchen_zone: 'Grill' }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102, kitchen_zone: 'Fry' }),
    ]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    // Click the "Grill" zone chip
    await userEvent.click(screen.getByText('Grill'));

    // The Grill chip should have the active class (zones sorted: Fry, Grill → index 2)
    await waitFor(() => {
      const chips = document.querySelectorAll('.kds-zone-chip');
      expect(chips[0]?.classList.contains('kds-zone-chip--active')).toBe(false);
      expect(chips[2]?.classList.contains('kds-zone-chip--active')).toBe(true);
    });
  });

  it('resets to All zone when All chip is clicked', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101, kitchen_zone: 'Grill' }),
    ]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    // First click Grill to activate a specific zone
    await userEvent.click(screen.getByText('Grill'));
    await waitFor(() => {
      expect(screen.getByText('Grill').closest('.kds-zone-chip')?.classList.contains('kds-zone-chip--active')).toBe(true);
    });

    // Then click All to reset
    await userEvent.click(screen.getByText('All'));
    await waitFor(() => {
      expect(screen.getByText('All').closest('.kds-zone-chip')?.classList.contains('kds-zone-chip--active')).toBe(true);
      expect(screen.getByText('Grill').closest('.kds-zone-chip')?.classList.contains('kds-zone-chip--active')).toBe(false);
    });
  });

  // ── 3a: Zone-switching filtering tests ──────────────────────
  it('shows only Grill orders when Grill zone is selected', async () => {
    const allOrders = [
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101, kitchen_zone: 'Grill' }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102, kitchen_zone: 'Fry' }),
      makeOrder({ id: 'o-3', status: 'pending', display_number: 103, kitchen_zone: 'Grill' }),
    ];
    mockGetKdsQueue.mockResolvedValue(allOrders);

    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    // Click the Grill zone chip
    await userEvent.click(screen.getByText('Grill'));
    await waitFor(() => {
      expect(screen.getByText('Grill').closest('.kds-zone-chip')?.classList.contains('kds-zone-chip--active')).toBe(true);
    });

    // Should only show Grill orders (o-1 and o-3)
    expect(screen.getByText('#101')).toBeInTheDocument();
    expect(screen.queryByText('#102')).not.toBeInTheDocument(); // Fry order should be hidden
    expect(screen.getByText('#103')).toBeInTheDocument();
  });

  it('shows only Fry orders when Fry zone is selected', async () => {
    const allOrders = [
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101, kitchen_zone: 'Grill' }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102, kitchen_zone: 'Fry' }),
      makeOrder({ id: 'o-3', status: 'pending', display_number: 103, kitchen_zone: 'Grill' }),
    ];
    mockGetKdsQueue.mockResolvedValue(allOrders);

    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    // Click the Fry zone chip
    await userEvent.click(screen.getByText('Fry'));
    await waitFor(() => {
      expect(screen.getByText('Fry').closest('.kds-zone-chip')?.classList.contains('kds-zone-chip--active')).toBe(true);
    });

    // Should only show Fry orders (o-2)
    expect(screen.queryByText('#101')).not.toBeInTheDocument(); // Grill order should be hidden
    expect(screen.getByText('#102')).toBeInTheDocument();
    expect(screen.queryByText('#103')).not.toBeInTheDocument(); // Grill order should be hidden
  });

  it('shows all orders when All zone is selected', async () => {
    const allOrders = [
      makeOrder({ id: 'o-1', status: 'pending', display_number: 101, kitchen_zone: 'Grill' }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102, kitchen_zone: 'Fry' }),
      makeOrder({ id: 'o-3', status: 'pending', display_number: 103, kitchen_zone: 'Grill' }),
    ];
    mockGetKdsQueue.mockResolvedValue(allOrders);

    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    // Click the Grill zone chip first
    await userEvent.click(screen.getByText('Grill'));
    await waitFor(() => {
      expect(screen.getByText('Grill').closest('.kds-zone-chip')?.classList.contains('kds-zone-chip--active')).toBe(true);
    });

    // Then click All to reset
    await userEvent.click(screen.getByText('All'));
    await waitFor(() => {
      expect(screen.getByText('All').closest('.kds-zone-chip')?.classList.contains('kds-zone-chip--active')).toBe(true);
    });

    // Should show all orders
    expect(screen.getByText('#101')).toBeInTheDocument();
    expect(screen.getByText('#102')).toBeInTheDocument();
    expect(screen.getByText('#103')).toBeInTheDocument();
  });

  // ── 2b: History view tests ─────────────────────────────────────

  it('renders history toggle button', async () => {
    mockGetKdsQueue.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const toggle = document.querySelector('.kds-history-toggle');
      expect(toggle).not.toBeNull();
    });
  });

  it('shows history panel when toggle is clicked', async () => {
    mockGetKdsQueue.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const toggle = document.querySelector('.kds-history-toggle');
      expect(toggle).not.toBeNull();
    });

    // Click the history toggle
    const toggle = document.querySelector('.kds-history-toggle') as HTMLButtonElement;
    await userEvent.click(toggle);

    // History panel should render
    await waitFor(() => {
      expect(document.querySelector('.kds-history')).not.toBeNull();
    });
  });

  // ── 3d: Voice callout tests ────────────────────────────────────

  it('announces order up via TTS when ticket advances to ready', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'preparing', display_number: 42 }),
    ]);
    mockUpdateKdsStatus.mockResolvedValue({});

    renderScreen();
    await waitFor(() => expect(screen.getByText('#42')).toBeDefined());

    const ticket = document.querySelector('.kds-ticket')!;
    await userEvent.click(ticket);

    await waitFor(() =>
      expect(mockUpdateKdsStatus).toHaveBeenCalledWith('o-1', 'ready'),
    );

    // Should have called speak with the TTS string
    await waitFor(() =>
      expect(mockSpeak).toHaveBeenCalledWith('Order 42 up!'),
    );
  });

  it('does not announce when advancing to preparing (only on ready)', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending', display_number: 42 }),
    ]);
    mockUpdateKdsStatus.mockResolvedValue({});

    renderScreen();
    await waitFor(() => expect(screen.getByText('#42')).toBeDefined());

    const ticket = document.querySelector('.kds-ticket')!;
    await userEvent.click(ticket);

    await waitFor(() =>
      expect(mockUpdateKdsStatus).toHaveBeenCalledWith('o-1', 'preparing'),
    );

    // speak should NOT have been called (not advancing to ready)
    expect(mockSpeak).not.toHaveBeenCalled();
  });

  it('navigates selection with ArrowDown and ArrowUp', async () => {
    mockGetKdsQueue.mockResolvedValue([
      makeOrder({ id: 'o-1', status: 'pending' }),
      makeOrder({ id: 'o-2', status: 'pending', display_number: 102 }),
      makeOrder({ id: 'o-3', status: 'pending', display_number: 103 }),
    ]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#101')).toBeDefined());

    const kds = document.querySelector('.kds')!;
    // Start at second order
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: '2', bubbles: true }));
    await waitFor(() => {
      const tickets = document.querySelectorAll('.kds-ticket');
      expect(tickets[1]?.classList.contains('kds-ticket--selected')).toBe(true);
    });

    // ArrowDown goes to third
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await waitFor(() => {
      const tickets = document.querySelectorAll('.kds-ticket');
      expect(tickets[2]?.classList.contains('kds-ticket--selected')).toBe(true);
    });

    // ArrowUp goes back to second
    kds.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }));
    await waitFor(() => {
      const tickets = document.querySelectorAll('.kds-ticket');
      expect(tickets[1]?.classList.contains('kds-ticket--selected')).toBe(true);
    });
  });

  it('a double-tap on the picker Confirm adds the items once (no duplicate merge)', async () => {
    mockGetKdsQueue.mockResolvedValue([makeOrder()]);
    mockGetKdsOrderLines.mockResolvedValue([]);
    mockListProducts.mockResolvedValue([
      {
        sku: 'ESPR', name: 'Espresso Shot', category: 'Hot Drinks',
        price: { minor_units: 15000, currency: 'IDR' }, barcode: '8990000000001',
        in_stock: true, stock_qty: 10, tax_rate_ids: [], created_at: '',
        price_updated_at: '', product_type: 'restaurant',
      },
    ]);
    // Defer the merge so the modal stays open with Confirm enabled — the
    // second tap lands while the first merge is still in flight.
    let release!: () => void;
    const gate = new Promise<void>((r) => { release = r; });
    mockUpdateKdsOrderItems.mockReturnValue(gate.then(() => ({ id: 'o-1' })));

    renderScreen();
    await waitFor(() => expect(screen.getByText('Burger x1, Fries x1')).toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: /add items to order/i }));
    await waitFor(() => expect(screen.getByText('Espresso Shot')).toBeInTheDocument());

    await userEvent.click(screen.getByRole('button', { name: /^espresso shot( \(added\))?$/i }));
    const confirmBtn = screen.getByRole('button', { name: /add .* item/i });
    await userEvent.click(confirmBtn);
    // The modal surfaces the in-flight save: Confirm disables until release.
    await waitFor(() => expect(confirmBtn).toBeDisabled());
    await userEvent.click(confirmBtn); // double-tap before the merge resolves

    release();
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    // The second tap must NOT re-merge the picked items onto the ticket.
    expect(mockUpdateKdsOrderItems).toHaveBeenCalledTimes(1);
  });
});
