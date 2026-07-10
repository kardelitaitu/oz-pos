import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withToastProviders } from '@/__tests__/test-utils/providers';
import settingsFtl from '@/locales/settings.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import SettingsPage from '@/features/settings/SettingsPage';
import { AuthProvider } from '@/contexts/AuthContext';
import { BrandProvider } from '@/contexts/BrandContext';
import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { LocaleContext } from '@/i18n/LocaleContext';
import { getAvailableLocales, getLocaleLabel } from '@/i18n';

const SAMPLE_CURRENCIES = [
  { code: 'USD', name: 'US Dollar', minor_exponent: 2, symbol: '$' },
  { code: 'EUR', name: 'Euro', minor_exponent: 2, symbol: '€' },
];

const { invokeMock } = vi.hoisted(() => {
  const invoke = vi.fn((cmd: string) => {
    if (cmd === 'get_store_settings') {
      return Promise.resolve({ name: '', address: '', taxId: '' });
    }
    if (cmd === 'list_currencies') {
      return Promise.resolve(SAMPLE_CURRENCIES);
    }
    if (cmd === 'get_default_currency') {
      return Promise.resolve('USD');
    }
    if (cmd === 'set_default_currency') {
      return Promise.resolve(undefined);
    }
    if (cmd === 'get_brand_settings') {
      return Promise.resolve({
        primary_colour: '#4f46e5',
        logo_path: null,
        store_name: '',
      });
    }
    return Promise.resolve({
      showCurrency: false,
      decimalSeparator: 'dot',
      showTax: true,
      footer: '',
      paperWidth: 'standard',
    });
  });
  return { invokeMock: invoke };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  localStorage.clear();
  invokeMock.mockClear();
});

function TestWrapper({ children }: { children: React.ReactNode }) {
  return withToastProviders(
    <LocaleContext.Provider
      value={{
        locale: 'en',
        setLocale: () => {},
        availableLocales: getAvailableLocales(),
        getLocaleLabel,
      }}
    >
      <BrandProvider>
        <ThemeProvider>
          <AuthProvider>{children}</AuthProvider>
        </ThemeProvider>
      </BrandProvider>
    </LocaleContext.Provider>,
    settingsFtl,
    sharedFtl,
  );
}

const wrap = (children: React.ReactNode) => <TestWrapper>{children}</TestWrapper>;

describe('SettingsPage', () => {
  it('renders the general section by default and has sidebar categories', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });
    // Sidebar has the Business category expanded and Operations category available
    expect(screen.getByRole('button', { name: /business/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /system/i })).toBeInTheDocument();
  });

  it('loads receipt settings and populates the form', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    // Expand Operations category, then navigate to Receipt section
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /receipt/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /receipt/i }));

    await waitFor(() => {
      expect(screen.getByLabelText(/show currency symbol/i)).not.toBeChecked();
      expect(screen.getByLabelText(/show tax line/i)).toBeChecked();
    });

    const select = screen.getByLabelText(/decimal separator/i);
    expect((select as HTMLSelectElement).value).toBe('dot');
  });

  it('toggles show-currency and show-tax checkboxes', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    // Expand Operations category, then navigate to Receipt section
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /receipt/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /receipt/i }));

    await waitFor(() => {
      expect(screen.getByLabelText(/show currency symbol/i)).toBeInTheDocument();
    });

    const showCurrency = screen.getByLabelText(/show currency symbol/i);
    const showTax = screen.getByLabelText(/show tax line/i);

    await userEvent.click(showCurrency);
    expect(showCurrency).toBeChecked();

    await userEvent.click(showTax);
    expect(showTax).not.toBeChecked();
  });

  it('changes decimal separator and paper width via select', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    // Expand Operations category, then navigate to Receipt section
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /receipt/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /receipt/i }));

    await waitFor(() => {
      expect(screen.getByLabelText(/decimal separator/i)).toBeInTheDocument();
    });

    const decimalSep = screen.getByLabelText(/decimal separator/i) as HTMLSelectElement;
    const paperWidth = screen.getByLabelText(/paper width/i) as HTMLSelectElement;

    await userEvent.selectOptions(decimalSep, 'comma');
    expect(decimalSep.value).toBe('comma');

    await userEvent.selectOptions(paperWidth, 'narrow');
    expect(paperWidth.value).toBe('narrow');
  });

  it('updates footer input', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /operations/i })).toBeInTheDocument();
    });
    // Expand Operations category, then navigate to Receipt section
    await userEvent.click(screen.getByRole('button', { name: /operations/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /receipt/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: /receipt/i }));

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/thank you/i)).toBeInTheDocument();
    });

    const footer = screen.getByPlaceholderText(/thank you/i);
    await userEvent.clear(footer);
    await userEvent.type(footer, 'Come again!');
    expect(footer).toHaveValue('Come again!');
  });

  it('calls set_receipt_settings and set_store_settings on save', async () => {
    render(wrap(<SettingsPage />));

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
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });

    await userEvent.click(screen.getByRole('button', { name: /save settings/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /saved!/i })).toBeInTheDocument();
    });
  });

  it('renders the Store section with name, address, and tax ID fields', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /store/i })).toBeInTheDocument();
    });

    expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Address' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: /tax.*id/i })).toBeInTheDocument();
  });

  it('updates store name input', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Store name' })).toBeInTheDocument();
    });

    const name = screen.getByRole('textbox', { name: 'Store name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Acme Corp');
    expect(name).toHaveValue('Acme Corp');
  });

  it('updates store address input', async () => {
    render(wrap(<SettingsPage />));

    await waitFor(() => {
      expect(screen.getByRole('textbox', { name: 'Address' })).toBeInTheDocument();
    });

    const addr = screen.getByRole('textbox', { name: 'Address' });
    await userEvent.clear(addr);
    await userEvent.type(addr, '456 Oak Ave');
    expect(addr).toHaveValue('456 Oak Ave');
  });
});
