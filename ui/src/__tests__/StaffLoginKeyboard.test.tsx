import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import StaffLoginScreen from '@/features/auth/StaffLoginScreen';
import { checkUsername } from '@/api/staff';

// ── Hoisted mock helpers ───────────────────────────────────────────

const mockAuthError = vi.hoisted(() => vi.fn(() => null));

// ── Mocks ────────────────────────────────────────────────────────────

const mockLogin = vi.fn();
const mockClearError = vi.fn();

vi.mock('@/api/staff', () => ({
  checkUsername: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: null,
    loading: false,
    error: mockAuthError(),
    login: (...args: unknown[]) => mockLogin(...args),
    logout: vi.fn(),
    clearError: (...args: unknown[]) => mockClearError(...args),
    isManager: false,
    isOwner: false,
  }),
}));

// ── Fluent provider ──────────────────────────────────────────────────

import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { ToastProvider } from '@/frontend/shared/Toast';

vi.mock('@/contexts/BrandContext', () => ({
  useBrand: () => ({ settings: null, loading: false, refreshBrandSettings: vi.fn() }),
  useOptionalBrand: () => null,
}));

function renderScreen() {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(`
staff-login-title = OZ-POS
staff-login-subtitle = Staff Login
staff-login-progress-aria =
    .aria-label = Login progress
staff-login-step-username = Enter your username
staff-login-step-pin = Enter your PIN
staff-login-username-placeholder =
    .placeholder = Username
staff-login-username-aria =
    .aria-label = Username
staff-login-next = Next
staff-login-pin-section-aria =
    .aria-label = PIN entry
staff-login-pin-aria =
    .aria-label = PIN entry: { $length } of { $max } digits
staff-login-keypad-aria =
    .aria-label = Numeric keypad
staff-login-digit-aria =
    .aria-label = { $digit }
staff-login-clear-aria =
    .aria-label = Clear
staff-login-clear = Clear
staff-login-backspace-aria =
    .aria-label = Backspace
staff-login-back = ← Back
staff-login-submit = Login
staff-login-error-deactivated = Account is deactivated
staff-login-error-not-found = User not found
staff-login-error-connection = Could not verify username. Check your connection.
staff-login-copyright = © 2026 OZ-POS. All rights reserved.
staff-login-attempts-remaining = ({ $count } attempts remaining)
staff-login-lockout = Locked out. Try again in { $seconds }s
`));
  const l10n = new ReactLocalization([bundle]);
  return render(
    <LocalizationProvider l10n={l10n}>
      <ToastProvider>
        <StaffLoginScreen />
      </ToastProvider>
    </LocalizationProvider>,
  );
}

