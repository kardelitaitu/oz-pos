import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import inventoryFtl from '@/locales/inventory.ftl?raw';
import InventoryAdjustmentScreen from '@/features/inventory/InventoryAdjustmentScreen';

const wrap = (children: React.ReactNode) => withFluent(children, inventoryFtl);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const SAMPLE_PRODUCTS: any[] = [
  { sku: 'LATTE', name: 'Caffè Latte', price: { minor_units: 450, currency: 'USD' }, barcode: null, in_stock: true, stock_qty: 20, category: 'Beverages' },
  { sku: 'BAGEL', name: 'Plain Bagel', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567890', in_stock: true, stock_qty: 5, category: 'Food' },
  { sku: 'BROWNIE', name: 'Fudge Brownie', price: { minor_units: 295, currency: 'USD' }, barcode: null, in_stock: false, stock_qty: 0, category: 'Food' },
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
    expect(screen.getByText('$ 4,50')).toBeInTheDocument();
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
