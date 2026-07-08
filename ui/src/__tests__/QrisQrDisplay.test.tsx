// ── QrisQrDisplay exit-animation tests ────────────────────────────
//
// Pins the contract for the entry+exit cohesion added in the
// sibling-surfaces polish. The × button triggers a layered
// (overlay + container) fade via mirror keyframes. The payment-
// confirmed flow intentionally SNAPS (it's a navigate-to-next-state
// transition where the parent unmounts QrisQrDisplay directly —
// adding a fade-out here would visually double-up per the skill's
// rule).
//
// Test consumer pattern: a real React component (HostModal) drives
// the modal's `isOpen` state, with a mutable `hostRef` so the test
// can peek at state synchronously WITHOUT calling React hooks
// outside a render context (which would throw "useState is null").

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useState } from 'react';
import { render, act, fireEvent, screen } from '@testing-library/react';
import QrisQrDisplay from '@/components/QrisQrDisplay';

// ── Mutable host-state ref (synchronously accessible from tests) ──
//
// `hostRef.open` mirrors HostModal's internal `open` state after each
// render. `hostRef.setOpen` is a fresh React state setter each render
// — calls to it trigger the same React-controlled re-render path as
// any component-internal state update.

interface HostRef {
  open: boolean;
  paid: boolean;
  setOpen: (v: boolean) => void;
  setPaid: (v: boolean) => void;
}

function makeHostRef(): HostRef {
  return {
    open: true,
    paid: false,
    setOpen: () => {},
    setPaid: () => {},
  };
}

function HostModal({
  initialOpen = true,
  hostRef,
  onPaymentConfirmed,
}: {
  initialOpen?: boolean;
  hostRef: HostRef;
  onPaymentConfirmed?: () => void;
}) {
  const [open, setOpen] = useState(initialOpen);
  const [paid, setPaid] = useState(false);
  // Mirror state into the ref on every render so tests can read it
  // synchronously after fireEvent advances ticks.
  hostRef.open = open;
  hostRef.paid = paid;
  hostRef.setOpen = setOpen;
  hostRef.setPaid = setPaid;
  return (
    <>
      <span data-testid="open-state">{String(open)}</span>
      <span data-testid="paid-state">{String(paid)}</span>
      <QrisQrDisplay
        amount={25000}
        currency="IDR"
        reference="REF-1234"
        isOpen={open}
        onClose={() => setOpen(false)}
        onPaymentConfirmed={() => {
          setPaid(true);
          onPaymentConfirmed?.();
        }}
      />
    </>
  );
}

describe('QrisQrDisplay exit-animation polish', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not render when isOpen=false', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} initialOpen={false} />);
    expect(document.querySelector('.qris-overlay')).toBeNull();
  });

  it('renders overlay + container with NO exit class when open', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />);
    const overlay = document.querySelector('.qris-overlay');
    const container = document.querySelector('.qris-container');
    expect(overlay).toBeTruthy();
    expect(container).toBeTruthy();
    expect(overlay?.classList.contains('qris-overlay--exiting')).toBe(false);
    expect(container?.classList.contains('qris-container--exiting')).toBe(false);
  });

  it('× button applies BOTH --exiting classes (layered exit) then fades', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />);

    fireEvent.click(document.querySelector('.qris-close') as HTMLElement);

    expect(
      document.querySelector('.qris-overlay')?.classList.contains(
        'qris-overlay--exiting',
      ),
    ).toBe(true);
    expect(
      document.querySelector('.qris-container')?.classList.contains(
        'qris-container--exiting',
      ),
    ).toBe(true);

    act(() => { vi.advanceTimersByTime(199); });
    expect(document.querySelector('.qris-overlay')).toBeTruthy();

    act(() => { vi.advanceTimersByTime(1); });
    expect(document.querySelector('.qris-overlay')).toBeNull();
    expect(hostRef.open).toBe(false);
    expect(screen.getByTestId('open-state').textContent).toBe('false');
  });

  it('disables the × button during the fade (no double-click race)', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />);
    fireEvent.click(document.querySelector('.qris-close') as HTMLElement);

    const btn = document.querySelector<HTMLButtonElement>('.qris-close');
    expect(btn?.disabled).toBe(true);
  });

  it('× click mid-fade is idempotent (no double-timer)', () => {
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />);
    fireEvent.click(document.querySelector('.qris-close') as HTMLElement);
    fireEvent.click(document.querySelector<HTMLButtonElement>('.qris-close')!);

    act(() => { vi.advanceTimersByTime(200); });
    expect(document.querySelector('.qris-overlay')).toBeNull();
  });

  it('payment-confirmed flow is allowed to snap (parent bypass)', () => {
    // The skill's "navigate to next state" pattern: parent flips
    // isOpen=false directly (no requestClose), so the surface just
    // unmounts. No fade expected.
    const hostRef = makeHostRef();
    render(<HostModal hostRef={hostRef} />);
    expect(document.querySelector('.qris-overlay')).toBeTruthy();

    act(() => { hostRef.setOpen(false); });
    expect(document.querySelector('.qris-overlay')).toBeNull();
  });
});
