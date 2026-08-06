import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import type { ReactElement, ReactNode } from 'react';
import SessionLockScreen from '@/features/auth/SessionLockScreen';

// vi.hoisted ensures the mock references exist before vitest hoists the
// vi.mock() factories to the top of the file.  Without this, vitest
// throws "Cannot access 'mockStaffLogin' before initialization".
const { mockOnUnlock, mockStaffLogin, mockCheckLicenseStatus } = vi.hoisted(() => ({
  mockOnUnlock: vi.fn(),
  mockStaffLogin: vi.fn(),
  mockCheckLicenseStatus: vi.fn(() => Promise.resolve({ active: true })),
}));

vi.mock('@/api/staff', () => ({
  staffLogin: mockStaffLogin,
}));

vi.mock('@/api/license', () => ({
  checkLicenseStatus: () => mockCheckLicenseStatus(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: {
      user_id: 'user-1',
      username: 'testuser',
      role_name: 'cashier',
      token: 'mock-token',
      role_id: 'role-1',
      display_name: 'Kasir Test',
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

vi.mock('@/hooks/useSyncConnection', () => ({
  useSyncConnection: () => ({
    state: 'connected' as const,
    latencyMs: 42,
    error: null,
  }),
}));

const STAFF_FTL = `
staff-login-connection-checking = Checking…
staff-login-connection-connected = Connected
staff-login-connection-disconnected = Disconnected
staff-login-connection-auth = Auth
staff-login-connection-sync = Sync
staff-login-clear = Clear
session-lock-expired = Session expired. Please log in again.
session-lock-invalid-pin = Invalid PIN
session-lock-enter-pin = Enter PIN to unlock
session-lock-pin-aria = PIN: { $length } of { $max } digits entered
session-lock-pad-aria = PIN pad
session-lock-lockout = Wait { $seconds }s.
`;

function withProviders(children: ReactNode): ReactElement {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(STAFF_FTL));
  const l10n = new ReactLocalization([bundle]);

  return (
    <LocalizationProvider l10n={l10n}>
      {children}
    </LocalizationProvider>
  );
}

function renderScreen() {
  return render(withProviders(<SessionLockScreen onUnlock={mockOnUnlock} />));
}

/** Return all PIN-dot elements (always 4). */
function getDots(): Element[] {
  return Array.from(document.querySelectorAll('.session-lock-pin-dot'));
}

/** Whether dot i is filled. */
function isDotFilled(i: number): boolean {
  return getDots()[i]?.classList.contains('session-lock-pin-dot--filled') ?? false;
}

beforeEach(() => {
  vi.clearAllMocks();
  mockStaffLogin.mockResolvedValue(undefined);
  mockCheckLicenseStatus.mockResolvedValue({ active: true });
  sessionStorage.setItem('current-username', 'testuser');
});

// ── Rendering ───────────────────────────────────────────────────

describe('SessionLockScreen rendering', () => {
  it('renders the lock icon', () => {
    renderScreen();
    const lockIcon = document.querySelector('.session-lock-icon');
    expect(lockIcon).toBeTruthy();
    expect(lockIcon?.querySelector('svg')).toBeTruthy();
  });

  it('displays the current time', () => {
    renderScreen();
    const timeEl = document.querySelector('.session-lock-time');
    expect(timeEl).toBeTruthy();
    expect(timeEl?.textContent).toBeTruthy();
  });

  it('displays the current date', () => {
    renderScreen();
    const dateEl = document.querySelector('.session-lock-date');
    expect(dateEl).toBeTruthy();
    expect(dateEl?.textContent).toBeTruthy();
  });

  it('shows "Enter PIN to unlock" message', () => {
    renderScreen();
    expect(screen.getByText('Enter PIN to unlock')).toBeInTheDocument();
  });

  it('renders 4 PIN dots (all unfilled)', () => {
    renderScreen();
    const dots = getDots();
    expect(dots).toHaveLength(4);
    dots.forEach((dot) => {
      expect(dot.classList.contains('session-lock-pin-dot--filled')).toBe(false);
    });
  });

  it('renders the PIN pad with digit keys 0-9', () => {
    renderScreen();
    for (let i = 0; i <= 9; i++) {
      expect(screen.getByRole('button', { name: String(i) })).toBeInTheDocument();
    }
  });

  it('renders Clear and backspace buttons', () => {
    renderScreen();
    expect(screen.getByRole('button', { name: 'Clear' })).toBeInTheDocument();
    // Backspace is an SVG-only button — verify there are 2 --action keys
    const pad = document.querySelector('.session-lock-pad');
    expect(pad).toBeTruthy();
    const actionKeys = pad!.querySelectorAll('.session-lock-pad-key--action');
    expect(actionKeys.length).toBeGreaterThanOrEqual(2);
  });

  it('shows auth and sync connection status indicators', () => {
    renderScreen();
    const indicators = document.querySelectorAll('.connection-status');
    expect(indicators.length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText('Auth')).toBeInTheDocument();
    expect(screen.getByText('Sync')).toBeInTheDocument();
  });
});

// ── PIN entry ────────────────────────────────────────────────────

describe('SessionLockScreen PIN entry', () => {
  it('fills PIN dots as digits are entered', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    expect(isDotFilled(0)).toBe(true);
    expect(isDotFilled(1)).toBe(false);

    await user.click(screen.getByRole('button', { name: '2' }));
    expect(isDotFilled(1)).toBe(true);
  });

  it('auto-submits after 4 digits and calls staffLogin', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '2' }));
    await user.click(screen.getByRole('button', { name: '3' }));
    await user.click(screen.getByRole('button', { name: '4' }));

    await waitFor(() => {
      expect(mockStaffLogin).toHaveBeenCalledWith({
        username: 'testuser',
        pin: '1234',
      });
    });
  });

  it('calls onUnlock after successful PIN verification', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '2' }));
    await user.click(screen.getByRole('button', { name: '3' }));
    await user.click(screen.getByRole('button', { name: '4' }));

    await waitFor(() => {
      expect(mockOnUnlock).toHaveBeenCalled();
    });
  });

  it('shows error message on invalid PIN', async () => {
    mockStaffLogin.mockRejectedValueOnce({ message: 'Invalid PIN' });
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '1' }));

    await waitFor(() => {
      expect(screen.getByText('Invalid PIN')).toBeInTheDocument();
    });
  });

  it('clears PIN dots after a failed attempt', async () => {
    mockStaffLogin.mockRejectedValueOnce({ message: 'Invalid PIN' });
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '2' }));
    await user.click(screen.getByRole('button', { name: '3' }));
    await user.click(screen.getByRole('button', { name: '4' }));

    await waitFor(() => {
      getDots().forEach((dot) => {
        expect(dot.classList.contains('session-lock-pin-dot--filled')).toBe(false);
      });
    });
  });
});

