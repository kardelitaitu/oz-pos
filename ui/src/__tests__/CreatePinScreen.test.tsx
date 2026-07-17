// ── CreatePinScreen interaction tests ──────────────────────────────
//
// Covers: form validation, API interaction, form input behavior.
// Uses fireEvent.change for form fields (saves ~20ms/char vs
// userEvent.type — the component only reads values on submit)
// and fireEvent.click for the submit button.
// 10 tests (2 sync render tests moved to CreatePinScreenRender.test.tsx).

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import CreatePinScreen from '@/features/auth/CreatePinScreen';
import sharedFtl from '@/locales/shared.ftl?raw';

const mockBootstrapOwner = vi.fn();
const mockSwapSession = vi.fn();
const mockAddToast = vi.fn();

vi.mock('@/api/staff', () => ({
  bootstrapOwner: (...args: unknown[]) => mockBootstrapOwner(...args),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: null,
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    swapSession: (...args: unknown[]) => mockSwapSession(...args),
  }),
}));

vi.mock('@/frontend/shared/Toast', async (importOriginal) => {
  // eslint-disable-next-line @typescript-eslint/consistent-type-imports
  const actual = await importOriginal<typeof import('@/frontend/shared/Toast')>();
  return {
    ...actual,
    useToast: () => ({ addToast: (...args: unknown[]) => mockAddToast(...args) }),
  };
});

beforeEach(() => {
  mockBootstrapOwner.mockReset();
  mockSwapSession.mockReset();
  mockAddToast.mockReset();
});

const onCreated = vi.fn();

function renderScreen() {
  return renderWithProvidersSync(<CreatePinScreen onCreated={onCreated} />, sharedFtl);
}

// ── Field helpers (fireEvent.change ~1ms vs userEvent.type ~20ms/char) ─

function fillField(label: string, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
}

function fillAllFields(overrides: Partial<Record<string, string>> = {}) {
  fillField('Display Name', overrides['Display Name'] ?? 'Store Owner');
  fillField('Username', overrides['Username'] ?? 'admin');
  fillField('PIN', overrides['PIN'] ?? '1234');
  fillField('Confirm PIN', overrides['Confirm PIN'] ?? '1234');
}

function clickSubmit() {
  fireEvent.click(screen.getByRole('button', { name: /create owner account/i }));
}

describe('CreatePinScreen', () => {
  describe('validation', () => {
    it('shows error when all fields are whitespace-only', () => {
      renderScreen();

      fillField('Display Name', '   ');
      fillField('Username', '   ');
      fillField('PIN', '    ');
      fillField('Confirm PIN', '    ');
      clickSubmit();

      expect(screen.getByRole('alert')).toHaveTextContent('All fields are required.');
    });

    it('shows error when display name is whitespace-only', () => {
      renderScreen();

      fillField('Display Name', '   ');
      fillField('Username', 'owner');
      fillField('PIN', '1234');
      fillField('Confirm PIN', '1234');
      clickSubmit();

      expect(screen.getByRole('alert')).toHaveTextContent('All fields are required.');
    });

    it('shows error when PIN is too short', () => {
      renderScreen();

      fillField('Display Name', 'Owner');
      fillField('Username', 'owner');
      fillField('PIN', '12');
      fillField('Confirm PIN', '12');
      clickSubmit();

      expect(screen.getByRole('alert')).toHaveTextContent('PIN must be at least 4 characters.');
    });

    it('shows error when PINs do not match', () => {
      renderScreen();

      fillField('Display Name', 'Owner');
      fillField('Username', 'owner');
      fillField('PIN', '1234');
      fillField('Confirm PIN', '5678');
      clickSubmit();

      expect(screen.getByRole('alert')).toHaveTextContent('PINs do not match.');
    });
  });

  describe('API interaction', () => {
    it('calls bootstrapOwner on submit and navigates on success', async () => {
      mockBootstrapOwner.mockResolvedValue({
        session: { user_id: 'owner-1', display_name: 'Owner', role_name: 'owner', role_id: 'role-owner' },
      });

      renderScreen();
      fillAllFields();
      clickSubmit();

      await waitFor(() => {
        expect(mockBootstrapOwner).toHaveBeenCalledWith({
          username: 'admin',
          pin: '1234',
          display_name: 'Store Owner',
        });
      });

      expect(mockSwapSession).toHaveBeenCalledWith({
        user_id: 'owner-1', display_name: 'Owner', role_name: 'owner', role_id: 'role-owner',
      });
      expect(mockAddToast).toHaveBeenCalled();
      expect(onCreated).toHaveBeenCalled();
    });

    it('navigates on "already exist" error', async () => {
      mockBootstrapOwner.mockRejectedValue(new Error('Staff already exist'));

      renderScreen();
      fillAllFields();
      clickSubmit();

      await waitFor(() => {
        expect(onCreated).toHaveBeenCalled();
      });
    });

    it('displays error message on API failure', async () => {
      mockBootstrapOwner.mockRejectedValue(new Error('Network error'));

      renderScreen();
      fillAllFields();
      clickSubmit();

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('Network error');
      });
    });

    it('handles string rejection', async () => {
      mockBootstrapOwner.mockRejectedValue('string error');

      renderScreen();
      fillAllFields();
      clickSubmit();

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('string error');
      });
    });
  });

  describe('form inputs', () => {
    it('disables inputs while loading', async () => {
      mockBootstrapOwner.mockImplementation(() => new Promise(() => {}));

      renderScreen();
      fillAllFields();
      clickSubmit();

      await waitFor(() => {
        expect(screen.getByLabelText('Display Name')).toBeDisabled();
        expect(screen.getByLabelText('Username')).toBeDisabled();
        expect(screen.getByLabelText('PIN')).toBeDisabled();
        expect(screen.getByLabelText('Confirm PIN')).toBeDisabled();
      });
    });

    it('converts username to lowercase', () => {
      renderScreen();

      const input = screen.getByLabelText('Username') as HTMLInputElement;
      fireEvent.change(input, { target: { value: 'ADMIN' } });

      expect(input.value).toBe('admin');
    });
  });
});
