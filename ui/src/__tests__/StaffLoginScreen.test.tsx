import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import type { ReactElement, ReactNode } from 'react';
import StaffLoginScreen from '@/features/auth/StaffLoginScreen';

const mockLogin = vi.fn();
const mockLogout = vi.fn();
const mockClearError = vi.fn();

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: null,
    loading: false,
    error: null,
    login: mockLogin,
    logout: mockLogout,
    clearError: mockClearError,
    isManager: false,
    isOwner: false,
  }),
}));

function withFluent(children: ReactNode): ReactElement {
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
staff-login-back = \u2190 Back
staff-login-submit = Login
`));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
}

function renderScreen() {
  return render(withFluent(<StaffLoginScreen />));
}

describe('StaffLoginScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('focuses username input when the screen background is clicked', async () => {
    const user = userEvent.setup();
    renderScreen();

    const input = screen.getByRole('textbox', { name: /username/i });

    const screenEl = document.querySelector('.staff-login-screen')!;
    await user.click(screenEl);

    expect(document.activeElement).toBe(input);
  });

  it('focuses username input when the card area is clicked', async () => {
    const user = userEvent.setup();
    renderScreen();

    const input = screen.getByRole('textbox', { name: /username/i });

    const card = document.querySelector('.staff-login-card')!;
    await user.click(card);

    expect(document.activeElement).toBe(input);
  });

  it('focuses the pin section when the screen is clicked on the PIN step', async () => {
    const user = userEvent.setup();
    renderScreen();

    const input = screen.getByRole('textbox', { name: /username/i });
    await user.type(input, 'alice');
    await user.click(screen.getByRole('button', { name: /next/i }));

    const pinSection = document.querySelector('.staff-login-pin-section')!;
    expect(pinSection).toBeTruthy();

    const card = document.querySelector('.staff-login-card')!;
    await user.click(card);

    expect(document.activeElement).toBe(pinSection);
  });
});
