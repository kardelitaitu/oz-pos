// ── RefundModal exit-animation tests ──────────────────────────────
//
// Pins the contract for the layered (overlay + modal) exit added in
// the sibling-surfaces polish. Both the overlay and the modal
// container must fade in parallel — the overlay goes `opacity
// 1 → 0` and the modal goes `translate + scale 1 → 0.98 + opacity
// 1 → 0`. The Cancel × button, the "Cancel" footer button, and the
// Done button all flow through the same requestClose() →
// 200 ms → onClose() path.
//
// Test consumer pattern: a real React component (HostModal) drives
// the modal's `open` state, with a mutable `hostRef` so the test
// can peek at state synchronously WITHOUT calling React hooks
// outside a render context.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useState } from 'react';
import { render, fireEvent, screen } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import RefundModal from '@/features/sales/RefundModal';

import {
  ExitAnimHost,
  createExitAnimHostRef,
  advanceFadeSync,
  expectExiting,
  expectNotExiting,
} from './test-utils/exitAnimHost';
import type { ExitAnimHostRef } from './test-utils/exitAnimHost';

// ── Mock @/api/sales.processRefund so the async submit is deterministic ──

vi.mock('@/api/sales', async () => {
  const actual = await vi.importActual<typeof import('@/api/sales')>('@/api/sales');
  return {
    ...actual,
    processRefund: vi.fn(
      async (req: { saleId: string; lines: unknown[]; userId: string }) => ({
        refundId: 'refund-1',
        totalMinor: 12345,
        saleId: req.saleId,
        lineCount: req.lines.length,
      }),
    ),
  };
});

// ── Mock @/contexts/AuthContext so useAuth() doesn't throw ────────
//
// RefundModal destructures `{ session }` from useAuth() to call the
// refund API with the current user's id. We provide a minimal stub
// session so the component renders without an <AuthProvider> wrapper.

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: {
      user_id: 'user-1',
      username: 'testuser',
      role_name: 'cashier',
      token: 'mock-token',
      role_id: 'role-1',
    },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager: false,
    isOwner: false,
  }),
}));

