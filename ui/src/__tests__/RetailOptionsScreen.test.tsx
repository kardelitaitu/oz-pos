// ── RetailOptionsScreen tests ─────────────────────────────────────
//
// Covers: tab navigation, settings loading/saving, receipt preview,
// scanner list, keyboard shortcuts (Escape), credit toggle.

import { describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ToastProvider } from '@/frontend/shared/Toast';
import { withFluent } from '@/locales/test-utils';
import salesFtl from '@/locales/sales.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import settingsFtl from '@/locales/settings.ftl?raw';
import RetailOptionsScreen from '@/features/retail/RetailOptionsScreen';

// ── Mock modules ──────────────────────────────────────────────────

vi.mock('@/api/settings', () => ({
  getStoreSettings: vi.fn(() =>
    Promise.resolve({ name: 'TOKO TEST', address: 'Jl. Contoh No. 123', taxId: '12345', currency: 'IDR', branch: 'Cabang A', logo: '' }),
  ),
  setStoreSettings: vi.fn(() => Promise.resolve()),
  getReceiptSettings: vi.fn(() =>
    Promise.resolve({ showCurrency: true, decimalSeparator: 'dot', showTax: true, footer: 'Terima kasih', paperWidth: 'standard', showTableNumber: false, marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0 }),
  ),
  setReceiptSettings: vi.fn(() => Promise.resolve()),
  getCreditSettings: vi.fn(() =>
    Promise.resolve({ enabled: true, reminderIntervalHours: 24, maxLimitMinor: 500000 }),
  ),
  setCreditSettings: vi.fn(() => Promise.resolve()),
  listCreditSales: vi.fn(() => Promise.resolve([])),
  settleCredit: vi.fn(),
  getHardwareSettings: vi.fn(() =>
    Promise.resolve({ printerConnection: 'auto', printerDevicePath: '/dev/usb/lp0', printerPaperSize: '80', scannerDeviceId: 'scanner-01', scannerInputMode: 'auto' }),
  ),
  setHardwareSettings: vi.fn(() => Promise.resolve()),
  completeSetup: vi.fn(),
  dismissSetupWizard: vi.fn(),
  getSetupStatus: vi.fn(),
  getEnabledFeatures: vi.fn(),
  getUserPreferences: vi.fn(),
  setUserPreferences: vi.fn(),
}));

vi.mock('@/api/tax', () => ({
  listTaxRates: vi.fn(() => Promise.resolve([])),
}));

vi.mock('@/api/hardware', () => ({
  listScanners: vi.fn(() =>
    Promise.resolve([{ id: 'scanner-01' }, { id: 'scanner-02' }]),
  ),
  listDisplays: vi.fn(() => Promise.resolve([])),
  displayShow: vi.fn(() => Promise.resolve()),
  displayClear: vi.fn(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve('')),
}));

vi.mock('@/hooks/useCloudSync', () => ({
  useCloudSync: () => ({
    enabled: false, setEnabled: vi.fn(),
    serverURL: '', setServerURL: vi.fn(),
    token: '', setToken: vi.fn(),
    autoMinutes: 0, setAutoMinutes: vi.fn(),
    status: 'offline', lastAt: null, pending: 0,
    syncing: false, pulling: false, tokenLoaded: true,
    persist: vi.fn(),
    syncNow: vi.fn(), testConnection: vi.fn(), pullFromServer: vi.fn(),
  }),
}));

vi.mock('@/i18n/LanguageSelector', () => ({
  LanguageSelector: () => <select aria-label="Language selector"><option>English</option></select>,
}));

vi.mock('@/features/settings/DataManagementScreen', () => ({
  __esModule: true,
  default: () => <div data-testid="data-management-screen">Data Management</div>,
}));

vi.mock('@/features/settings/AppearanceSettings', () => ({
  AppearanceSettings: () => <div data-testid="appearance-settings">Appearance</div>,
}));

vi.mock('@/features/settings/FeatureToggleScreen', () => ({
  __esModule: true,
  default: () => <div data-testid="feature-toggle-screen">Feature Toggles</div>,
}));

vi.mock('@/contexts/ZoomContext', () => ({
  useAppZoom: () => ({ zoomLevel: 'auto', setZoomLevel: vi.fn() }),
}));

vi.mock('@/contexts/HardwareAccelContext', () => ({
  useHardwareAccel: () => ({ enabled: true, setEnabled: vi.fn() }),
}));

vi.mock('@/contexts/BrandContext', () => ({
  useBrand: () => ({
    settings: { primary_colour: '#10b981', logo_path: null, store_name: '' },
    refreshBrandSettings: vi.fn(),
  }),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', username: 'testuser', role_name: 'manager', token: 'mock-token', role_id: 'role-1', display_name: 'Budi Manager' },
    loading: false, error: null, login: vi.fn(), logout: vi.fn(), clearError: vi.fn(),
    isManager: true, isOwner: false,
  }),
}));

// ── Test wrapper ──────────────────────────────────────────────────

