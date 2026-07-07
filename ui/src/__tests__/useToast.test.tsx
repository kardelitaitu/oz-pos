import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act, fireEvent } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import { ToastProvider, useToast } from '@/hooks/useToast';

const ftl = `
toast-dismiss-aria = Dismiss notification
`;

const bundle = new FluentBundle('en');
bundle.addResource(new FluentResource(ftl));
const l10n = new ReactLocalization([bundle]);

function TestConsumer() {
  const { toasts, addToast, removeToast } = useToast();
  return (
    <div>
      <span data-testid="count">{toasts.length}</span>
      <ul>
        {toasts.map((t) => (
          <li key={t.id} data-testid={`toast-${t.id}`}>
            <span data-testid={`msg-${t.id}`}>{t.message}</span>
            <span data-testid={`variant-${t.id}`}>{t.variant}</span>
          </li>
        ))}
      </ul>
      <button data-testid="add-info" onClick={() => addToast('Info toast')}>
        Add info
      </button>
      <button data-testid="add-success" onClick={() => addToast('Success!', 'success')}>
        Add success
      </button>
      <button data-testid="add-error" onClick={() => addToast('Failed!', 'error')}>
        Add error
      </button>
      <button data-testid="add-warning" onClick={() => addToast('Careful', 'warning')}>
        Add warning
      </button>
      <button
        data-testid="remove-last"
        onClick={() => {
          if (toasts.length > 0) removeToast(toasts[0]!.id);
        }}
      >
        Remove last
      </button>
    </div>
  );
}

function renderProvider() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <ToastProvider>
        <TestConsumer />
      </ToastProvider>
    </LocalizationProvider>,
  );
}

describe('ToastProvider + useToast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts with zero toasts', () => {
    renderProvider();
    expect(screen.getByTestId('count').textContent).toBe('0');
  });

  it('adds an info toast with default variant', () => {
    renderProvider();
    fireEvent.click(screen.getByTestId('add-info'));
    expect(screen.getByTestId('count').textContent).toBe('1');
    // Find the new toast's message
    const messages = document.querySelectorAll('[data-testid^="msg-"]');
    expect(messages[0]!.textContent).toBe('Info toast');
    const variants = document.querySelectorAll('[data-testid^="variant-"]');
    expect(variants[0]!.textContent).toBe('info');
  });

  it('adds toasts with different variants', () => {
    renderProvider();
    fireEvent.click(screen.getByTestId('add-success'));
    fireEvent.click(screen.getByTestId('add-error'));
    fireEvent.click(screen.getByTestId('add-warning'));
    expect(screen.getByTestId('count').textContent).toBe('3');

    const variants = document.querySelectorAll('[data-testid^="variant-"]');
    expect(variants[0]!.textContent).toBe('success');
    expect(variants[1]!.textContent).toBe('error');
    expect(variants[2]!.textContent).toBe('warning');
  });

  it('removes a toast manually', () => {
    renderProvider();
    fireEvent.click(screen.getByTestId('add-info'));
    expect(screen.getByTestId('count').textContent).toBe('1');

    fireEvent.click(screen.getByTestId('remove-last'));
    expect(screen.getByTestId('count').textContent).toBe('0');
  });

  it('auto-dismisses toasts after 4 seconds', () => {
    renderProvider();
    fireEvent.click(screen.getByTestId('add-info'));
    expect(screen.getByTestId('count').textContent).toBe('1');

    // 3999ms — still there
    act(() => { vi.advanceTimersByTime(3999); });
    expect(screen.getByTestId('count').textContent).toBe('1');

    // 1ms more = 4000ms — dismissed
    act(() => { vi.advanceTimersByTime(1); });
    expect(screen.getByTestId('count').textContent).toBe('0');
  });

  it('renders the toast container with role=status and aria-live=polite', () => {
    renderProvider();
    const container = document.querySelector('.toast-container');
    expect(container).toBeTruthy();
    expect(container!.getAttribute('role')).toBe('status');
    expect(container!.getAttribute('aria-live')).toBe('polite');
  });

  it('throws when useToast is used outside ToastProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => {
      render(<TestConsumer />);
    }).toThrow('useToast must be used within a <ToastProvider>');
    spy.mockRestore();
  });

  it('renders dismiss buttons with aria-label from FTL', () => {
    renderProvider();
    fireEvent.click(screen.getByTestId('add-info'));

    const dismissBtn = document.querySelector('.toast__dismiss');
    expect(dismissBtn).toBeTruthy();
    expect(dismissBtn!.getAttribute('aria-label')).toBe('Dismiss notification');
  });
});
