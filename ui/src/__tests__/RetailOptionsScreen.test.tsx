// ── RetailOptionsScreen tests ─────────────────────────────────────
//
// Covers: tab navigation, settings loading/saving, receipt preview,
// scanner list, keyboard shortcuts (Escape), credit toggle,
// Payments tab (gateway config + tender presets), sound toggle,
// language selector.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import type { ReactNode } from 'react';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ToastProvider } from '@/frontend/shared/Toast';
import { withFluent } from '@/locales/test-utils';
import { renderInAct } from '@/test-utils/renderInAct';
import salesFtl from '@/locales/sales.ftl?raw';

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

// Mock @tauri-apps/api/core invoke for get_setting / set_setting
// vi.hoisted ensures the object exists before vi.mock factories (which are hoisted) run
const { mockSettingsDb } = vi.hoisted(() => ({
  mockSettingsDb: {} as Record<string, string>,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === 'get_setting') {
      const key = args?.['key'] as string;
      return mockSettingsDb[key] ?? null;
    }
    if (cmd === 'set_setting') {
      const key = args?.['key'] as string;
      const value = args?.['value'] as string;
      if (value === '') {
        delete mockSettingsDb[key];
      } else {
        mockSettingsDb[key] = value;
      }
      return;
    }
    // Default success response for the real cloud-sync round-trip.
    // Individual tests can override by spying on `invoke` directly.
    if (cmd === 'sync_run') {
      return { synced: 1, failed: 0, error: null };
    }
    if (cmd === 'pending_sync_count') {
      return 0;
    }
    if (cmd === 'sync_pull') {
      return { productsPulled: 5, taxRatesPulled: 1, usersPulled: 2, error: null };
    }
    return null;
  }),
}));

vi.mock('@/api/gateway', () => ({
  getGatewayStatus: vi.fn(async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const stripeKey: string | null = await invoke('get_setting', { key: 'stripe.api_key' }) as string | null;
    const squareKey: string | null = await invoke('get_setting', { key: 'square.api_key' }) as string | null;
    const midtransKey: string | null = await invoke('get_setting', { key: 'midtrans.server_key' }) as string | null;
    const statuses: { name: string; configured: boolean; online: boolean }[] = [];
    // Always show all three gateways (their configured state depends on whether a key exists)
    statuses.push({ name: 'Stripe', configured: stripeKey !== null, online: stripeKey !== null });
    statuses.push({ name: 'Square', configured: squareKey !== null, online: squareKey !== null });
    statuses.push({ name: 'QRIS (Midtrans)', configured: midtransKey !== null, online: midtransKey !== null });
    return statuses;
  }),
}));

vi.mock('@/api/hardware', () => ({
  listScanners: vi.fn(() =>
    Promise.resolve([{ id: 'scanner-01' }, { id: 'scanner-02' }]),
  ),
  listDisplays: vi.fn(() => Promise.resolve([])),
  displayShow: vi.fn(() => Promise.resolve()),
  displayClear: vi.fn(() => Promise.resolve()),
}));

vi.mock('@/hooks/useIdleTimer', () => ({
  useIdleTimer: vi.fn(),
  getAutoLockMinutes: vi.fn(() => 5),
  setAutoLockMinutes: vi.fn(),
}));

vi.mock('@/i18n/LanguageSelector', () => ({
  LanguageSelector: () => {
    // Render a simple select so tests can find it
    return (
      <select aria-label="Language">
        <option value="en">English</option>
        <option value="id">Bahasa Indonesia</option>
      </select>
    );
  },
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', username: 'testuser', role_name: 'manager', token: 'mock-token', role_id: 'role-1', display_name: 'Budi Manager' },
    loading: false, error: null, login: vi.fn(), logout: vi.fn(), clearError: vi.fn(),
    isManager: true, isOwner: false,
  }),
}));

