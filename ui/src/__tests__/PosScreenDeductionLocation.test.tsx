// ── ADR-19 §17 – deduction location badge & unbound guard tests ──
//
// Tests for the two ADR-19 frontend contract items:
//   1. cart_panel_renders_locked_deduction_location
//   2. cart_panel_unbound_rejects_add_line

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { act } from 'react';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/__tests__/test-utils/render';
import salesFtl from '@/locales/sales.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import inventoryFtl from '@/locales/inventory.ftl?raw';
import settingsFtl from '@/locales/settings.ftl?raw';
import PosScreen from '@/features/sales/PosScreen';
import * as salesApi from '@/api/sales';
import * as productsApi from '@/api/products';
import type { CartId } from '@/types/domain';
import type { BarcodeScannedPayload } from '@/api/hardware';
import type * as HardwareModule from '@/api/hardware';

// ── Hoisted mock helpers ──────────────────────────────────────────

const mockedBarcode = vi.hoisted(() => {
  let onProductFound: ((payload: BarcodeScannedPayload) => void) | null = null;
  return {
    triggerScan(code: string) {
      onProductFound?.({ code, symbology: 'test' });
    },
    reset() {
      onProductFound = null;
    },
    useBarcodeScanner: vi.fn(
      (opts: {
        onProductFound: (p: BarcodeScannedPayload) => void;
        onError?: (error: string) => void;
      }) => {
        onProductFound = opts.onProductFound;
      },
    ),
  };
});

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
  lookupByBarcodeScoped: vi.fn(() => Promise.resolve(null)),
  lookupProductBySku: vi.fn((sku: string) => {
    const products: Record<string, unknown> = {
      'ITEM-001': {
        sku: 'ITEM-001',
        name: 'Test Item',
        category: 'Test',
        price: { minor_units: 400, currency: 'USD' },
        barcode: null,
        in_stock: true,
        stock_qty: 100,
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
  lookupBundleBySku: vi.fn(() => Promise.resolve(null)),
  listBundles: vi.fn(() => Promise.resolve([])),
  getBundle: vi.fn(() => Promise.resolve(null)),
  createBundle: vi.fn(),
  updateBundle: vi.fn(),
  deleteBundle: vi.fn(),
}));

vi.mock('@/api/shifts', async () => {
  const { createShiftsApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createShiftsApiMock();
});

vi.mock('@/api/settings', async () => {
  const { createSettingsApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createSettingsApiMock();
});

vi.mock('@/api/sales', async () => {
  const { createSalesApiMock } = await import('@/__tests__/test-utils/mocks/api');
  return createSalesApiMock();
});

vi.mock('@/utils/interaction', () => ({
  triggerInteraction: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', async () => {
  const { createAuthContextMock } = await import('@/__tests__/test-utils/mocks/contexts');
  return {
    useAuth: createAuthContextMock({ isManager: true }),
  };
});

vi.mock('@/contexts/WorkspaceContext', async () => {
  const { createWorkspaceContextMock } = await import('@/__tests__/test-utils/mocks/contexts');
  return createWorkspaceContextMock();
});

vi.mock('@/features/settings/AppearanceSettings', () => ({
  AppearanceSettings: () => <div data-testid="mock-appearance">Appearance Settings Stub</div>,
}));
vi.mock('@/features/settings/FeatureToggleScreen', () => ({
  default: () => <div data-testid="mock-features">Feature Toggles Stub</div>,
}));
vi.mock('@/features/settings/DataManagementScreen', () => ({
  default: () => <div data-testid="mock-data">Data Management Stub</div>,
}));

// FAST_WAIT: 5ms polling for async assertions.
const FAST_WAIT = { interval: 5, timeout: 500 } as const;

describe('PosScreen – ADR-19 §17 deduction location', () => {
  beforeEach(() => {
    mockedBarcode.reset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('cart_panel_renders_locked_deduction_location', async () => {
    // Override startSaleScoped to return a deduction location ID
    vi.mocked(salesApi.startSaleScoped).mockResolvedValue({
      cartId: 'cart-1' as CartId,
      deductionLocationId: 'loc-store-inventory',
    });
    vi.mocked(salesApi.getCartDeductionLocation).mockResolvedValue({
      locationId: 'loc-store-inventory',
      locationName: 'Store Inventory',
    });

    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    }, FAST_WAIT);
    await screen.findByText('Close');

    // Override lookupByBarcodeScoped to find a product
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 400, currency: 'USD' },
      barcode: 'BARCODE-001',
      in_stock: true,
      stock_qty: 100,
      product_type: 'standard',
      tax_rate_ids: [],
      created_at: '',
      price_updated_at: '',
    });

    // Scan a product to add it to the cart
    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-001');
    });

    await waitFor(() => {
      expect(screen.getByText('Test Item')).toBeInTheDocument();
    }, FAST_WAIT);

    // Click the Override button to trigger ensureCart (creates cart + ded. location)
    const overrideBtn = screen.getByRole('button', { name: /override price for test item/i });
    await userEvent.click(overrideBtn);

    // Wait for the deduction location badge to appear
    await waitFor(() => {
      expect(screen.getByTestId('deduction-location-badge')).toBeInTheDocument();
    }, FAST_WAIT);

    expect(screen.getByTestId('deduction-location-badge').textContent).toContain('Deducting');
    expect(screen.getByTestId('deduction-location-badge').textContent).toContain('Store Inventory');
  });

  it('cart_panel_unbound_rejects_add_line', async () => {
    // Override startSaleScoped to return NO deduction location ID (unbound)
    vi.mocked(salesApi.startSaleScoped).mockResolvedValue({
      cartId: 'cart-1' as CartId,
    } as unknown as Awaited<ReturnType<typeof salesApi.startSaleScoped>>);
    vi.mocked(salesApi.getCartDeductionLocation).mockResolvedValue(null);

    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    }, FAST_WAIT);
    await screen.findByText('Close');

    // Override lookupByBarcodeScoped to find a product
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 400, currency: 'USD' },
      barcode: 'BARCODE-001',
      in_stock: true,
      stock_qty: 100,
      product_type: 'standard',
      tax_rate_ids: [],
      created_at: '',
      price_updated_at: '',
    });

    // Scan a product to add it to the cart
    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-001');
    });

    await waitFor(() => {
      expect(screen.getByText('Test Item')).toBeInTheDocument();
    }, FAST_WAIT);

    // Click Override to trigger ensureCart (creates cart without deduction location)
    const overrideBtn = screen.getByRole('button', { name: /override price for test item/i });
    await userEvent.click(overrideBtn);

    // After ensureCart resolves, the cart exists but has no deduction location.
    // Wait a tick for the async resolve + state update.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });

    // Now try adding another product via scan (handleAddProduct checks cartId +
    // deductionLocationIdRef.current and should show an error toast)
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 400, currency: 'USD' },
      barcode: 'BARCODE-002',
      in_stock: true,
      stock_qty: 100,
      product_type: 'standard',
      tax_rate_ids: [],
      created_at: '',
      price_updated_at: '',
    });

    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-002');
    });

    // An error toast should appear indicating the cart has no deduction location
    const toast = await screen.findByRole('alert');
    expect(toast.textContent).toMatch(/no deduction location|cannot add/i);
  });

  it('cart_panel_badge_click_opens_fastpin_overlay', async () => {
    // Override startSaleScoped to return a deduction location ID
    vi.mocked(salesApi.startSaleScoped).mockResolvedValue({
      cartId: 'cart-1' as CartId,
      deductionLocationId: 'loc-store-inventory',
    });
    vi.mocked(salesApi.getCartDeductionLocation).mockResolvedValue({
      locationId: 'loc-store-inventory',
      locationName: 'Store Inventory',
    });

    await renderWithProviders(<PosScreen />, salesFtl, productsFtl, inventoryFtl, settingsFtl);

    await waitFor(() => {
      expect(mockedBarcode.useBarcodeScanner).toHaveBeenCalled();
    }, FAST_WAIT);
    await screen.findByText('Close');

    // Override lookupByBarcodeScoped to find a product
    vi.mocked(productsApi.lookupByBarcodeScoped).mockResolvedValueOnce({
      sku: 'ITEM-001',
      name: 'Test Item',
      category: 'Test',
      price: { minor_units: 400, currency: 'USD' },
      barcode: 'BARCODE-001',
      in_stock: true,
      stock_qty: 100,
      product_type: 'standard',
      tax_rate_ids: [],
      created_at: '',
      price_updated_at: '',
    });

    // Scan a product to add it to the cart
    await act(async () => {
      mockedBarcode.triggerScan('BARCODE-001');
    });

    await waitFor(() => {
      expect(screen.getByText('Test Item')).toBeInTheDocument();
    }, FAST_WAIT);

    // Click Override to trigger ensureCart (creates cart + ded. location)
    const overrideBtn = screen.getByRole('button', { name: /override price for test item/i });
    await userEvent.click(overrideBtn);

    // Wait for the deduction location badge to appear
    await waitFor(() => {
      expect(screen.getByTestId('deduction-location-badge')).toBeInTheDocument();
    }, FAST_WAIT);

    // Close PriceOverrideModal that popped up from the Override click
    const cancelBtn = screen.getByRole('button', { name: /cancel/i });
    await userEvent.click(cancelBtn);

    // Click the badge to open FastPINOverlay
    const badge = screen.getByTestId('deduction-location-badge');
    await userEvent.click(badge);

    // FastPINOverlay username input should now be visible
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    }, FAST_WAIT);
  });
});