const wrapper = ({ children }: { children: React.ReactNode }) => {
  const ftl = `
refund-dialog-aria = Process refund
refund-title = Process refund
refund-close-aria = Cancel refund
refund-cancel = Cancel
refund-done-title = Refund processed
refund-done-amount = Refunded: { $amount }
refund-done = Done
refund-sale-id = Sale { $id }
refund-sale-total = Total: { $amount }
refund-sale-date = Date: { $date }
refund-items-title = Select items
refund-reason-label = Reason
refund-reason-placeholder = Why are you refunding?
refund-reason-aria = Refund reason
refund-note-label = Note
refund-note-placeholder = Internal note (optional)
refund-note-aria = Refund note
refund-error = Refund failed
refund-item-aria = Refund { $sku }
refund-qty-decrease-aria = Decrease quantity
refund-qty-increase-aria = Increase quantity
refund-total-label = Refund total
refund-submit = Process refund
`;
  const bundle = new FluentBundle('en');
  bundle.addResource(new FluentResource(ftl));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

const fakeSale = {
  id: 'sale-abcdef1234567890',
  createdAt: '2025-01-01T00:00:00Z',
  total: { minor_units: 60000, currency: 'USD' },
  subtotal: { minor_units: 60000, currency: 'USD' },
  lines: [
    {
      id: 'line-1',
      sku: 'ITEM-001',
      name: 'Item 1',
      qty: 2,
      total_minor: 8000,
      unitPriceMinor: 4000,
    },
    {
      id: 'line-2',
      sku: 'ITEM-002',
      name: 'Item 2',
      qty: 1,
      total_minor: 2000,
      unitPriceMinor: 2000,
    },
  ],
};

// ── Mutable host-state ref (extends shared ExitAnimHostRef) ───────

interface HostRef extends ExitAnimHostRef {
  refunded: boolean;
  setRefunded: (v: boolean) => void;
}

function makeHostRef(): HostRef {
  return { ...createExitAnimHostRef(), refunded: false, setRefunded: () => {} };
}

function HostModal({
  initialOpen = true,
  hostRef,
}: {
  initialOpen?: boolean;
  hostRef: HostRef;
}) {
  const [refunded, setRefunded] = useState(false);
  hostRef.refunded = refunded;
  hostRef.setRefunded = setRefunded;
  return (
    <ExitAnimHost hostRef={hostRef} initialOpen={initialOpen}>
      {(open, setOpen) => (
        <>
          <span data-testid="open-state">{String(open)}</span>
          <span data-testid="refunded-state">{String(refunded)}</span>
          <RefundModal
            open={open}
            sale={fakeSale as never}
            onClose={() => setOpen(false)}
            onRefunded={() => setRefunded(true)}
          />
        </>
      )}
    </ExitAnimHost>
  );
}

describe('RefundModal exit-animation polish', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not render when closed initially', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} initialOpen={false} />, { wrapper });
    expect(document.querySelector('.refund-overlay')).toBeNull();
  });

  it('renders the overlay + modal when open', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />, { wrapper });
    const overlay = document.querySelector('.refund-overlay');
    const modal = document.querySelector('.refund-modal');
    expect(overlay).toBeTruthy();
    expect(modal).toBeTruthy();
    expectNotExiting(overlay, 'refund-overlay');
    expectNotExiting(modal, 'refund-modal');
  });

  it('× button applies BOTH overlay and modal --exiting classes (layered exit), then fades', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />, { wrapper });

    fireEvent.click(document.querySelector('.refund-close') as HTMLElement);

    const overlay = document.querySelector('.refund-overlay');
    const modal = document.querySelector('.refund-modal');
    expectExiting(overlay, 'refund-overlay');
    expectExiting(modal, 'refund-modal');

    // Mid-fade: still in DOM.
    advanceFadeSync(199);
    expect(document.querySelector('.refund-overlay')).toBeTruthy();
    expect(document.querySelector('.refund-modal')).toBeTruthy();

    // After 200 ms: unmounted and parent open=false.
    advanceFadeSync(1);
    expect(document.querySelector('.refund-overlay')).toBeNull();
    expect(hostRef.open).toBe(false);
  });

  it('Cancel footer button also triggers the layered exit', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />, { wrapper });

    // The footer Cancel button has visible text "Cancel". The close
    // × button has only an aria-label of "Cancel refund" (the SVG
    // icon has no text content). Use text-based query to uniquely
    // match the footer button by visible text, not aria-label.
    fireEvent.click(screen.getByText('Cancel'));

    advanceFadeSync(200);
    expect(document.querySelector('.refund-overlay')).toBeNull();
    expect(hostRef.open).toBe(false);
  });

  it('Done button applies the layered exit and fires onRefunded SYNCHRONOUSLY', async () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />, { wrapper });

    // Simulate clicking the refund checkbox to enable the submit.
    fireEvent.click(document.querySelector<HTMLInputElement>('input[type="checkbox"]')!);
    fireEvent.change(document.querySelector<HTMLInputElement>('input[type="text"]')!, {
      target: { value: 'Customer changed mind' },
    });
    // Click submit.
    fireEvent.click(screen.getByRole('button', { name: /Process refund/i }));

    // Flush microtasks so the processRefund Promise resolves and the
    // done-state mounts.
    await vi.advanceTimersByTimeAsync(0);

    // Now the done-state is visible. Click Done.
    fireEvent.click(screen.getByRole('button', { name: /Done/i }));

    // onRefunded fired eagerly (before the fade).
    expect(hostRef.refunded).toBe(true);

    // --exiting class applied to both layers.
    expectExiting(document.querySelector('.refund-overlay'), 'refund-overlay');
    expectExiting(document.querySelector('.refund-modal'), 'refund-modal');

    // After the fade, parent unmounted.
    await vi.advanceTimersByTimeAsync(200);
    expect(document.querySelector('.refund-overlay')).toBeNull();
    expect(hostRef.open).toBe(false);
  });
});