// Sub-screen stubs — kept minimal so the parent-only assertions stay focused
vi.mock('@/features/settings/AppearanceSettings', () => ({
  AppearanceSettings: () => <div data-testid="mock-appearance">Appearance Settings Stub</div>,
}));
vi.mock('@/features/settings/FeatureToggleScreen', () => ({
  default: () => <div data-testid="mock-features">Feature Toggles Stub</div>,
}));
vi.mock('@/features/settings/DataManagementScreen', () => ({
  default: () => <div data-testid="mock-data">Data Management Stub</div>,
}));

// ── Test wrapper ──────────────────────────────────────────────────

function Wrapper({ children }: { children: ReactNode }) {
  return withFluent(<ToastProvider>{children}</ToastProvider>, salesFtl, settingsFtl);
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
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    // Clear mock settings DB used by the invoke('get_setting'/'set_setting') mock
    for (const k of Object.keys(mockSettingsDb)) delete mockSettingsDb[k];
  });

  // ── Loading & Rendering ────────────────────────────────────────

  it('shows loading state while fetching settings', async () => {
    const { getStoreSettings } = await import('@/api/settings');
    vi.mocked(getStoreSettings).mockImplementationOnce(() => new Promise(() => {}));

    await renderInAct(wrap());

    expect(screen.getByText('Loading\u2026')).toBeInTheDocument();
  });

  it('renders the General tab by default', async () => {
    await renderInAct(wrap());

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
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Receipt')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Receipt'));

    expect(screen.getByText('Receipt Settings')).toBeInTheDocument();
    // Footer text is rendered in a textarea value, not as visible text
    expect(screen.getByDisplayValue('Terima kasih')).toBeInTheDocument();
  });

  it('switches to Printer tab', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Printer')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Printer'));

    expect(screen.getByText('Receipt Printer')).toBeInTheDocument();
    expect(screen.getByDisplayValue('/dev/usb/lp0')).toBeInTheDocument();
  });

  it('switches to Scanner tab', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Scanner')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Scanner'));

    expect(screen.getByText('Barcode Scanner')).toBeInTheDocument();
  });

  it('switches to Credit tab', async () => {
    await renderInAct(wrap());

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
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    expect(screen.getByText(/App version/)).toBeInTheDocument();
    expect(screen.getByDisplayValue('0.0.3')).toBeInTheDocument();
    expect(screen.getByDisplayValue(/Budi Manager/)).toBeInTheDocument();
  });

  // ── Payments tab ────────────────────────────────────────────────

  it('switches to Payments tab and shows gateway status badges', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    expect(screen.getByText('Payment Gateways')).toBeInTheDocument();
    // Gateway status badges — use getAllByText since both badges and closed
    // <summary> text are in the DOM and may match the same string
    expect(screen.getAllByText('Stripe').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Square').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('QRIS (Midtrans)').length).toBeGreaterThanOrEqual(1);
  });

  it('shows no-gateways message when gateway list is empty', async () => {
    const { getGatewayStatus } = await import('@/api/gateway');
    vi.mocked(getGatewayStatus).mockResolvedValueOnce([]);

    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    await waitFor(() => {
      expect(screen.getByText(/No payment gateways configured/)).toBeInTheDocument();
    });
  });

  it('handles gateway status load failure gracefully', async () => {
    const { getGatewayStatus } = await import('@/api/gateway');
    vi.mocked(getGatewayStatus).mockRejectedValueOnce(new Error('Network error'));

    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    await waitFor(() => {
      expect(screen.getByText(/No payment gateways configured/)).toBeInTheDocument();
    });
  });

  it('shows Stripe API key input in Payments tab', async () => {
    mockSettingsDb['stripe.api_key'] = 'sk_test_stored_key';
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    // Stripe section is in a <details> <summary> — use emoji prefix to target summary, not badge
    const stripeSummary = screen.getByText('💳 Stripe');
    await userEvent.click(stripeSummary);

    await waitFor(() => {
      expect(screen.getByDisplayValue('sk_test_stored_key')).toBeInTheDocument();
    });
  });

  it('shows Square API key input in Payments tab', async () => {
    mockSettingsDb['square.api_key'] = 'sq0atp_test';
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    const squareSummary = screen.getByText('🟦 Square');
    await userEvent.click(squareSummary);

    await waitFor(() => {
      expect(screen.getByDisplayValue('sq0atp_test')).toBeInTheDocument();
    });
  });

  it('shows Midtrans key input in Payments tab', async () => {
    mockSettingsDb['midtrans.server_key'] = 'Mid-server-test';
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    const midtransSummary = screen.getByText('📱 QRIS (Midtrans)');
    await userEvent.click(midtransSummary);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Mid-server-test')).toBeInTheDocument();
    });
  });

  // ── Tender presets ──────────────────────────────────────────────

  it('renders default tender presets in Payments tab', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    expect(screen.getByText('Quick Cash Tender Buttons')).toBeInTheDocument();
    // Default values: 5000, 10000, 20000, 50000, 100000
    expect(screen.getByDisplayValue('5000')).toBeInTheDocument();
    expect(screen.getByDisplayValue('10000')).toBeInTheDocument();
    expect(screen.getByDisplayValue('20000')).toBeInTheDocument();
    expect(screen.getByDisplayValue('50000')).toBeInTheDocument();
    expect(screen.getByDisplayValue('100000')).toBeInTheDocument();
  });

  it('adds a new tender preset when Add preset is clicked', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    const addBtn = screen.getByText(/Add preset/);
    await userEvent.click(addBtn);

    // New preset added with value 0
    expect(screen.getByDisplayValue('0')).toBeInTheDocument();
  });

  it('disables Add preset button when 8 presets exist', async () => {
    localStorage.setItem('retail-tender-presets', JSON.stringify([1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000]));
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    const addBtn = screen.getByText(/Add preset/);
    expect(addBtn).toBeDisabled();
  });

  it('removes a tender preset when remove button is clicked', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    // There are 5 default presets, remove the first one
    const removeButtons = screen.getAllByText('\u00D7'); // × character
    expect(removeButtons.length).toBe(5);

    await userEvent.click(removeButtons[0]!);

    // Should now have 4 presets
    const remainingRemoveButtons = screen.getAllByText('\u00D7');
    expect(remainingRemoveButtons.length).toBe(4);
  });

  it('disables remove buttons when only 2 presets remain', async () => {
    localStorage.setItem('retail-tender-presets', JSON.stringify([5000, 10000]));
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    // Only 2 presets, remove buttons should be disabled
    const removeButtons = screen.getAllByText('\u00D7');
    expect(removeButtons.length).toBe(2);
    for (const btn of removeButtons) {
      expect(btn).toBeDisabled();
    }
  });

  it('loads tender presets from localStorage on mount', async () => {
    localStorage.setItem('retail-tender-presets', JSON.stringify([25000, 75000]));
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    expect(screen.getByDisplayValue('25000')).toBeInTheDocument();
    expect(screen.getByDisplayValue('75000')).toBeInTheDocument();
  });

  it('saves tender presets to localStorage when Save is clicked', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      const saved = localStorage.getItem('retail-tender-presets');
      expect(saved).not.toBeNull();
      const parsed = JSON.parse(saved!);
      expect(parsed).toEqual([5000, 10000, 20000, 50000, 100000]);
    });
  });

  // ── Sound toggle ────────────────────────────────────────────────

  it('shows sound toggle checkbox checked by default in System tab', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    const soundCheckbox = screen.getByRole('checkbox', { name: 'Sound Effects' }) as HTMLInputElement;
    expect(soundCheckbox).toBeInTheDocument();
    expect(soundCheckbox.checked).toBe(true);
  });

  it('loads sound preference from localStorage (disabled)', async () => {
    localStorage.setItem('retail-sound-enabled', 'false');
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    const soundCheckbox = screen.getByRole('checkbox', { name: 'Sound Effects' }) as HTMLInputElement;
    expect(soundCheckbox.checked).toBe(false);
  });

  it('saves sound preference to localStorage when Save is clicked', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(localStorage.getItem('retail-sound-enabled')).toBe('true');
    });
  });

  it('saves disabled sound preference to localStorage', async () => {
    localStorage.setItem('retail-sound-enabled', 'false');
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(localStorage.getItem('retail-sound-enabled')).toBe('false');
    });
  });

  // ── Language selector ───────────────────────────────────────────

  it('renders language selector in System tab', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    const langSelect = screen.getByLabelText('Language');
    expect(langSelect).toBeInTheDocument();
  });

  // ── Quick links info ────────────────────────────────────────────

  it('shows quick links informational text in System tab', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('System'));

    expect(screen.getByText('More Configuration')).toBeInTheDocument();
    expect(screen.getByText(/Tax rates and feature toggles can be configured/)).toBeInTheDocument();
  });

  // ── Gateway keys localStorage ───────────────────────────────────

  it('saves gateway keys to settings DB when Save is clicked', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    // Open Stripe section and type a key — use emoji prefix to target summary, not badge
    const stripeSummary = screen.getByText('💳 Stripe');
    await userEvent.click(stripeSummary);

    const stripeInput = screen.getByPlaceholderText(/sk_live_/);
    await userEvent.clear(stripeInput);
    await userEvent.type(stripeInput, 'sk_test_my_key');

    // Open Square section and type a key — use emoji prefix to target summary, not badge
    const squareSummary = screen.getByText('🟦 Square');
    await userEvent.click(squareSummary);

    const squareInput = screen.getByPlaceholderText(/sq0atp-/);
    await userEvent.clear(squareInput);
    await userEvent.type(squareInput, 'sq0atp_my_key');

    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockSettingsDb['stripe.api_key']).toBe('sk_test_my_key');
      expect(mockSettingsDb['square.api_key']).toBe('sq0atp_my_key');
    });
  });

  it('removes gateway keys from settings DB when cleared and saved', async () => {
    mockSettingsDb['stripe.api_key'] = 'old_key';
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Payments')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Payments'));

    // Open Stripe section and clear the key — use emoji prefix to target summary, not badge
    const stripeSummary = screen.getByText('💳 Stripe');
    await userEvent.click(stripeSummary);

    const stripeInput = screen.getByPlaceholderText(/sk_live_/);
    await userEvent.clear(stripeInput);

    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockSettingsDb['stripe.api_key']).toBeUndefined();
    });
  });

  // ── Save ───────────────────────────────────────────────────────

  it('saves all settings when Save is clicked', async () => {
    const { setStoreSettings, setReceiptSettings, setCreditSettings, setHardwareSettings } = await import('@/api/settings');

    await renderInAct(wrap());

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
    await renderInAct(wrap());

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

    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    const saveBtn = screen.getByText('Save');
    await userEvent.click(saveBtn);

    expect(screen.getByText('Saving\u2026')).toBeInTheDocument();
  });

  // ── Scanner tab ────────────────────────────────────────────────

  it('shows detected scanners in Scanner tab', async () => {
    await renderInAct(wrap());

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
    await renderInAct(wrap());

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
    await renderInAct(wrap());

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
    await renderInAct(wrap(onClose));

    await waitFor(() => {
      expect(screen.getByText(/Back/)).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText(/Back/));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it('calls onClose when Escape is pressed', async () => {
    const onClose = vi.fn();
    await renderInAct(wrap(onClose));

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.keyboard('{Escape}');

    expect(onClose).toHaveBeenCalledOnce();
  });

  it('calls onClose when Close button is clicked', async () => {
    const onClose = vi.fn();
    await renderInAct(wrap(onClose));

    await waitFor(() => {
      expect(screen.getByText('General Settings')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Close'));

    expect(onClose).toHaveBeenCalledOnce();
  });

  // ── Credit toggle ──────────────────────────────────────────────

  it('shows credit limit info when credit is enabled', async () => {
    await renderInAct(wrap());

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

  // ── New tab navigation (Appearance / Features / Data / Sync) ────

  it('switches to Appearance tab and renders AppearanceSettings', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Appearance')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Appearance'));

    await waitFor(() => {
      expect(screen.getByTestId('mock-appearance')).toBeInTheDocument();
    });
  });

  it('switches to Features tab and renders FeatureToggleScreen', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Features')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Features'));

    await waitFor(() => {
      expect(screen.getByTestId('mock-features')).toBeInTheDocument();
    });
  });

  it('switches to Data tab and renders DataManagementScreen', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Data')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Data'));

    await waitFor(() => {
      expect(screen.getByTestId('mock-data')).toBeInTheDocument();
    });
  });

  it('switches to Sync tab and renders the Cloud Sync form', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Sync'));

    // Heading comes from the inline Cloud Sync section
    expect(screen.getByText('Cloud Sync')).toBeInTheDocument();
    // Server URL + Auth Token + Interval inputs all wired to the hook
    expect(screen.getByLabelText('Server URL')).toBeInTheDocument();
    expect(screen.getByLabelText('Authentication Token')).toBeInTheDocument();
    expect(screen.getByLabelText('Auto-sync interval (minutes)')).toBeInTheDocument();
  });

  // ── Sync tab behaviour ──────────────────────────────────────────

  it('disables the Sync enable toggle until a server URL is set', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });
    await userEvent.click(screen.getByText('Sync'));

    const enableCheckbox = screen.getByRole('checkbox', { name: /Enable cloud sync/i }) as HTMLInputElement;
    expect(enableCheckbox).toBeInTheDocument();
    // Server URL is empty on mount → toggle must be disabled.
    expect(enableCheckbox).toBeDisabled();

    // Typing a server URL re-enables it.
    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');

    await waitFor(() => {
      expect(enableCheckbox).not.toBeDisabled();
    });
  });

  it('Sync now simulates a successful round-trip and flips status to Online', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });
    await userEvent.click(screen.getByText('Sync'));

    // Server URL is required before the round-trip will run.
    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');

    // Status pill starts in "Offline" state (default `useState('offline')`).
    expect(screen.getByText('Offline')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /Sync now/i }));

    // Simulated round-trip is 600ms; waitFor polls until the pill flips.
    await waitFor(() => {
      expect(screen.getByText('Online')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Success toast surfaces as a role=alert element.
    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/Sync completed successfully/);
    });
  });

  it('persists the sync auth token via the secure set_setting IPC', async () => {
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });
    await userEvent.click(screen.getByText('Sync'));

    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');
    await userEvent.type(
      screen.getByLabelText('Authentication Token'),
      'tok_secure_persisted',
    );

    // Save is in the footer and works regardless of which tab is active.
    await userEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockSettingsDb['sync.auth_token']).toBe('tok_secure_persisted');
    });

    // Non-secret config also lands in localStorage through the same persist().
    expect(localStorage.getItem('retail-sync-server')).toBe('https://sync.example.com');
    expect(localStorage.getItem('retail-sync-enabled')).toBe('false'); // default toggled off
  });

  it('invokes the real sync_run Tauri command when Sync now is clicked', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });
    await userEvent.click(screen.getByText('Sync'));
    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');
    await userEvent.click(screen.getByRole('button', { name: /Sync now/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('sync_run');
    });
  });

  it('surfaces an error toast when sync_run reports a failure', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    await renderInAct(wrap());

    await waitFor(() => {
      expect(screen.getByText('Sync')).toBeInTheDocument();
    });
    await userEvent.click(screen.getByText('Sync'));
    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');

    // Queue the override AFTER mount so it lands on the next sync_run
    // call (otherwise the mount-time `pending_sync_count` consumes the
    // one-shot and the actual sync falls through to the default).
    vi.mocked(invoke).mockImplementationOnce(async (cmd: string) => {
      if (cmd === 'sync_run') {
        return { synced: 0, failed: 0, error: 'server unreachable' };
      }
      return null;
    });

    await userEvent.click(screen.getByRole('button', { name: /Sync now/i }));

    // Error toast surfaces as a role=alert element with the backend's
    // error string.
    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/server unreachable/);
    });

    // Status pill must NOT flip to Online on error. (Offline is the
    // initial state, so the stronger check is that Online is absent.)
    await waitFor(() => {
      expect(screen.queryByText('Online')).not.toBeInTheDocument();
    });
  });

  // ── Pull from server ────────────────────────────────────────────

  it('does not invoke sync_pull when the user cancels the confirm dialog', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);

    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Sync')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Sync'));
    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');

    // Reset the invoke call log so we only count the click under test,
    // not the mount-time `pending_sync_count` or auto-sync noise.
    vi.mocked(invoke).mockClear();
    await userEvent.click(screen.getByTestId('sync-pull-btn'));

    expect(confirmSpy).toHaveBeenCalled();
    // The confirm dialog was dismissed — no sync_pull IPC should fire.
    const pullCalls = vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'sync_pull');
    expect(pullCalls).toHaveLength(0);

    confirmSpy.mockRestore();
  });

  it('invokes sync_pull and surfaces a success toast after the user confirms', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Sync')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Sync'));
    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');

    await userEvent.click(screen.getByTestId('sync-pull-btn'));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('sync_pull');
    });

    // Default mock returns a populated PullResult; the success toast
    // surfaces as a role=alert element.
    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/Pulled/);
    });

    confirmSpy.mockRestore();
  });

  it('surfaces an error toast when sync_pull reports a failure', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Sync')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Sync'));
    await userEvent.type(screen.getByLabelText('Server URL'), 'https://sync.example.com');

    // Queue the override AFTER mount so it lands on the next sync_pull
    // call (otherwise the mount-time `pending_sync_count` could consume
    // the one-shot, depending on order).
    vi.mocked(invoke).mockImplementationOnce(async (cmd: string) => {
      if (cmd === 'sync_pull') {
        return { productsPulled: 0, taxRatesPulled: 0, usersPulled: 0, error: 'pull refused by server' };
      }
      return null;
    });

    await userEvent.click(screen.getByTestId('sync-pull-btn'));

    await waitFor(() => {
      const toast = screen.getByRole('alert');
      expect(toast.textContent).toMatch(/pull refused by server/);
    });

    confirmSpy.mockRestore();
  });

  it('disables the Pull from server button when no server URL is set', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Sync')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Sync'));

    const pullBtn = screen.getByTestId('sync-pull-btn') as HTMLButtonElement;
    expect(pullBtn).toBeInTheDocument();
    expect(pullBtn).toBeDisabled();
  });

  // ── htmlFor/id wiring (a11y + getByLabelText) ──────────────────

  it('wires htmlFor/id on every General-tab input so screen readers can announce them', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('General Settings')).toBeInTheDocument());

    // All five General inputs are addressable via getByLabelText.
    expect(screen.getByLabelText('Store name')).toBeInTheDocument();
    expect(screen.getByLabelText('Address')).toBeInTheDocument();
    expect(screen.getByLabelText('Branch')).toBeInTheDocument();
    expect(screen.getByLabelText('Tax ID')).toBeInTheDocument();
    expect(screen.getByLabelText('Default currency')).toBeInTheDocument();
  });

  it('wires htmlFor/id on the Receipt tab checkboxes and selects', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Receipt')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Receipt'));

    // Checkboxes (row layout) are addressable by their label text.
    expect(screen.getByLabelText('Show currency symbol')).toBeInTheDocument();
    expect(screen.getByLabelText('Show tax line')).toBeInTheDocument();
    expect(screen.getByLabelText('Show table number')).toBeInTheDocument();
    // Selects
    expect(screen.getByLabelText('Decimal separator')).toBeInTheDocument();
    expect(screen.getByLabelText('Paper width')).toBeInTheDocument();
    // Textarea
    expect(screen.getByLabelText('Receipt footer')).toBeInTheDocument();
  });

  it('wires htmlFor/id on the Credit tab inputs', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Credit')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Credit'));

    expect(screen.getByLabelText('Enable credit sales')).toBeInTheDocument();
    expect(screen.getByLabelText('Reminder interval (hours)')).toBeInTheDocument();
    expect(screen.getByLabelText('Max credit limit (Rp)')).toBeInTheDocument();
  });

  it('wires htmlFor/id on the System tab display fields', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('System')).toBeInTheDocument());
    await userEvent.click(screen.getByText('System'));

    // The disabled display fields are still addressable.
    expect(screen.getByLabelText('App version')).toBeInTheDocument();
    expect(screen.getByLabelText('Cashier')).toBeInTheDocument();
    expect(screen.getByLabelText('Terminal')).toBeInTheDocument();
    expect(screen.getByLabelText('Auto-lock after (minutes)')).toBeInTheDocument();
  });

  it('wires htmlFor/id on the dynamic tender-preset labels in Payments tab', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Payments')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Payments'));

    // The default 5 presets each have a "Preset N" label wired to a
    // matching input id. We verify the wiring by checking the DOM
    // directly (id exists + label[for=...] references it) because the
    // test wrapper's `l10n.getString` may return the raw FTL string
    // ("Preset { $n }") rather than the interpolated value, depending
    // on whether the test Fluent provider supports vars interpolation.
    const id1 = document.getElementById('payments-tender-preset-1') as HTMLInputElement | null;
    const id2 = document.getElementById('payments-tender-preset-2') as HTMLInputElement | null;
    const id5 = document.getElementById('payments-tender-preset-5') as HTMLInputElement | null;
    expect(id1).toBeInTheDocument();
    expect(id2).toBeInTheDocument();
    expect(id5).toBeInTheDocument();
    // And each input is referenced by a matching label htmlFor.
    expect(id1?.labels?.length ?? 0).toBeGreaterThan(0);
    expect(id2?.labels?.length ?? 0).toBeGreaterThan(0);
    expect(id5?.labels?.length ?? 0).toBeGreaterThan(0);
  });

  it('wires htmlFor/id on the Printer and Scanner tabs', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Printer')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Printer'));

    expect(screen.getByLabelText('Connection')).toBeInTheDocument();
    expect(screen.getByLabelText('Device path')).toBeInTheDocument();
    expect(screen.getByLabelText('Paper size')).toBeInTheDocument();

    await userEvent.click(screen.getByText('Scanner'));
    expect(screen.getByLabelText('Scanner device')).toBeInTheDocument();
    expect(screen.getByLabelText('Input mode')).toBeInTheDocument();
  });

  it('wires htmlFor/id on the Payments tab gateway keys after opening the details', async () => {
    await renderInAct(wrap());
    await waitFor(() => expect(screen.getByText('Payments')).toBeInTheDocument());
    await userEvent.click(screen.getByText('Payments'));

    // Stripe / Square / Midtrans inputs only render after the user
    // opens the corresponding <details> summary.
    await userEvent.click(screen.getByText('💳 Stripe'));
    expect(screen.getByLabelText('Stripe API Key')).toBeInTheDocument();

    await userEvent.click(screen.getByText('🟦 Square'));
    expect(screen.getByLabelText('Square API Key')).toBeInTheDocument();

    await userEvent.click(screen.getByText('📱 QRIS (Midtrans)'));
    expect(screen.getByLabelText('Midtrans Server Key')).toBeInTheDocument();
  });

});