function Wrapper({ children }: { children: ReactNode }) {
  return withFluent(<ToastProvider>{children}</ToastProvider>, salesFtl, sharedFtl, settingsFtl);
}

function wrap(onClose?: () => void) {
  return (
    <Wrapper>
      <RetailOptionsScreen onClose={onClose ?? vi.fn()} />
    </Wrapper>
  );
}

// ── Tests ─────────────────────────────────────────────────────────

describe('RetailOptionsScreen', () => {
  // ── Loading & Rendering ────────────────────────────────────────

  it('shows loading state while fetching settings', async () => {
    const { getStoreSettings } = await import('@/api/settings');
    vi.mocked(getStoreSettings).mockImplementationOnce(() => new Promise(() => {}));

    const { container } = render(wrap());

    await waitFor(() => {
      expect(container.querySelector('.retail-options-loading-skeleton')).toBeInTheDocument();
    });
    expect(container.querySelector('[aria-hidden="true"]')).toBeInTheDocument();
  });

  it('renders the General tab by default', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });
    expect(screen.getByDisplayValue('TOKO TEST')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Jl. Contoh No. 123')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Cabang A')).toBeInTheDocument();
    expect(screen.getByDisplayValue('12345')).toBeInTheDocument();
  });

  // ── Tab navigation ─────────────────────────────────────────────

  it('switches to Receipt tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Receipt')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Receipt'));

    expect(screen.getByText('Receipt Settings')).toBeInTheDocument();
    // Footer text is rendered in a textarea value, not as visible text
    expect(screen.getByDisplayValue('Terima kasih')).toBeInTheDocument();
  });

  it('switches to Printer tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Printer')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Printer'));

    expect(screen.getByText('Receipt Printer')).toBeInTheDocument();
    expect(screen.getByDisplayValue('/dev/usb/lp0')).toBeInTheDocument();
  });

  it('switches to Scanner tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Scanner')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Scanner'));

    expect(screen.getByText('Barcode Scanner')).toBeInTheDocument();
  });

  it('switches to Credit tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Credit')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Credit'));

    expect(screen.getByText('Credit Settings')).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText(/Enable credit sales/i)).toBeInTheDocument();
    });
  });

  it('switches to System tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    expect(screen.getByText(/App version/)).toBeInTheDocument();
    expect(screen.getByDisplayValue('0.0.9')).toBeInTheDocument();
    expect(screen.getByDisplayValue(/Budi Manager/)).toBeInTheDocument();
  });

  // ── Save ───────────────────────────────────────────────────────

  it('saves all settings when Save is clicked', async () => {
    const { setStoreSettings, setReceiptSettings, setCreditSettings, setHardwareSettings } = await import('@/api/settings');

    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(setStoreSettings).toHaveBeenCalledOnce();
      expect(setReceiptSettings).toHaveBeenCalledOnce();
      expect(setCreditSettings).toHaveBeenCalledOnce();
      expect(setHardwareSettings).toHaveBeenCalledOnce();
    });
  });

  it('shows success toast after saving', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/Settings saved/);
    });
  });

  it('disables Save button while saving', async () => {
    const { setStoreSettings } = await import('@/api/settings');
    vi.mocked(setStoreSettings).mockImplementationOnce(() => new Promise(() => {}));

    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    const saveBtn = screen.getByText('Save');
    await userEvent.click(saveBtn);

    expect(screen.getByText('Saving\u2026')).toBeInTheDocument();
  });

  // ── Scanner tab ────────────────────────────────────────────────

  it('shows detected scanners in Scanner tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Scanner')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Scanner'));

    await waitFor(() => {
      expect(screen.getByText('scanner-01')).toBeInTheDocument();
      expect(screen.getByText('scanner-02')).toBeInTheDocument();
    });
  });

  // ── Receipt Preview ────────────────────────────────────────────

  it('shows receipt preview popup when clicking the preview', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Receipt')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Receipt'));

    await waitFor(() => {
      expect(screen.getByDisplayValue('Terima kasih')).toBeInTheDocument();
    });

    const previewHint = screen.getByText('Click to preview');
    await userEvent.click(previewHint);

    await waitFor(() => {
      const closeBtn = screen.queryByText('\u00D7');
      expect(closeBtn).toBeInTheDocument();
    });
  });

  it('closes receipt preview popup when clicking close', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Receipt')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Receipt'));

    await waitFor(() => {
      expect(screen.getByDisplayValue('Terima kasih')).toBeInTheDocument();
    });

    const previewHint = screen.getByText('Click to preview');
    await userEvent.click(previewHint);

    await waitFor(() => {
      expect(screen.getByText('\u00D7')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('\u00D7'));

    await waitFor(() => {
      expect(screen.queryByText('\u00D7')).not.toBeInTheDocument();
    });
  });

  // ── Back / Close ───────────────────────────────────────────────

  it('calls onClose when Back is clicked', async () => {
    const onClose = vi.fn();
    render(wrap(onClose));

    await waitFor(() => {
      expect(screen.getByText(/Back/)).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText(/Back/));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it('calls onClose when Escape is pressed', async () => {
    const onClose = vi.fn();
    render(wrap(onClose));

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.keyboard('{Escape}');

    expect(onClose).toHaveBeenCalledOnce();
  });

  it('calls onClose when Close button is clicked', async () => {
    const onClose = vi.fn();
    render(wrap(onClose));

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Close'));

    expect(onClose).toHaveBeenCalledOnce();
  });

  // ── Credit toggle ──────────────────────────────────────────────

  // ── API error handling ─────────────────────────────────────────

  it('shows error toast when store settings fail to load', async () => {
    const { getStoreSettings } = await import('@/api/settings');
    vi.mocked(getStoreSettings).mockRejectedValueOnce(new Error('Network error'));

    render(wrap());

    await waitFor(() => {
      const toasts = screen.getAllByRole('alert');
      const errorToast = toasts.find((t) => t.textContent?.includes('Failed to load'));
      expect(errorToast).toBeTruthy();
    });
  });

  it('shows error toast when receipt settings fail to load', async () => {
    const { getReceiptSettings } = await import('@/api/settings');
    vi.mocked(getReceiptSettings).mockRejectedValueOnce(new Error('Network error'));

    render(wrap());

    await waitFor(() => {
      const toasts = screen.getAllByRole('alert');
      const errorToast = toasts.find((t) => t.textContent?.includes('Failed to load'));
      expect(errorToast).toBeTruthy();
    });
  });

  // ── Payments tab ──────────────────────────────────────────────

  it('switches to Payments tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    await waitFor(() => {
      expect(screen.getByText(/Payment Gateways/)).toBeInTheDocument();
    });
  });

  it('renders tender presets add/remove in Payments tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    await waitFor(() => {
      expect(screen.getByText(/Add preset/)).toBeInTheDocument();
    });
  });

  it('adds a tender preset when add button is clicked', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    await waitFor(() => {
      expect(screen.getByText(/Add preset/)).toBeInTheDocument();
    });

    const addBtn = screen.getByText(/Add preset/);
    await userEvent.click(addBtn);

    // Should now have 6 presets (5 default + 1 new)
    const removeBtns = screen.getAllByRole('button', { name: /remove preset/i });
    expect(removeBtns).toHaveLength(6);
  });

  it('removes a tender preset when remove button is clicked', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    await waitFor(() => {
      expect(screen.getByText(/Add preset/)).toBeInTheDocument();
    });

    const removeBtns = screen.getAllByRole('button', { name: /remove preset/i });
    await userEvent.click(removeBtns[0]!);

    const remaining = screen.getAllByRole('button', { name: /remove preset/i });
    expect(remaining).toHaveLength(4);
  });

  // ── Sync tab ──────────────────────────────────────────────────

  it('switches to Sync tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Sync'));

    await waitFor(() => {
      expect(screen.getByText(/Cloud Sync/)).toBeInTheDocument();
    });
  });

  it('renders sync toggle and server URL field in Sync tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Sync'));

    await waitFor(() => {
      expect(screen.getByText(/Server URL/)).toBeInTheDocument();
      expect(screen.getByText(/Enable cloud sync/)).toBeInTheDocument();
      expect(screen.getByText(/Authentication Token/)).toBeInTheDocument();
    });
  });

  it('shows sync status box in Sync tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Sync'));

    await waitFor(() => {
      expect(screen.getByText(/Offline/)).toBeInTheDocument();
      expect(screen.getByText(/Never synced/)).toBeInTheDocument();
    });
  });

  // ── Appearance / Features / Data tabs ─────────────────────────

  it('switches to Appearance tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Appearance')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Appearance'));

    await waitFor(() => {
      expect(screen.getByTestId('appearance-settings')).toBeInTheDocument();
    });
  });

  it('switches to Features tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Features')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Features'));

    await waitFor(() => {
      expect(screen.getByTestId('feature-toggle-screen')).toBeInTheDocument();
    });
  });

  it('switches to Data tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Data')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Data'));

    await waitFor(() => {
      expect(screen.getByTestId('data-management-screen')).toBeInTheDocument();
    });
  });

  // ── System tab additional options ─────────────────────────────

  it('renders theme selector in System tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    await waitFor(() => {
      expect(screen.getByText(/Theme/)).toBeInTheDocument();
      expect(screen.getByText(/Light/)).toBeInTheDocument();
      expect(screen.getByText(/Dark/)).toBeInTheDocument();
    });
  });

  it('renders language selector in System tab', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    await waitFor(() => {
      expect(screen.getByLabelText('Language selector')).toBeInTheDocument();
    });
  });

  it('shows credit limit info when credit is enabled', async () => {
    render(wrap());

    await waitFor(() => {
      expect(screen.getByText('Credit')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Credit'));

    await waitFor(() => {
      // maxLimitMinor=500000 → input value / 100 = 5000, formatted as IDR "5.000"
      // Fluent wraps { $amount } in FSI/PDI Unicode markers (U+2068/U+2069)
      expect(screen.getByText((content) =>
        content.replace(/[\u2068\u2069]/g, '').includes('Max limit: Rp 5.000')
      )).toBeInTheDocument();
    });
  });

});
