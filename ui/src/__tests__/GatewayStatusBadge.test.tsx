import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import { GatewayStatusBadge } from '@/components/GatewayStatusBadge';

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(`
gateway-status-online-aria = { $name } is online
gateway-status-offline-aria = { $name } is offline
`));
const l10n = new ReactLocalization([bundle]);

function renderBadge(props: {
  gatewayName: string;
  isConfigured: boolean;
  isOnline: boolean;
}) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <GatewayStatusBadge {...props} />
    </LocalizationProvider>,
  );
}

describe('GatewayStatusBadge', () => {
  it('renders null when not configured', () => {
    const { container } = renderBadge({
      gatewayName: 'Stripe',
      isConfigured: false,
      isOnline: false,
    });
    expect(container.innerHTML).toBe('');
  });

  it('renders the gateway name when configured', () => {
    renderBadge({
      gatewayName: 'Stripe',
      isConfigured: true,
      isOnline: true,
    });
    expect(screen.getByText('Stripe')).toBeInTheDocument();
  });

  it('has role="status" when configured', () => {
    renderBadge({
      gatewayName: 'Square',
      isConfigured: true,
      isOnline: true,
    });
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('shows online dot when online', () => {
    renderBadge({
      gatewayName: 'Stripe',
      isConfigured: true,
      isOnline: true,
    });
    const dot = document.querySelector('.gateway-badge__dot');
    expect(dot).toHaveClass('online');
    expect(dot).not.toHaveClass('offline');
  });

  it('shows offline dot when offline', () => {
    renderBadge({
      gatewayName: 'Square',
      isConfigured: true,
      isOnline: false,
    });
    const dot = document.querySelector('.gateway-badge__dot');
    expect(dot).toHaveClass('offline');
    expect(dot).not.toHaveClass('online');
  });

  it('has correct aria-label for online status', () => {
    renderBadge({
      gatewayName: 'Midtrans',
      isConfigured: true,
      isOnline: true,
    });
    expect(screen.getByRole('status')).toHaveAttribute(
      'aria-label',
      'Midtrans is online',
    );
  });

  it('has correct aria-label for offline status', () => {
    renderBadge({
      gatewayName: 'GoPay',
      isConfigured: true,
      isOnline: false,
    });
    expect(screen.getByRole('status')).toHaveAttribute(
      'aria-label',
      'GoPay is offline',
    );
  });
});
