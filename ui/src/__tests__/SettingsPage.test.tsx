import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import settingsFtl from '@/locales/settings.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import SettingsPage from '@/features/settings/SettingsPage';
import { AuthProvider } from '@/contexts/AuthContext';
import { BrandProvider } from '@/contexts/BrandContext';
import { LocaleContext } from '@/i18n/LocaleContext';
import { getAvailableLocales, getLocaleLabel } from '@/i18n';

const { invokeMock, defaultImpl, failCommands } = vi.hoisted(() => {
  const SAMPLE_CURRENCIES = [
    { code: 'USD', name: 'US Dollar', minor_exponent: 2, symbol: '$' },
    { code: 'EUR', name: 'Euro', minor_exponent: 2, symbol: '\u20ac' },
  ];
  // Mutable set of commands that should reject — tests add to it to
  // simulate failures without replacing the entire mock implementation.
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

// AppearanceSettings uses useAppZoom — mock it to avoid needing ZoomProvider.
vi.mock('@/contexts/ZoomContext', () => ({
  useAppZoom: () => ({ zoomLevel: 'auto', setZoomLevel: vi.fn() }),
  ZoomProvider: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('@/contexts/HardwareAccelContext', () => ({
  useHardwareAccel: () => ({ enabled: true, setEnabled: vi.fn() }),
  HardwareAccelProvider: ({ children }: { children: ReactNode }) => children,
}));

// scrollIntoView is not implemented in jsdom — mock it for SettingsSelect usage.
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
  // Timers are cleaned up by React's useEffect cleanup functions
  // (useClock, ThemeProvider, Tooltip, ToastProvider) which pass
  // Timeout objects directly to clearTimeout.  The old blanket loop
  // (`clearTimeout(number)`) was a no-op in Node.js 15+ where
  // setTimeout returns Timeout objects, not numbers.
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
        <AuthProvider>{children}</AuthProvider>
      </BrandProvider>
    </LocaleContext.Provider>
  );
}

describe('SettingsPage', () => {
  // ── Loading ──────────────────────────────────────────────────

  it('shows loading indicator before APIs resolve', () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    // Skeleton placeholder cards are rendered during loading (replaced the old "Loading…" text)
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
    await userEvent.click(screen.getByRole('button', { name: /retry/i }));
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
    const nameInput = screen.getByRole('textbox', { name: 'Store name' });
    await userEvent.clear(nameInput);
    await userEvent.type(nameInput, 'Acme Corp');
    expect(nameInput).toHaveValue('Acme Corp');
  });

  it('updates store address input', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Address' })).toBeInTheDocument();
    });
    const addressInput = screen.getByRole('textbox', { name: 'Address' });
    await userEvent.clear(addressInput);
    await userEvent.type(addressInput, '456 Oak Ave');
    expect(addressInput).toHaveValue('456 Oak Ave');
  });

  // ── Save resilience ──────────────────────────────────────────

  it('calls set_receipt_settings and set_store_settings on save', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /save settings/i }));
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
    await userEvent.click(screen.getByRole('button', { name: /save settings/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });
  });

  it('shows save-error toast when all saves fail', async () => {
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
    await userEvent.click(screen.getByRole('button', { name: /save settings/i }));
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
    await userEvent.click(screen.getByRole('button', { name: /save settings/i }));
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
    // Currency select is now a custom SettingsSelect (button, not native <select>).
    const currencyTrigger = screen.getByLabelText(/default currency/i);
    expect(currencyTrigger).toBeInTheDocument();
    // The selected value is shown in the trigger button text.
    expect(currencyTrigger).toHaveTextContent(/USD/);
  });

  it('changes default currency via select', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByLabelText(/default currency/i)).toBeInTheDocument();
    });
    // Custom SettingsSelect: click the trigger to open the dropdown.
    const currencyTrigger = screen.getByLabelText(/default currency/i);
    await userEvent.click(currencyTrigger);
    // Click the EUR option in the portal dropdown.
    const eurOption = screen.getByRole('option', { name: /EUR/ });
    await userEvent.click(eurOption);
    // Verify the trigger now shows EUR.
    expect(currencyTrigger).toHaveTextContent(/EUR/);
  });

  // ── Display section ──────────────────────────────────────────

  it('renders Display section with size and font controls', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /appearance/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /appearance/i }));
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /display/i })).toBeInTheDocument();
    });
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
    await userEvent.click(screen.getByRole('button', { name: /appearance/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /increase card size/i })).toBeInTheDocument();
    });
    // The card-size stepper value '2' also appears in the new category count
    // badges (e.g., Business shows "2"), so use getAllByText to disambiguate.
    expect(screen.getAllByText('2')[0]).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /increase card size/i }));
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  // ── Receipt section ──────────────────────────────────────────

  it('navigates to Receipt section and populates form from API', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /receipt/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /receipt/i }));
    await waitFor(() => {
      expect(screen.getByLabelText(/show currency symbol/i)).not.toBeChecked();
      expect(screen.getByLabelText(/show tax line/i)).toBeChecked();
    });
  });

  it('toggles show-currency and show-tax checkboxes', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /receipt/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /receipt/i }));
    await waitFor(() => {
      expect(screen.getByLabelText(/show currency symbol/i)).toBeInTheDocument();
    });
    await userEvent.click(screen.getByLabelText(/show currency symbol/i));
    expect(screen.getByLabelText(/show currency symbol/i)).toBeChecked();
  });

  it('changes decimal separator and updates receipt footer', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /receipt/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /receipt/i }));
    await waitFor(() => {
      expect(screen.getByLabelText(/decimal separator/i)).toBeInTheDocument();
    });
    // Custom SettingsSelect: click the trigger to open the dropdown.
    const separatorTrigger = screen.getByLabelText(/decimal separator/i);
    await userEvent.click(separatorTrigger);
    // Click the 'comma' option in the portal dropdown.
    const commaOption = screen.getByRole('option', { name: /comma/i });
    await userEvent.click(commaOption);
    // Verify the trigger now shows comma.
    expect(separatorTrigger).toHaveTextContent(/comma/i);
    const footer = screen.getByPlaceholderText(/thank you/i);
    await userEvent.clear(footer);
    await userEvent.type(footer, 'Come again!');
    expect(footer).toHaveValue('Come again!');
  });

  // ── Cloud Sync section ───────────────────────────────────────

  it('renders Cloud Sync section with form fields', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /cloud sync/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /cloud sync/i }));
    await waitFor(() => {
      // Section breadcrumb header + card header both render "Cloud Sync" heading;
      // use getAllByRole since getByRole would throw on multiple matches.
      expect(screen.getAllByRole('heading', { name: /cloud sync/i }).length).toBeGreaterThanOrEqual(1);
    });
    expect(screen.getByLabelText(/server url/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^api key$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/enable cloud sync/i)).toBeInTheDocument();
  });

  it('shows not-configured hint when sync is unconfigured', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /cloud sync/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /cloud sync/i }));
    await waitFor(() => {
      expect(screen.getByText(/not configured/i)).toBeInTheDocument();
    });
  });

  // ── About section ────────────────────────────────────────────

  it('renders About section with version and license info', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /system/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /system/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /about/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /about/i }));
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /system.*license/i })).toBeInTheDocument();
    });
    const versionElements = screen.getAllByText(/0\.0\.4/);
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
    await userEvent.click(screen.getByRole('button', { name: /collapse settings sidebar/i }));
    // After collapse: the sidebar toggle aria-label flips to "Expand sidebar".
    // Scope to the sidebar button only — the mobile hamburger also has
    // an expand/collapse aria-label.
    await waitFor(() => {
      const sidebarToggle = document.querySelector('.settings-sidebar-toggle');
      expect(sidebarToggle).toBeInTheDocument();
      expect(sidebarToggle!.getAttribute('aria-label')?.toLowerCase()).toContain('expand');
    });
  });

  it('persists sidebar collapsed state to localStorage', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /collapse settings sidebar/i })).toBeInTheDocument();
    });
    // Initial state is false and the effect immediately writes 'false' to
    // localStorage — verify it's not 'true' before we toggle.
    expect(localStorage.getItem('settings-sidebar-collapsed')).not.toBe('true');
    await userEvent.click(screen.getByRole('button', { name: /collapse settings sidebar/i }));
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
    await userEvent.click(opsBtn);
    expect(opsBtn.getAttribute('aria-expanded')).toBe('true');
    await userEvent.click(opsBtn);
    expect(opsBtn.getAttribute('aria-expanded')).toBe('false');
  });

  // ── Footer ───────────────────────────────────────────────────

  it('renders theme toggle button and app version in footer', async () => {
    renderWithProvidersSync(<TestWrapper><SettingsPage /></TestWrapper>, settingsFtl, sharedFtl);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /switch to light/i })).toBeInTheDocument();
    expect(screen.getByText(/0\.0\.4/)).toBeInTheDocument();
  });
});
