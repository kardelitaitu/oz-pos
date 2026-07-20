import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import KdsScreen from '@/features/kds/KdsScreen';
import kdsFtl from '@/locales/kds.ftl?raw';
import type { KdsOrder } from '@/api/kds';

const { mockGetKdsQueue, mockUpdateKdsStatus, mockUseTicketSla, mockPlayAlert, mockUseWorkspaceScope } = vi.hoisted(() => ({
  mockGetKdsQueue: vi.fn(),
  mockUpdateKdsStatus: vi.fn(),
  mockUseTicketSla: vi.fn((): { level: 'green' | 'yellow' | 'red'; elapsedSeconds: number; display: string } => ({
    level: 'green',
    elapsedSeconds: 120,
    display: '2m 0s',
  })),
  mockPlayAlert: vi.fn(),
  mockUseWorkspaceScope: vi.fn<() => { storeId: string; instanceId: string; typeKey: string } | null>(() => null),
}));

vi.mock('@/api/kds', () => ({
  getKdsQueue: (_userId: string) => mockGetKdsQueue(),
  updateKdsStatus: (_userId: string, id: string, status: string) => mockUpdateKdsStatus(id, status),
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
    setSoundEnabled: vi.fn(),
  }),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspaceScope: () => mockUseWorkspaceScope(),
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
    ...overrides,
  };
}

describe('KdsScreen', () => {
  beforeEach(() => {
    mockGetKdsQueue.mockResolvedValue([]);
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

  it('displays error when getKdsQueue fails', async () => {
    mockGetKdsQueue.mockRejectedValue(new Error('Network down'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Network down')).toBeDefined());
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
      // All three columns should be empty
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
});
