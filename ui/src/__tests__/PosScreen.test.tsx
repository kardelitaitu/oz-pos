// ── PosScreen bundle-scan toast tests ─────────────────────────────
//
// These tests verify that when a barcode is scanned:
// 1. A known bundle SKU → success toast with bundle name and item count
// 2. An unknown code → warning toast about unrecognised barcode
// 3. Expanded items appear in the cart

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { act } from 'react';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
import { ScannerError } from '@/api/hardware';
import type * as HardwareModule from '@/api/hardware';
import salesFtl from '@/locales/sales.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import inventoryFtl from '@/locales/inventory.ftl?raw';
import settingsFtl from '@/locales/settings.ftl?raw';
import PosScreen from '@/features/sales/PosScreen';
import * as productsApi from '@/api/products';
import * as bundlesApi from '@/api/bundles';
import type { BarcodeScannedPayload } from '@/api/hardware';
import {
  createAuthContextMock,
  createWorkspaceContextMock,
} from '@/__tests__/test-utils/mocks/contexts';
import {
  createSalesApiMock,
  createSettingsApiMock,
  createShiftsApiMock,
} from '@/__tests__/test-utils/mocks/api';

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
  const actual = await vi.importActual<typeof HardwareModule>('@/api/hardware');
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

vi.mock('@/api/shifts', () => createShiftsApiMock());

vi.mock('@/api/settings', () => createSettingsApiMock());

// Mock sales API to prevent unhandled promise rejections from
// loadHeldCarts being called on mount.
vi.mock('@/api/sales', () => createSalesApiMock());

// Mock interaction utils so jsdom's non-Promise play() doesn't
// trigger a TypeError inside triggerInteraction (the catch {}
// in PosScreen's onProductFound would swallow the entire scan
// handler before addProduct/addToast can execute).
vi.mock('@/utils/interaction', () => ({
  triggerInteraction: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: createAuthContextMock({ displayName: undefined }),
}));

vi.mock('@/contexts/WorkspaceContext', createWorkspaceContextMock);

// Sub-screen stubs for the Settings 4-tab-routing pattern.
// Kept minimal so the parent-only assertions stay focused.
vi.mock('@/features/settings/AppearanceSettings', () => ({
  AppearanceSettings: () => <div data-testid="mock-appearance">Appearance Settings Stub</div>,
}));
vi.mock('@/features/settings/FeatureToggleScreen', () => ({
  default: () => <div data-testid="mock-features">Feature Toggles Stub</div>,
}));
vi.mock('@/features/settings/DataManagementScreen', () => ({
  default: () => <div data-testid="mock-data">Data Management Stub</div>,
}));

// ── Tests ─────────────────────────────────────────────────────────

