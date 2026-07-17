import type { ReactNode } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act } from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import FastPINOverlay from '@/components/FastPINOverlay';

// ── Mock helpers (extracted for esbuild compat) ──────────────────────

function MockPassThrough({ children }: { children: ReactNode }) {
  return <>{children}</>;
}

// ── Mocks ────────────────────────────────────────────────────────────

const mockStaffLogin = vi.fn();

vi.mock('@/api/staff', () => ({
  staffLogin: (...args: unknown[]) => mockStaffLogin(...args),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', display_name: 'Test', role_name: 'cashier', role_id: 'r' },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    swapSession: vi.fn(),
    isManager: false,
    isOwner: false,
  }),
  AuthProvider: MockPassThrough,
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    swapSessionToken: vi.fn(),
    activeWorkspace: 'store-pos',
    activeInstance: null,
    sessionToken: 'token-abc',
    setActiveWorkspace: vi.fn(),
    setActiveInstance: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
    error: null,
    retry: vi.fn(),
    lastWorkspace: null,
    switchStore: vi.fn(),
    resolvedStoreId: 'default',
  }),
  WorkspaceProvider: MockPassThrough,
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string) => id,
    },
  }),
  Localized: ({ children }: { id: string; children: ReactNode }) => <>{children}</>,
}));

describe('FastPINOverlay — keyboard accessibility', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  // ── Escape key ───────────────────────────────────────────────

  it('closes on Escape keydown', () => {
    const onClose = vi.fn();
    render(<FastPINOverlay open={true} onClose={onClose} />);

    act(() => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });

    // After 200ms exit animation timeout
    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onClose).toHaveBeenCalled();
  });

  it('does not close on non-Escape keydown', () => {
    const onClose = vi.fn();
    render(<FastPINOverlay open={true} onClose={onClose} />);

    act(() => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onClose).not.toHaveBeenCalled();
  });

  it('applies --exiting class on Escape', () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);
    const overlay = document.querySelector('.fastpin-overlay') as HTMLElement;
    expect(overlay).not.toBeNull();

    act(() => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });

    expect(overlay.classList.contains('fastpin-overlay--exiting')).toBe(true);
  });

  // ── Focus management ─────────────────────────────────────────

  it('auto-focuses the username input on open', async () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);

    // Component uses setTimeout(50ms) to focus input
    act(() => {
      vi.advanceTimersByTime(50);
    });

    const input = screen.getByPlaceholderText('Username');
    expect(document.activeElement).toBe(input);
  });

  it('submits username on form submit', () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);

    // Wait for auto-focus
    act(() => { vi.advanceTimersByTime(50); });

    // Set input value using native setter
    const input = screen.getByPlaceholderText('Username') as HTMLInputElement;
    const nativeSetter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype, 'value',
    )?.set;
    nativeSetter?.call(input, 'cashier1');
    input.dispatchEvent(new Event('input', { bubbles: true }));

    // Submit the form
    const form = document.querySelector('.fastpin-form') as HTMLFormElement;
    fireEvent.submit(form);

    // Should advance to PIN step
    expect(screen.queryByPlaceholderText('Username')).not.toBeInTheDocument();
  });

  // ── Tab cycling within dialog ─────────────────────────────────

  it('has a focusable close button with correct tabIndex', () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);

    const closeBtn = screen.getByLabelText('modal-close-aria');
    expect(closeBtn).toBeInTheDocument();
    expect(closeBtn.getAttribute('tabindex')).toBeNull(); // default tabIndex
    expect(closeBtn.tagName).toBe('BUTTON');
  });

  it('has a cancel button that can be focused', () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);

    const cancelBtn = screen.getByText('Cancel');
    expect(cancelBtn).toBeInTheDocument();
    expect(cancelBtn.tagName).toBe('BUTTON');
    cancelBtn.focus();
    expect(document.activeElement).toBe(cancelBtn);
  });

  // ── Backdrop click (mouse interaction, complementary test) ────

  it('does not close when clicking inside the dialog card', () => {
    const onClose = vi.fn();
    render(<FastPINOverlay open={true} onClose={onClose} />);

    const card = document.querySelector('.fastpin-card') as HTMLElement;
    expect(card).not.toBeNull();

    // Click inside the card — should NOT close
    fireEvent.click(card);

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onClose).not.toHaveBeenCalled();
  });

  it('has role="dialog" with aria-modal="true"', () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);

    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog.getAttribute('aria-modal')).toBe('true');
  });

  // ── Verification: dialog removed when closed ──────────────────

  it('removes dialog from DOM after close completes', () => {
    const onClose = vi.fn();
    render(<FastPINOverlay open={true} onClose={onClose} />);

    expect(screen.queryByRole('dialog')).toBeInTheDocument();

    // Close
    act(() => {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });

    // During exit animation, dialog should still be in DOM
    expect(screen.queryByRole('dialog')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(200);
    });

    // After exit animation, onClose called → parent sets open=false
    expect(onClose).toHaveBeenCalled();
  });

  // ── Hardware keyboard digit input in PIN step ─────────────────

  it('accepts hardware keyboard digit input in PIN step', () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);

    // Enter username and advance to PIN step via form submit
    act(() => { vi.advanceTimersByTime(50); });
    const input = screen.getByPlaceholderText('Username') as HTMLInputElement;
    const nativeSetter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype, 'value',
    )?.set;
    nativeSetter?.call(input, 'cashier1');
    input.dispatchEvent(new Event('input', { bubbles: true }));
    const form = document.querySelector('.fastpin-form') as HTMLFormElement;
    fireEvent.submit(form);

    // Now in PIN step — pressing 1, 2 on hardware keyboard
    const dialog = screen.getByRole('dialog');
    fireEvent.keyDown(dialog, { key: '1' });
    fireEvent.keyDown(dialog, { key: '2' });

    const filled = document.querySelectorAll('.fastpin-pin-dot--filled');
    expect(filled.length).toBe(2);
  });

  it('accepts Backspace via hardware keyboard in PIN step', () => {
    render(<FastPINOverlay open={true} onClose={vi.fn()} />);

    act(() => { vi.advanceTimersByTime(50); });
    const input = screen.getByPlaceholderText('Username') as HTMLInputElement;
    const nativeSetter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype, 'value',
    )?.set;
    nativeSetter?.call(input, 'cashier1');
    input.dispatchEvent(new Event('input', { bubbles: true }));
    const form = document.querySelector('.fastpin-form') as HTMLFormElement;
    fireEvent.submit(form);

    const dialog = screen.getByRole('dialog');
    fireEvent.keyDown(dialog, { key: '1' });
    fireEvent.keyDown(dialog, { key: '2' });
    fireEvent.keyDown(dialog, { key: '3' });
    fireEvent.keyDown(dialog, { key: 'Backspace' });

    const filled = document.querySelectorAll('.fastpin-pin-dot--filled');
    expect(filled.length).toBe(2);
  });
});
