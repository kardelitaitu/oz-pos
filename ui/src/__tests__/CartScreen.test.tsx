import { describe, expect, it, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import CartScreen from '@/features/sales/CartScreen';
import type { CartLine, Money, Sku, LineId } from '@/types/domain';

describe('CartScreen', () => {
  it('renders the empty state', () => {
        renderWithFluentSync(<CartScreen />, salesFtl);
    expect(screen.getByRole('heading', { name: /cart/i })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent(/empty/i);
  });

  it('renders a line with formatted money', () => {
    const usd: Money = { minor_units: 350, currency: 'USD' };
    const line: CartLine = {
      id: 'line-1' as LineId,
      sku: 'COFFEE' as Sku,
      name: 'Coffee',
      qty: 2,
      unit_price: usd,
    };
        renderWithFluentSync(<CartScreen lines={[line]} total={usd} />, salesFtl);
    expect(screen.getByText(/COFFEE/)).toBeInTheDocument();
    // formatMoney uses id-ID locale by default → $ 3,50
    expect(screen.getAllByText(/\$ 3,50/)).toHaveLength(2);
  });

  it('invokes the onAddSample callback', async () => {
    const handler = vi.fn();
        renderWithFluentSync(<CartScreen onAddSample={handler} />, salesFtl);
    await userEvent.click(screen.getByRole('button'));
    expect(handler).toHaveBeenCalledTimes(1);
  });
});