describe('PosScreen – bundle scanning toast', () => {
  beforeEach(() => {
    mockedBarcode.reset();
  });

  it('shows a success toast when a bundle barcode is scanned', async () => {
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    // Wait for component to mount, register the barcode callback, and
    // load an active shift (the onProductFound callback checks for it).
    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Simulate scanning a bundle SKU barcode.
    await act(async () => { mockedBarcode.triggerScan('BUNDLE-SKU-001'); });

    // The success toast should appear with bundle name and item count.
    // Fluent wraps interpolated variables in Unicode formatting markers.
    const toast = await screen.findByRole('alert');
    expect(toast.textContent).toContain('Bundle');
    expect(toast.textContent).toContain('items');
  });

  it('shows a warning toast when an unknown barcode is scanned', async () => {
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Simulate scanning an unrecognised code.
    await act(async () => { mockedBarcode.triggerScan('UNKNOWN-CODE'); });

    // A warning toast should appear.
    expect(
      await screen.findByText(/No product or bundle matches this barcode/),
    ).toBeInTheDocument();
  });

  it('includes the expanded items in the cart when a bundle is scanned', async () => {
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Simulate scanning a bundle SKU.
    await act(async () => { mockedBarcode.triggerScan('BUNDLE-SKU-001'); });

    // Wait for the toast to confirm expansion.
    // Fluent wraps interpolated variables in Unicode formatting markers.
    const toast = await screen.findByRole('alert');
    expect(toast.textContent).toContain('Bundle');
    expect(toast.textContent).toContain('items');

    // Cart should show the expanded items by product name.
    // CartLineItem renders `line.name ?? line.sku`.
    expect(screen.getByText('Item 1')).toBeInTheDocument();
    expect(screen.getByText('Item 2')).toBeInTheDocument();
  });

  it('silently swallows when lookupByBarcode rejects (catch block)', async () => {
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Make lookupByBarcode throw on the next call.
    vi.mocked(productsApi.lookupByBarcode).mockRejectedValueOnce(
      new Error('USB disconnect'),
    );

    // Simulate a barcode scan. Since lookupByBarcode rejects, the
    // catch block fires and silently ignores the error.
    await act(async () => { mockedBarcode.triggerScan('ANY-CODE'); });

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
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Make lookupByBarcode throw a ScannerError (scanner hardware failure).
    vi.mocked(productsApi.lookupByBarcode).mockRejectedValueOnce(
      new ScannerError(
        'Scanner disconnected — check USB connection',
        ScannerError.codes.DISCONNECTED,
      ),
    );

    // Simulate a barcode scan. The catch block handles the typed error
    // the same way as a generic Error — silently ignored.
    await act(async () => { mockedBarcode.triggerScan('ANY-CODE'); });

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
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // lookupByBarcode returns null (as mocked by default), so the
    // callback falls through to lookupBundleBySku. Make it reject.
    vi.mocked(bundlesApi.lookupBundleBySku).mockRejectedValueOnce(
      new Error('Backend unreachable'),
    );

    // Simulate scanning a bundle SKU. The rejection occurs after
    // lookupByBarcode returns null, and is swallowed by the catch block.
    await act(async () => { mockedBarcode.triggerScan('BUNDLE-SKU-001'); });

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
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

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
    await act(async () => { mockedBarcode.triggerScan('BUNDLE-SKU-001'); });

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
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // lookupByBarcode returns null (default), lookupBundleBySku succeeds.
    // But make lookupProductBySku reject — this is used as the lookupItem
    // callback inside expandBundleItems, so the bundle expansion will fail.
    vi.mocked(productsApi.lookupProductBySku).mockRejectedValueOnce(
      new Error('SKU lookup timeout'),
    );

    // Simulate scanning a bundle SKU. The callback reaches the bundle
    // expansion path, but expandBundleItems rejects because the underlying
    // product lookup fails. The catch block swallows it.
    await act(async () => { mockedBarcode.triggerScan('BUNDLE-SKU-001'); });

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
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

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
    await act(async () => { mockedBarcode.triggerScan('BUNDLE-SKU-001'); });

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
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

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
      created_at: '',
      price_updated_at: '',
      product_type: 'retail',
    });

    // Simulate scanning a barcode that matches a product (not a bundle).
    await act(async () => { mockedBarcode.triggerScan('4901234567890'); });

    // Wait for lookupByBarcode to be called with the scanned code.
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupByBarcode)).toHaveBeenCalledWith('4901234567890');
    });

    // The product name appears in both the product grid and the cart.
    expect(screen.getAllByText('Caffè Latte').length).toBeGreaterThanOrEqual(1);

    // Cart should no longer be empty — wait for the async state update
    // from addProduct (setLines inside usePosState adds the item).
    await waitFor(() => {
      expect(screen.queryByText(/Cart is empty/)).not.toBeInTheDocument();
    });

    // The subtotal area should appear, confirming the product is in the cart.
    expect(screen.getByText(/Subtotal/)).toBeInTheDocument();

    // lookupBundleBySku should NOT have been called (early return).
    expect(bundlesApi.lookupBundleBySku).not.toHaveBeenCalled();

    // No bundle toast — this was a direct product add.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
  });

  it('gives product barcode priority when the same code matches both a product and a bundle', async () => {
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

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
      created_at: '',
      price_updated_at: '',
      product_type: 'retail',
    });

    // Scan a code that matches both a product barcode and a bundle SKU.
    await act(async () => { mockedBarcode.triggerScan('4900000000000'); });

    // Wait for the product lookup to be called.
    await waitFor(() => {
      expect(vi.mocked(productsApi.lookupByBarcode)).toHaveBeenCalledWith('4900000000000');
    });

    // The product should appear in the cart (product path won).
    expect(screen.getAllByText('Espresso Shot').length).toBeGreaterThanOrEqual(1);
    await waitFor(() => {
      expect(screen.queryByText(/Cart is empty/)).not.toBeInTheDocument();
    });

    // The bundle lookup should NOT have been called — the product path
    // returns early via `return` before falling through to the bundle path.
    expect(bundlesApi.lookupBundleBySku).not.toHaveBeenCalled();

    // No bundle toast.
    expect(screen.queryByText(/Bundle.*added/)).not.toBeInTheDocument();
  });

  it('shows a warning toast when a bundle is found but inactive', async () => {
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    // Wait for the barcode scanner callback to be registered and an
    // active shift to be loaded.
    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Scan an SKU the mock factory recognizes as an inactive bundle.
    // lookupByBarcode returns null (default) → falls through to bundle
    // lookup → lookupBundleBySku returns a bundle with active=false.
    await act(async () => { mockedBarcode.triggerScan('INACTIVE-SKU'); });

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
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });

    // Simulate a scanner hardware error event (e.g. USB disconnect).
    await act(async () => { mockedBarcode.triggerError('Scanner disconnected — check USB connection'); });

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

    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

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

  // ── Settings sub-screen (4-tab-routing) ────────────────────────
  //
  // Mirrors the same pattern tested in RetailOptionsScreen.test.tsx
  // for the desktop: the Settings sub-screen has four tabs
  // (Appearance / Features / Data / Sync) that route to dedicated
  // settings sub-screens. Lets the restaurant tablet cover the same
  // Settings surface as the desktop client.

  it('opens the Settings sub-screen and renders the Appearance tab by default', async () => {
    const user = userEvent.setup();
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Open the Settings sub-screen via the gear button in the header.
    await user.click(screen.getByTitle('Settings'));

    // AppearanceSettings is the default tab — the Appearance sub-screen
    // stub should mount immediately.
    await waitFor(() => {
      expect(screen.getByTestId('mock-appearance')).toBeInTheDocument();
    });

    // The 4 tab buttons are all present and addressable by role=tab.
    expect(screen.getByRole('tab', { name: 'Appearance' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Features' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Data' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Sync' })).toBeInTheDocument();
  });

  it('routes the Features tab to FeatureToggleScreen', async () => {
    const user = userEvent.setup();
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    await user.click(screen.getByTitle('Settings'));
    await waitFor(() => {
      expect(screen.getByTestId('mock-appearance')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('tab', { name: 'Features' }));

    await waitFor(() => {
      expect(screen.getByTestId('mock-features')).toBeInTheDocument();
    });
    // The Appearance sub-screen is unmounted when the Features tab is active.
    expect(screen.queryByTestId('mock-appearance')).not.toBeInTheDocument();
  });

  it('routes the Data tab to DataManagementScreen', async () => {
    const user = userEvent.setup();
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    await user.click(screen.getByTitle('Settings'));
    await waitFor(() => {
      expect(screen.getByTestId('mock-appearance')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('tab', { name: 'Data' }));

    await waitFor(() => {
      expect(screen.getByTestId('mock-data')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('mock-appearance')).not.toBeInTheDocument();
  });

  it('routes the Sync tab to the Cloud Sync placeholder', async () => {
    const user = userEvent.setup();
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    await user.click(screen.getByTitle('Settings'));
    await waitFor(() => {
      expect(screen.getByTestId('mock-appearance')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('tab', { name: 'Sync' }));

    // The Sync tab renders the "Cloud Sync" heading and an info
    // paragraph. The settings FTL key is `settings-sync-heading` →
    // "Cloud Sync". We match on the heading + the "snapshot" or
    // "sync cycle" sentence to verify the placeholder rendered.
    await waitFor(() => {
      expect(screen.getByText(/Cloud Sync/)).toBeInTheDocument();
    });
    // None of the other tab stubs should be mounted.
    expect(screen.queryByTestId('mock-appearance')).not.toBeInTheDocument();
    expect(screen.queryByTestId('mock-features')).not.toBeInTheDocument();
    expect(screen.queryByTestId('mock-data')).not.toBeInTheDocument();
  });

  it('returns to the main PosScreen when the Settings back button is clicked', async () => {
    const user = userEvent.setup();
    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    });
    await screen.findByText('Close');

    // Open Settings, then close it.
    await user.click(screen.getByTitle('Settings'));
    await waitFor(() => {
      expect(screen.getByTestId('mock-appearance')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /Back/i }));

    // The Appearance sub-screen is gone and the main cart is back.
    await waitFor(() => {
      expect(screen.queryByTestId('mock-appearance')).not.toBeInTheDocument();
    });
    expect(screen.getByText(/Cart is empty/)).toBeInTheDocument();
  });

});