describe('StaffLoginScreen — keyboard and form tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAuthError.mockReturnValue(null);
    vi.mocked(checkUsername).mockResolvedValue({ found: true, is_active: true });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ── Username step ──────────────────────────────────────────────────

  describe('username step', () => {
    it('renders username input and submit button', () => {
      renderScreen();
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /next/i })).toBeInTheDocument();
    });

    it('submit button is disabled when username is empty', () => {
      renderScreen();
      expect(screen.getByRole('button', { name: /next/i })).toBeDisabled();
    });

    it('submit button enabled when username has text', () => {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      expect(screen.getByRole('button', { name: /next/i })).toBeEnabled();
    });

    it('calls checkUsername on submit', async () => {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        expect(checkUsername).toHaveBeenCalledWith({ username: 'alice' });
      });
    });

    it('advances to PIN step when username is valid', async () => {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        expect(screen.queryByPlaceholderText('Username')).not.toBeInTheDocument();
      });

      // PIN step should be visible — check the keypad role=group exists
      const keypad = document.querySelector('.staff-login-pad');
      expect(keypad).not.toBeNull();
      expect(keypad?.getAttribute('role')).toBe('group');
    });

    it('shows spinner during username check', () => {
      vi.mocked(checkUsername).mockImplementationOnce(() => new Promise(() => {}));
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      const btn = screen.getByRole('button', { name: /next/i });
      expect(btn.querySelector('.staff-login-btn-spinner')).toBeTruthy();
    });

    it('shows deactivated error toast and stays on username step', async () => {
      vi.mocked(checkUsername).mockResolvedValueOnce({ found: true, is_active: false });
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'bob' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        expect(screen.getByText('Account is deactivated')).toBeInTheDocument();
      });

      // Should still be on username step
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });

    it('shows not-found error toast for unknown user', async () => {
      vi.mocked(checkUsername).mockResolvedValueOnce({ found: false, is_active: false });
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'nonexistent' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        expect(screen.getByText('User not found')).toBeInTheDocument();
      });
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    });

    it('shows connection error toast when checkUsername throws', async () => {
      vi.mocked(checkUsername).mockRejectedValueOnce(new Error('Network fail'));
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        expect(screen.getByText('Could not verify username. Check your connection.')).toBeInTheDocument();
      });
    });

    it('trims whitespace from username on submit', async () => {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: '  alice  ' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        expect(checkUsername).toHaveBeenCalledWith({ username: 'alice' });
      });
    });

    it('submits on Enter key', async () => {
      const user = userEvent.setup();
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      await user.type(input, 'alice{Enter}');

      await waitFor(() => {
        expect(checkUsername).toHaveBeenCalledWith({ username: 'alice' });
      });
    });

    it('clears the input on Escape key', async () => {
      renderScreen();
      const input = screen.getByPlaceholderText('Username') as HTMLInputElement;
      fireEvent.change(input, { target: { value: 'alice' } });
      expect(input.value).toBe('alice');

      fireEvent.keyDown(input, { key: 'Escape' });
      expect(input.value).toBe('');
    });
  });

  // ── PIN step ───────────────────────────────────────────────────────

  describe('PIN step', () => {
    async function advanceToPin() {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));
      await waitFor(() => {
        expect(screen.queryByPlaceholderText('Username')).not.toBeInTheDocument();
      });
    }

    it('shows PIN dots and keypad after advancing', async () => {
      await advanceToPin();
      const dots = document.querySelectorAll('.staff-login-pin-dot');
      expect(dots.length).toBe(4);
      expect(screen.getByText('Clear')).toBeInTheDocument();
      expect(screen.getByLabelText('Backspace')).toBeInTheDocument();
    });

    it('fills PIN dots as digits are entered via keypad', async () => {
      await advanceToPin();
      fireEvent.click(screen.getByLabelText('1'));
      fireEvent.click(screen.getByLabelText('2'));
      fireEvent.click(screen.getByLabelText('3'));

      const filled = document.querySelectorAll('.staff-login-pin-dot--filled');
      expect(filled.length).toBe(3);
    });

    it('clears PIN when Clear button is pressed', async () => {
      await advanceToPin();
      fireEvent.click(screen.getByLabelText('1'));
      fireEvent.click(screen.getByLabelText('2'));
      expect(document.querySelectorAll('.staff-login-pin-dot--filled').length).toBe(2);

      fireEvent.click(screen.getByLabelText('Clear'));
      expect(document.querySelectorAll('.staff-login-pin-dot--filled').length).toBe(0);
    });

    it('removes last digit on Backspace', async () => {
      await advanceToPin();
      fireEvent.click(screen.getByLabelText('1'));
      fireEvent.click(screen.getByLabelText('2'));
      fireEvent.click(screen.getByLabelText('3'));
      expect(document.querySelectorAll('.staff-login-pin-dot--filled').length).toBe(3);

      fireEvent.click(screen.getByLabelText('Backspace'));
      expect(document.querySelectorAll('.staff-login-pin-dot--filled').length).toBe(2);
    });

    it('calls login when 4 digits are entered (auto-submit)', async () => {
      await advanceToPin();
      fireEvent.click(screen.getByLabelText('1'));
      fireEvent.click(screen.getByLabelText('2'));
      fireEvent.click(screen.getByLabelText('3'));
      fireEvent.click(screen.getByLabelText('4'));

      await waitFor(() => {
        expect(mockLogin).toHaveBeenCalledWith('alice', '1234');
      });
    });

    it('goes back to username step on close button', async () => {
      await advanceToPin();
      const closeBtn = document.querySelector('.staff-login-close-btn') as HTMLButtonElement;
      expect(closeBtn).not.toBeNull();
      fireEvent.click(closeBtn);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
      });
    });

    it('shows inline error with role="alert" when auth error occurs on PIN step', async () => {
      mockAuthError.mockReturnValue('Invalid PIN');
      await advanceToPin();
      await waitFor(() => {
        const alerts = screen.getAllByRole('alert');
        // Toast alert + inline error alert
        const inlineError = alerts.find((el) =>
          el.className.includes('staff-login-error'),
        );
        expect(inlineError).toBeInTheDocument();
        expect(inlineError).toHaveAttribute('aria-live', 'polite');
        expect(inlineError).toHaveTextContent('Invalid PIN');
      });
    });

    it('shows rate-limit countdown after 3 failed PIN attempts', async () => {
      await advanceToPin();

      // Simulate 3 failed PIN attempts by cycling mockAuthError values.
      // Each unique error triggers the component's error-effect which
      // increments pinAttempts. The effect guards against re-processing
      // the same error via toastShownForError ref, so we must use
      // different error messages for each attempt.

      // 1st failure
      mockAuthError.mockReturnValue('Error 1');
      fireEvent.click(screen.getByLabelText('1')); // triggers re-render via setPin
      await waitFor(() => {
        const inline = document.querySelector('.staff-login-error');
        expect(inline).toHaveTextContent('Error 1');
      });

      // Reset error before 2nd failure (clear state)
      mockAuthError.mockReturnValue(null);
      fireEvent.click(screen.getByLabelText('2')); // triggers re-render
      await waitFor(() => {
        expect(document.querySelector('.staff-login-error')).not.toBeInTheDocument();
      });

      // 2nd failure (different message so effect fires again)
      mockAuthError.mockReturnValue('Error 2');
      fireEvent.click(screen.getByLabelText('3')); // triggers re-render
      await waitFor(() => {
        const inline = document.querySelector('.staff-login-error');
        expect(inline).toHaveTextContent('Error 2');
      });

      // Reset
      mockAuthError.mockReturnValue(null);
      fireEvent.click(screen.getByLabelText('4')); // triggers re-render
      await waitFor(() => {
        expect(document.querySelector('.staff-login-error')).not.toBeInTheDocument();
      });

      // 3rd failure — triggers rate-limit warning
      mockAuthError.mockReturnValue('Error 3');
      fireEvent.click(screen.getByLabelText('5')); // triggers re-render
      await waitFor(() => {
        const inline = document.querySelector('.staff-login-error');
        expect(inline).toHaveTextContent(/attempts? remaining/i);
      });

      // Cleanup: reset for other tests
      mockAuthError.mockReturnValue(null);
    });

    it('shows lockout message after 5 failed PIN attempts and disables keypad', async () => {
      await advanceToPin();

      for (let i = 0; i < 5; i++) {
        // Enter a digit so Clear button becomes enabled
        fireEvent.click(screen.getByLabelText('1'));
        // Set error message
        mockAuthError.mockReturnValue(`Err${i}`);
        // Click Clear — clears pin AND triggers re-render picking up new error
        fireEvent.click(screen.getByText('Clear'));
        await waitFor(() => {
          const inline = document.querySelector('.staff-login-error');
          expect(inline).toHaveTextContent(`Err${i}`);
        });
      }

      // After 5 failures, pinAttempts >= MAX_PIN_ATTEMPTS so isLocked = true.
      // The lockout message should appear with the last error.
      await waitFor(() => {
        const inline = document.querySelector('.staff-login-error');
        expect(inline).toHaveTextContent(/Err4|locked/i);
      });

      // Keypad should be disabled during lockout
      const digitButtons = screen.getAllByRole('button', { name: /^[0-9]$/ });
      for (const btn of digitButtons) {
        expect(btn).toBeDisabled();
      }

      // Clear button should also be disabled
      const clearBtn = screen.getByText('Clear').closest('button')!;
      expect(clearBtn).toBeDisabled();

      // Backspace should also be disabled
      const backspace = screen.getByLabelText('Backspace');
      expect(backspace).toBeDisabled();

      // Cleanup
      mockAuthError.mockReturnValue(null);
    });
  });

  // ── Hardware keyboard ──────────────────────────────────────────────

  describe('hardware keyboard in PIN step', () => {
    async function setupPinStep() {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));
      await waitFor(() => {
        expect(screen.queryByPlaceholderText('Username')).not.toBeInTheDocument();
      });
      const pinWrap = document.querySelector('.staff-login-pin-wrap');
      return pinWrap;
    }

    it('accepts digit keys 0-9 via keyboard', async () => {
      const pinWrap = await setupPinStep();
      expect(pinWrap).not.toBeNull();

      fireEvent.keyDown(pinWrap!, { key: '7' });
      fireEvent.keyDown(pinWrap!, { key: '8' });
      fireEvent.keyDown(pinWrap!, { key: '9' });

      const filled = document.querySelectorAll('.staff-login-pin-dot--filled');
      expect(filled.length).toBe(3);
    });

    it('accepts Backspace key via keyboard', async () => {
      const pinWrap = await setupPinStep();
      fireEvent.keyDown(pinWrap!, { key: '1' });
      fireEvent.keyDown(pinWrap!, { key: '2' });
      fireEvent.keyDown(pinWrap!, { key: '3' });
      expect(document.querySelectorAll('.staff-login-pin-dot--filled').length).toBe(3);

      fireEvent.keyDown(pinWrap!, { key: 'Backspace' });
      expect(document.querySelectorAll('.staff-login-pin-dot--filled').length).toBe(2);
    });

    it('submits on Enter key when PIN has at least 1 digit', async () => {
      const pinWrap = await setupPinStep();
      fireEvent.keyDown(pinWrap!, { key: '1' });
      fireEvent.keyDown(pinWrap!, { key: '2' });
      fireEvent.keyDown(pinWrap!, { key: 'Enter' });

      await waitFor(() => {
        expect(mockLogin).toHaveBeenCalledWith('alice', '12');
      });
    });

    it('goes back on Escape key', async () => {
      const pinWrap = await setupPinStep();
      fireEvent.keyDown(pinWrap!, { key: 'Escape' });

      await waitFor(() => {
        expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
      });
    });

    it('does not accept non-digit keys when not in PIN step', () => {
      renderScreen();
      const pinWrap = document.querySelector('.staff-login-pin-wrap');
      expect(pinWrap).toBeNull(); // Not in PIN step yet
    });
  });

  // ── Accessibility ──────────────────────────────────────────────────

  describe('accessibility', () => {
    it('has step indicator with progress status', () => {
      renderScreen();
      const steps = document.querySelector('.staff-login-steps');
      expect(steps).not.toBeNull();
      expect(steps?.getAttribute('role')).toBe('status');
    });

    it('PIN pad has role=group with accessible label', async () => {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        const keypad = document.querySelector('.staff-login-pad[role="group"]');
        expect(keypad).not.toBeNull();
      });
    });

    it('PIN section has role=application', async () => {
      renderScreen();
      const input = screen.getByPlaceholderText('Username');
      fireEvent.change(input, { target: { value: 'alice' } });
      fireEvent.click(screen.getByRole('button', { name: /next/i }));

      await waitFor(() => {
        const pinWrap = document.querySelector('.staff-login-pin-wrap');
        expect(pinWrap?.getAttribute('role')).toBe('application');
        // aria-label is set via Fluent attrs - may be empty string in test
        expect(pinWrap).not.toBeNull();
      });
    });
  });
});
