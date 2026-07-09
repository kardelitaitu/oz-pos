import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import InventoryReportScreen from '@/features/reports/InventoryReportScreen';

// ── shared.ftl keys used by this component ─────────────────────────────
const sharedFtl = `
print = Print
`;

// ── inventory.ftl keys used by this component ──────────────────────────
const inventoryFtl = `
inv-report-title = Inventory Report
inv-report-threshold = Threshold
inv-report-export-csv = Export CSV
inv-report-sku = SKU
inv-report-product = Product
inv-report-current-stock = Stock
inv-report-loading-aria = Loading inventory report
inv-report-region-aria = Inventory Report
inv-report-threshold-aria = Low stock threshold
inv-report-print-aria = Print report
inv-report-export-aria = Export to CSV
inv-report-csv-header-sku = SKU
inv-report-csv-header-product = Product
inv-report-csv-header-stock = Current Stock
inv-report-csv-header-threshold = Threshold
inv-report-no-results = No results found
`;

// ── mock API functions ─────────────────────────────────────────────────
const mockGetLowStockAlerts = vi.fn();
const mockPrintSalesReceipt = vi.fn();

vi.mock('@/api/reports', () => ({
  getLowStockAlerts: (...args: unknown[]) => mockGetLowStockAlerts(...args),
}));

vi.mock('@/api/sales', () => ({
  printSalesReceipt: (...args: unknown[]) => mockPrintSalesReceipt(...args),
}));

vi.mock('@/components/Card', () => ({
  Card: ({ children, className, shadow }: Record<string, unknown>) => (
    <div className={className as string} data-shadow={shadow as string}>{children as React.ReactNode}</div>
  ),
}));

vi.mock('@/components/Button', () => ({
  Button: ({ children, onClick, variant, ...props }: Record<string, unknown>) => (
    <button
      onClick={onClick as () => void}
      className={`btn btn--${variant as string || 'primary'}`}
      {...props}
    >
      {children as React.ReactNode}
    </button>
  ),
}));

vi.mock('@/components/Spinner', () => ({
  Spinner: (props: Record<string, unknown>) => <div data-testid="spinner" aria-label={props['aria-label'] as string} />,
}));

vi.mock('@/features/reports/InventoryReportScreen.css', () => ({}));

// ── helpers ────────────────────────────────────────────────────────────
const bundle = new FluentBundle('en');
bundle.addResource(new FluentResource(sharedFtl));
bundle.addResource(new FluentResource(inventoryFtl));
const l10n = new ReactLocalization([bundle]);

function buildSampleAlert(overrides: Partial<{
  product_id: string;
  sku: string;
  name: string;
  current_qty: number;
  threshold: number;
}> = {}) {
  return {
    product_id: overrides.product_id ?? 'prod-1',
    sku: overrides.sku ?? 'SKU001',
    name: overrides.name ?? 'Test Product',
    current_qty: overrides.current_qty ?? 5,
    threshold: overrides.threshold ?? 10,
  };
}

function renderScreen() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <InventoryReportScreen />
    </LocalizationProvider>,
  );
}

