// ── PosScreen bundle-scan toast tests ─────────────────────────────
//
// These tests verify that when a barcode is scanned:
// 1. A known bundle SKU → success toast with bundle name and item count
// 2. An unknown code → warning toast about unrecognised barcode
// 3. Expanded items appear in the cart

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { ToastProvider } from '@/components/Toast';
import { withFluent } from '@/locales/test-utils';
import { ScannerError } from '@/api/hardware';
import salesFtl from '@/locales/sales.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import sharedFtl from '@/locales/shared.ftl?raw';
import inventoryFtl from '@/locales/inventory.ftl?raw';
import PosScreen from '@/features/sales/PosScreen';
import * as productsApi from '@/api/products';
import * as bundlesApi from '@/api/bundles';
import type { BarcodeScannedPayload } from '@/api/hardware';

// ── Hoisted mock helpers ──────────────────────────────────────────
// vi.mock is hoisted, so any mutable state must be declared via
// vi.hoisted() so it can be referenced inside the mock factories.

// Mock useBarcodeScanner to capture the onProductFound callback
// instead of doing its own lookupByBarcode (which would bypass
// PosScreen's bundle expansion logic — the hook only calls
// onProductFound when its own barcode lookup succeeds).
const mockedBarcode = vi.hoisted(() => {
  let onProductFound: ((payload: BarcodeScannedPayload) => void) | null = null;
  let onError: ((error: string) => void) | null = null;
  return {
    triggerScan(code: string) {
      onProductFound?.({ code, symbology: 'test' });
    },
    triggerError(error: string) {
      onError?.(error);
    },
    reset() {
      onProductFound = null;
      onError = null;
    },
    useBarcodeScanner: vi.fn(
      (opts: {
        onProductFound: (p: BarcodeScannedPayload) => void;
        onError?: (error: string) => void;
      }) => {
        onProductFound = opts.onProductFound;
        onError = opts.onError ?? null;
      },
    ),
  };
});

// Mock hardware functions used by useCustomerDisplay.
vi.mock('@/api/hardware', async () => {
  const actual = await vi.importActual<typeof import('@/api/hardware')>('@/api/hardware');
  return {
    ...actual,
    listDisplays: vi.fn(() => Promise.resolve([])),
    displayShow: vi.fn(() => Promise.resolve()),
    displayClear: vi.fn(() => Promise.resolve()),
  };
});

vi.mock('@/features/sales/useBarcodeScanner', () => ({
  useBarcodeScanner: mockedBarcode.useBarcodeScanner,
}));

vi.mock('@/api/products', () => ({
  lookupByBarcode: vi.fn(() => Promise.resolve(null)),
  lookupProductBySku: vi.fn((sku: string) => {
    const products: Record<string, unknown> = {
      'ITEM-001': {
        sku: 'ITEM-001',
        name: 'Item 1',
        category: 'Test',
        price: { minor_units: 400, currency: 'USD' },
        barcode: null,
        in_stock: true,
        stock_qty: 100,
        tax_rate_ids: [],
      },
      'ITEM-002': {
        sku: 'ITEM-002',
        name: 'Item 2',
        category: 'Test',
        price: { minor_units: 200, currency: 'USD' },
        barcode: null,
        in_stock: true,
        stock_qty: 50,
        tax_rate_ids: [],
      },
    };
    return Promise.resolve(products[sku] ?? null);
  }),
  listProducts: vi.fn(() => Promise.resolve([])),
  listCategories: vi.fn(() => Promise.resolve([])),
  createProduct: vi.fn(),
  updateProduct: vi.fn(),
  deleteProduct: vi.fn(),
  adjustStock: vi.fn(),
  listProductVariants: vi.fn(() => Promise.resolve([])),
  getProductVariant: vi.fn(() => Promise.resolve(null)),
  createProductVariant: vi.fn(),
  updateProductVariant: vi.fn(),
  deleteProductVariant: vi.fn(),
  createCategory: vi.fn(),
  deleteCategory: vi.fn(),
}));

