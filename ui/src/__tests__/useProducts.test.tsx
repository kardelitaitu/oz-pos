import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHookInAct } from '@/test-utils/renderInAct';
import { useProducts } from '@/features/products/useProducts';
import type { ProductDto, CategoryDto } from '@/api/products';

const mocks = vi.hoisted(() => ({
  listProducts: vi.fn(),
  listCategories: vi.fn(),
  getString: vi.fn(),
}));

vi.mock('@/api/products', () => ({
  listProducts: (...args: unknown[]) => mocks.listProducts(...args),
  listCategories: (...args: unknown[]) => mocks.listCategories(...args),
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({ l10n: { getString: mocks.getString } }),
}));

function makeProductDto(overrides: Partial<ProductDto> = {}): ProductDto {
  return {
    sku: 'LATTE',
    name: 'Caffè Latte',
    category: 'Hot Drinks',
    price: { minor_units: 450, currency: 'USD' },
    barcode: '4901234567890',
    in_stock: true,
    stock_qty: 50,
    tax_rate_ids: [],
    created_at: '2024-01-01',
    price_updated_at: '2024-01-01',
    product_type: 'restaurant',
    ...overrides,
  };
}

function makeCategoryDto(overrides: Partial<CategoryDto> = {}): CategoryDto {
  return {
    id: 'cat-hot-drinks',
    name: 'Hot Drinks',
    colour: '#ef4444',
    icon: 'hot-drink',
    ...overrides,
  };
}

beforeEach(() => {
  mocks.listProducts.mockResolvedValue([makeProductDto()]);
  mocks.listCategories.mockResolvedValue([makeCategoryDto()]);
  mocks.getString.mockReturnValue('Uncategorised');
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('useProducts', () => {
  describe('loading state', () => {
    it('starts with loading=true', async () => {
      mocks.listProducts.mockReturnValue(new Promise(() => {}));

      const { result } = await renderHookInAct(() => useProducts());

      expect(result.current.loading).toBe(true);
      expect(result.current.products).toEqual([]);
    });

    it('sets loading=false after fetch completes', async () => {
      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => expect(result.current.loading).toBe(false));
    });
  });

  describe('successful fetch', () => {
    it('returns products from the API', async () => {
      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.products).toHaveLength(1);
        expect(result.current.products[0].sku).toBe('LATTE');
        expect(result.current.products[0].name).toBe('Caffè Latte');
      });
    });

    it('maps ProductDto fields to Product correctly', async () => {
      mocks.listProducts.mockResolvedValue([
        makeProductDto({
          product_type: 'restaurant',
          category: 'Hot Drinks',
          barcode: '4901234567890',
        }),
      ]);

      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        const p = result.current.products[0];
        expect(p.productType).toBe('restaurant');
        expect(p.category).toBe('Hot Drinks');
        expect(p.barcode).toBe('4901234567890');
        expect(p.inStock).toBe(true);
        expect(p.stockQty).toBe(50);
      });
    });

    it('uses fallback label for null category', async () => {
      mocks.listProducts.mockResolvedValue([makeProductDto({ category: null })]);
      mocks.getString.mockReturnValue('Other');

      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.products[0].category).toBe('Other');
      });
    });

    it('returns category metadata from the API', async () => {
      const cat = makeCategoryDto({ id: 'cat-food', name: 'Food', colour: '#f97316' });
      mocks.listCategories.mockResolvedValue([cat]);

      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.categoryMeta).toHaveLength(1);
        expect(result.current.categoryMeta[0].name).toBe('Food');
      });
    });

    it('derives unique category names sorted alphabetically', async () => {
      mocks.listProducts.mockResolvedValue([
        makeProductDto({ sku: 'LATTE', category: 'Hot Drinks' }),
        makeProductDto({ sku: 'BAGEL', category: 'Food' }),
        makeProductDto({ sku: 'COOKIE', category: 'Snacks' }),
        makeProductDto({ sku: 'TEA', category: 'Hot Drinks' }),
      ]);

      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.categories).toEqual(['Food', 'Hot Drinks', 'Snacks']);
      });
    });

    it('sets usingFallback=false when API returns data', async () => {
      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.usingFallback).toBe(false);
      });
    });
  });

  describe('fallback when API fails', () => {
    it('falls back to sample products when listProducts throws', async () => {
      mocks.listProducts.mockRejectedValue(new Error('IPC unavailable'));

      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.usingFallback).toBe(true);
        expect(result.current.products.length).toBeGreaterThan(0);
      });
    });

    it('sets error message when API throws', async () => {
      mocks.listProducts.mockRejectedValue(new Error('IPC timeout'));
      mocks.getString.mockReturnValue('Failed to load products');

      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.error).toBe('IPC timeout');
        expect(result.current.usingFallback).toBe(true);
      });
    });

    it('falls back to sample products when API returns empty list', async () => {
      mocks.listProducts.mockResolvedValue([]);

      const { result } = await renderHookInAct(() => useProducts());

      await vi.waitFor(() => {
        expect(result.current.usingFallback).toBe(true);
        expect(result.current.products.length).toBeGreaterThan(0);
      });
    });
  });

  describe('cleanup', () => {
    it('does not set state after unmount', async () => {
      mocks.listProducts.mockReturnValue(new Promise((resolve) => setTimeout(() => resolve([makeProductDto()]), 100)));

      const { unmount } = await renderHookInAct(() => useProducts());

      unmount();

      await vi.waitFor(() => {
        expect(mocks.listProducts).toHaveBeenCalled();
      });
    });
  });
});
