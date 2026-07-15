import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, createEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LicenseActivationScreen from '../LicenseActivationScreen';
import { activateLicense, getMachineId } from '@/api/license';
import { getVersion, getLocalIp, type VersionInfo } from '@/api/system';
import { readText } from '@tauri-apps/plugin-clipboard-manager';

const mockAddToast = vi.fn();
const mockOnActivated = vi.fn();

vi.mock('@/frontend/shared/Toast', () => ({
  useToast: () => ({ addToast: mockAddToast })
}));

vi.mock('@/api/license', () => ({
  activateLicense: vi.fn(),
  getMachineId: vi.fn()
}));

vi.mock('@/api/system', () => ({
  getVersion: vi.fn(),
  getLocalIp: vi.fn()
}));


vi.mock('@fluent/react', () => ({
  Localized: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useLocalization: () => ({ l10n: {
    getString: (id: string, args?: Record<string, unknown>) => {
      const map = {
        'auth-validation-required': 'License key and Email are required.',
        'auth-validation-invalid-email': 'Invalid email format.',
        'auth-activation-success': 'License activated successfully!',
        'auth-activation-failed': 'Failed to activate license.',
        'auth-activation-error': 'An error occurred during activation.',
        'auth-clipboard-error': 'Clipboard error: ' + (args && args['message'] ? args['message'] : ''),
        'auth-error-title': 'Error',
        'auth-version': 'Version ' + (args ? args['version'] : ''),
        'auth-ip-address': 'IP Address : ' + (args ? args['ip'] : ''),
        'auth-copyright': 'OZ-POS © ' + (args ? args['year'] : '') + ' All rights reserved.',
        'auth-email-placeholder': 'store@example.com',
        'auth-phone-placeholder': '08123456789',
        'auth-license-placeholder': 'OZ-PRO-XXXX-XXXX-XXXX',
        'auth-paste': 'Paste',
        'auth-activating': 'Activating...',
        'auth-activate-button': 'Activate License',
        'auth-activate-title': 'Activate License',
        'auth-activate-subtitle': 'Enter your information below',
        'auth-email-label': 'Email Address',
        'auth-phone-label': 'Phone Number',
        'auth-license-label': 'License Key',
        'auth-validation-phone-required': 'Phone number is required.',
        'auth-validation-invalid-phone': 'Invalid phone number format.'
      };
      return (map as Record<string, string>)[id] || id;
    }
  } })
}));

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  readText: vi.fn()
}));

vi.mock('@/components/ConnectionStatus', () => ({
  default: ({ label, url }: { label: string; url: string }) => <div data-testid={`conn-status-${label}`}>{label}: {url}</div>
}));
vi.mock('@/components/MachineIdStatus', () => ({
  default: () => <div data-testid="conn-status-machine-id">Machine Status</div>
}));
vi.mock('@/frontend/shell/ThemeToggle', () => ({
  default: () => <div data-testid="theme-toggle">ThemeToggle</div>,
}));

