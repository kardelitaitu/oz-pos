import { describe, expect, it, vi, beforeEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import type { KdsOrder } from '@/api/kds';

// ── Mock useSound ────────────────────────────────────────────────────

const mockPlayBeep = vi.fn();

vi.mock('@/frontend/shared/useSound', () => ({
  useSound: () => ({ playBeep: mockPlayBeep }),
}));

// We need to import AFTER setting up the mock.
const { useNewTicketSound } = await import('@/features/kds/hooks/useNewTicketSound');

// ── Helpers ──────────────────────────────────────────────────────────

function makeOrder(id: string): KdsOrder {
  return {
    id,
    status: 'pending',
    ticket_number: 1,
    order_type: 'dine_in',
    table_id: null,
    server_name: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    items: [],
    channel: 'pos',
  } as KdsOrder;
}

// ── Tests ────────────────────────────────────────────────────────────

describe('useNewTicketSound', () => {
  beforeEach(() => {
    mockPlayBeep.mockClear();
  });

  it('plays beep when a new ticket arrives', () => {
    const orders: KdsOrder[] = [makeOrder('o-1')];
    renderHook(() => useNewTicketSound(orders));
    expect(mockPlayBeep).toHaveBeenCalledTimes(1);
  });

  it('does not play beep for duplicate ticket', () => {
    const orders: KdsOrder[] = [makeOrder('o-1')];
    const { rerender } = renderHook(
      (props: { orders: KdsOrder[] }) => useNewTicketSound(props.orders),
      { initialProps: { orders } },
    );
    expect(mockPlayBeep).toHaveBeenCalledTimes(1);
    // Re-render with same orders.
    rerender({ orders: [makeOrder('o-1')] });
    expect(mockPlayBeep).toHaveBeenCalledTimes(1);
  });

  it('plays beep when a second distinct ticket arrives after debounce', async () => {
    const { rerender } = renderHook(
      (props: { orders: KdsOrder[] }) => useNewTicketSound(props.orders),
      { initialProps: { orders: [makeOrder('o-1')] } },
    );
    expect(mockPlayBeep).toHaveBeenCalledTimes(1);
    // Advance past debounce window.
    await new Promise((r) => setTimeout(r, 6000));
    rerender({ orders: [makeOrder('o-1'), makeOrder('o-2')] });
    expect(mockPlayBeep).toHaveBeenCalledTimes(2);
  });

  it('debounces rapid new tickets within 5 seconds', async () => {
    const { rerender } = renderHook(
      (props: { orders: KdsOrder[] }) => useNewTicketSound(props.orders),
      { initialProps: { orders: [makeOrder('o-1')] } },
    );
    expect(mockPlayBeep).toHaveBeenCalledTimes(1);
    // Advance only 2 seconds (within 5s debounce).
    await new Promise((r) => setTimeout(r, 2000));
    // Add two new tickets.
    rerender({ orders: [makeOrder('o-1'), makeOrder('o-2'), makeOrder('o-3')] });
    // Should NOT have played — still within debounce window.
    expect(mockPlayBeep).toHaveBeenCalledTimes(1);
  });

  it('does not play beep when enabled=false', () => {
    const orders: KdsOrder[] = [makeOrder('o-1')];
    renderHook(() => useNewTicketSound(orders, false));
    expect(mockPlayBeep).not.toHaveBeenCalled();
  });

  it('does not play beep for empty orders array', () => {
    renderHook(() => useNewTicketSound([]));
    expect(mockPlayBeep).not.toHaveBeenCalled();
  });
});
