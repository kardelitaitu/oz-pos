import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import productsFtl from '@/locales/products.ftl?raw';
import ProductManagementScreen from '@/features/products/ProductManagementScreen';

const wrap = (children: React.ReactNode) => withFluent(children, productsFtl);

const SAMPLE_PRODUCTS = [
  {
    sku: 'LATTE',
    name: 'Caffè Latte',
    category: 'Beverages',
    price: { minor_units: 450, currency: 'USD' },
    barcode: '4901234567890',
    in_stock: true,
    stock_qty: null,
  },
  {
    sku: 'BAGEL',
    name: 'Plain Bagel',
    category: 'Food',
    price: { minor_units: 250, currency: 'USD' },
    barcode: null,
    in_stock: true,
    stock_qty: null,
  },
  {
    sku: 'BROWNIE',
    name: 'Fudge Brownie',
    category: 'Food',
    price: { minor_units: 295, currency: 'USD' },
    barcode: '4901234567906',
    in_stock: false,
    stock_qty: null,
  },
];

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn((cmd: string) => {
    if (cmd === 'list_products') {
      return Promise.resolve(SAMPLE_PRODUCTS);
    }
    if (
      cmd === 'create_product' ||
      cmd === 'update_product' ||
      cmd === 'delete_product'
    ) {
      return Promise.resolve({ sku: 'LATTE' });
    }
    return Promise.resolve([]);
  }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockClear();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'list_products') {
      return Promise.resolve(SAMPLE_PRODUCTS);
    }
    if (
      cmd === 'create_product' ||
      cmd === 'update_product' ||
      cmd === 'delete_product'
    ) {
      return Promise.resolve({ sku: 'LATTE' });
    }
    return Promise.resolve([]);
  });
});

async function waitForTable() {
  await screen.findByRole('table', { name: /product catalog/i });
}

describe('ProductManagementScreen', () => {
  it('renders title and add button', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    expect(screen.getByRole('heading', { name: /products/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add product/i })).toBeInTheDocument();
  });

  it('renders product rows from IPC data', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    expect(screen.getByText('Caffè Latte')).toBeInTheDocument();
    expect(screen.getByText('Plain Bagel')).toBeInTheDocument();
    expect(screen.getByText('Fudge Brownie')).toBeInTheDocument();
  });

  it('renders column headers', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    expect(screen.getByText('SKU')).toBeInTheDocument();
    expect(screen.getByText('Name')).toBeInTheDocument();
    expect(screen.getByText('Category')).toBeInTheDocument();
    expect(screen.getByText('Price')).toBeInTheDocument();
    expect(screen.getByText('Stock')).toBeInTheDocument();
  });

  it('shows stock status', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    const inStock = screen.getAllByText(/in stock/i);
    expect(inStock.length).toBe(2);
    expect(screen.getByText(/out of stock/i)).toBeInTheDocument();
  });

  it('shows formatted prices', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    expect(screen.getByText('$4.50')).toBeInTheDocument();
    expect(screen.getByText('$2.50')).toBeInTheDocument();
  });

  it('shows barcode or dash', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    expect(screen.getByText('4901234567890')).toBeInTheDocument();
    const dashes = screen.getAllByText('\u2014');
    expect(dashes.length).toBeGreaterThanOrEqual(1);
  });

  it('opens add modal when clicking Add Product', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add product/i }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByRole('heading', { name: /add product/i })).toBeInTheDocument();
  });

  it('opens edit modal prefilled with product data', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    const editBtn = screen.getByRole('button', { name: /edit caffè latte/i });
    await userEvent.click(editBtn);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByDisplayValue('LATTE')).toBeInTheDocument();
    expect(within(dialog).getByDisplayValue('Caffè Latte')).toBeInTheDocument();
    expect(within(dialog).getByDisplayValue('450')).toBeInTheDocument();
  });

  it('disables SKU field when editing', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /edit caffè latte/i }));
    const skuInput = screen.getByDisplayValue('LATTE');
    expect(skuInput).toBeDisabled();
  });

  it('calls createProduct on save in add modal', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add product/i }));
    await userEvent.type(screen.getByPlaceholderText('e.g. LATTE'), 'NEWSKU');
    await userEvent.type(screen.getByPlaceholderText('e.g. Caffè Latte'), 'New Product');
    await userEvent.type(screen.getByPlaceholderText('450'), '999');
    await userEvent.click(screen.getByRole('button', { name: /create/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('create_product', expect.any(Object));
    });
  });

  it('calls updateProduct on save in edit modal', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /edit caffè latte/i }));
    const nameInput = screen.getByDisplayValue('Caffè Latte');
    await userEvent.clear(nameInput);
    await userEvent.type(nameInput, 'Latte Macchiato');
    await userEvent.click(screen.getByRole('button', { name: /update/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('update_product', expect.any(Object));
    });
  });

  it('calls deleteProduct on delete button click', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /delete caffè latte/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('delete_product', expect.objectContaining({
        args: { sku: 'LATTE' },
      }));
    });
  });

  it('shows empty state when no products', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_products') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(wrap(<ProductManagementScreen />));
    await waitFor(() => {
      expect(screen.getByText(/no products yet/i)).toBeInTheDocument();
    });
  });

  it('disables save when SKU or name is empty', async () => {
    render(wrap(<ProductManagementScreen />));
    await waitForTable();
    await userEvent.click(screen.getByRole('button', { name: /add product/i }));
    const createBtn = screen.getByRole('button', { name: /create/i });
    expect(createBtn).toBeDisabled();
  });
});