vi.mock('@/api/bundles', () => ({
  lookupBundleBySku: vi.fn((sku: string) => {
    if (sku === 'BUNDLE-SKU-001') {
      return Promise.resolve({
        bundle: {
          id: 'bundle-1',
          bundle_sku: 'BUNDLE-SKU-001',
          name: 'Test Bundle',
          description: 'A test bundle',
          bundle_price_minor: 500,
          currency: 'USD',
          active: true,
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
        items: [
          { id: 'item-1', bundle_id: 'bundle-1', sku: 'ITEM-001', qty: 1, unit_price_minor: null },
          { id: 'item-2', bundle_id: 'bundle-1', sku: 'ITEM-002', qty: 2, unit_price_minor: null },
        ],
      });
    }
    if (sku === 'INACTIVE-SKU') {
      return Promise.resolve({
        bundle: {
          id: 'bundle-inactive',
          bundle_sku: 'INACTIVE-SKU',
          name: 'Retired Bundle',
          description: 'No longer sold',
          bundle_price_minor: null,
          currency: 'USD',
          active: false,
          created_at: '2025-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
        items: [
          { id: 'item-z', bundle_id: 'bundle-inactive', sku: 'ITEM-001', qty: 1, unit_price_minor: null },
        ],
      });
    }
    return Promise.resolve(null);
  }),
  listBundles: vi.fn(() => Promise.resolve([])),
  getBundle: vi.fn(() => Promise.resolve(null)),
  createBundle: vi.fn(),
  updateBundle: vi.fn(),
  deleteBundle: vi.fn(),
}));

vi.mock('@/api/shifts', () => ({
  getActiveShift: vi.fn(() => Promise.reject(new Error('no shift'))),
  openShift: vi.fn(),
  closeShift: vi.fn(),
}));

// Mock sales API to prevent unhandled promise rejections from
// loadHeldCarts being called on mount.
vi.mock('@/api/sales', () => ({
  holdCart: vi.fn(),
  listHeldCarts: vi.fn(() => Promise.resolve([])),
  getHeldCart: vi.fn(),
  deleteHeldCart: vi.fn(),
  startSale: vi.fn(),
  addLine: vi.fn(),
  completeSale: vi.fn(),
  setCartDiscount: vi.fn(),
  listSales: vi.fn(() => Promise.resolve([])),
  getSale: vi.fn(),
  voidSale: vi.fn(),
  processRefund: vi.fn(),
  listRefunds: vi.fn(() => Promise.resolve([])),
  exportDailySummary: vi.fn(() => Promise.resolve([])),
  exportSalesByHour: vi.fn(() => Promise.resolve([])),
  exportEodReport: vi.fn(),
  printSalesReceipt: vi.fn(),
  onReceiptPrinted: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: {
      user_id: 'user-1',
      username: 'testuser',
      role_name: 'cashier',
      token: 'mock-token',
    },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager: false,
    isOwner: false,
  }),
}));

// ── Test wrapper ──────────────────────────────────────────────────

function wrap(children: React.ReactNode) {
  return withFluent(<ToastProvider>{children}</ToastProvider>, salesFtl, productsFtl, sharedFtl, inventoryFtl);
}

// ── Tests ─────────────────────────────────────────────────────────

