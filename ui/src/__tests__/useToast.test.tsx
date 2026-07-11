import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act, within } from '@testing-library/react';
import { withFluent } from '@/locales/test-utils';
import sharedFtl from '@/locales/shared.ftl?raw';
import { ToastProvider, useToast, type ToastVariant } from '@/hooks/useToast';
import { useRef, useCallback } from 'react';
import type { ReactNode } from 'react';

// ── Test harness component ────────────────────────────────────────────

function TestConsumer({
  onRender,
}: {
  onRender?: (add: (msg: string, v?: ToastVariant) => string, remove: (id: string) => void) => void;
}) {
  const { toasts, addToast, removeToast } = useToast();
  const lastIdRef = useRef('');

  const add = useCallback(
    (msg: string, v?: ToastVariant) => {
      addToast(msg, v);
      // We can't get the id synchronously, so capture via a timeout
      setTimeout(() => {
        // After state flushes, grab the latest toast id
      }, 0);
      return lastIdRef.current;
    },
    [addToast],
  );

  // Track the latest toast id as a side-effect
  if (toasts.length > 0) {
    lastIdRef.current = toasts[toasts.length - 1]!.id;
  }

  if (onRender) onRender(add, removeToast);
  return (
    <div>
      <div data-testid="toast-count">{toasts.length}</div>
      {toasts.map((t) => (
        <div key={t.id} data-testid={`toast-${t.variant}`}>
          {t.message}
        </div>
      ))}
    </div>
  );
}

function renderWithProvider(ui: ReactNode) {
  return render(withFluent(<ToastProvider>{ui}</ToastProvider>, sharedFtl));
}

// ── Tests ─────────────────────────────────────────────────────────────

describe('useToast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('adds a toast with default info variant', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Hello world');
    });

    const container = document.querySelector<HTMLElement>('.toast-container')!;
    expect(within(container).getByText('Hello world')).toBeTruthy();
    expect(screen.getByTestId('toast-info')).toBeTruthy();
    expect(screen.getByTestId('toast-count').textContent).toBe('1');
  });

  it('adds a toast with explicit variant', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Success!', 'success');
    });

    const container = document.querySelector<HTMLElement>('.toast-container')!;
    expect(within(container).getByText('Success!')).toBeTruthy();
    expect(screen.getByTestId('toast-success')).toBeTruthy();
  });

  it('supports all four variants', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => { add('info', 'info'); });
    act(() => { add('success', 'success'); });
    act(() => { add('warning', 'warning'); });
    act(() => { add('error', 'error'); });

    expect(screen.getByTestId('toast-info')).toBeTruthy();
    expect(screen.getByTestId('toast-success')).toBeTruthy();
    expect(screen.getByTestId('toast-warning')).toBeTruthy();
    expect(screen.getByTestId('toast-error')).toBeTruthy();
    expect(screen.getByTestId('toast-count').textContent).toBe('4');
  });

  it('removes a toast by explicit remove', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Temp');
    });

    expect(screen.getByTestId('toast-count').textContent).toBe('1');

    // Grab the toast element and click its dismiss button.
    const toastEl = document.querySelector<HTMLElement>('.toast')!;
    const dismissBtn = toastEl.querySelector<HTMLElement>('.toast__dismiss')!;
    act(() => {
      dismissBtn.click();
    });

    expect(screen.getByTestId('toast-count').textContent).toBe('0');
  });

  it('auto-dismisses a toast after 4 seconds', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Fading');
    });

    expect(screen.getByTestId('toast-count').textContent).toBe('1');

    act(() => {
      vi.advanceTimersByTime(4000);
    });

    expect(screen.getByTestId('toast-count').textContent).toBe('0');
  });

  it('does not dismiss before 4 seconds', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Staying');
    });

    act(() => {
      vi.advanceTimersByTime(3999);
    });

    expect(screen.getByTestId('toast-count').textContent).toBe('1');
  });

  it('renders nothing when there are no toasts', () => {
    renderWithProvider(<TestConsumer />);

    expect(screen.getByTestId('toast-count').textContent).toBe('0');
  });

  it('throws error when used outside ToastProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => {
      render(<TestConsumer />);
    }).toThrow('useToast must be used within a <ToastProvider>');

    spy.mockRestore();
  });

  it('has an accessible toast container', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Accessible');
    });

    const container = document.querySelector<HTMLElement>('.toast-container')!;
    expect(container).toBeTruthy();
    expect(container.getAttribute('role')).toBe('status');
    expect(container.getAttribute('aria-live')).toBe('polite');
  });

  it('has dismiss buttons on each toast', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Dismiss me');
    });

    const dismissBtns = document.querySelectorAll<HTMLElement>('.toast__dismiss');
    expect(dismissBtns.length).toBe(1);
    expect(dismissBtns[0]!.textContent).toContain('×');

    // Clicking dismiss removes the toast (use act + direct click with fake timers)
    act(() => {
      dismissBtns[0]!.click();
    });

    expect(screen.getByTestId('toast-count').textContent).toBe('0');
  });

  it('each toast has the correct variant CSS class', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => { add('error toast', 'error'); });
    act(() => { add('success toast', 'success'); });

    const toasts = document.querySelectorAll<HTMLElement>('.toast');
    expect(toasts[0]!.classList.contains('toast--error')).toBe(true);
    expect(toasts[1]!.classList.contains('toast--success')).toBe(true);
  });

  it('toast messages are rendered in a span', () => {
    let add: (msg: string, v?: ToastVariant) => string = () => '';
    renderWithProvider(<TestConsumer onRender={(a) => { add = a; }} />);

    act(() => {
      add('Span message');
    });

    const msgEl = document.querySelector<HTMLElement>('.toast__message')!;
    expect(msgEl).toBeTruthy();
    expect(msgEl.tagName).toBe('SPAN');
    expect(msgEl.textContent).toBe('Span message');
  });
});
