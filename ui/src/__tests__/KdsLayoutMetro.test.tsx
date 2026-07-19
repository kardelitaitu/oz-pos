import { describe, expect, it, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import kdsFtl from '@/locales/kds.ftl?raw';

vi.mock('@/features/kds/components/KdsTicketCard', () => ({
  KdsTicketCard: vi.fn(({ order, onAdvance }: {
    order: { id: string; display_number: number | null; items_summary: string; item_count: number; status: string };
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

import { KdsLayoutMetro } from '@/features/kds/KdsLayoutMetro';
import type { KdsOrder } from '@/api/kds';

const now = Date.now();
const MIN = 60000;

const orders: KdsOrder[] = [
  {
    id: 'order-1', sale_id: 'sale-1', store_id: null, status: 'pending',
    items_summary: 'Coffee x2', item_count: 2, display_number: 101,
    received_at: new Date(now - 30 * MIN).toISOString(),
    started_at: null, ready_at: null, served_at: null,
    prep_time_seconds: 0, kitchen_zone: null, notes: '',
  },
  {
    id: 'order-2', sale_id: 'sale-2', store_id: null, status: 'preparing',
    items_summary: 'Tea x1', item_count: 1, display_number: 102,
    received_at: new Date(now - 15 * MIN).toISOString(),
    started_at: new Date(now - 12 * MIN).toISOString(), ready_at: null, served_at: null,
    prep_time_seconds: 180, kitchen_zone: null, notes: '',
  },
  {
    id: 'order-3', sale_id: 'sale-3', store_id: null, status: 'ready',
    items_summary: 'Burger x1', item_count: 1, display_number: 103,
    received_at: new Date(now - 5 * MIN).toISOString(),
    started_at: new Date(now - 10 * MIN).toISOString(), ready_at: new Date(now - 2 * MIN).toISOString(),
    served_at: null, prep_time_seconds: 480, kitchen_zone: null, notes: '',
  },
];

const defaultProps = {
  orders,
  onAdvance: vi.fn(),
  showOrderId: true,
  showTableNumber: true,
};

describe('KdsLayoutMetro', () => {
  // ── Rendering ─────────────────────────────────────────────────

  it('renders a metro tile for each order', () => {
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} />, kdsFtl);
    const tickets = screen.getAllByTestId(/^ticket-/);
    expect(tickets.length).toBe(3);
  });

  it('renders ticket with display number and summary text', () => {
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} />, kdsFtl);
    expect(screen.getByText('101')).toBeInTheDocument();
    expect(screen.getByText('Coffee x2')).toBeInTheDocument();
    expect(screen.getByText('102')).toBeInTheDocument();
    expect(screen.getByText('Tea x1')).toBeInTheDocument();
    expect(screen.getByText('103')).toBeInTheDocument();
    expect(screen.getByText('Burger x1')).toBeInTheDocument();
  });

  it('renders all orders in the metro layout container', () => {
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} />, kdsFtl);
    const container = document.querySelector('.kds-metro');
    expect(container).toBeInTheDocument();
    const tickets = container!.querySelectorAll('.kds-ticket-card');
    expect(tickets.length).toBe(3);
  });

  // ── Item count ────────────────────────────────────────────────

  it('shows item count for each ticket', () => {
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} />, kdsFtl);
    const counts = screen.getAllByTestId('item-count');
    expect(counts.length).toBe(3);
    expect(counts[0]!.textContent).toBe('2');
    expect(counts[1]!.textContent).toBe('1');
    expect(counts[2]!.textContent).toBe('1');
  });

  // ── Empty state ───────────────────────────────────────────────

  it('shows empty state when no orders', () => {
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} orders={[]} />, kdsFtl);
    expect(screen.getByText('No orders yet')).toBeInTheDocument();
  });

  it('does not show empty state when orders exist', () => {
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} />, kdsFtl);
    expect(screen.queryByText('No orders yet')).not.toBeInTheDocument();
  });

  // ── Advance action ────────────────────────────────────────────

  it('calls onAdvance with order id when Advance button is clicked', async () => {
    const user = userEvent.setup();
    const onAdvance = vi.fn();
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} onAdvance={onAdvance} />, kdsFtl);

    const advanceBtns = screen.getAllByText('Advance');
    await user.click(advanceBtns[0]!);
    expect(onAdvance).toHaveBeenCalledWith('order-1');
  });

  it('calls onAdvance for each ticket independently', async () => {
    const user = userEvent.setup();
    const onAdvance = vi.fn();
    renderWithFluentSync(<KdsLayoutMetro {...defaultProps} onAdvance={onAdvance} />, kdsFtl);

    const advanceBtns = screen.getAllByText('Advance');
    await user.click(advanceBtns[1]!);
    expect(onAdvance).toHaveBeenCalledWith('order-2');
    await user.click(advanceBtns[2]!);
    expect(onAdvance).toHaveBeenCalledWith('order-3');
  });
});
