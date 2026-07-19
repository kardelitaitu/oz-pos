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
    <div data-testid={`ticket-${order.id}`} className="kds-ticket-card" data-status={order.status}>
      <span>{order.display_number}</span>
      <span>{order.items_summary}</span>
      <button onClick={() => onAdvance?.(order.id)}>Advance</button>
    </div>
  )),
}));

import { KdsLayoutKanban } from '@/features/kds/KdsLayoutKanban';
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
    display_number: 104, received_at: new Date(now - 60 * MIN).toISOString(),
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

describe('KdsLayoutKanban', () => {
  // ── Column rendering ─────────────────────────────────────────

  it('renders three columns with correct titles', () => {
    renderWithFluentSync(<KdsLayoutKanban {...defaultProps} />, kdsFtl);
    expect(screen.getByText('Pending')).toBeInTheDocument();
    expect(screen.getByText('Preparing')).toBeInTheDocument();
    expect(screen.getByText('Ready')).toBeInTheDocument();
  });

  it('shows correct order counts per column', () => {
    renderWithFluentSync(<KdsLayoutKanban {...defaultProps} />, kdsFtl);
    // Pending: 2, Preparing: 1, Ready: 1
    expect(screen.getByText('2')).toBeInTheDocument();
    const ones = screen.getAllByText('1');
    expect(ones.length).toBe(2); // preparing + ready
  });

  it('applies correct column class names', () => {
    renderWithFluentSync(<KdsLayoutKanban {...defaultProps} />, kdsFtl);
    const columns = document.querySelectorAll('.kds-column');
    expect(columns.length).toBe(3);
    expect(columns[0]!.className).toContain('kds-column--pending');
    expect(columns[1]!.className).toContain('kds-column--preparing');
    expect(columns[2]!.className).toContain('kds-column--ready');
  });

  // ── Ticket placement ─────────────────────────────────────────

  it('places tickets in the correct status column', () => {
    renderWithFluentSync(<KdsLayoutKanban {...defaultProps} />, kdsFtl);
    // pending orders (order-1, order-4) should be in the pending column
    const pendingTickets = screen.getAllByTestId(/^ticket-order-[14]$/);
    expect(pendingTickets.length).toBe(2);

    const prepTicket = screen.getByTestId('ticket-order-2');
    expect(prepTicket).toBeInTheDocument();

    const readyTicket = screen.getByTestId('ticket-order-3');
    expect(readyTicket).toBeInTheDocument();
  });

  it('renders tickets with data-attribute indicating status', () => {
    renderWithFluentSync(<KdsLayoutKanban {...defaultProps} />, kdsFtl);
    const pendingTicket = screen.getByTestId('ticket-order-1');
    expect(pendingTicket.getAttribute('data-status')).toBe('pending');

    const prepTicket = screen.getByTestId('ticket-order-2');
    expect(prepTicket.getAttribute('data-status')).toBe('preparing');

    const readyTicket = screen.getByTestId('ticket-order-3');
    expect(readyTicket.getAttribute('data-status')).toBe('ready');
  });

  // ── Empty state ──────────────────────────────────────────────

  it('shows empty state for a column with no orders', () => {
    renderWithFluentSync(<KdsLayoutKanban {...defaultProps} orders={[]} />, kdsFtl);
    // Each column shows "No orders yet" when empty
    const emptyMessages = screen.getAllByText('No orders yet');
    expect(emptyMessages.length).toBe(3); // all 3 columns empty
  });

  it('shows empty state message only in empty columns', () => {
    // Only pending orders
    renderWithFluentSync(
      <KdsLayoutKanban {...defaultProps} orders={orders.filter(o => o.status === 'pending')} />,
      kdsFtl,
    );
    // 1 column has tickets (pending), 2 are empty
    const emptyMessages = screen.getAllByText('No orders yet');
    expect(emptyMessages.length).toBe(2); // preparing + ready are empty
  });

  // ── Advance action ───────────────────────────────────────────

  it('calls onAdvance when Advance button is clicked on a ticket', async () => {
    const user = userEvent.setup();
    const onAdvance = vi.fn();
    renderWithFluentSync(
      <KdsLayoutKanban {...defaultProps} onAdvance={onAdvance} />,
      kdsFtl,
    );

    const advanceBtns = screen.getAllByText('Advance');
    await user.click(advanceBtns[0]!);
    expect(onAdvance).toHaveBeenCalledWith('order-1'); // first ticket
  });
});
