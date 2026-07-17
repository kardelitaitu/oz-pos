/**
 * @file CloudSyncSettings.test.tsx
 * @description Comprehensive test suite for the Cloud Sync section in SettingsPage.
 *
 * Covers:
 *   - Navigation to sync section
 *   - Server URL field rendering, editing, and save
 *   - API key field rendering with masked/unmasked placeholder
 *   - API key visibility toggle
 *   - Enabled toggle
 *   - Not-configured hint visibility
 *   - Sync Now button visibility
 *   - Save flow: correct args sent, state persisted
 *   - hasApiKey and serverUrl state update after save
 *   - API key cleared after successful save, retained after failed save
 */

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, cleanup, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import settingsFtl from '@/locales/settings.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import SettingsPage from '@/features/settings/SettingsPage';
import { AuthProvider } from '@/contexts/AuthContext';
import { BrandProvider } from '@/contexts/BrandContext';
import { CurrencyProvider } from '@/contexts/CurrencyContext';
import { LocaleContext } from '@/i18n/LocaleContext';
import { getAvailableLocales, getLocaleLabel } from '@/i18n';

// ── Mock infra ────────────────────────────────────────────────────

const { invokeMock, defaultImpl, failCommands, lastCallArgs } = vi.hoisted(() => {
  const failCommands = new Set<string>();
  const lastCallArgs = new Map<string, unknown>();

  const impl = (cmd: string, args?: unknown): Promise<unknown> => {
    if (failCommands.has(cmd)) {
      return Promise.reject(new Error(`Mock failure: ${cmd}`));
    }
    if (args && typeof args === 'object') {
      lastCallArgs.set(cmd, args);
    }
    if (cmd === 'get_store_settings') {
      return Promise.resolve({ name: '', address: '', taxId: '', currency: 'IDR', branch: '' });
    }
    if (cmd === 'get_receipt_settings') {
      return Promise.resolve({
        showCurrency: false, decimalSeparator: 'dot', showTax: true, footer: '',
        paperWidth: 'standard', showTableNumber: false,
        marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
      });
    }
    if (cmd === 'list_currencies') {
      return Promise.resolve([{ code: 'USD', name: 'US Dollar', minor_exponent: 2, symbol: '$' }]);
    }
    if (cmd === 'get_default_currency') {
      return Promise.resolve('USD');
    }
    if (cmd === 'get_sync_settings') {
      return Promise.resolve({ serverUrl: null, hasApiKey: false, enabled: false });
    }
    if (cmd === 'get_user_preferences') {
      return Promise.resolve({ cardsize: '2', fontsize: '1', 'font-smoothing': 'antialiased' });
    }
    if (cmd === 'get_brand_settings') {
      return Promise.resolve({ primary_colour: '#4f46e5', logo_path: null, store_name: '' });
    }
    if (cmd === 'version') {
      return Promise.resolve({ name: 'oz-pos', version: '0.0.9', rustVersion: '1.80', target: 'x86_64' });
    }
    if (
      cmd === 'set_receipt_settings' || cmd === 'set_store_settings' ||
      cmd === 'set_default_currency' || cmd === 'set_user_preferences' ||
      cmd === 'update_sync_settings' || cmd === 'set_brand_primary_colour' ||
      cmd === 'set_brand_store_name'
    ) {
      return Promise.resolve(undefined);
    }
    if (cmd === 'sync_run') {
      return Promise.resolve({ synced: 3, failed: 0, error: null });
    }
    return Promise.resolve(undefined);
  };
  return { invokeMock: vi.fn(impl), defaultImpl: impl, failCommands, lastCallArgs };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

vi.mock('@/contexts/ZoomContext', () => ({
  useAppZoom: () => ({ zoomLevel: 'auto', setZoomLevel: vi.fn() }),
  ZoomProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('@/contexts/HardwareAccelContext', () => ({
  useHardwareAccel: () => ({ enabled: true, setEnabled: vi.fn() }),
  HardwareAccelProvider: ({ children }: { children: React.ReactNode }) => children,
}));

Element.prototype.scrollIntoView = vi.fn();

beforeEach(() => {
  cleanup();
  failCommands.clear();
  lastCallArgs.clear();
  invokeMock.mockReset();
  invokeMock.mockImplementation(defaultImpl);
});

afterEach(() => {
  cleanup();
});

function TestWrapper({ children }: { children: React.ReactNode }) {
  return (
    <LocaleContext.Provider
      value={{
        locale: 'en',
        setLocale: () => {},
        availableLocales: getAvailableLocales(),
        getLocaleLabel,
      }}
    >
      <BrandProvider>
        <CurrencyProvider>
          <AuthProvider>{children}</AuthProvider>
        </CurrencyProvider>
      </BrandProvider>
    </LocaleContext.Provider>
  );
}

// ── Helper: navigate to Cloud Sync section ───────────────────────

function navigateToSync() {
  fireEvent.click(screen.getByRole('button', { name: /operations/i }));
  fireEvent.click(screen.getByRole('button', { name: /cloud sync/i }));
}

async function waitForSyncSection() {
  renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
  await waitFor(() => {
    expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
  });
  navigateToSync();
}

// ── Helpers ──────────────────────────────────────────────────────

function getApiKeyInput(): HTMLInputElement {
  return screen.getByLabelText(/^api key$/i) as HTMLInputElement;
}

function getServerUrlInput(): HTMLInputElement {
  return screen.getByLabelText(/server url/i) as HTMLInputElement;
}

function getEnabledCheckbox(): HTMLInputElement {
  return screen.getByLabelText(/enable cloud sync/i) as HTMLInputElement;
}

describe('CloudSyncSettings', () => {
  // ═══════════════════════════════════════════════════════════════
  //  Navigation
  // ═══════════════════════════════════════════════════════════════

  it('navigates to Cloud Sync section after clicking sidebar nav item', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });

    navigateToSync();

    expect(screen.getAllByRole('heading', { name: /cloud sync/i }).length).toBeGreaterThanOrEqual(1);
  });

  // ═══════════════════════════════════════════════════════════════
  //  Server URL field
  // ═══════════════════════════════════════════════════════════════

  it('renders server URL input with placeholder', async () => {
    await waitForSyncSection();

    const urlInput = getServerUrlInput();
    expect(urlInput).toBeInTheDocument();
    expect(urlInput.type).toBe('url');
    expect(urlInput).toHaveValue('');
  });

  it('updates server URL input value when typing', async () => {
    await waitForSyncSection();

    const urlInput = getServerUrlInput();
    fireEvent.change(urlInput, { target: { value: 'https://sync.example.com' } });
    expect(urlInput).toHaveValue('https://sync.example.com');
  });

  it('sends server URL to backend on save', async () => {
    await waitForSyncSection();

    fireEvent.change(getServerUrlInput(), { target: { value: 'https://sync.example.com' } });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    const syncArgs = lastCallArgs.get('update_sync_settings') as Record<string, unknown> | undefined;
    expect(syncArgs).toBeDefined();
    const args = syncArgs?.args as { serverUrl?: string | null; enabled?: boolean };
    expect(args?.serverUrl).toBe('https://sync.example.com');
  });

  it('keeps server URL visible after save (no regression)', async () => {
    await waitForSyncSection();

    fireEvent.change(getServerUrlInput(), { target: { value: 'https://keep-this-url.com' } });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    // Server URL field must still show the saved value
    expect(getServerUrlInput()).toHaveValue('https://keep-this-url.com');
  });

  // ═══════════════════════════════════════════════════════════════
  //  API Key field
  // ═══════════════════════════════════════════════════════════════

  it('renders API key input as password field with placeholder', async () => {
    await waitForSyncSection();

    const keyInput = getApiKeyInput();
    expect(keyInput).toBeInTheDocument();
    expect(keyInput.type).toBe('password');
    expect(keyInput.getAttribute('placeholder')).toBe('Enter API key');
  });

  it('shows masked placeholder when hasApiKey is true', async () => {
    // Override get_sync_settings to return hasApiKey: true
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_settings') {
        return Promise.resolve({ serverUrl: null, hasApiKey: true, enabled: false });
      }
      return defaultImpl(cmd);
    });

    await waitForSyncSection();

    const keyInput = getApiKeyInput();
    expect(keyInput.getAttribute('placeholder')).toBe('••••••••');
  });

  it('toggles API key visibility between password and text', async () => {
    await waitForSyncSection();

    const keyInput = getApiKeyInput();
    expect(keyInput.type).toBe('password');

    // The toggle only renders when text is typed (not for placeholder dots)
    fireEvent.change(keyInput, { target: { value: 'sk-xyz' } });

    const toggleBtn = document.querySelector('.settings-input-toggle') as HTMLElement;
    expect(toggleBtn).not.toBeNull();
    fireEvent.click(toggleBtn);
    expect(keyInput.type).toBe('text');

    fireEvent.click(toggleBtn);
    expect(keyInput.type).toBe('password');
  });

  it('updates API key input value and sends it on save', async () => {
    await waitForSyncSection();

    fireEvent.change(getApiKeyInput(), { target: { value: 'sk-abc-123' } });
    expect(getApiKeyInput()).toHaveValue('sk-abc-123');

    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    const syncArgs = lastCallArgs.get('update_sync_settings') as Record<string, unknown> | undefined;
    const args = syncArgs?.args as { apiKey?: string };
    expect(args?.apiKey).toBe('sk-abc-123');
  });

  it('clears API key input after successful save with a key', async () => {
    await waitForSyncSection();

    fireEvent.change(getApiKeyInput(), { target: { value: 'sk-clear-me' } });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      const input = getApiKeyInput();
      expect(input).toHaveValue('');
    });
  });

  it('keeps API key value after save when sync save fails', async () => {
    await waitForSyncSection();

    fireEvent.change(getApiKeyInput(), { target: { value: 'sk-keep-me' } });
    expect(getApiKeyInput()).toHaveValue('sk-keep-me');

    failCommands.add('update_sync_settings');
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    expect(getApiKeyInput()).toHaveValue('sk-keep-me');
  });

  it('does NOT send apiKey when field is empty', async () => {
    await waitForSyncSection();

    // API key field is empty by default — do NOT type anything
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    const syncArgs = lastCallArgs.get('update_sync_settings') as Record<string, unknown> | undefined;
    const args = syncArgs?.args as Record<string, unknown>;
    // apiKey must be absent (not included in the object)
    expect(args).not.toHaveProperty('apiKey');
  });

  // ═══════════════════════════════════════════════════════════════
  //  Enabled toggle
  // ═══════════════════════════════════════════════════════════════

  it('renders enabled toggle unchecked by default', async () => {
    await waitForSyncSection();

    const checkbox = getEnabledCheckbox();
    expect(checkbox).toBeInTheDocument();
    expect(checkbox.checked).toBe(false);
  });

  it('toggles enabled state on click', async () => {
    const user = userEvent.setup();
    await waitForSyncSection();

    const checkbox = getEnabledCheckbox();
    const wrapper = checkbox.closest('.settings-toggle') as HTMLLabelElement;

    await user.click(wrapper);
    expect(checkbox.checked).toBe(true);

    await user.click(wrapper);
    expect(checkbox.checked).toBe(false);
  });

  it('sends enabled flag to backend on save', async () => {
    const user = userEvent.setup();
    await waitForSyncSection();

    const checkbox = getEnabledCheckbox();
    const wrapper = checkbox.closest('.settings-toggle') as HTMLLabelElement;
    await user.click(wrapper);

    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    const syncArgs = lastCallArgs.get('update_sync_settings') as Record<string, unknown> | undefined;
    const args = syncArgs?.args as { enabled?: boolean };
    expect(args?.enabled).toBe(true);
  });

  // ═══════════════════════════════════════════════════════════════
  //  Not-configured hint
  // ═══════════════════════════════════════════════════════════════

  it('shows not-configured hint when serverUrl is null and enabled is false', async () => {
    await waitForSyncSection();

    expect(screen.getByText(/not configured/i)).toBeInTheDocument();
  });

  it('hides not-configured hint when enabled toggle is on', async () => {
    const user = userEvent.setup();
    await waitForSyncSection();

    // Initially: not configured
    expect(screen.getByText(/not configured/i)).toBeInTheDocument();

    // Toggle enabled ON
    const checkbox = getEnabledCheckbox();
    const wrapper = checkbox.closest('.settings-toggle') as HTMLLabelElement;
    await user.click(wrapper);
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    // Hint should be gone because enabled is now true
    expect(screen.queryByText(/not configured/i)).not.toBeInTheDocument();
  });

  // ═══════════════════════════════════════════════════════════════
  //  Sync Now button
  // ═══════════════════════════════════════════════════════════════

  it('does not render Sync Now button when sync is unconfigured', async () => {
    await waitForSyncSection();

    expect(screen.queryByRole('button', { name: /sync now/i })).not.toBeInTheDocument();
  });

  it('renders Sync Now button when serverUrl is set', async () => {
    // Override load to return a configured serverUrl
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_settings') {
        return Promise.resolve({ serverUrl: 'https://sync.example.com', hasApiKey: false, enabled: false });
      }
      return defaultImpl(cmd);
    });

    await waitForSyncSection();

    expect(screen.getByRole('button', { name: /sync now/i })).toBeInTheDocument();
  });

  it('calls sync_run when Sync Now is clicked and displays result', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_settings') {
        return Promise.resolve({ serverUrl: 'https://sync.example.com', hasApiKey: true, enabled: true });
      }
      return defaultImpl(cmd);
    });

    await waitForSyncSection();

    const syncNowBtn = screen.getByRole('button', { name: /sync now/i });
    fireEvent.click(syncNowBtn);

    await waitFor(() => {
      // syncRun() calls invoke('sync_run') with no args object, so the
      // mock receives ('sync_run', undefined)
      expect(invokeMock).toHaveBeenCalledWith('sync_run', undefined);
    });

    await waitFor(() => {
      expect(screen.getByText(/3 synced/i)).toBeInTheDocument();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  //  hasApiKey state update after save (regression guard)
  // ═══════════════════════════════════════════════════════════════

  it('updates placeholder to masked dots after saving a new API key', async () => {
    await waitForSyncSection();

    // Initially: hasApiKey = false → placeholder shows "Enter API key"
    const keyInputBefore = getApiKeyInput();
    expect(keyInputBefore.getAttribute('placeholder')).toBe('Enter API key');

    // Type and save a new key
    fireEvent.change(keyInputBefore, { target: { value: 'sk-new-key' } });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    // After save: hasApiKey should be true → placeholder shows "••••••••"
    const keyInputAfter = getApiKeyInput();
    expect(keyInputAfter.getAttribute('placeholder')).toBe('••••••••');
  });

  // ═══════════════════════════════════════════════════════════════
  //  serverUrl state update after save (regression guard)
  // ═══════════════════════════════════════════════════════════════

  it('updates serverUrl in sync state after save so not-configured hint disappears', async () => {
    await waitForSyncSection();

    // Initially: not configured
    expect(screen.getByText(/not configured/i)).toBeInTheDocument();

    // Fill server URL and save
    fireEvent.change(getServerUrlInput(), { target: { value: 'https://sync.example.com' } });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    // Not-configured hint should disappear after save updates sync.serverUrl
    expect(screen.queryByText(/not configured/i)).not.toBeInTheDocument();
  });

  it('preserves hasApiKey state across saves without retyping the key', async () => {
    await waitForSyncSection();

    // First save: type a key
    fireEvent.change(getApiKeyInput(), { target: { value: 'sk-first' } });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    // Placeholder should now show masked dots
    expect(getApiKeyInput().getAttribute('placeholder')).toBe('••••••••');

    // Second save: wait for Saved! to revert back to normal, do NOT touch
    // the API key field, just save again.
    // The "Saved!" button auto-reverts after 2 seconds; wait for the normal
    // "Save settings" button to reappear.
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    }, { timeout: 3000 });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    // Placeholder should STILL show masked dots
    expect(getApiKeyInput().getAttribute('placeholder')).toBe('••••••••');
  });

  it('does not downgrade hasApiKey from true to false on save without key', async () => {
    // Start with a pre-existing key
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_settings') {
        return Promise.resolve({ serverUrl: 'https://exists.com', hasApiKey: true, enabled: true });
      }
      return defaultImpl(cmd);
    });

    await waitForSyncSection();

    // Placeholder already shows masked dots
    expect(getApiKeyInput().getAttribute('placeholder')).toBe('••••••••');

    // Save without touching the key field
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    // Placeholder should still show masked dots (not downgraded to "Enter API key")
    expect(getApiKeyInput().getAttribute('placeholder')).toBe('••••••••');
  });
});
