import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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

describe('CreatePinScreen', () => {
  describe('validation', () => {
    it('shows error when all fields are whitespace-only', async () => {
      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), '   ');
      await user.type(screen.getByLabelText('Username'), '   ');
      await user.type(screen.getByLabelText('PIN'), '    ');
      await user.type(screen.getByLabelText('Confirm PIN'), '    ');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      expect(screen.getByRole('alert')).toHaveTextContent('All fields are required.');
    });

    it('shows error when display name is whitespace-only', async () => {
      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), '   ');
      await user.type(screen.getByLabelText('Username'), 'owner');
      await user.type(screen.getByLabelText('PIN'), '1234');
      await user.type(screen.getByLabelText('Confirm PIN'), '1234');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      expect(screen.getByRole('alert')).toHaveTextContent('All fields are required.');
    });

    it('shows error when PIN is too short', async () => {
      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), 'Owner');
      await user.type(screen.getByLabelText('Username'), 'owner');
      await user.type(screen.getByLabelText('PIN'), '12');
      await user.type(screen.getByLabelText('Confirm PIN'), '12');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      expect(screen.getByRole('alert')).toHaveTextContent('PIN must be at least 4 characters.');
    });

    it('shows error when PINs do not match', async () => {
      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), 'Owner');
      await user.type(screen.getByLabelText('Username'), 'owner');
      await user.type(screen.getByLabelText('PIN'), '1234');
      await user.type(screen.getByLabelText('Confirm PIN'), '5678');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      expect(screen.getByRole('alert')).toHaveTextContent('PINs do not match.');
    });
  });

  describe('API interaction', () => {
    it('calls bootstrapOwner on submit and navigates on success', async () => {
      mockBootstrapOwner.mockResolvedValue({
        session: { user_id: 'owner-1', display_name: 'Owner', role_name: 'owner', role_id: 'role-owner' },
      });

      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), 'Store Owner');
      await user.type(screen.getByLabelText('Username'), 'admin');
      await user.type(screen.getByLabelText('PIN'), '1234');
      await user.type(screen.getByLabelText('Confirm PIN'), '1234');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

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
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), 'Store Owner');
      await user.type(screen.getByLabelText('Username'), 'admin');
      await user.type(screen.getByLabelText('PIN'), '1234');
      await user.type(screen.getByLabelText('Confirm PIN'), '1234');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      await waitFor(() => {
        expect(onCreated).toHaveBeenCalled();
      });
    });

    it('displays error message on API failure', async () => {
      mockBootstrapOwner.mockRejectedValue(new Error('Network error'));

      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), 'Store Owner');
      await user.type(screen.getByLabelText('Username'), 'admin');
      await user.type(screen.getByLabelText('PIN'), '1234');
      await user.type(screen.getByLabelText('Confirm PIN'), '1234');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('Network error');
      });
    });

    it('handles string rejection', async () => {
      mockBootstrapOwner.mockRejectedValue('string error');

      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), 'Store Owner');
      await user.type(screen.getByLabelText('Username'), 'admin');
      await user.type(screen.getByLabelText('PIN'), '1234');
      await user.type(screen.getByLabelText('Confirm PIN'), '1234');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('string error');
      });
    });
  });

  describe('form inputs', () => {
    it('renders all form fields', () => {
      renderScreen();

      expect(screen.getByLabelText('Display Name')).toBeTruthy();
      expect(screen.getByLabelText('Username')).toBeTruthy();
      expect(screen.getByLabelText('PIN')).toBeTruthy();
      expect(screen.getByLabelText('Confirm PIN')).toBeTruthy();
    });

    it('disables inputs while loading', async () => {
      mockBootstrapOwner.mockImplementation(() => new Promise(() => {}));

      renderScreen();
      const user = userEvent.setup();

      await user.type(screen.getByLabelText('Display Name'), 'Store Owner');
      await user.type(screen.getByLabelText('Username'), 'admin');
      await user.type(screen.getByLabelText('PIN'), '1234');
      await user.type(screen.getByLabelText('Confirm PIN'), '1234');
      await user.click(screen.getByRole('button', { name: /create owner account/i }));

      await waitFor(() => {
        expect(screen.getByLabelText('Display Name')).toBeDisabled();
        expect(screen.getByLabelText('Username')).toBeDisabled();
        expect(screen.getByLabelText('PIN')).toBeDisabled();
        expect(screen.getByLabelText('Confirm PIN')).toBeDisabled();
      });
    });

    it('converts username to lowercase', async () => {
      renderScreen();
      const user = userEvent.setup();

      const input = screen.getByLabelText('Username') as HTMLInputElement;
      await user.type(input, 'ADMIN');

      expect(input.value).toBe('admin');
    });
  });

  describe('form labels', () => {
    it('uses placeholder attributes on inputs', () => {
      renderScreen();

      const displayName = screen.getByLabelText('Display Name') as HTMLInputElement;
      const username = screen.getByLabelText('Username') as HTMLInputElement;
      const pin = screen.getByLabelText('PIN') as HTMLInputElement;
      const confirmPin = screen.getByLabelText('Confirm PIN') as HTMLInputElement;

      expect(displayName.placeholder).toBe('Store Owner');
      expect(username.placeholder).toBe('owner');
      expect(pin.placeholder).toBe('At least 4 digits');
      expect(confirmPin.placeholder).toBe('Re-enter PIN');
    });
  });
});
