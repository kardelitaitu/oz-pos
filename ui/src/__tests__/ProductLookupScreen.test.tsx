import { describe, expect, it, vi } from 'vitest';
import { render, screen, within, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import type { Product } from '@/types/domain';

// ── Fluent wrapper (matches main.tsx + en-US.ftl) ───────────────

const LOCALE_STRINGS = [
  'product-lookup-title = Products',
  'product-lookup-search-placeholder = Search products…',
  'product-lookup-barcode-placeholder = Scan barcode…',
  'product-lookup-barcode-scan = Scan',
  'product-lookup-no-results = No products found',
  'product-lookup-loading = Loading products…',
  'product-lookup-add = Add to cart',
  'product-lookup-in-stock = In stock',
  'product-lookup-out-of-stock = Out of stock',
  'product-lookup-all-categories = All Categories',
].join('\n');

const wrap = (children: React.ReactNode) => {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(LOCALE_STRINGS));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

// ── Tests ────────────────────────────────────────────────────────

describe('ProductLookupScreen', () => {
  it('renders the search bar and barcode input', () => {
    render(wrap(<ProductLookupScreen />));
    expect(screen.getByRole('searchbox', { name: /search products/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/barcode input/i)).toBeInTheDocument();
  });

  it('renders category filter chips after loading', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitFor(() => {
      expect(screen.getByRole('radiogroup', { name: /filter by category/i })).toBeInTheDocument();
    });
    // "All Categories", "Beverages", "Food" (after async fallback loads)
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: /all categories/i })).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: /^Beverages$/ })).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole('radio', { name: /^Food$/ })).toBeInTheDocument();
    });
  });

  it('renders all products in the grid by default', async () => {
    render(wrap(<ProductLookupScreen />));
    const list = await screen.findByRole('list', { name: /products/i });
    // 18 sample products
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(18);
  });

  async function waitForProducts() {
    // Wait for the async IPC fallback to load sample products.
    await screen.findByRole('list', { name: /products/i });
  }

  it('filters products by search query (name)', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search products/i });
    await userEvent.type(search, 'Latte');
    const list = screen.getByRole('list', { name: /products/i });
    const items = within(list).getAllByRole('listitem');
    // "Caffè Latte", "Matcha Latte"
    expect(items.length).toBe(2);
    expect(screen.getByText('Caffè Latte')).toBeInTheDocument();
    expect(screen.getByText('Matcha Latte')).toBeInTheDocument();
  });

  it('filters products by search query (SKU)', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search products/i });
    await userEvent.type(search, 'ESPR');
    const list = screen.getByRole('list', { name: /products/i });
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(1);
    expect(screen.getByText(/Espresso Shot/)).toBeInTheDocument();
  });

  it('filters products by search query (barcode)', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search products/i });
    // Search for barcode "4901234567904" (Orange Juice)
    await userEvent.type(search, '7904');
    const list = screen.getByRole('list', { name: /products/i });
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(1);
    expect(screen.getByText(/Orange Juice/)).toBeInTheDocument();
  });

  it('shows empty state when no products match', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const search = screen.getByRole('searchbox', { name: /search products/i });
    await userEvent.type(search, 'zzzznotfound');
    expect(screen.getByText(/no products found/i)).toBeInTheDocument();
  });

  it('filters by category using chip button', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const foodChip = screen.getByRole('radio', { name: /^Food$/ });
    await userEvent.click(foodChip);

    const list = screen.getByRole('list', { name: /products/i });
    const items = within(list).getAllByRole('listitem');
    // 10 food items (Bagel, Bagel-S, Croissant, Blueberry Muffin, Chocolate Muffin,
    // Chicken Sandwich, Veggie Sandwich, Cookie, Brownie, Banana Muffin)
    expect(items.length).toBe(10);
    // No beverage items
    expect(screen.queryByText('Caffè Latte')).not.toBeInTheDocument();
  });

  it('switching to "All Categories" shows all products', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    // First filter to Food
    await userEvent.click(screen.getByRole('radio', { name: /^Food$/ }));
    // Then back to All
    await userEvent.click(screen.getByRole('radio', { name: /all categories/i }));

    const list = screen.getByRole('list', { name: /products/i });
    const items = within(list).getAllByRole('listitem');
    expect(items.length).toBe(18);
  });

  it('renders product card with name, price, SKU, and stock indicator', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    // Check a specific product is rendered
    expect(screen.getByText('Caffè Latte')).toBeInTheDocument();
    expect(screen.getByText('$4.50')).toBeInTheDocument();
    expect(screen.getByText('LATTE')).toBeInTheDocument();
    expect(screen.getAllByText(/in stock/i).length).toBeGreaterThanOrEqual(1);
  });

  it('marks out-of-stock products with disabled style and disabled button', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    // Brownie is out of stock
    const brownie = screen.getByText('Fudge Brownie');
    expect(brownie).toBeInTheDocument();

    // Out-of-stock text
    const outOfStock = screen.getAllByText(/out of stock/i);
    expect(outOfStock.length).toBeGreaterThanOrEqual(2); // Brownie + Chocolate Muffin

    // All 18 cards have buttons (disabled for out-of-stock)
    const productButtons = screen.getAllByRole('button', { name: /—/ });
    expect(productButtons.length).toBe(18);

    // 2 buttons should be disabled (out of stock)
    const disabledBtns = productButtons.filter((btn) => btn.hasAttribute('disabled'));
    expect(disabledBtns.length).toBe(2);
  });

  it('calls onAddProduct when clicking the add button', async () => {
    const handler = vi.fn();
    render(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const addBtn = screen.getByRole('button', { name: /caffè latte —/i });
    await userEvent.click(addBtn);

    expect(handler).toHaveBeenCalledTimes(1);
    const product = handler.mock.calls[0]![0] as Product;
    expect(product.sku).toBe('LATTE');
    expect(product.name).toBe('Caffè Latte');
  });

  it('calls onAddProduct on Enter key for in-stock product card', async () => {
    const handler = vi.fn();
    render(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const cardBtn = screen.getByRole('button', { name: /caffè latte —/i });
    cardBtn.focus();
    await userEvent.keyboard('{Enter}');

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('handles barcode scan via Enter key in barcode input', async () => {
    const handler = vi.fn();
    render(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/barcode input/i);
    await userEvent.type(barcodeInput, '4901234567890{Enter}');

    expect(handler).toHaveBeenCalledTimes(1);
    const product = handler.mock.calls[0]![0] as Product;
    expect(product.sku).toBe('LATTE');
  });

  it('handles barcode scan via Scan button', async () => {
    const handler = vi.fn();
    render(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/barcode input/i);
    await userEvent.type(barcodeInput, '4901234567904');

    const scanBtn = screen.getByRole('button', { name: /submit barcode/i });
    await userEvent.click(scanBtn);

    expect(handler).toHaveBeenCalledTimes(1);
    const product = handler.mock.calls[0]![0] as Product;
    expect(product.sku).toBe('JUICE-O');
  });

  it('does not call onAddProduct for unknown barcode', async () => {
    const handler = vi.fn();
    render(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/barcode input/i);
    await userEvent.type(barcodeInput, '0000000000000');
    await userEvent.keyboard('{Enter}');

    expect(handler).not.toHaveBeenCalled();
  });

  it('clears barcode input after scan', async () => {
    const handler = vi.fn();
    render(wrap(<ProductLookupScreen onAddProduct={handler} />));
    await waitForProducts();

    const barcodeInput = screen.getByLabelText(/barcode input/i);
    await userEvent.type(barcodeInput, '4901234567890{Enter}');

    expect(barcodeInput).toHaveValue('');
  });

  it('renders product category badges', async () => {
    render(wrap(<ProductLookupScreen />));
    await waitForProducts();
    const badges = screen.getAllByText(/^Beverages$|^Food$/);
    expect(badges.length).toBeGreaterThanOrEqual(17);
  });
});
