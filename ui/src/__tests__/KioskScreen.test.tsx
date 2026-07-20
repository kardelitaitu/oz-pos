import { describe, expect, it, vi } from 'vitest';
import { act } from 'react';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluent, renderWithProviders } from '@/__tests__/test-utils/render';
import kioskFtl from '@/locales/kiosk.ftl?raw';

vi.mock('@/api/products', () => ({
  listProducts: vi.fn(),
  listCategories: vi.fn(),
}));

import KioskScreen from '@/features/kiosk/KioskScreen';

// KioskScreen uses useToast — must wrap in ToastProvider via renderWithProviders.
const renderKiosk = (ftl: string) => renderWithProviders(<KioskScreen />, ftl);
import { listProducts, listCategories } from '@/api/products';

const mockListProducts = listProducts as ReturnType<typeof vi.fn>;
const mockListCategories = listCategories as ReturnType<typeof vi.fn>;

const sampleProducts = [
  { sku: 'SKU-001', name: 'Indomie Goreng', category: 'cat-food', price: { minor_units: 3500, currency: 'IDR' }, barcode: '8991002100110', in_stock: true, stock_qty: 100, tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'SKU-002', name: 'Teh Botol Sosro', category: 'cat-drink', price: { minor_units: 5000, currency: 'IDR' }, barcode: '8991002100220', in_stock: true, stock_qty: 50, tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'SKU-003', name: 'Nasi Goreng Spesial', category: 'cat-food', price: { minor_units: 15000, currency: 'IDR' }, barcode: null, in_stock: true, stock_qty: 20, tax_rate_ids: [], created_at: '', price_updated_at: '' },
  { sku: 'SKU-004', name: 'Aqua 600ml', category: 'cat-drink', price: { minor_units: 3000, currency: 'IDR' }, barcode: '8991002100330', in_stock: true, stock_qty: 3, tax_rate_ids: [], created_at: '', price_updated_at: '' },
];

const sampleCategories = [
  { id: 'cat-food', name: 'Makanan', colour: '#e74c3c' },
  { id: 'cat-drink', name: 'Minuman', colour: '#3498db' },
];



describe('KioskScreen', () => {
  it('loads and displays products', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
    expect(screen.getByText('Nasi Goreng Spesial')).toBeInTheDocument();
    expect(screen.getByText('Aqua 600ml')).toBeInTheDocument();
  });

  it('renders category filter buttons', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('All')).toBeInTheDocument();
    });
    expect(screen.getByText('Makanan')).toBeInTheDocument();
    expect(screen.getByText('Minuman')).toBeInTheDocument();
  });

  it('filters products by category', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Minuman'));

    expect(screen.queryByText('Indomie Goreng')).not.toBeInTheDocument();
    expect(screen.queryByText('Nasi Goreng Spesial')).not.toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
    expect(screen.getByText('Aqua 600ml')).toBeInTheDocument();
  });

  it('shows all products when All category is selected', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Makanan'));
    expect(screen.queryByText('Teh Botol Sosro')).not.toBeInTheDocument();

    await userEvent.click(screen.getByText('All'));
    expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    expect(screen.getByText('Teh Botol Sosro')).toBeInTheDocument();
  });

  it('adds product to cart when clicked', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Indomie Goreng'));

    await waitFor(() => {
      expect(screen.getByText('Checkout')).toBeInTheDocument();
    });
  });

  it('shows cart items after adding products', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Indomie Goreng'));
    await userEvent.click(screen.getByText('Teh Botol Sosro'));

    await waitFor(() => {
      const indomieItems = screen.getAllByText(/Indomie/);
      expect(indomieItems.length).toBeGreaterThanOrEqual(2);
    });
    expect(screen.getAllByText(/Teh Botol/).length).toBeGreaterThanOrEqual(1);
  });

  it('shows attract screen after idle timeout', async () => {
    vi.useFakeTimers();
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await act(async () => {
      vi.advanceTimersByTime(61000);
    });
    vi.useRealTimers();

    expect(screen.getByText('Tap to start')).toBeInTheDocument();
  });

  it('exits attract screen on click', async () => {
    vi.useFakeTimers();
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await act(async () => {
      vi.advanceTimersByTime(61000);
    });
    vi.useRealTimers();

    expect(screen.getByText('Tap to start')).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(screen.getByText('Tap to start'));

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });
  });

  it('increments quantity when same product is added again', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    const productCard1 = screen.getAllByText('Indomie Goreng');
    await userEvent.click(productCard1[0]!);
    const productCard2 = screen.getAllByText('Indomie Goreng');
    await userEvent.click(productCard2[0]!);

    await waitFor(() => {
      expect(screen.getByText('Checkout')).toBeInTheDocument();
    });
  });

  it('opens checkout screen when Checkout is clicked', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Indomie Goreng'));

    await userEvent.click(screen.getByText('Checkout'));

    await waitFor(() => {
      expect(screen.getByText(/Checkout/)).toBeInTheDocument();
    });
  });

  it('shows low stock badge for products with stock <= 5', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText(/3 left/)).toBeInTheDocument();
    });
  });

  it('shows back button in checkout', async () => {
    mockListProducts.mockResolvedValue(sampleProducts);
    mockListCategories.mockResolvedValue(sampleCategories);
    await renderKiosk(kioskFtl);

    await waitFor(() => {
      expect(screen.getByText('Indomie Goreng')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('Indomie Goreng'));
    await userEvent.click(screen.getByText('Checkout'));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /back/i })).toBeInTheDocument();
    });
  });
});
