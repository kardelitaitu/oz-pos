import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import taxFtl from '@/locales/tax.ftl?raw';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';

const SAMPLE_TAX_RATES = [
  { id: 'tax-1', name: 'Sales Tax', rate_bps: 825, is_default: true, display_rate: '8.25%', is_inclusive: false, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
  { id: 'tax-2', name: 'VAT', rate_bps: 2000, is_default: false, display_rate: '20%', is_inclusive: true, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
];

const SAMPLE_CATEGORIES = [
  { id: 'cat-1', name: 'Food', colour: '#f97316', icon: 'food' },
  { id: 'cat-2', name: 'Drinks', colour: '#3b82f6', icon: 'drink' },
];

const SAMPLE_CAT_TAX_RATES = [
  { category_id: 'cat-1', tax_rate_ids: ['tax-1'] },
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
    if (cmd === 'list_tax_rates' || cmd === 'list_tax_rates_scoped') return Promise.resolve(SAMPLE_TAX_RATES);
    if (cmd === 'list_categories' || cmd === 'list_categories_scoped') return Promise.resolve(SAMPLE_CATEGORIES);
    if (cmd === 'list_category_tax_rates') return Promise.resolve(SAMPLE_CAT_TAX_RATES);
    if (cmd === 'create_tax_rate') return Promise.resolve({ ...SAMPLE_TAX_RATES[0], name: 'New Tax' });
    if (cmd === 'update_tax_rate') return Promise.resolve(SAMPLE_TAX_RATES[0]);
    if (cmd === 'delete_tax_rate') return Promise.resolve(undefined);
    if (cmd === 'set_category_tax_rates') return Promise.resolve(undefined);
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  });
});

async function waitForTable() {
  // The tax rates table has exact aria-label "Tax rates" (from Fluent key tax-config-table-aria).
  // The category table has "Category tax rates" — don't match that one.
  await screen.findByRole('table', { name: 'Tax rates' });
}