describe('PosScreen – bundle scanning toast', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedBarcode.reset();
  });

  it('shows a success toast when a bundle barcode is scanned', async () => {
    render(wrap(<PosScreen />));

    // Wait for component to mount and register the barcode callback.
    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Simulate scanning a bundle SKU barcode.
    mockedBarcode.triggerScan('BUNDLE-SKU-001');

    // The success toast should appear with bundle name and item count.
    // Fluent wraps interpolated variables in Unicode formatting markers.
    const toast = await screen.findByRole('alert');
    expect(toast.textContent).toContain('Bundle');
    expect(toast.textContent).toContain('items');
  });

  it('shows a warning toast when an unknown barcode is scanned', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Simulate scanning an unrecognised code.
    mockedBarcode.triggerScan('UNKNOWN-CODE');

    // A warning toast should appear.
    expect(
      await screen.findByText(/No product or bundle matches this barcode/),
    ).toBeInTheDocument();
  });

  it('includes the expanded items in the cart when a bundle is scanned', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Simulate scanning a bundle SKU.
    mockedBarcode.triggerScan('BUNDLE-SKU-001');

    // Wait for the toast to confirm expansion.
    // Fluent wraps interpolated variables in Unicode formatting markers.
    const toast = await screen.findByRole('alert');
    expect(toast.textContent).toContain('Bundle');
    expect(toast.textContent).toContain('items');

    // Cart should show the expanded items by SKU.
    // The CartLineItem component renders SKU in .pos-cart-line-sku.
    expect(screen.getByText(/ITEM-001/)).toBeInTheDocument();
    expect(screen.getByText(/ITEM-002/)).toBeInTheDocument();
  });

  it('silently swallows when lookupByBarcode rejects (catch block)', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Make lookupByBarcode throw on the next call.
    vi.mocked(productsApi.lookupByBarcode).mockRejectedValueOnce(
      new Error('USB disconnect'),
    );

    // Simulate a barcode scan. Since lookupByBarcode rejects, the
    // catch block fires and silently ignores the error.
    mockedBarcode.triggerScan('ANY-CODE');

    // Wait for the error to propagate through the catch block.
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupByBarcode)).toHaveBeenCalledWith('ANY-CODE');
    });

    // No success or warning toast should appear.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
    expect(screen.queryByText(/No product or bundle/)).not.toBeInTheDocument();

    // Cart should remain empty.
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

  it('silently swallows when lookupByBarcode rejects with a ScannerError (typed error)', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Make lookupByBarcode throw a ScannerError (scanner hardware failure).
    vi.mocked(productsApi.lookupByBarcode).mockRejectedValueOnce(
      new ScannerError(
        'Scanner disconnected — check USB connection',
        ScannerError.codes.DISCONNECTED,
      ),
    );

    // Simulate a barcode scan. The catch block handles the typed error
    // the same way as a generic Error — silently ignored.
    mockedBarcode.triggerScan('ANY-CODE');

    // Wait for the error to propagate through the catch block.
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupByBarcode)).toHaveBeenCalledWith('ANY-CODE');
    });

    // No success or warning toast should appear.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
    expect(screen.queryByText(/No product or bundle/)).not.toBeInTheDocument();

    // Cart should remain empty.
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

  it('silently swallows when lookupBundleBySku rejects (catch block)', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // lookupByBarcode returns null (as mocked by default), so the
    // callback falls through to lookupBundleBySku. Make it reject.
    vi.mocked(bundlesApi.lookupBundleBySku).mockRejectedValueOnce(
      new Error('Backend unreachable'),
    );

    // Simulate scanning a bundle SKU. The rejection occurs after
    // lookupByBarcode returns null, and is swallowed by the catch block.
    mockedBarcode.triggerScan('BUNDLE-SKU-001');

    // Wait for the bundle lookup to be called (confirms we got past
    // the barcode lookup and into the bundle path before the error).
    await waitFor(() => {
      expect(vi.mocked(bundlesApi.lookupBundleBySku)).toHaveBeenCalledWith('BUNDLE-SKU-001');
    });

    // No toast should appear — the catch block swallowed the error.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
    expect(screen.queryByText(/No product or bundle/)).not.toBeInTheDocument();

    // Cart should remain empty.
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

  it('silently swallows when lookupBundleBySku rejects with a ScannerError (typed error)', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // lookupByBarcode returns null (default), so the callback falls
    // through to lookupBundleBySku. Make it reject with a ScannerError.
    vi.mocked(bundlesApi.lookupBundleBySku).mockRejectedValueOnce(
      new ScannerError(
        'Scanner conflict — device in use by another terminal',
        ScannerError.codes.CONFLICT,
      ),
    );

    // Simulate scanning a bundle SKU. The catch block handles the
    // typed error the same way as a generic Error — silently ignored.
    mockedBarcode.triggerScan('BUNDLE-SKU-001');

    // Wait for the bundle lookup to be called (confirms we got past
    // the barcode lookup and into the bundle path before the error).
    await waitFor(() => {
      expect(vi.mocked(bundlesApi.lookupBundleBySku)).toHaveBeenCalledWith('BUNDLE-SKU-001');
    });

    // No toast should appear — the catch block swallowed the error.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
    expect(screen.queryByText(/No product or bundle/)).not.toBeInTheDocument();

    // Cart should remain empty.
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

  it('silently swallows when expandBundleItems rejects inside the callback (catch block)', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // lookupByBarcode returns null (default), lookupBundleBySku succeeds.
    // But make lookupProductBySku reject — this is used as the lookupItem
    // callback inside expandBundleItems, so the bundle expansion will fail.
    vi.mocked(productsApi.lookupProductBySku).mockRejectedValueOnce(
      new Error('SKU lookup timeout'),
    );

    // Simulate scanning a bundle SKU. The callback reaches the bundle
    // expansion path, but expandBundleItems rejects because the underlying
    // product lookup fails. The catch block swallows it.
    mockedBarcode.triggerScan('BUNDLE-SKU-001');

    // Wait for lookupProductBySku to be called (confirms expansion started).
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupProductBySku)).toHaveBeenCalled();
    });

    // No toast should appear — the catch block swallowed the error.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
    expect(screen.queryByText(/No product or bundle/)).not.toBeInTheDocument();

    // Cart should remain empty — no items were added.
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

  it('silently swallows when expandBundleItems rejects with a ScannerError via lookupProductBySku (typed error)', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // lookupByBarcode returns null (default), lookupBundleBySku succeeds.
    // But make lookupProductBySku reject with a ScannerError — this is
    // used as the lookupItem callback inside expandBundleItems.
    vi.mocked(productsApi.lookupProductBySku).mockRejectedValueOnce(
      new ScannerError(
        'Scanner hardware failure — USB error',
        ScannerError.codes.HARDWARE_FAILURE,
      ),
    );

    // Simulate scanning a bundle SKU. The callback reaches the bundle
    // expansion path, but expandBundleItems rejects because the
    // ScannerError propagates up from the product lookup.
    mockedBarcode.triggerScan('BUNDLE-SKU-001');

    // Wait for lookupProductBySku to be called (confirms expansion started).
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupProductBySku)).toHaveBeenCalled();
    });

    // No toast should appear — the catch block swallowed the error.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
    expect(screen.queryByText(/No product or bundle/)).not.toBeInTheDocument();

    // Cart should remain empty — no items were added.
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

  it('adds product directly when lookupByBarcode returns a DTO (no bundle path)', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Make lookupByBarcode return a product DTO for this scan.
    vi.mocked(productsApi.lookupByBarcode).mockResolvedValueOnce({
      sku: 'LATTE',
      name: 'Caffè Latte',
      category: 'Beverages',
      price: { minor_units: 450, currency: 'USD' },
      barcode: '4901234567890',
      in_stock: true,
      stock_qty: 25,
      tax_rate_ids: [],
    });

    // Simulate scanning a barcode that matches a product (not a bundle).
    mockedBarcode.triggerScan('4901234567890');

    // Wait for lookupByBarcode to be called with the scanned code.
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupByBarcode)).toHaveBeenCalledWith('4901234567890');
    });

    // The product name appears in both the product grid and the cart.
    expect(screen.getAllByText('Caffè Latte').length).toBeGreaterThanOrEqual(1);

    // Cart should no longer be empty (product was added).
    expect(screen.queryByText(/Cart is empty/)).not.toBeInTheDocument();

    // The subtotal area should appear, confirming the product is in the cart.
    expect(screen.getByText(/Subtotal/)).toBeInTheDocument();

    // lookupBundleBySku should NOT have been called (early return).
    expect(bundlesApi.lookupBundleBySku).not.toHaveBeenCalled();

    // No bundle toast — this was a direct product add.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
  });

  it('gives product barcode priority when the same code matches both a product and a bundle', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // lookupByBarcode returns a product DTO for the scanned code.
    // Note: we deliberately do NOT set mockResolvedValueOnce on
    // lookupBundleBySku — it should never be called because the
    // product path returns early before the bundle path is reached.
    vi.mocked(productsApi.lookupByBarcode).mockResolvedValueOnce({
      sku: 'ESPRESSO',
      name: 'Espresso Shot',
      category: 'Beverages',
      price: { minor_units: 250, currency: 'USD' },
      barcode: '4900000000000',
      in_stock: true,
      stock_qty: 50,
      tax_rate_ids: [],
    });

    // Scan a code that matches both a product barcode and a bundle SKU.
    mockedBarcode.triggerScan('4900000000000');

    // Wait for the product lookup to be called.
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupByBarcode)).toHaveBeenCalledWith('4900000000000');
    });

    // The product should appear in the cart (product path won).
    expect(screen.getAllByText('Espresso Shot').length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText(/Cart is empty/)).not.toBeInTheDocument();

    // The bundle lookup should NOT have been called — the product path
    // returns early via `return` before falling through to the bundle path.
    expect(bundlesApi.lookupBundleBySku).not.toHaveBeenCalled();

    // No bundle toast.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
  });

  it('shows a warning toast when a bundle is found but inactive', async () => {
    render(wrap(<PosScreen />));

    // Wait for the barcode scanner callback to be registered.
    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Scan an SKU the mock factory recognizes as an inactive bundle.
    // lookupByBarcode returns null (default) → falls through to bundle
    // lookup → lookupBundleBySku returns a bundle with active=false.
    mockedBarcode.triggerScan('INACTIVE-SKU');

    // The warning toast should appear (the else branch fires because
    // bundle is truthy but bundle.bundle.active is false).
    expect(
      await screen.findByText(/No product or bundle matches this barcode/),
    ).toBeInTheDocument();

    // No success toast since the bundle was not expanded.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();

    // Cart should remain empty since no items were added.
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

  it('shows an error toast when the barcode scanner emits a hardware error', async () => {
    render(wrap(<PosScreen />));

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Simulate a scanner hardware error event (e.g. USB disconnect).
    mockedBarcode.triggerError('Scanner disconnected — check USB connection');

    // The toast message is resolved via Fluent, which wraps variable values
    // in Unicode isolate characters (⁨...⁩). Match just the error detail
    // (which appears after the isolate char) rather than the full prefix.
    expect(
      await screen.findByText(/Scanner disconnected/),
    ).toBeInTheDocument();
  });

  it('renders normally when the barcode scanner is offline / fails to initialize', async () => {
    // Override the hook to simulate scanner hardware failure.
    // The hook is called during render but doesn't register a callback
    // (simulating that the scanner device never connected).
    mockedBarcode.useBarcodeScanner.mockImplementationOnce(() => {
      // Scanner offline — no callback captured.
    });

    render(wrap(<PosScreen />));

    // PosScreen should render without crashing — cart shows empty state.
    await waitFor(() => {
      expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
    });

    // The product lookup screen should be visible and functional.
    // (ProductLookupScreen renders search and barcode inputs.)
    expect(screen.getByPlaceholderText(/Search products/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/Scan barcode/)).toBeInTheDocument();

    // The hook was still called — component lifecycle wasn't broken by
    // the scanner failure.
    expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
  });
});