// ── tests ──────────────────────────────────────────────────────────────
describe('InventoryReportScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default: never resolves (loading state)
    mockGetLowStockAlerts.mockImplementation(() => new Promise(() => {}));
    mockPrintSalesReceipt.mockResolvedValue(undefined);
  });

  // ── Loading state ──────────────────────────────────────────────────
  it('shows loading spinner initially', () => {
    renderScreen();
    expect(screen.getByTestId('spinner')).toBeTruthy();
    expect(screen.getByTestId('spinner').getAttribute('aria-label')).toBe('Loading inventory report');
  });

  // ── Error state ────────────────────────────────────────────────────
  it('shows error message on fetch failure', async () => {
    mockGetLowStockAlerts.mockRejectedValue(new Error('Network error'));
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeTruthy();
    });
  });

  // ── Empty state ────────────────────────────────────────────────────
  it('shows "No results found" when no items', async () => {
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('No results found')).toBeTruthy();
    });
  });

  // ── Title & headers ────────────────────────────────────────────────
  it('renders the title "Inventory Report"', async () => {
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Inventory Report')).toBeTruthy();
    });
  });

  it('renders table headers: SKU, Product, Stock', async () => {
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('SKU')).toBeTruthy();
      expect(screen.getByText('Product')).toBeTruthy();
      expect(screen.getByText('Stock')).toBeTruthy();
    });
  });

  // ── Data rows ──────────────────────────────────────────────────────
  it('renders rows with SKU, name, and quantity', async () => {
    const alerts = [
      buildSampleAlert({ product_id: 'p1', sku: 'SKU-001', name: 'Widget', current_qty: 5 }),
      buildSampleAlert({ product_id: 'p2', sku: 'SKU-002', name: 'Gadget', current_qty: 3 }),
    ];
    mockGetLowStockAlerts.mockResolvedValue(alerts);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('SKU-001')).toBeTruthy();
      expect(screen.getByText('Widget')).toBeTruthy();
      expect(screen.getByText('5')).toBeTruthy();
      expect(screen.getByText('SKU-002')).toBeTruthy();
      expect(screen.getByText('Gadget')).toBeTruthy();
      expect(screen.getByText('3')).toBeTruthy();
    });
  });

  it('applies "critical" CSS class when quantity is 0', async () => {
    const alerts = [buildSampleAlert({ product_id: 'p1', current_qty: 0 })];
    mockGetLowStockAlerts.mockResolvedValue(alerts);
    renderScreen();
    await waitFor(() => {
      const row = document.querySelector('.inventory-report-row');
      expect(row).toBeTruthy();
      expect(row!.className).toContain('critical');
      const qtySpan = document.querySelector('.inventory-report-qty');
      expect(qtySpan!.className).toContain('critical');
    });
  });

  it('applies "low" CSS class when quantity > 0 but <= threshold', async () => {
    const alerts = [buildSampleAlert({ product_id: 'p1', current_qty: 3, threshold: 10 })];
    mockGetLowStockAlerts.mockResolvedValue(alerts);
    renderScreen();
    await waitFor(() => {
      const row = document.querySelector('.inventory-report-row');
      expect(row!.className).toContain('low');
      const qtySpan = document.querySelector('.inventory-report-qty');
      expect(qtySpan!.className).toContain('low');
    });
  });

  it('applies no special class when quantity > threshold', async () => {
    const alerts = [buildSampleAlert({ product_id: 'p1', current_qty: 15, threshold: 10 })];
    mockGetLowStockAlerts.mockResolvedValue(alerts);
    renderScreen();
    await waitFor(() => {
      const row = document.querySelector('.inventory-report-row');
      expect(row!.className).not.toContain('critical');
      expect(row!.className).not.toContain('low');
    });
  });

  // ── Threshold ──────────────────────────────────────────────────────
  it('renders threshold input with default value 10', async () => {
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const input = screen.getByLabelText('Low stock threshold') as HTMLInputElement;
      expect(input.value).toBe('10');
    });
  });

  it('refetches when threshold changes', async () => {
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(mockGetLowStockAlerts).toHaveBeenCalledWith(10);
    });
    mockGetLowStockAlerts.mockClear();

    const input = screen.getByLabelText('Low stock threshold') as HTMLInputElement;
    fireEvent.change(input, { target: { value: '5' } });

    await waitFor(() => {
      expect(mockGetLowStockAlerts).toHaveBeenCalledWith(5);
    });
  });

  // ── Print & CSV ────────────────────────────────────────────────────
  it('renders Print button and calls printSalesReceipt on click', async () => {
    mockGetLowStockAlerts.mockResolvedValue([buildSampleAlert()]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Print')).toBeTruthy();
    });
    fireEvent.click(screen.getByText('Print'));
    await waitFor(() => {
      expect(mockPrintSalesReceipt).toHaveBeenCalledTimes(1);
    });
  });

  it('renders Export CSV button and triggers CSV download on click', async () => {
    mockGetLowStockAlerts.mockResolvedValue([buildSampleAlert()]);
    // jsdom doesn't provide URL.createObjectURL; define it before spying
    if (!URL.createObjectURL) {
      Object.defineProperty(URL, 'createObjectURL', {
        value: vi.fn().mockReturnValue('blob:test'),
        writable: true,
      });
    }
    if (!URL.revokeObjectURL) {
      Object.defineProperty(URL, 'revokeObjectURL', { value: vi.fn(), writable: true });
    }
    const clickSpy = vi.fn();
    const origCreateElement = document.createElement.bind(document);
    const createElementSpy = vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      const el = origCreateElement(tag);
      if (tag === 'a') {
        el.click = clickSpy;
      }
      return el;
    });

    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Export CSV')).toBeTruthy();
    });
    fireEvent.click(screen.getByText('Export CSV'));

    await waitFor(() => {
      expect(URL.createObjectURL).toHaveBeenCalled();
      expect(clickSpy).toHaveBeenCalled();
      expect(URL.revokeObjectURL).toHaveBeenCalled();
    });

    createElementSpy.mockRestore();
  });

  // ── ARIA ───────────────────────────────────────────────────────────
  it('has role="region" with aria-label on the container', async () => {
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      const region = screen.getByRole('region', { name: 'Inventory Report' });
      expect(region).toBeTruthy();
    });
  });

  it('renders the threshold label', async () => {
    mockGetLowStockAlerts.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Threshold')).toBeTruthy();
    });
  });
});
