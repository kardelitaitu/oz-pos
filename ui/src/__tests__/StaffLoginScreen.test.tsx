import { describe, expect, it, vi, beforeAll } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import type { ReactElement, ReactNode } from 'react';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { ToastProvider } from '@/frontend/shared/Toast';
import StaffLoginScreen from '@/features/auth/StaffLoginScreen';
import { checkUsername } from '@/api/staff';

const mockLogin = vi.fn();
const mockLogout = vi.fn();
const mockClearError = vi.fn();

vi.mock('@/api/staff', () => ({
  checkUsername: vi.fn(() => Promise.resolve({ found: true, is_active: true })),
}));

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
staff-login-error-deactivated = Account is deactivated
staff-login-error-not-found = User not found
staff-login-error-connection = Could not verify username. Check your connection.
`));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}><ToastProvider>{children}</ToastProvider></LocalizationProvider>;
}

function renderScreen() {
  return render(withFluent(<StaffLoginScreen />));
}

describe('StaffLoginScreen', () => {
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

    const pinWrap = document.querySelector('.staff-login-pin-wrap')!;
    expect(pinWrap).toBeTruthy();

    const card = document.querySelector('.staff-login-card')!;
    await user.click(card);

    expect(document.activeElement).toBe(pinWrap);
  });

  it('shows deactivated toast and stays on username step when account is inactive', async () => {
    vi.mocked(checkUsername).mockResolvedValueOnce({ found: true, is_active: false });
    const user = userEvent.setup();
    renderScreen();

    const input = screen.getByRole('textbox', { name: /username/i });
    await user.type(input, 'deactivated_user');
    await user.click(screen.getByRole('button', { name: /next/i }));

    const toast = await screen.findByRole('alert');
    expect(toast).toHaveTextContent('Account is deactivated');

    // should NOT advance to PIN step
    expect(screen.getByRole('button', { name: /next/i })).toBeInTheDocument();
  });
});

// ── CSS integrity: guard against regression of empty or missing :focus-visible rules ──
// Uses readFileSync to inspect the CSS source directly; JSDOM with css:false
// cannot reliably reflect CSS rules via getComputedStyle.

describe('StaffLoginScreen CSS integrity', () => {
  const UI_SRC = resolve(__dirname, '..');
  const CSS_PATH = resolve(UI_SRC, 'features', 'auth', 'StaffLoginScreen.css');

  let css: string;

  beforeAll(() => {
    css = readFileSync(CSS_PATH, 'utf-8');
  });

  it('has a non-empty .staff-login-pin-wrap:focus-visible rule in the CSS', () => {
    // Find the rule block for .staff-login-pin-wrap:focus-visible
    const ruleMatch = css.match(/\.staff-login-pin-wrap:focus-visible\s*\{([^}]*)\}/);
    expect(ruleMatch,
      '.staff-login-pin-wrap:focus-visible rule must exist in StaffLoginScreen.css',
    ).not.toBeNull();

    const ruleBody = ruleMatch![1]!.trim();
    expect(ruleBody,
      '.staff-login-pin-wrap:focus-visible rule body must not be empty — ' +
      'an empty :focus-visible block causes the browser default blue ' +
      'outline to appear on the PIN keyboard wrapper',
    ).not.toHaveLength(0);
  });

  it('has outline: none on .staff-login-pin-wrap:focus-visible', () => {
    const ruleMatch = css.match(/\.staff-login-pin-wrap:focus-visible\s*\{([^}]*)\}/);
    expect(ruleMatch,
      '.staff-login-pin-wrap:focus-visible rule must exist in StaffLoginScreen.css',
    ).not.toBeNull();

    const ruleBody = ruleMatch![1]!.trim();

    // The rule should suppress the visible outline since visual feedback
    // is provided by PIN dots and keypad interactions.
    expect(ruleBody).toContain('outline: none');
  });

  it('has no empty :focus-visible rules in the CSS file', () => {
    // Find ALL :focus-visible rules
    const focusRules = css.match(/[^,{}]*:focus-visible[^{]*\{[^}]*\}/g) || [];

    const emptyRules = focusRules.filter((rule) => {
      const body = rule.slice(rule.indexOf('{') + 1, -1).trim();
      return body.length === 0;
    });

    expect(emptyRules,
      'No :focus-visible rules should have an empty body. ' +
      'Empty :focus-visible blocks let the browser default blue outline show through. ' +
      `Found ${emptyRules.length} empty rule(s): ${emptyRules.join('; ')}`,
    ).toHaveLength(0);
  });
});
