// ── useToast tests ─────────────────────────────────────────────────
//
// Covers: throw outside provider, ToastProvider renders children,
// addToast / removeToast lifecycle, variant CSS classes, dismiss
// button, and auto-dismiss timer.

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ToastProvider, useToast } from '@/hooks/useToast';
import type { ToastVariant } from '@/hooks/useToast';

// ── Hoisted mocks ──────────────────────────────────────────────────

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string) => id,
    },
  }),
  Localized: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// ── Helpers ────────────────────────────────────────────────────────

/** Minimal consumer that exposes the hook API for inspection. */
function ToastConsumer({
  onRender,
}: {
  onRender?: (api: ReturnType<typeof useToast>) => void;
}) {
  const api = useToast();
  onRender?.(api);
  return (
    <div>
      <span data-testid="toast-count">{api.toasts.length}</span>
      <button
        data-testid="add-toast"
        onClick={() => api.addToast('Hello', 'info')}
      >
        Add
      </button>
    </div>
  );
}

// ── Tests ──────────────────────────────────────────────────────────

describe('useToast', () => {
  describe('context', () => {
    it('throws when used outside ToastProvider', () => {
      // Suppress React error boundary noise in test output.
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => render(<ToastConsumer />)).toThrow(
        'useToast must be used within a <ToastProvider>',
      );

      spy.mockRestore();
    });
  });
});

