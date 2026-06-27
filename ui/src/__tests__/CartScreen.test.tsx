import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import CartScreen from '@/features/sales/CartScreen';
import type { CartLine, Money, Sku, LineId } from '@/types/domain';

const wrap = (children: React.ReactNode) => {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource('cart-title = Cart\ncart-empty = Cart is empty\ncart-total-label = Total\nsale-pay-button = Pay\n'));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

describe('CartScreen', () => {
  it('renders the empty state', () => {
    render(wrap(<CartScreen />));
    expect(screen.getByRole('heading', { name: /cart/i })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent(/empty/i);
  });

  it('renders a line with formatted money', () => {
    const usd: Money = { minor_units: 350, currency: 'USD' };
    const line: CartLine = {
      id: 'line-1' as LineId,
      sku: 'COFFEE' as Sku,
      qty: 2,
      unit_price: usd,
    };
    render(wrap(<CartScreen lines={[line]} total={usd} />));
    expect(screen.getByText(/COFFEE/)).toBeInTheDocument();
    expect(screen.getAllByText(/\$3\.50/)).toHaveLength(2);
  });

  it('invokes the onAddSample callback', async () => {
    const handler = vi.fn();
    render(wrap(<CartScreen onAddSample={handler} />));
    await userEvent.click(screen.getByRole('button'));
    expect(handler).toHaveBeenCalledTimes(1);
  });
});
