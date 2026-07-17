// ── SettingsPage tests ────────────────────────────────────────────
//
// Covers: loading, error recovery, store/receipt/currency/display/cloud
// sections, sidebar, accordion, validation, revert, keyboard shortcuts.
//
// Uses fireEvent.click for all button clicks (~1ms vs userEvent ~60ms),
// fireEvent.change for form fields (~1ms vs userEvent.type ~20ms/char),
// and fireEvent.blur for validation triggers (~1ms vs userEvent.tab ~50ms).
// 26 tests.
//
// Note: SettingsPage requires API data to load before any form/navigation
// elements appear. Tests that interact with the page must wait for the
// initial data load via await waitFor / await screen.findByText before
// using fireEvent.

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, cleanup, fireEvent } from '@testing-library/react';
import type { ReactNode } from 'react';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import settingsFtl from '@/locales/settings.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import SettingsPage from '@/features/settings/SettingsPage';
import { AuthProvider } from '@/contexts/AuthContext';
import { BrandProvider } from '@/contexts/BrandContext';
import { CurrencyProvider } from '@/contexts/CurrencyContext';
import { LocaleContext } from '@/i18n/LocaleContext';
import { getAvailableLocales, getLocaleLabel } from '@/i18n';

const { invokeMock, defaultImpl, failCommands } = vi.hoisted(() => {
  const SAMPLE_CURRENCIES = [
    { code: 'USD', name: 'US Dollar', minor_exponent: 2, symbol: '$' },
    { code: 'EUR', name: 'Euro', minor_exponent: 2, symbol: '\u20ac' },
  ];
  const failCommands = new Set<string>();

  const impl = (_cmd: string, _args?: unknown): Promise<unknown> => {
    const cmd = _cmd;
    if (failCommands.has(cmd)) {
      return Promise.reject(new Error(`Mock failure: ${cmd}`));
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
      return Promise.resolve(SAMPLE_CURRENCIES);
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
      return Promise.resolve({ name: 'oz-pos', version: '0.0.4', rustVersion: '1.80', target: 'x86_64' });
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
      return Promise.resolve({ synced: 0, failed: 0, error: null });
    }
    return Promise.resolve(undefined);
  };
  return { invokeMock: vi.fn(impl), defaultImpl: impl, failCommands };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@/contexts/ZoomContext', () => ({
  useAppZoom: () => ({ zoomLevel: 'auto', setZoomLevel: vi.fn() }),
  ZoomProvider: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('@/contexts/HardwareAccelContext', () => ({
  useHardwareAccel: () => ({ enabled: true, setEnabled: vi.fn() }),
  HardwareAccelProvider: ({ children }: { children: ReactNode }) => children,
}));

Element.prototype.scrollIntoView = vi.fn();

beforeEach(() => {
  cleanup();
  failCommands.clear();
  invokeMock.mockReset();
  invokeMock.mockImplementation(defaultImpl);
  document.documentElement.removeAttribute('data-theme');
  document.documentElement.removeAttribute('data-font-smoothing');
  document.documentElement.classList.remove('is-theme-transitioning');
  Array.from(document.documentElement.style)
    .filter((p) => p.startsWith('--color-accent'))
    .forEach((p) => document.documentElement.style.removeProperty(p));
});

afterEach(() => {
  cleanup();
});

function TestWrapper({ children }: { children: ReactNode }) {
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

// ── Field helpers (fireEvent ~1ms vs userEvent ~20ms/char) ────────

function fillField(label: string, value: string) {
  fireEvent.change(screen.getByRole('textbox', { name: label }), { target: { value } });
}

function blurField(label: string) {
  fireEvent.blur(screen.getByRole('textbox', { name: label }));
}

describe('SettingsPage', () => {
  // ── Loading ──────────────────────────────────────────────────

  it('shows loading indicator before APIs resolve', () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    expect(document.querySelector('.settings-loading-card')).toBeInTheDocument();
  });

  it('transitions from loading to ready after APIs resolve', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });
    expect(screen.queryByText(/loading/i)).not.toBeInTheDocument();
  });

  // ── Full error state ─────────────────────────────────────────

  it('renders error with retry button when all APIs fail', async () => {
    invokeMock.mockRejectedValue(new Error('IPC error'));
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByText(/failed to load/i)).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
  });

  it('recovers when retry is clicked after full failure', async () => {
    invokeMock.mockRejectedValue(new Error('IPC error'));
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    });
    invokeMock.mockImplementation(defaultImpl);
    fireEvent.click(screen.getByRole('button', { name: /retry/i }));
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });
  });

  // ── Partial load failure ─────────────────────────────────────

  it('shows partial-load toast when some APIs fail', async () => {
    failCommands.add('get_sync_settings');
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByText(/some settings could not be loaded/i)).toBeInTheDocument();
    });
  });

  // ── Store section ────────────────────────────────────────────

  it('renders Store section with name, address, tax ID, and language fields', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });
    expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Address' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: /tax.*id/i })).toBeInTheDocument();
  });

  it('updates store name input', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });
    fillField('Store name', 'Acme Corp');
    expect(screen.getByRole('textbox', { name: 'Store name' })).toHaveValue('Acme Corp');
  });

  it('updates store address input', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Address' })).toBeInTheDocument();
    });
    fillField('Address', '456 Oak Ave');
    expect(screen.getByRole('textbox', { name: 'Address' })).toHaveValue('456 Oak Ave');
  });

  // ── Save resilience ──────────────────────────────────────────

  it('calls set_receipt_settings and set_store_settings on save', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('set_receipt_settings', expect.any(Object));
      expect(invokeMock).toHaveBeenCalledWith('set_store_settings', expect.any(Object));
    });
  });

  it('shows "Saved!" after successful save', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });
  });

  it('shows full save-error toast when every save API call fails', async () => {
    failCommands.add('set_receipt_settings');
    failCommands.add('set_store_settings');
    failCommands.add('set_default_currency');
    failCommands.add('set_user_preferences');
    failCommands.add('update_sync_settings');
    failCommands.add('set_brand_primary_colour');
    failCommands.add('set_brand_store_name');
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));
    await waitFor(() => {
      expect(screen.getByText(/failed to save settings/i)).toBeInTheDocument();
    });
  });

  it('shows save-partial toast when some saves fail', async () => {
    failCommands.add('set_receipt_settings');
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByText(/some settings could not be saved/i)).toBeInTheDocument();
    });
  });

  // ── Currency section ─────────────────────────────────────────

  it('renders Currency section with default currency select', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /currency/i })).toBeInTheDocument();
    });
    const currencyTrigger = screen.getByRole('button', { name: /default currency/i });
    expect(currencyTrigger).toBeInTheDocument();
    expect(currencyTrigger).toHaveTextContent(/USD/);
  });

  it('changes default currency via select', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /default currency/i })).toBeInTheDocument();
    });

    const currencyTrigger = screen.getByRole('button', { name: /default currency/i });
    fireEvent.click(currencyTrigger);

    const dropdown = document.querySelector('.ssel-dropdown')!;
    const eurOption = Array.from(dropdown.querySelectorAll('.ssel-option')).find(
      (el) => el.textContent?.includes('EUR'),
    )!;
    fireEvent.click(eurOption);

    expect(currencyTrigger).toHaveTextContent(/EUR/);
  });

  // ── Display section ──────────────────────────────────────────

  it('renders Display section with size and font controls', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /appearance/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /appearance/i }));

    expect(screen.getByRole('button', { name: /decrease card size/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /increase card size/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /decrease font size/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /increase font size/i })).toBeInTheDocument();
  });

  it('increments card size value', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /appearance/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /appearance/i }));

    expect(screen.getAllByText('2')[0]).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /increase card size/i }));
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  // ── Receipt section ──────────────────────────────────────────

  it('navigates to Receipt section and populates form from API', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /receipt/i }));

    expect(screen.getByLabelText(/show currency symbol/i)).not.toBeChecked();
    expect(screen.getByLabelText(/show tax line/i)).toBeChecked();
  });

  it('toggles show-currency and show-tax checkboxes', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /receipt/i }));

    fireEvent.change(screen.getByLabelText(/show currency symbol/i), { target: { checked: true } });
    expect(screen.getByLabelText(/show currency symbol/i)).toBeChecked();
  });

  it('changes decimal separator and updates receipt footer', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /receipt/i }));

    const separatorTrigger = screen.getByLabelText(/decimal separator/i);
    fireEvent.click(separatorTrigger);
    const commaOption = screen.getByRole('option', { name: /comma/i });
    fireEvent.click(commaOption);
    expect(separatorTrigger).toHaveTextContent(/comma/i);

    fireEvent.change(screen.getByPlaceholderText(/thank you/i), { target: { value: 'Come again!' } });
    expect(screen.getByPlaceholderText(/thank you/i)).toHaveValue('Come again!');
  });

  // ── Cloud Sync section ───────────────────────────────────────

  it('renders Cloud Sync section with form fields', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /cloud sync/i }));

    expect(screen.getAllByRole('heading', { name: /cloud sync/i }).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByLabelText(/server url/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^api key$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/enable cloud sync/i)).toBeInTheDocument();
  });

  it('shows not-configured hint when sync is unconfigured', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /cloud sync/i }));

    expect(screen.getByText(/not configured/i)).toBeInTheDocument();
  });

  // ── About section ────────────────────────────────────────────

  it('renders About section with version and license info', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /system/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /system/i }));
    fireEvent.click(screen.getByRole('button', { name: /about/i }));

    expect(screen.getByRole('heading', { name: /system.*license/i })).toBeInTheDocument();
    const versionElements = screen.getAllByText(/0\.0\.\d+/);
    expect(versionElements.length).toBeGreaterThanOrEqual(1);
    const licenseElements = screen.getAllByText(/proprietary/i);
    expect(licenseElements.length).toBeGreaterThanOrEqual(1);
  });

  // ── Sidebar ──────────────────────────────────────────────────

  it('toggles collapse and expand via sidebar toggle button', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /collapse settings sidebar/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /collapse settings sidebar/i }));

    const sidebarToggle = document.querySelector('.settings-sidebar-toggle');
    expect(sidebarToggle).toBeInTheDocument();
    expect(sidebarToggle!.getAttribute('aria-label')?.toLowerCase()).toContain('expand');
  });

  it('persists sidebar collapsed state to localStorage', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /collapse settings sidebar/i })).toBeInTheDocument();
    });
    expect(localStorage.getItem('settings-sidebar-collapsed')).not.toBe('true');
    fireEvent.click(screen.getByRole('button', { name: /collapse settings sidebar/i }));
    await waitFor(() => {
      expect(localStorage.getItem('settings-sidebar-collapsed')).toBe('true');
    });
  });

  it('toggles category accordion', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    const opsBtn = screen.getByRole('button', { name: /operations/i });
    expect(opsBtn.getAttribute('aria-expanded')).toBe('false');
    fireEvent.click(opsBtn);
    expect(opsBtn.getAttribute('aria-expanded')).toBe('true');
    fireEvent.click(opsBtn);
    expect(opsBtn.getAttribute('aria-expanded')).toBe('false');
  });

  // ── Footer ───────────────────────────────────────────────────

  it('renders theme toggle button and app version in footer', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /switch to light/i })).toBeInTheDocument();
    expect(document.body.textContent).toContain('0.0.4');
  });

  // ── Keyboard shortcut ─────────────────────────────────────────

  it('saves latest form values when Ctrl+S is pressed after editing', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });

    fillField('Store name', 'Ctrl+S Store');
    fillField('Address', '456 Keyboard Blvd');
    fillField('Address', '456 Keyboard Blvd');

    fireEvent.keyDown(document, { key: 's', ctrlKey: true });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'set_store_settings',
        expect.objectContaining({
          args: expect.objectContaining({
            name: 'Ctrl+S Store',
            address: '456 Keyboard Blvd',
          }),
        }),
      );
    });
  });

  // ── Field validation ──────────────────────────────────────────

  it('shows "store name is required" when store name is empty on blur', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });

    const nameInput = screen.getByRole('textbox', { name: 'Store name' });
    fillField('Store name', '');
    blurField('Store name');

    await waitFor(() => {
      expect(screen.getByText('Store name is required')).toBeInTheDocument();
    });
    expect(nameInput.className).toContain('settings-input--error');
  });

  it('clears store-name error when user starts typing', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });

    fillField('Store name', '');
    blurField('Store name');
    await waitFor(() => {
      expect(screen.getByText('Store name is required')).toBeInTheDocument();
    });

    fillField('Store name', 'A');
    expect(screen.queryByText('Store name is required')).not.toBeInTheDocument();
  });

  it('shows tax-id pattern error for invalid characters', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: /tax.*id/i })).toBeInTheDocument();
    });

    const taxInput = screen.getByRole('textbox', { name: /tax.*id/i });
    fireEvent.change(taxInput, { target: { value: '12-345@#' } });
    fireEvent.blur(taxInput);

    await waitFor(() => {
      expect(screen.getByText(/only letters, numbers, dashes, dots, and slashes allowed/i)).toBeInTheDocument();
    });
    expect(taxInput.className).toContain('settings-input--error');
  });

  it('does not show tax-id error for valid characters', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: /tax.*id/i })).toBeInTheDocument();
    });

    const taxInput = screen.getByRole('textbox', { name: /tax.*id/i });
    fireEvent.change(taxInput, { target: { value: '12-345.67/89' } });
    fireEvent.blur(taxInput);

    expect(screen.queryByText(/only letters, numbers, dashes, dots, and slashes allowed/i)).not.toBeInTheDocument();
  });

  // ── Revert button ─────────────────────────────────────────────

  it('clears field validation errors when Revert is clicked', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });

    fillField('Store name', '');
    blurField('Store name');
    await waitFor(() => {
      expect(screen.getByText('Store name is required')).toBeInTheDocument();
    });

    const revertBtn = document.querySelector('.settings-btn-revert') as HTMLElement;
    expect(revertBtn).toBeInTheDocument();
    fireEvent.click(revertBtn);

    expect(screen.queryByText('Store name is required')).not.toBeInTheDocument();
  });

  it('shows Revert button only when form is dirty', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });

    const revertBtn = document.querySelector('.settings-btn-revert') as HTMLElement;
    expect(revertBtn.className).toContain('settings-btn-revert--hidden');

    fillField('Store name', 'x');
    expect(revertBtn.className).not.toContain('settings-btn-revert--hidden');
  });

  // ── Arrow keyboard navigation ─────────────────────────────────

  it('navigates sections with ArrowDown key', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });

    fireEvent.keyDown(document, { key: 'ArrowDown' });

    // After ArrowDown from 'general', it should navigate to 'appearance'.
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /appearance/i })?.className).toContain('settings-nav-item--active');
    });
  });

  it('navigates sections with ArrowUp key', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });

    // Navigate down first to move to a different section, then up.
    fireEvent.keyDown(document, { key: 'ArrowDown' });
    fireEvent.keyDown(document, { key: 'ArrowDown' });
    fireEvent.keyDown(document, { key: 'ArrowUp' });

    await waitFor(() => {
      const appearanceBtn = screen.getByRole('button', { name: /appearance/i });
      expect(appearanceBtn.className).toContain('settings-nav-item--active');
    });
  });

  // ── API key visibility toggle ─────────────────────────────────

  it('toggles API key visibility in Cloud Sync section', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /cloud sync/i }));

    const apiKeyInput = screen.getByLabelText(/^api key$/i) as HTMLInputElement;
    expect(apiKeyInput.type).toBe('password');

    const toggleBtn = document.querySelector('.settings-input-toggle') as HTMLElement;
    fireEvent.click(toggleBtn);

    expect(apiKeyInput.type).toBe('text');

    fireEvent.click(toggleBtn);
    expect(apiKeyInput.type).toBe('password');
  });

  // ── Save button loading state ─────────────────────────────────

  it('shows loading state on Save button while saving', async () => {
    // Make a save command hang to keep saving=true.
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd.startsWith('set_') || cmd === 'update_sync_settings') {
        return new Promise(() => {});
      }
      return defaultImpl(cmd);
    });

    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });

    const saveBtn = screen.getByRole('button', { name: /save settings/i });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(saveBtn).toHaveAttribute('aria-busy', 'true');
    });
  });

  // ══════════════════════════════════════════════════════════════
  //  SectionKey — multiple navigations don't break rendering
  // ══════════════════════════════════════════════════════════════

  it('renders correct section after navigating through multiple sections', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /receipt/i }));

    await waitFor(() => {
      expect(screen.getByLabelText(/show currency symbol/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /system/i }));
    fireEvent.click(screen.getByRole('button', { name: /about/i }));

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /system.*license/i })).toBeInTheDocument();
    });

    // Navigate back to General via the sidebar.
    const businessHeader = screen.getByRole('button', { name: /business/i });
    fireEvent.click(businessHeader);

    fireEvent.click(screen.getByRole('button', { name: /general/i }));

    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });
  });

  // ══════════════════════════════════════════════════════════════
  //  Sync API key — cleared only after successful sync save
  // ══════════════════════════════════════════════════════════════

  function navigateToSync() {
    fireEvent.click(screen.getByRole('button', { name: /operations/i }));
    fireEvent.click(screen.getByRole('button', { name: /cloud sync/i }));
  }

  it('clears API key after save when sync save succeeds', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    navigateToSync();

    const apiKeyInput = screen.getByLabelText(/^api key$/i) as HTMLInputElement;
    fireEvent.change(apiKeyInput, { target: { value: 'sk-abc123' } });
    expect(apiKeyInput).toHaveValue('sk-abc123');

    // Make every non-sync save command fail so only sync succeeds.
    failCommands.add('set_receipt_settings');
    failCommands.add('set_store_settings');
    failCommands.add('set_default_currency');
    failCommands.add('set_user_preferences');
    failCommands.add('set_brand_primary_colour');
    failCommands.add('set_brand_store_name');

    const saveBtn = screen.getByRole('button', { name: /save settings/i });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      const inputAfterSave = screen.getByLabelText(/^api key$/i) as HTMLInputElement;
      expect(inputAfterSave).toHaveValue('');
    });
  });

  it('keeps API key after save when sync save fails', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    navigateToSync();

    const apiKeyInput = screen.getByLabelText(/^api key$/i) as HTMLInputElement;
    fireEvent.change(apiKeyInput, { target: { value: 'sk-xyz789' } });
    expect(apiKeyInput).toHaveValue('sk-xyz789');

    // Make sync save fail while all other saves succeed.
    failCommands.add('update_sync_settings');

    const saveBtn = screen.getByRole('button', { name: /save settings/i });
    fireEvent.click(saveBtn);

    // Wait for save to settle (Saved! appears when at least one save succeeds).
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });

    const inputAfterSave = screen.getByLabelText(/^api key$/i) as HTMLInputElement;
    expect(inputAfterSave).toHaveValue('sk-xyz789');
  });

  it('shows partial-save toast when sync save is the only failure', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    navigateToSync();

    failCommands.add('update_sync_settings');

    const saveBtn = screen.getByRole('button', { name: /save settings/i });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(screen.getByText(/some settings could not be saved/i)).toBeInTheDocument();
    });
  });
});