describe('LicenseActivationScreen - Exhaustive Suite', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getVersion).mockResolvedValue({ version: '1.0.0', name: 'oz-pos', rustVersion: '1.70', target: 'windows' });
    vi.mocked(getLocalIp).mockResolvedValue('192.168.1.100');
    vi.mocked(getMachineId).mockResolvedValue('test-machine-id');
    vi.mocked(activateLicense).mockResolvedValue(true);
    vi.mocked(readText).mockResolvedValue('clipboard-text');
  });

  describe('1. Mounting & Lifecycle', () => {
    it('1. getVersion resolves and displays the correct version on mount', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      await waitFor(() => expect(screen.getByText('Version 1.0.0')).toBeInTheDocument());
    });

    it('2. getLocalIp resolves and displays the correct IP on mount', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      await waitFor(() => expect(screen.getByText('IP Address : 192.168.1.100')).toBeInTheDocument());
    });

    it('3. getLocalIp rejects and gracefully falls back to "Unknown"', async () => {
      vi.mocked(getLocalIp).mockRejectedValue(new Error('IP Fail'));
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      await waitFor(() => expect(screen.getByText('IP Address : Unknown')).toBeInTheDocument());
    });

    it('4. getVersion rejects gracefully without crashing the app', async () => {
      vi.mocked(getVersion).mockRejectedValue(new Error('Version Fail'));
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      await waitFor(() => expect(screen.getByText('Version 0.0.8')).toBeInTheDocument());
    });

    it('5. Component unmounting during getVersion fetch prevents state updates', () => {
      let resolveVersion: (value: VersionInfo) => void = () => {};
      const promise = new Promise<VersionInfo>(resolve => { resolveVersion = resolve; });
      vi.mocked(getVersion).mockReturnValue(promise);
      
      const { unmount } = render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      unmount();
      expect(() => resolveVersion({ name: 'oz-pos', version: '9.9.9', rustVersion: '1.75', target: 'x86' })).not.toThrow();
    });

    it('6. Component unmounting during getLocalIp fetch prevents state updates', () => {
      let resolveIp: (value: string) => void = () => {};
      const promise = new Promise<string>(resolve => { resolveIp = resolve; });
      vi.mocked(getLocalIp).mockReturnValue(promise);
      
      const { unmount } = render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      unmount();
      expect(() => resolveIp('1.1.1.1')).not.toThrow();
    });
  });

  describe('2. Form Rendering & Input Validation', () => {
    it('7. Email input is present, enabled, and accepts typing', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      expect(emailInput).toBeEnabled();
      fireEvent.change(emailInput, { target: { value: 'test@example.com' } });
      expect(emailInput).toHaveValue('test@example.com');
    });

    it('8. Phone input is present, enabled, and accepts typing', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const phoneInput = screen.getByLabelText(/Phone Number/i);
      expect(phoneInput).toBeEnabled();
      fireEvent.change(phoneInput, { target: { value: '1234' } });
      expect(phoneInput).toHaveValue('1234');
    });

    it('9. License Key input is present, enabled, and accepts typing', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const keyInput = screen.getByLabelText(/License Key/i);
      expect(keyInput).toBeEnabled();
      fireEvent.change(keyInput, { target: { value: '1234' } });
      expect(keyInput).toHaveValue('1234');
    });

    it('10. License Key strictly forces characters to uppercase', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const keyInput = screen.getByLabelText(/License Key/i);
      fireEvent.change(keyInput, { target: { value: 'aBcDeFg' } });
      expect(keyInput).toHaveValue('ABCDEFG');
    });

    it('11. Activate License button is disabled initially', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      expect(screen.getByRole('button', { name: /Activate License/i })).toBeDisabled();
    });

    it('12. Activate License button is disabled if email is filled but key is empty', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      expect(screen.getByRole('button', { name: /Activate License/i })).toBeDisabled();
    });

    it('13. Activate License button is disabled if key is filled but email is empty', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      expect(screen.getByRole('button', { name: /Activate License/i })).toBeDisabled();
    });

    it('14. Activate License button is enabled only when email, phone, and key have text', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      expect(screen.getByRole('button', { name: /Activate License/i })).toBeEnabled();
    });

    it('15. Inline Clear button correctly clears the Email field', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      fireEvent.change(emailInput, { target: { value: 'test@test.com' } });
      const clearBtn = screen.getAllByRole('button').find(b => b.className === 'license-input-clear')!;
      await userEvent.click(clearBtn);
      expect(emailInput).toHaveValue('');
    });

    it('16. Inline Clear button correctly clears the Phone field', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const phoneInput = screen.getByLabelText(/Phone Number/i);
      fireEvent.change(phoneInput, { target: { value: '1234' } });
      const clearBtn = screen.getAllByRole('button').find(b => b.className === 'license-input-clear')!;
      await userEvent.click(clearBtn);
      expect(phoneInput).toHaveValue('');
    });

    it('17. Inline Clear button correctly clears the License Key field', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const keyInput = screen.getByLabelText(/License Key/i);
      fireEvent.change(keyInput, { target: { value: 'KEY123' } });
      const clearBtn = screen.getAllByRole('button').find(b => b.className === 'license-input-clear')!;
      await userEvent.click(clearBtn);
      expect(keyInput).toHaveValue('');
    });
  });

  describe('3. Loading State Behavior', () => {
    it('18. Inputs become disabled when loading is true', async () => {
      let resolveActivate: (value: boolean) => void = () => {};
      const promise = new Promise<boolean>(resolve => { resolveActivate = resolve; });
      vi.mocked(activateLicense).mockReturnValue(promise);
      
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      expect(screen.getByLabelText(/Email Address/i)).toBeDisabled();
      expect(screen.getByLabelText(/Phone Number/i)).toBeDisabled();
      expect(screen.getByLabelText(/License Key/i)).toBeDisabled();
      
      resolveActivate(true);
    });

    it('19. Clear buttons disappear while loading is true', async () => {
      let resolveActivate: (value: boolean) => void = () => {};
      vi.mocked(activateLicense).mockReturnValue(new Promise<boolean>(resolve => { resolveActivate = resolve; }));
      
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      expect(screen.queryByRole('button', { name: /clear/i })).not.toBeInTheDocument();
      resolveActivate(true);
    });

    it('20. The Submit button becomes disabled while loading is true', async () => {
      let resolveActivate: (value: boolean) => void = () => {};
      vi.mocked(activateLicense).mockReturnValue(new Promise<boolean>(resolve => { resolveActivate = resolve; }));
      
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      const submitBtn = screen.getByRole('button', { name: /Activate License/i });
      await userEvent.click(submitBtn);
      
      expect(submitBtn).toBeDisabled();
      resolveActivate(true);
    });

    it('21. The Submit button text changes to "Activating..." when loading', async () => {
      let resolveActivate: (value: boolean) => void = () => {};
      vi.mocked(activateLicense).mockReturnValue(new Promise<boolean>(resolve => { resolveActivate = resolve; }));
      
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      expect(screen.getByText(/Activating\.\.\./i)).toBeInTheDocument();
      resolveActivate(true);
    });

    it('22. A loading spinner SVG is rendered inside the Submit button while loading', async () => {
      let resolveActivate: (value: boolean) => void = () => {};
      vi.mocked(activateLicense).mockReturnValue(new Promise<boolean>(resolve => { resolveActivate = resolve; }));
      
      const { container } = render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      expect(container.querySelector('svg.spinner')).toBeInTheDocument();
      resolveActivate(true);
    });
  });

  describe('4. Form Submission & API Calls', () => {
    it('23. Submitting the form clears any pre-existing inline error messages', async () => {
      vi.mocked(activateLicense).mockResolvedValueOnce(false).mockResolvedValueOnce(true);
      
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      await waitFor(() => expect(screen.getByText('Failed to activate license.')).toBeInTheDocument());
      
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      expect(screen.queryByText('Failed to activate license.')).not.toBeInTheDocument();
    });

    it('24. Submitting with whitespace-only Key shows validation error', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@example.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: '   ' } });
      
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      expect(screen.getByText('License key and Email are required.')).toBeInTheDocument();
      expect(activateLicense).not.toHaveBeenCalled();
    });

    it('25. Submitting trims whitespace from the Email payload', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: '  test@test.com  ' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '  08123456789  ' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(activateLicense).toHaveBeenCalledWith('KEY123', 'test@test.com', 'test-machine-id', '08123456789'));
    });

    it('26. Submitting trims whitespace from the License Key payload', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: '  KEY123  ' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(activateLicense).toHaveBeenCalledWith('KEY123', 'test@test.com', 'test-machine-id', '08123456789'));
    });

    it('27. Submitting trims whitespace from the Phone payload', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '  08123456789  ' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(activateLicense).toHaveBeenCalledWith('KEY123', 'test@test.com', 'test-machine-id', '08123456789'));
    });

    it('28. Happy path: Successful activation calls getMachineId, activateLicense, fires success toast', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => {
        expect(getMachineId).toHaveBeenCalled();
        expect(activateLicense).toHaveBeenCalled();
        expect(mockAddToast).toHaveBeenCalledWith({ type: 'success', message: 'License activated successfully!' });
        expect(mockOnActivated).toHaveBeenCalled();
      });
    });

    it('29. API returns false: Displays the specific inline red error banner', async () => {
      vi.mocked(activateLicense).mockResolvedValue(false);
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(screen.getByText('Failed to activate license.')).toBeInTheDocument());
    });

    it('30. Form handles extremely long input strings without UI crashing', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const longStr = 'a'.repeat(500);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: longStr } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY' } });
      expect(screen.getByLabelText(/Email Address/i)).toHaveValue(longStr);
    });

    it('31. Multiple rapid submission attempts are blocked', async () => {
      let resolveActivate: (value: boolean) => void = () => {};
      vi.mocked(activateLicense).mockReturnValue(new Promise<boolean>(resolve => { resolveActivate = resolve; }));
      
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      
      const submitBtn = screen.getByRole('button', { name: /Activate License/i });
      await userEvent.click(submitBtn);
      await userEvent.click(submitBtn); // Should be ignored as it's disabled
      
      expect(activateLicense).toHaveBeenCalledTimes(1);
      resolveActivate(true);
    });
  });

  describe('5. Error Catching & Formatting', () => {
    it('32. Thrown Error instance: Fires an error toast with err.message', async () => {
      vi.mocked(activateLicense).mockRejectedValue(new Error('Network Failure 500'));
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(mockAddToast).toHaveBeenCalledWith({ type: 'error', message: 'Network Failure 500' }));
    });

    it('33. Thrown string primitive: Fires an error toast using the string itself', async () => {
      vi.mocked(activateLicense).mockRejectedValue('String Error');
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(mockAddToast).toHaveBeenCalledWith({ type: 'error', message: 'String Error' }));
    });

    it('34. Thrown object with message property: Fires an error toast parsing the message field', async () => {
      vi.mocked(activateLicense).mockRejectedValue({ message: 'Object Error' });
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(mockAddToast).toHaveBeenCalledWith({ type: 'error', message: 'Object Error' }));
    });

    it('35. Thrown unknown object: Gracefully falls back to stringifying the unknown object', async () => {
      vi.mocked(activateLicense).mockRejectedValue({ unknown: true });
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      fireEvent.change(screen.getByLabelText(/Email Address/i), { target: { value: 'test@test.com' } });
      fireEvent.change(screen.getByLabelText(/Phone Number/i), { target: { value: '08123456789' } });
      fireEvent.change(screen.getByLabelText(/License Key/i), { target: { value: 'KEY123' } });
      await userEvent.click(screen.getByRole('button', { name: /Activate License/i }));
      
      await waitFor(() => expect(mockAddToast).toHaveBeenCalledWith({ type: 'error', message: 'An error occurred during activation.' }));
    });
  });

  describe('6. Custom Context Menu & Pasting', () => {
    it('36. Right-clicking an input opens the context menu exactly at the mouse coordinates', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      fireEvent.contextMenu(emailInput, { clientX: 150, clientY: 250 });
      
      const menu = screen.getByText('Paste');
      expect(menu).toBeInTheDocument();
      expect(menu).toHaveStyle('top: 250px');
      expect(menu).toHaveStyle('left: 150px');
    });

    it('37. Right-clicking the container prevents the default browser menu and ensures custom menu is closed', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      
      const container = document.querySelector('.license-activation-container')!;
      const event = createEvent.contextMenu(container);
      fireEvent(container, event);
      
      expect(event.defaultPrevented).toBe(true);
      expect(screen.queryByText('Paste')).not.toBeInTheDocument();
    });

    it('38. Clicking the container (global click) closes an open context menu', async () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      fireEvent.contextMenu(emailInput, { clientX: 150, clientY: 250 });
      
      expect(screen.getByText('Paste')).toBeInTheDocument();
      
      const container = document.querySelector('.license-activation-container')!;
      await userEvent.click(container);
      
      expect(screen.queryByText('Paste')).not.toBeInTheDocument();
    });

    it('39. Right-clicking an input while context menu is already open relocates the menu', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      
      fireEvent.contextMenu(emailInput, { clientX: 100, clientY: 100 });
      expect(screen.getByText('Paste')).toHaveStyle('top: 100px');
      
      fireEvent.contextMenu(emailInput, { clientX: 200, clientY: 200 });
      expect(screen.getByText('Paste')).toHaveStyle('top: 200px');
    });

    it('40. Pasting into the Email field updates ONLY the email field', async () => {
      vi.mocked(readText).mockResolvedValue('test@paste.com');
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      
      fireEvent.contextMenu(emailInput, { clientX: 100, clientY: 100 });
      await userEvent.click(screen.getByText('Paste'));
      
      await waitFor(() => expect(emailInput).toHaveValue('test@paste.com'));
      expect(screen.getByLabelText(/Phone Number/i)).toHaveValue('');
    });

    it('41. Pasting into the Phone field updates ONLY the phone field', async () => {
      vi.mocked(readText).mockResolvedValue('0899999');
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const phoneInput = screen.getByLabelText(/Phone Number/i);
      
      fireEvent.contextMenu(phoneInput, { clientX: 100, clientY: 100 });
      await userEvent.click(screen.getByText('Paste'));
      
      await waitFor(() => expect(phoneInput).toHaveValue('0899999'));
      expect(screen.getByLabelText(/Email Address/i)).toHaveValue('');
    });

    it('42. Pasting into the License Key field updates ONLY the key field, and forces to uppercase', async () => {
      vi.mocked(readText).mockResolvedValue('oz-key-abc');
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const keyInput = screen.getByLabelText(/License Key/i);
      
      fireEvent.contextMenu(keyInput, { clientX: 100, clientY: 100 });
      await userEvent.click(screen.getByText('Paste'));
      
      await waitFor(() => expect(keyInput).toHaveValue('OZ-KEY-ABC'));
      expect(screen.getByLabelText(/Email Address/i)).toHaveValue('');
    });

    it('43. Clipboard returning empty text does nothing', async () => {
      vi.mocked(readText).mockResolvedValue('');
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      fireEvent.change(emailInput, { target: { value: 'existing@email.com' } });
      
      fireEvent.contextMenu(emailInput, { clientX: 100, clientY: 100 });
      await userEvent.click(screen.getByText('Paste'));
      
      await waitFor(() => expect(screen.queryByText('Paste')).not.toBeInTheDocument());
      expect(emailInput).toHaveValue('existing@email.com'); // Unchanged
    });

    it('44. Clipboard throwing an OS permission error is caught, fires an error toast', async () => {
      vi.mocked(readText).mockRejectedValue(new Error('Permission denied'));
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      
      fireEvent.contextMenu(emailInput, { clientX: 100, clientY: 100 });
      await userEvent.click(screen.getByText('Paste'));
      
      await waitFor(() => {
        expect(mockAddToast).toHaveBeenCalledWith({ type: 'error', message: 'Error: Clipboard error: Permission denied' });
      });
      expect(screen.queryByText('Paste')).not.toBeInTheDocument();
    });

    it('45. Component unmounting while readText() is awaiting does not cause errors', async () => {
      let resolveReadText: (value: string) => void = () => {};
      vi.mocked(readText).mockReturnValue(new Promise<string>(resolve => { resolveReadText = resolve; }));
      
      const { unmount } = render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const emailInput = screen.getByLabelText(/Email Address/i);
      
      fireEvent.contextMenu(emailInput, { clientX: 100, clientY: 100 });
      await userEvent.click(screen.getByText('Paste'));
      
      unmount();
      expect(() => resolveReadText('late@email.com')).not.toThrow();
    });
  });

  describe('7. Child Components & Hero', () => {
    it('46. Renders the ConnectionStatus for the Auth server with the correct URL', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const authConn = screen.getByTestId('conn-status-Auth');
      expect(authConn).toHaveTextContent('Auth: https://auth--oz-pos-license-service--76cyv4d6bn54.code.run');
    });

    it('47. Renders the ConnectionStatus for the Sync server', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const syncConn = screen.getByTestId('conn-status-Sync');
      expect(syncConn).toHaveTextContent('Sync:');
    });

    it('48. Renders the MachineIdStatus component', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      expect(screen.getByTestId('conn-status-machine-id')).toBeInTheDocument();
    });

    it('49. Renders the 256x256 OZ-POS logo hero image', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const img = screen.getByAltText('OZ-POS Logo');
      expect(img).toBeInTheDocument();
      expect(img).toHaveAttribute('src', '/256x256.png');
    });

    it('50. Renders the copyright footer with the current dynamic year', () => {
      render(<LicenseActivationScreen onActivated={mockOnActivated} />);
      const year = new Date().getFullYear().toString();
      expect(screen.getByText(new RegExp(`OZ-POS © ${year} All rights reserved.`))).toBeInTheDocument();
    });
  });
});
