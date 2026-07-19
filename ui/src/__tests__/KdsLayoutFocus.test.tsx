import { describe, expect, it, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import kdsFtl from '@/locales/kds.ftl?raw';

vi.mock('@/features/kds/components/KdsTicketCard', () => ({
  KdsTicketCard: vi.fn(({ order, onAdvance }: {
    order: { id: string; status: string; display_number: number | null; items_summary: string; item_count: number };
    onAdvance?: (id: string) => void;
  }) => (
    <div data-testid={`ticket-${order.id}`} className="kds-ticket-card">
      <span>{order.display_number}</span>
      <span>{order.items_summary}</span>
      <span data-testid="item-count">{order.item_count}</span>
      <button onClick={() => onAdvance?.(order.id)}>Advance</button>
    </div>
  )),
}));

import { KdsLayoutFocus } from '@/features/kds/KdsLayoutFocus';
import type { KdsOrder } from '@/api/kds';

// ── Test data ─────────────────────────────────────────────────────

const now = Date.now();
const MIN = 60000;

const orders: KdsOrder[] = [
  {
    id: 'order-1', sale_id: 'sale-1', store_id: null,
    status: 'pending', items_summary: 'Coffee x2', item_count: 2,
    display_number: 101, received_at: new Date(now - 30 * MIN).toISOString(),
    started_at: null, ready_at: null, served_at: null,
    prep_time_seconds: 0, kitchen_zone: null, notes: '',
  },
  {
    id: 'order-2', sale_id: 'sale-2', store_id: null,
    status: 'preparing', items_summary: 'Tea x1, Toast x1', item_count: 2,
    display_number: 102, received_at: new Date(now - 15 * MIN).toISOString(),
    started_at: new Date(now - 12 * MIN).toISOString(), ready_at: null, served_at: null,
    prep_time_seconds: 180, kitchen_zone: null, notes: '',
  },
  {
    id: 'order-3', sale_id: 'sale-3', store_id: null,
    status: 'ready', items_summary: 'Burger x1', item_count: 1,
    display_number: 103, received_at: new Date(now - 5 * MIN).toISOString(),
    started_at: new Date(now - 10 * MIN).toISOString(), ready_at: new Date(now - 2 * MIN).toISOString(),
    served_at: null, prep_time_seconds: 480, kitchen_zone: null, notes: '',
  },
  {
    id: 'order-4', sale_id: 'sale-4', store_id: null,
    status: 'pending', items_summary: 'Pancake x3', item_count: 3,
    display_number: 104, received_at: new Date(now - 60 * MIN).toISOString(), // oldest
    started_at: null, ready_at: null, served_at: null,
    prep_time_seconds: 0, kitchen_zone: null, notes: '',
  },
];

const defaultProps = {
  orders,
  onAdvance: vi.fn(),
  showOrderId: true,
  showTableNumber: true,
};

describe('KdsLayoutFocus', () => {
  // ── Default rendering ─────────────────────────────────────────

  it('renders filter buttons for all statuses', () => {
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} />, kdsFtl);
    // kds-filter-all has no Fluent key, uses fallback 'all' (lowercase)
    expect(screen.getByText('all')).toBeInTheDocument();
    expect(screen.getByText('Pending')).toBeInTheDocument();
    expect(screen.getByText('Preparing')).toBeInTheDocument();
    expect(screen.getByText('Ready')).toBeInTheDocument();
  });

  it('shows counts on filter buttons', () => {
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} />, kdsFtl);
    // 4 total, 2 pending, 1 preparing, 1 ready
    expect(screen.getByText('4')).toBeInTheDocument();
    // Pending count = 2
    const allPendingCounts = screen.getAllByText('2');
    expect(allPendingCounts.length).toBeGreaterThanOrEqual(1);
    // '1' appears 3 times: preparing count, ready count, and order-3 item_count
    const ones = screen.getAllByText('1');
    expect(ones.length).toBe(3);
  });

  // ── Sorting by urgency ────────────────────────────────────────

  it('sorts orders by urgency (oldest first)', () => {
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} />, kdsFtl);
    const tickets = screen.getAllByTestId(/^ticket-/);
    // Sorted by SLA weight descending: oldest received_at first
    // order-4 (60min ago) should be first, order-1 (30min), order-2 (15min), order-3 (5min)
    expect(tickets[0]!.getAttribute('data-testid')).toBe('ticket-order-4');
    expect(tickets[1]!.getAttribute('data-testid')).toBe('ticket-order-1');
    expect(tickets[2]!.getAttribute('data-testid')).toBe('ticket-order-2');
    expect(tickets[3]!.getAttribute('data-testid')).toBe('ticket-order-3');
  });

  // ── Filtering ─────────────────────────────────────────────────

  it('filters by pending status when Pending button is clicked', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} />, kdsFtl);
    await user.click(screen.getByText('Pending'));
    const tickets = screen.getAllByTestId(/^ticket-/);
    expect(tickets.length).toBe(2); // order-1 and order-4 (both pending)
  });

  it('filters by preparing status', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} />, kdsFtl);
    await user.click(screen.getByText('Preparing'));
    const tickets = screen.getAllByTestId(/^ticket-/);
    expect(tickets.length).toBe(1); // order-2
  });

  it('filters by ready status', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} />, kdsFtl);
    await user.click(screen.getByText('Ready'));
    const tickets = screen.getAllByTestId(/^ticket-/);
    expect(tickets.length).toBe(1); // order-3
  });

  // ── Empty state ───────────────────────────────────────────────

  it('shows empty state when no orders array is empty', () => {
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} orders={[]} />, kdsFtl);
    expect(screen.getByText('No orders yet')).toBeInTheDocument();
  });

  // ── Active filter styling ─────────────────────────────────────

  it('applies active class to the current filter button', async () => {
    const user = userEvent.setup();
    renderWithFluentSync(<KdsLayoutFocus {...defaultProps} />, kdsFtl);

    // "all" (lowercase, Fluent fallback) should be active by default
    const allBtn = screen.getByText('all').closest('button')!;
    expect(allBtn.className).toContain('--active');

    // Click Pending
    await user.click(screen.getByText('Pending'));
    expect(screen.getByText('Pending').closest('button')!.className).toContain('--active');
    expect(allBtn.className).not.toContain('--active');
  });
});
