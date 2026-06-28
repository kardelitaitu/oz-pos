import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';

const LOCALE_STRINGS = [
  'tax-config-title = Tax Configuration',
  'tax-config-add = Add Tax Rate',
  'tax-config-empty = No tax rates configured',
  'tax-config-loading = Loading tax rates…',
  'tax-config-col-name = Name',
  'tax-config-col-rate = Rate (%)',
  'tax-config-field-name = Tax Name',
  'tax-config-field-rate = Rate (%)',
  'tax-config-btn-cancel = Cancel',
  'tax-config-btn-save = Save',
  'tax-config-btn-delete = Delete',
  'tax-config-edit = Edit',
  'tax-config-modal-title = { $editing -> [true] Edit Tax Rate *[other] Add Tax Rate }',
].join('\n');

const wrap = (children: React.ReactNode) => {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(LOCALE_STRINGS));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

const SAMPLE_TAX_RATES = [
  { id: 'tax-1', name: 'Sales Tax', rate_bps: 825, is_default: true, display_rate: '8.25%', created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
  { id: 'tax-2', name: 'VAT', rate_bps: 2000, is_default: false, display_rate: '20%', created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
];

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn() as any,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'list_tax_rates') return Promise.resolve(SAMPLE_TAX_RATES);
    if (cmd === 'create_tax_rate') return Promise.resolve({ ...SAMPLE_TAX_RATES[0], name: 'New Tax' });
    if (cmd === 'delete_tax_rate') return Promise.resolve(undefined);
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  });
});

async function waitForTable() {
  await screen.findByRole('table', { name: /tax rates/i });
}

describe('TaxConfigurationScreen', () => {
  it('renders title', async () => {
    render(wrap(<TaxConfigurationScreen />));
    await waitForTable();
    expect(screen.getByRole('heading', { name: /tax configuration/i })).toBeInTheDocument();
  });

  it('shows loading state', async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    render(wrap(<TaxConfigurationScreen />));
    expect(screen.getByText(/loading tax rates/i)).toBeInTheDocument();
  });

  it('renders tax rate rows', async () => {
    render(wrap(<TaxConfigurationScreen />));
    await waitForTable();
    expect(screen.getByText('Sales Tax')).toBeInTheDocument();
    expect(screen.getByText('VAT')).toBeInTheDocument();
    expect(screen.getByText('8.25%')).toBeInTheDocument();
    expect(screen.getByText('20%')).toBeInTheDocument();
  });

  it('shows default badge', async () => {
    render(wrap(<TaxConfigurationScreen />));
    await waitForTable();
    expect(screen.getAllByText('Default').length).toBeGreaterThanOrEqual(1);
  });

  it('shows empty state', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_tax_rates') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(wrap(<TaxConfigurationScreen />));
    await waitFor(() => {
      expect(screen.getByText(/no tax rates configured/i)).toBeInTheDocument();
    });
  });

  it('opens add modal', async () => {
    render(wrap(<TaxConfigurationScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add tax rate/i }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText(/tax name/i)).toBeInTheDocument();
    expect(within(dialog).getByText(/rate \(%\)/i)).toBeInTheDocument();
  });
});
