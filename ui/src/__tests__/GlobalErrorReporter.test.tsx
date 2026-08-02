import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { GlobalErrorReporter } from '@/components/GlobalErrorReporter';
import { withToastProviders } from '@/__tests__/test-utils/providers';
import sharedFtl from '@/locales/shared.ftl?raw';

/**
 * ERR-01 — global async-failure reporting layer tests.
 *
 * The React error boundary cannot catch `window.error` / `unhandledrejection`
 * failures. These tests pin the reporter's contract: expected typed API
 * failures are logged but NOT toasted (screens handle them), while unexpected
 * defects surface a recoverable toast with localized copy.
 *
 * jsdom lacks a `PromiseRejectionEvent` constructor, so rejections are
 * dispatched as plain `Event`s with the `reason` attached (the reporter
 * reads `event.reason` defensively and falls back to the event itself).
 */
describe('GlobalErrorReporter (ERR-01)', () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function mountReporter() {
    render(withToastProviders(<GlobalErrorReporter />, sharedFtl));
  }

  /** Dispatch an `unhandledrejection`-shaped event without the jsdom-missing ctor. */
  function fireRejection(reason: unknown) {
    const evt = new Event('unhandledrejection', { cancelable: true }) as Event & { reason?: unknown };
    evt.reason = reason;
    window.dispatchEvent(evt);
  }

  it('surfaces a recoverable toast for an unhandled rejection (unexpected defect)', () => {
    mountReporter();
    act(() => {
      fireRejection(new Error('boom in a timer callback'));
    });
    expect(screen.getByText(/Something unexpected happened/)).toBeInTheDocument();
  });

  it('logs but does NOT toast expected typed AppError failures', () => {
    mountReporter();
    act(() => {
      fireRejection({ kind: 'permissionDenied', message: 'owner only' });
    });
    expect(screen.queryByText(/Something unexpected happened/)).not.toBeInTheDocument();
    expect(console.warn).toHaveBeenCalled();
  });

  it('surfaces a recoverable toast for an uncaught window error', () => {
    mountReporter();
    act(() => {
      window.dispatchEvent(new ErrorEvent('error', {
        message: 'Uncaught TypeError: cannot read properties of undefined',
        cancelable: true,
      }));
    });
    expect(screen.getByText(/Something unexpected happened/)).toBeInTheDocument();
  });

  it('skips expected validation failures logged at the IPC boundary', () => {
    mountReporter();
    act(() => {
      fireRejection({ kind: 'invalid', message: 'rate must be positive' });
    });
    expect(screen.queryByText(/Something unexpected happened/)).not.toBeInTheDocument();
  });

  it('cleans up listeners on unmount', () => {
    const { unmount } = render(withToastProviders(<GlobalErrorReporter />, sharedFtl));
    unmount();
    act(() => {
      fireRejection(new Error('after unmount'));
    });
    expect(screen.queryByText(/Something unexpected happened/)).not.toBeInTheDocument();
  });
});
