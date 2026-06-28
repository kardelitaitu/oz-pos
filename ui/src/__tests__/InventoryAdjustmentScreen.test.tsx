import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import InventoryAdjustmentScreen from '@/features/inventory/InventoryAdjustmentScreen';

const LOCALE_STRINGS = [
  'inv-title = Inventory Adjustment',
  'inv-step-select-product = 1. Select Product',
  'inv-step-adjustment-details = 2. Adjustment Details',
  'inv-change = Change',
  'inv-search-placeholder = Search by SKU, name, or barcode…',
  'inv-search-aria = Search products',
  'inv-loading = Loading products…',
  'inv-no-results = No products match your search.',
  'inv-hint = Type to search for a product by SKU, name, or barcode.',
  'inv-stock-off = Stock tracking off',
  'inv-stock-count = { $count } in stock',
  'inv-type-aria = Adjustment type',
  'inv-type-add-label = Stock In (Restock)',
  'inv-type-remove-label = Stock Out (Remove)',
  'inv-qty-label = Quantity',
  'inv-qty-placeholder = e.g. 10',
  'inv-reason-label = Reason',
  'inv-reason-select = Select a reason…',
  'inv-cancel = Cancel',
  'inv-apply-restock = Apply Restock',
  'inv-apply-removal = Apply Removal',
  'inv-qty-hint = Current stock: { $stock }',
  'inv-adjusting = Adjusting…',
  'inv-success-adjusted = Adjusted "{ $name }" by { $delta }. New stock: { $newQty }',
  'inv-error = { $message }',
  'inv-reason-restock = Restock (supplier delivery)',
  'inv-reason-stock-take = Stock take correction',
  'inv-reason-return = Customer return',
  'inv-reason-damaged = Damaged / spoiled',
  'inv-reason-write-off = Write-off / expiry',
  'inv-reason-transfer = Transfer to other location',
  'inv-reason-other = Other reason…',
  'inv-reason-custom-label = Describe the reason',
  'inv-reason-custom-placeholder = Enter the reason for this adjustment…',
].join('\n');

const wrap = (children: React.ReactNode) => {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(LOCALE_STRINGS));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

const SAMPLE_PRODUCTS: any[] = [
  { sku: 'LATTE', name: 'Caffè Latte', price: { minor_units: 450, currency: 'USD' }, barcode: null, in_stock: true, stock_qty: 20, category: 'Beverages' },
  { sku: 'BAGEL', name: 'Plain Bagel', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567890', in_stock: true, stock_qty: 5, category: 'Food' },
  { sku: 'BROWNIE', name: 'Fudge Brownie', price: { minor_units: 295, currency: 'USD' }, barcode: null, in_stock: false, stock_qty: 0, category: 'Food' },
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
    if (cmd === 'list_products') return Promise.resolve(SAMPLE_PRODUCTS);
    if (cmd === 'adjust_stock') return Promise.resolve(25);
    return Promise.reject(new Error(`Unknown command: ${cmd}`));
  });
});

describe('InventoryAdjustmentScreen', () => {
  it('renders title and step 1', async () => {
    render(wrap(<InventoryAdjustmentScreen />));
    await waitFor(() => {
      expect(screen.getByText(/inventory adjustment/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/1\. select product/i)).toBeInTheDocument();
  });

  it('shows search input', async () => {
    render(wrap(<InventoryAdjustmentScreen />));
    await waitFor(() => {
      expect(screen.getByRole('searchbox')).toBeInTheDocument();
    });
    expect(screen.getByPlaceholderText(/search by sku/i)).toBeInTheDocument();
  });

  it('shows hint text when no query', async () => {
    render(wrap(<InventoryAdjustmentScreen />));
    await waitFor(() => {
      expect(screen.getByText(/type to search for a product/i)).toBeInTheDocument();
    });
  });

  it('filters products by search', async () => {
    render(wrap(<InventoryAdjustmentScreen />));
    await waitFor(() => {
      expect(screen.getByRole('searchbox')).toBeInTheDocument();
    });
    const searchInput = screen.getByRole('searchbox');
    await userEvent.type(searchInput, 'latte');
    await waitFor(() => {
      expect(screen.getByText('Caffè Latte')).toBeInTheDocument();
    });
  });

  it('selects a product', async () => {
    render(wrap(<InventoryAdjustmentScreen />));
    await waitFor(() => {
      expect(screen.getByRole('searchbox')).toBeInTheDocument();
    });
    const searchInput = screen.getByRole('searchbox');
    await userEvent.type(searchInput, 'latte');
    await waitFor(() => {
      expect(screen.getByRole('option', { name: /caffè latte/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('option', { name: /caffè latte/i }));
    expect(screen.getByText('Caffè Latte')).toBeInTheDocument();
    expect(screen.getByText('$4.50')).toBeInTheDocument();
    expect(screen.getByText((c) => c.includes('20') && c.includes('in stock'))).toBeInTheDocument();
  });

  it('shows step 2 after product selected', async () => {
    render(wrap(<InventoryAdjustmentScreen />));
    await waitFor(() => {
      expect(screen.getByRole('searchbox')).toBeInTheDocument();
    });
    const searchInput = screen.getByRole('searchbox');
    await userEvent.type(searchInput, 'bagel');
    await waitFor(() => {
      expect(screen.getByRole('option', { name: /plain bagel/i })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('option', { name: /plain bagel/i }));
    await waitFor(() => {
      expect(screen.getByText(/2\. adjustment details/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/quantity/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/reason/i)).toBeInTheDocument();
  });
});