// ── Backspace and Clear ─────────────────────────────────────────

describe('SessionLockScreen backspace and clear', () => {
  it('removes the last digit on backspace click', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '2' }));

    expect(isDotFilled(0)).toBe(true);
    expect(isDotFilled(1)).toBe(true);

    // Click backspace — the second .session-lock-pad-key--action button.
    const pad = document.querySelector('.session-lock-pad');
    const backspaceBtn = pad?.querySelectorAll('.session-lock-pad-key--action')[1] as HTMLButtonElement | undefined;
    if (backspaceBtn) await user.click(backspaceBtn);

    expect(isDotFilled(1)).toBe(false);
  });

  it('clears all digits on Clear button click', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '2' }));
    await user.click(screen.getByRole('button', { name: '3' }));

    expect(isDotFilled(2)).toBe(true);

    await user.click(screen.getByRole('button', { name: 'Clear' }));

    getDots().forEach((dot) => {
      expect(dot.classList.contains('session-lock-pin-dot--filled')).toBe(false);
    });
  });

  it('shows error when username is not in sessionStorage', async () => {
    sessionStorage.removeItem('current-username');
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: '1' }));
    await user.click(screen.getByRole('button', { name: '2' }));
    await user.click(screen.getByRole('button', { name: '3' }));
    await user.click(screen.getByRole('button', { name: '4' }));

    await waitFor(() => {
      expect(screen.getByText('Session expired. Please log in again.')).toBeInTheDocument();
    });
  });
});

