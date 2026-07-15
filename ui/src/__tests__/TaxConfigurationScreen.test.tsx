import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import taxFtl from '@/locales/tax.ftl?raw';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';

const SAMPLE_TAX_RATES = [
  { id: 'tax-1', name: 'Sales Tax', rate_bps: 825, is_default: true, display_rate: '8.25%', created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
  { id: 'tax-2', name: 'VAT', rate_bps: 2000, is_default: false, display_rate: '20%', created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
];

const { invokeMock } = vi.hoisted(() => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  invokeMock: vi.fn() as any,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'list_tax_rates') return Promise.resolve(SAMPLE_TAX_RATES);
    if (cmd === 'list_categories') return Promise.resolve([]);
    if (cmd === 'list_category_tax_rates') return Promise.resolve([]);
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
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    expect(screen.getByRole('heading', { name: /tax configuration/i })).toBeInTheDocument();
  });

  it('shows loading state', async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    expect(screen.getByText(/loading tax rates/i)).toBeInTheDocument();
  });

  it('renders tax rate rows', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    expect(screen.getByText('Sales Tax')).toBeInTheDocument();
    expect(screen.getByText('VAT')).toBeInTheDocument();
    expect(screen.getByText('8.25%')).toBeInTheDocument();
    expect(screen.getByText('20%')).toBeInTheDocument();
  });

  it('shows default badge', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    expect(screen.getAllByText('Default').length).toBeGreaterThanOrEqual(1);
  });

  it('shows empty state', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_tax_rates') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitFor(() => {
      expect(screen.getByText(/no tax rates configured/i)).toBeInTheDocument();
    });
  });

  it('opens add modal', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add tax rate/i }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText(/tax name/i)).toBeInTheDocument();
    expect(within(dialog).getByText(/rate \(%\)/i)).toBeInTheDocument();
  });
});
