// ── RetailOptionsScreen tests ─────────────────────────────────────
//
// Covers: tab navigation, settings loading/saving, receipt preview,
// scanner list, keyboard shortcuts (Escape), credit toggle.

import { describe, expect, it, vi, beforeEach } from 'vitest';
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

    render(wrap());

    expect(screen.getByText('Loading\u2026')).toBeInTheDocument();
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
    expect(screen.getByDisplayValue('0.0.8')).toBeInTheDocument();
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
