import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { screen, render } from '@testing-library/react';
import { KdsTicketCard } from '@/features/kds/components/KdsTicketCard';
import kdsFtl from '@/locales/kds.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import { withFluent } from '@/locales/test-utils';
import type { KdsOrder } from '@/api/kds';

const mockPlayAlert = vi.fn();
const mockSlaResult = { level: 'green' as const, display: '0s', elapsedSeconds: 0 };

vi.mock('@/features/kds/hooks/useTicketSla', () => ({
  useTicketSla: () => mockSlaResult,
}));

vi.mock('@/frontend/shared/useSound', () => ({
  useSound: () => ({ playAlert: mockPlayAlert }),
}));

beforeEach(() => {
  mockPlayAlert.mockReset();
  mockSlaResult.level = 'green';
  mockSlaResult.display = '0s';
});

afterEach(() => {
  mockPlayAlert.mockReset();
});

const baseOrder: KdsOrder = {
  id: 'order-1',
  sale_id: 'sale-1',
  store_id: null,
  status: 'pending',
  items_summary: '2x Nasi Goreng, 1x Es Teh',
  item_count: 3,
  display_number: 42,
  received_at: new Date().toISOString(),
  started_at: null,
  ready_at: null,
  served_at: null,
  prep_time_seconds: 0,
  notes: '',
};

function renderCard(order: Partial<KdsOrder> = {}) {
  const merged = { ...baseOrder, ...order };
  return render(withFluent(<KdsTicketCard order={merged} onAdvance={onAdvance} />, sharedFtl, kdsFtl));
}

const onAdvance = vi.fn();

describe('KdsTicketCard', () => {
  it('renders display number', () => {
    renderCard();
    expect(screen.getByText('#42')).toBeTruthy();
  });

  it('renders items summary', () => {
    renderCard();
    expect(screen.getByText('2x Nasi Goreng, 1x Es Teh')).toBeTruthy();
  });

  it('renders item count', () => {
    renderCard();
    expect(screen.getByText('3 items')).toBeTruthy();
  });

  it('shows SLA time', () => {
    mockSlaResult.display = '5m 30s';
    renderCard();
    expect(screen.getByText('5m 30s')).toBeTruthy();
  });

  it('shows notes when present', () => {
    renderCard({ notes: 'No onion please' });
    expect(screen.getByText('No onion please')).toBeTruthy();
  });

  it('does not show notes when empty', () => {
    const { container } = renderCard({ notes: '' });
    expect(container.querySelector('.kds-ticket-notes')).toBeNull();
  });

  it('sets level class on the ticket', () => {
    mockSlaResult.level = 'red';
    const { container } = renderCard();
    const ticket = container.querySelector('.kds-ticket');
    expect(ticket?.className).toContain('kds-ticket--red');
  });

  it('calls onAdvance on click when status can advance', () => {
    renderCard({ status: 'pending' });
    screen.getByRole('button').click();
    expect(onAdvance).toHaveBeenCalledWith(expect.objectContaining({ id: 'order-1' }));
  });

  it('does not call onAdvance for served orders', () => {
    renderCard({ status: 'served' });
    screen.getByRole('button').click();
    expect(onAdvance).not.toHaveBeenCalled();
  });

  it('does not call onAdvance for cancelled orders', () => {
    renderCard({ status: 'cancelled' });
    screen.getByRole('button').click();
    expect(onAdvance).not.toHaveBeenCalled();
  });

  it('plays alert when transitioning to red', () => {
    const { rerender } = renderCard();
    expect(mockPlayAlert).not.toHaveBeenCalled();

    mockSlaResult.level = 'red';
    rerender(
      withFluent(
        <KdsTicketCard order={{ ...baseOrder, notes: 'trigger' }} onAdvance={onAdvance} />,
        sharedFtl, kdsFtl,
      ),
    );

    expect(mockPlayAlert).toHaveBeenCalledTimes(1);
  });

  it('does not play alert on first render', () => {
    renderCard();
    expect(mockPlayAlert).not.toHaveBeenCalled();
  });

  it('sets aria-label with SLA info', () => {
    mockSlaResult.level = 'yellow';
    mockSlaResult.display = '12m 0s';
    renderCard();
    const btn = screen.getByRole('button');
    expect(btn.getAttribute('aria-label')).toContain('42');
    expect(btn.getAttribute('aria-label')).toContain('yellow');
    expect(btn.getAttribute('aria-label')).toContain('12m 0s');
  });
});
