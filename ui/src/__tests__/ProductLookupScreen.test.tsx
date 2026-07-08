import { describe, expect, it, vi } from 'vitest';
import { screen, within, waitFor } from '@testing-library/react';
import { renderInAct } from '@/test-utils/renderInAct';
import userEvent from '@testing-library/user-event';
import { withFluent, withFluentLocale } from '@/locales/test-utils';
import productsFtl from '@/locales/products.ftl?raw';
import productsId from '@/locales/products.id.ftl?raw';

import { ToastProvider } from '@/frontend/shared/Toast';
import { ScannerError } from '@/api/hardware';
import * as bundlesApi from '@/api/bundles';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import type { Product } from '@/types/domain';

const wrap = (children: React.ReactNode) =>
  withFluent(<ToastProvider>{children}</ToastProvider>, productsFtl);

// ── Tests ────────────────────────────────────────────────────────

describe('ProductLookupScreen', () => {
  it('renders the search bar and barcode input', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    expect(screen.getByRole('searchbox', { name: /search for products/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/enter or scan a barcode/i)).toBeInTheDocument();
  });

  it('renders category filter chips after loading', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitFor(() => {
      expect(screen.getByRole('radiogroup', { name: /filter by category/i })).toBeInTheDocument();
    });
    // "All Categories", "Beverages", "Food" (after async fallback loads)
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: /all categories/i })).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: /^Cold Drinks$/ })).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: /^Food$/ })).toBeInTheDocument();
    });
  });

  it('renders all products in the grid by default', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    const list = await screen.findByRole('list', { name: /product search results/i });
    // 18 sample products
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(18);
  });

  async function waitForProducts() {
    // Wait for the async IPC fallback to load sample products.
    await screen.findByRole('list', { name: /product search results/i });
  }

  it('filters products by search query (name)', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search for products/i });
    await userEvent.type(search, 'Latte');
    const list = screen.getByRole('list', { name: /product search results/i });
    const items = within(list).getAllByRole('listitem');
    // "Caffè Latte", "Matcha Latte"
    expect(items.length).toBe(2);
    expect(screen.getByText('Caffè Latte')).toBeInTheDocument();
    expect(screen.getByText('Matcha Latte')).toBeInTheDocument();
  });

  it('filters products by search query (SKU)', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search for products/i });
    await userEvent.type(search, 'ESPR');
    const list = screen.getByRole('list', { name: /product search results/i });
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(1);
    expect(screen.getByText(/Espresso Shot/)).toBeInTheDocument();
  });

  it('filters products by search query (barcode)', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search for products/i });
    // Search for barcode "4901234567904" (Orange Juice)
    await userEvent.type(search, '7904');
    const list = screen.getByRole('list', { name: /product search results/i });
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(1);
    expect(screen.getByText(/Orange Juice/)).toBeInTheDocument();
  });

  it('shows empty state when no products match', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search for products/i });
    await userEvent.type(search, 'zzzznotfound');
    expect(screen.getByText(/no products found/i)).toBeInTheDocument();
  });

  it('filters by category using chip button', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const foodChip = screen.getByRole('radio', { name: /^Food$/ });
    await userEvent.click(foodChip);

    const list = screen.getByRole('list', { name: /product search results/i });
    const items = within(list).getAllByRole('listitem');
    // 5 food items (Plain Bagel, Sesame Bagel, Butter Croissant,
    // Chicken Sandwich, Veggie Sandwich)
    expect(items.length).toBe(5);
    // No beverage items
    expect(screen.queryByText('Caffè Latte')).not.toBeInTheDocument();
  });

  it('switching to "All Categories" shows all products', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    // First filter to Food
    await userEvent.click(screen.getByRole('radio', { name: /^Food$/ }));
    // Then back to All
    await userEvent.click(screen.getByRole('radio', { name: /all categories/i }));

    const list = screen.getByRole('list', { name: /product search results/i });
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(18);
  });

  it('renders product card with name, price, SKU, and stock indicator', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    // Check a specific product is rendered
    expect(screen.getByText('Caffè Latte')).toBeInTheDocument();
    expect(screen.getByText('$ 4,50')).toBeInTheDocument();
    expect(screen.getByText('LATTE')).toBeInTheDocument();
    expect(screen.getAllByText(/in stock/i).length).toBeGreaterThanOrEqual(1);
  });

  it('marks out-of-stock products with disabled style and disabled button', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    // Brownie is out of stock
    const brownie = screen.getByText('Fudge Brownie');
    expect(brownie).toBeInTheDocument();

    // Out-of-stock text
    const outOfStock = screen.getAllByText(/out of stock/i);
    expect(outOfStock.length).toBeGreaterThanOrEqual(2); // Brownie + Chocolate Muffin

    // All 18 cards have buttons (disabled for out-of-stock)
    const productButtons = screen.getAllByRole('button', { name: /sku:/i });
    expect(productButtons.length).toBe(18);

    // 2 buttons should be disabled (out of stock)
    const disabledBtns = productButtons.filter((btn) => btn.hasAttribute('disabled'));
    expect(disabledBtns.length).toBe(2);
  });

  it('calls onAddProduct when clicking the add button', async () => {
    const handler = vi.fn();
    await renderInAct(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const addBtn = screen.getByRole('button', { name: /Caffè Latte/i });
    await userEvent.click(addBtn);

    expect(handler).toHaveBeenCalledTimes(1);
    const product = handler.mock.calls[0]![0] as Product;
    expect(product.sku).toBe('LATTE');
    expect(product.name).toBe('Caffè Latte');
  });

  it('calls onAddProduct on Enter key for in-stock product card', async () => {
    const handler = vi.fn();
    await renderInAct(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const cardBtn = screen.getByRole('button', { name: /Caffè Latte/i });
    cardBtn.focus();
    await userEvent.keyboard('{Enter}');

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('handles barcode scan via Enter key in barcode input', async () => {
    const handler = vi.fn();
    await renderInAct(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/enter or scan a barcode/i);
    await userEvent.type(barcodeInput, '4901234567890{Enter}');

    expect(handler).toHaveBeenCalledTimes(1);
    const product = handler.mock.calls[0]![0] as Product;
    expect(product.sku).toBe('LATTE');
  });

  it('handles barcode scan via Scan button', async () => {
    const handler = vi.fn();
    await renderInAct(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/enter or scan a barcode/i);
    await userEvent.type(barcodeInput, '4901234567904');

    const scanBtn = screen.getByRole('button', { name: /submit the entered barcode/i });
    await userEvent.click(scanBtn);

    expect(handler).toHaveBeenCalledTimes(1);
    const product = handler.mock.calls[0]![0] as Product;
    expect(product.sku).toBe('JUICE-O');
  });

  it('does not call onAddProduct for unknown barcode', async () => {
    const handler = vi.fn();
    await renderInAct(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/enter or scan a barcode/i);
    await userEvent.type(barcodeInput, '0000000000000');
    await userEvent.keyboard('{Enter}');

    expect(handler).not.toHaveBeenCalled();
  });

  it('clears barcode input after scan', async () => {
    const handler = vi.fn();
    await renderInAct(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/enter or scan a barcode/i);
    await userEvent.type(barcodeInput, '4901234567890{Enter}');

    expect(barcodeInput).toHaveValue('');
  });

  it('renders product category badges', async () => {
    await renderInAct(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const badges = screen.getAllByText(/^Cold Drinks$|^Hot Drinks$|^Food$|^Snacks$/);
    expect(badges.length).toBeGreaterThanOrEqual(17);
  });

  it('silently swallows when lookupBundleBySku rejects with a ScannerError in the barcode scan catch block', async () => {
    const handler = vi.fn();
    await renderInAct(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    // Spy on lookupBundleBySku and make it reject with a ScannerError.
    // The barcode doesn't match any product's barcode, so the callback
    // falls through to the bundle lookup path, which rejects.
    vi.spyOn(bundlesApi, 'lookupBundleBySku').mockRejectedValueOnce(
      new ScannerError(
        'Scanner disconnected — check USB connection',
        ScannerError.codes.DISCONNECTED,
      ),
    );

    const barcodeInput = screen.getByLabelText(/enter or scan a barcode/i);
    await userEvent.type(barcodeInput, '0000000000000{Enter}');

    // The catch block silently swallows the ScannerError.
    // No product should have been added.
    expect(handler).not.toHaveBeenCalled();

    // Barcode input should still be cleared after the catch block.
    expect(barcodeInput).toHaveValue('');
  });
});

// ── Indonesian locale ────────────────────────────────────────────
//
// Migrated from the in-line english-only `wrap()` helper to use the
// new `withFluentLocale('id', ...)` so this single test exercises
// the per-test fresh FluentBundle path end-to-end through a real
// production component. If a future contributor accidentally:
//   • drops `useIsolating: false` from `withFluentLocale` (DOM markers
//     reappear and selector-based queries break), or
//   • mutates the shared `getBundle('id')` cache from the helper,
// the failure will trace back to this test rather than to the hot
// bundle-loader smoke test which only sees messages, not DOM.
describe('ProductLookupScreen — locale: id', () => {
  it('renders the Indonesian-translated search and barcode inputs', async () => {
    await renderInAct(
      withFluentLocale(
        'id',
        <ToastProvider><ProductLookupScreen /></ToastProvider>,
        productsId,
        sharedId,
      ),
    );
    // products.id.ftl resolves product-lookup-search-aria to:
    //   "Cari produk berdasarkan nama, SKU, atau barcode"
    await waitFor(() => {
      expect(screen.getByRole('searchbox', { name: /Cari produk/i })).toBeInTheDocument();
    });
    // products.id.ftl resolves product-lookup-barcode-aria to:
    //   "Masukkan atau pindai barcode"
    expect(screen.getByLabelText(/Masukkan atau pindai barcode/i)).toBeInTheDocument();
  });
});