describe('TaxConfigurationScreen', () => {
  it('renders title', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    expect(screen.getByRole('heading', { name: /tax configuration/i })).toBeInTheDocument();
  });

  it('shows loading skeleton while fetching tax rates', async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    expect(document.querySelector('.tax-config-loading-skeleton')).toBeInTheDocument();
    expect(screen.queryByText(/loading tax rates/i)).not.toBeInTheDocument();
  });

  it('renders tax rate rows', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    // Use getAllByText — 'Sales Tax' appears in both the table and category badges
    expect(screen.getAllByText('Sales Tax').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('VAT')).toBeInTheDocument();
    expect(screen.getByText('8.25%')).toBeInTheDocument();
    expect(screen.getByText('20%')).toBeInTheDocument();
  });

  it('shows default badge for default tax rate', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    // Sales Tax is default, VAT is not
    const defaultBadges = screen.getAllByText('Default');
    expect(defaultBadges.length).toBeGreaterThanOrEqual(1);
  });

  it('shows empty state when no tax rates exist', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_tax_rates' || cmd === 'list_tax_rates_scoped') return Promise.resolve([]);
      if (cmd === 'list_categories' || cmd === 'list_categories_scoped') return Promise.resolve([]);
      if (cmd === 'list_category_tax_rates') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitFor(() => {
      expect(screen.getByText(/no tax rates configured/i)).toBeInTheDocument();
    });
  });

  it('opens add modal when Add Tax Rate is clicked', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add tax rate/i }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText('Tax Name')).toBeInTheDocument();
    // Rate label has exact text 'Rate (%)' — avoid partial match which could
    // also match the hint text 'Enter rate in basis points...'
    expect(within(dialog).getByText('Rate (%)')).toBeInTheDocument();
  });

  // ── New edge-case tests ─────────────────────────────────────────

  it('opens edit modal pre-filled when Edit is clicked', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();

    // 'Sales Tax' appears in both the rate table and category badges,
    // so use getAllByText and scope to the first matching row
    const salesTaxCells = screen.getAllByText('Sales Tax');
    // The first occurrence is in the rate table (row with 8.25%)
    const salesTaxRow = salesTaxCells[0]!.closest('tr')!;
    const editBtn = within(salesTaxRow).getByRole('button', { name: /edit/i });
    await userEvent.click(editBtn);

    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();

    // Modal should have the tax name input pre-filled
    const nameInput = within(dialog).getByDisplayValue('Sales Tax');
    expect(nameInput).toBeInTheDocument();
  });

  it('deletes a tax rate when Delete is clicked', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();

    // Find and click the Delete button for VAT (non-default)
    // 'VAT' appears in the rate table rows — scope to that table
    const vatRow = screen.getByText('VAT').closest('tr')!;
    const deleteBtn = within(vatRow).getByRole('button', { name: /delete/i });
    expect(deleteBtn).not.toBeDisabled();
    await userEvent.click(deleteBtn);

    // After delete, list_tax_rates should be called again (loadAll refreshes)
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('delete_tax_rate', expect.objectContaining({
        id: 'tax-2',
      }));
    });
  });

  it('renders the category tax rates section', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();

    // Category section heading
    expect(screen.getByText(/category tax rates/i)).toBeInTheDocument();
    expect(screen.getByText('Food')).toBeInTheDocument();
    expect(screen.getByText('Drinks')).toBeInTheDocument();
  });

  it('shows assigned tax rate badges in category section', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();

    // Food category has Sales Tax (tax-1) assigned
    const foodRow = screen.getByText('Food').closest('tr')!;
    expect(within(foodRow).getByText('Sales Tax')).toBeInTheDocument();

    // Drinks category has no rates assigned
    const drinksRow = screen.getByText('Drinks').closest('tr')!;
    expect(within(drinksRow).getByText(/no rates assigned/i)).toBeInTheDocument();
  });

  it('disables Delete button while deletion is in progress', async () => {
    // Make delete slow so we can see the loading state
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'delete_tax_rate') return new Promise(() => {});
      if (cmd === 'list_tax_rates' || cmd === 'list_tax_rates_scoped') return Promise.resolve(SAMPLE_TAX_RATES);
      if (cmd === 'list_categories' || cmd === 'list_categories_scoped') return Promise.resolve(SAMPLE_CATEGORIES);
      if (cmd === 'list_category_tax_rates') return Promise.resolve(SAMPLE_CAT_TAX_RATES);
      return Promise.resolve([]);
    });

    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();

    const vatRow = screen.getByText('VAT').closest('tr')!;
    const deleteBtn = within(vatRow).getByRole('button', { name: /delete/i });
    await userEvent.click(deleteBtn);

    // Should be disabled while delete is in flight
    await waitFor(() => {
      expect(deleteBtn).toBeDisabled();
    });
  });

  it('closes the add modal when Escape is pressed', async () => {
    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();

    // Open add modal
    await userEvent.click(screen.getByRole('button', { name: /add tax rate/i }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    // Press Escape
    await userEvent.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('handles save failure gracefully', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'create_tax_rate') return Promise.reject(new Error('DB error'));
      if (cmd === 'list_tax_rates' || cmd === 'list_tax_rates_scoped') return Promise.resolve(SAMPLE_TAX_RATES);
      if (cmd === 'list_categories' || cmd === 'list_categories_scoped') return Promise.resolve(SAMPLE_CATEGORIES);
      if (cmd === 'list_category_tax_rates') return Promise.resolve(SAMPLE_CAT_TAX_RATES);
      return Promise.resolve([]);
    });

    renderWithFluentSync(<TaxConfigurationScreen />, taxFtl);
    await waitForTable();

    // Open add modal, fill form, and save
    await userEvent.click(screen.getByRole('button', { name: /add tax rate/i }));
    const dialog = screen.getByRole('dialog');

    // Fill the name field
    const nameInput = within(dialog).getByRole('textbox', { name: /tax name/i });
    await userEvent.type(nameInput, 'New Tax');

    // Fill the rate field (type="number", role spinbutton) so save is enabled
    const rateInput = within(dialog).getByRole('spinbutton', { name: /rate/i });
    await userEvent.type(rateInput, '825');

    // Save and wait for error to be caught
    const saveBtn = within(dialog).getByRole('button', { name: /save/i });
    await userEvent.click(saveBtn);

    // Modal should stay open after failure and save should re-enable
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(saveBtn).not.toBeDisabled();
    });
  });
});