// ── Lockout (rate limiting) ──────────────────────────────────────
// Fake timers are needed here to control the lockout countdown timer.

describe('SessionLockScreen lockout', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('disables all digit keys when locked out', async () => {
    mockStaffLogin.mockRejectedValue({ message: 'Invalid PIN' });
    renderScreen();

    for (let i = 0; i < 5; i++) {
      await act(async () => {
        screen.getByRole('button', { name: '1' }).click();
        screen.getByRole('button', { name: '2' }).click();
        screen.getByRole('button', { name: '3' }).click();
        screen.getByRole('button', { name: '4' }).click();
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(50);
      });
    }

    // State is synchronous after act() — no waitFor needed.
    expect(screen.getByRole('button', { name: '1' })).toBeDisabled();
  });

  it('shows lockout countdown message after max attempts', async () => {
    mockStaffLogin.mockRejectedValue({ message: 'Invalid PIN' });
    renderScreen();

    for (let i = 0; i < 5; i++) {
      await act(async () => {
        screen.getByRole('button', { name: '1' }).click();
        screen.getByRole('button', { name: '2' }).click();
        screen.getByRole('button', { name: '3' }).click();
        screen.getByRole('button', { name: '4' }).click();
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(50);
      });
    }

    // The error alert div should contain the lockout countdown span.
    const rateLimit = document.querySelector('.session-lock-rate-limit');
    expect(rateLimit).toBeTruthy();
    expect(rateLimit?.textContent).toContain('Wait');
  });

  it('respects backend rate-limit instructions from error message', async () => {
    mockStaffLogin.mockRejectedValue({
      message: 'Try again in 5 seconds',
    });
    renderScreen();

    await act(async () => {
      screen.getByRole('button', { name: '1' }).click();
      screen.getByRole('button', { name: '2' }).click();
      screen.getByRole('button', { name: '3' }).click();
      screen.getByRole('button', { name: '4' }).click();
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(50);
    });

    const rateLimit = document.querySelector('.session-lock-rate-limit');
    expect(rateLimit).toBeTruthy();
    expect(rateLimit?.textContent).toContain('Wait');
  });
});// ── Keyboard input ──────────────────────────────────────────────

describe('SessionLockScreen keyboard input', () => {
  it('accepts digit keys from physical keyboard', async () => {
    const user = userEvent.setup();
    renderScreen();

    const pad = document.getElementById('session-lock-pin-pad');
    expect(pad).toBeTruthy();
    await user.type(pad!, '12');

    expect(isDotFilled(0)).toBe(true);
    expect(isDotFilled(1)).toBe(true);
  });

  it('handles Backspace key from physical keyboard', async () => {
    const user = userEvent.setup();
    renderScreen();

    const pad = document.getElementById('session-lock-pin-pad');
    await user.type(pad!, '12');
    await user.type(pad!, '{Backspace}');

    expect(isDotFilled(1)).toBe(false);
  });

  it('clears digits on Escape key', async () => {
    const user = userEvent.setup();
    renderScreen();

    const pad = document.getElementById('session-lock-pin-pad');
    await user.type(pad!, '123');
    await user.type(pad!, '{Escape}');

    getDots().forEach((dot) => {
      expect(dot.classList.contains('session-lock-pin-dot--filled')).toBe(false);
    });
  });
});

// ── License status ──────────────────────────────────────────────

describe('SessionLockScreen license status', () => {
  it('checks license status on mount', async () => {
    renderScreen();

    await waitFor(() => {
      expect(mockCheckLicenseStatus).toHaveBeenCalled();
    });
  });

  it('shows offline indicator when license check fails', async () => {
    mockCheckLicenseStatus.mockRejectedValue(new Error('network error'));
    renderScreen();

    await waitFor(() => {
      const offlineIndicator = document.querySelector('.status-indicator.offline');
      expect(offlineIndicator).toBeTruthy();
    });
  });
});