describe('ToastProvider', () => {
  // ── Basic rendering ────────────────────────────────────────────

  it('renders children', () => {
    render(
      <ToastProvider>
        <span data-testid="child">Hello</span>
      </ToastProvider>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('renders toast container', () => {
    render(
      <ToastProvider>
        <div />
      </ToastProvider>,
    );
    const container = document.querySelector('.toast-container');
    expect(container).toBeInTheDocument();
    expect(container?.getAttribute('role')).toBe('status');
    expect(container?.getAttribute('aria-live')).toBe('polite');
  });

  // ── addToast / removeToast ─────────────────────────────────────

  it('addToast creates a visible toast', async () => {
    render(
      <ToastProvider>
        <ToastConsumer />
      </ToastProvider>,
    );

    await userEvent.click(screen.getByTestId('add-toast'));

    await waitFor(() => {
      expect(screen.getByTestId('toast-count').textContent).toBe('1');
    });
    expect(screen.getByText('Hello')).toBeInTheDocument();
  });

  it('addToast includes a dismiss button with ARIA label', async () => {
    render(
      <ToastProvider>
        <ToastConsumer />
      </ToastProvider>,
    );

    await userEvent.click(screen.getByTestId('add-toast'));

    await waitFor(() => {
      const dismissBtn = document.querySelector('.toast__dismiss');
      expect(dismissBtn).toBeInTheDocument();
      expect(dismissBtn?.getAttribute('aria-label')).toBe('toast-dismiss-aria');
      expect(dismissBtn?.textContent?.trim()).toBe('×');
    });
  });

  it('removeToast via API removes the toast', async () => {
    let capturedApi: ReturnType<typeof useToast> | null = null;

    render(
      <ToastProvider>
        <ToastConsumer
          onRender={(api) => {
            capturedApi = api;
          }}
        />
      </ToastProvider>,
    );

    await userEvent.click(screen.getByTestId('add-toast'));
    await waitFor(() => {
      expect(screen.getByText('Hello')).toBeInTheDocument();
    });

    expect(capturedApi).not.toBeNull();
    act(() => {
      capturedApi!.removeToast(capturedApi!.toasts[0].id);
    });

    await waitFor(() => {
      expect(screen.queryByText('Hello')).not.toBeInTheDocument();
    });
  });

  it('dismiss button removes the toast', async () => {
    render(
      <ToastProvider>
        <ToastConsumer />
      </ToastProvider>,
    );

    await userEvent.click(screen.getByTestId('add-toast'));
    await waitFor(() => {
      expect(screen.getByText('Hello')).toBeInTheDocument();
    });

    const dismissBtn = document.querySelector('.toast__dismiss') as HTMLElement;
    await userEvent.click(dismissBtn);

    await waitFor(() => {
      expect(screen.queryByText('Hello')).not.toBeInTheDocument();
    });
  });

  // ── Variant classes ────────────────────────────────────────────

  const variants: ToastVariant[] = ['success', 'error', 'warning', 'info'];

  it.each(variants)(
    'applies toast--%s class for variant %s',
    async (variant) => {
      let capturedApi: ReturnType<typeof useToast> | null = null;

      render(
        <ToastProvider>
          <ToastConsumer
            onRender={(api) => {
              capturedApi = api;
            }}
          />
        </ToastProvider>,
      );

      expect(capturedApi).not.toBeNull();
      act(() => {
        capturedApi!.addToast('Test', variant);
      });

      await waitFor(() => {
        const toast = document.querySelector(`.toast--${variant}`);
        expect(toast).toBeInTheDocument();
        expect(toast?.querySelector('.toast__message')?.textContent).toBe('Test');
      });
    },
  );

  it('defaults to info variant when no variant is specified', () => {
    let capturedApi: ReturnType<typeof useToast> | null = null;

    render(
      <ToastProvider>
        <ToastConsumer
          onRender={(api) => {
            capturedApi = api;
          }}
        />
      </ToastProvider>,
    );

    act(() => {
      capturedApi!.addToast('Default');
    });

    expect(document.querySelector('.toast--info')).toBeInTheDocument();
  });

  // ── Multiple toasts ────────────────────────────────────────────

  it('supports multiple concurrent toasts', () => {
    let capturedApi: ReturnType<typeof useToast> | null = null;

    render(
      <ToastProvider>
        <ToastConsumer
          onRender={(api) => {
            capturedApi = api;
          }}
        />
      </ToastProvider>,
    );

    act(() => {
      capturedApi!.addToast('First');
      capturedApi!.addToast('Second');
      capturedApi!.addToast('Third');
    });

    expect(screen.getByText('First')).toBeInTheDocument();
    expect(screen.getByText('Second')).toBeInTheDocument();
    expect(screen.getByText('Third')).toBeInTheDocument();

    const toasts = document.querySelectorAll('.toast');
    expect(toasts.length).toBe(3);
  });
});

describe('ToastProvider (timers)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── Auto-dismiss ───────────────────────────────────────────────

  it('auto-dismisses toast after 4 seconds', async () => {
    let capturedApi: ReturnType<typeof useToast> | null = null;

    render(
      <ToastProvider>
        <ToastConsumer
          onRender={(api) => {
            capturedApi = api;
          }}
        />
      </ToastProvider>,
    );

    // Add toast via direct API call (not userEvent — incompatible with fake timers).
    act(() => {
      capturedApi!.addToast('Timer Toast', 'info');
    });
    expect(screen.getByText('Timer Toast')).toBeInTheDocument();

    // Advance past 4s. The setTimeout fires inside this act,
    // removeToast runs, and React flushes the state update.
    act(() => {
      vi.advanceTimersByTime(4000);
    });

    expect(screen.queryByText('Timer Toast')).not.toBeInTheDocument();
  });

  it('does not auto-dismiss before 4 seconds', () => {
    let capturedApi: ReturnType<typeof useToast> | null = null;

    render(
      <ToastProvider>
        <ToastConsumer
          onRender={(api) => {
            capturedApi = api;
          }}
        />
      </ToastProvider>,
    );

    act(() => {
      capturedApi!.addToast('Timer Toast', 'info');
    });
    expect(screen.getByText('Timer Toast')).toBeInTheDocument();

    // Advance 3.9 seconds — toast should still be visible.
    act(() => {
      vi.advanceTimersByTime(3900);
    });

    expect(screen.getByText('Timer Toast')).toBeInTheDocument();
  });
});
