import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from '@testing-library/react';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';

// Create a minimal English bundle for tests
const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(`
shared-loading = Loading…
nav-pos-terminal = POS Terminal
nav-products = Products
nav-inventory = Inventory
nav-settings = Settings
nav-section-app = App
nav-switch-workspace = Switch Workspace
nav-sidebar-collapse = Collapse sidebar
nav-sidebar-expand = Expand sidebar
nav-main-aria = Main navigation
product-lookup-search-placeholder = Search products…
product-lookup-search-aria = Search products
product-lookup-barcode-placeholder = Scan barcode…
product-lookup-barcode-aria = Barcode input
product-lookup-scan-btn-aria = Submit barcode
product-lookup-barcode-scan = Scan
product-lookup-categories-aria = Filter by category
product-lookup-all-categories = All
product-lookup-loading = Loading…
product-lookup-no-results = No products found
product-lookup-grid-aria = Products
product-lookup-in-stock = In stock
product-lookup-out-of-stock = Out of stock
product-lookup-dev-fallback = Using fallback data
product-lookup-card-aria = { $name } — { $price }
gateway-status-online-aria = { $name } gateway is online
gateway-status-offline-aria = { $name } gateway is offline
role-badge-logged-in-aria = Logged in as { $displayName } ({ $roleName })
role-badge-logout-aria = Log out { $displayName }
role-badge-logout-title = Log out
theme-toggle-aria = Switch to { $mode } mode
theme-toggle-label = Toggle theme
toast-dismiss-aria = Dismiss notification
toast-notifications-aria = Notifications
`));
const testL10n = new ReactLocalization([bundle]);

// Mock all required context providers
vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { display_name: 'Test User', role_name: 'manager', user_id: '1', store_id: '1' },
    logout: vi.fn(),
  }),
  AuthProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    activeWorkspace: 'store-pos',
    availableWorkspaces: [{ key: 'store-pos', name: 'Store POS', description: '' }],
    setActiveWorkspace: vi.fn(),
    loading: false,
  }),
  WorkspaceProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/hooks/useGatewayStatus', () => ({
  useGatewayStatus: () => ({ configured: false, online: false }),
}));

vi.mock('@/api/products', () => ({
  lookupProductBySku: vi.fn(),
}));

vi.mock('@/api/bundles', () => ({
  lookupBundleBySku: vi.fn(),
}));

vi.mock('@/features/products/useProducts', () => ({
  useProducts: () => ({
    products: [],
    categories: [],
    loading: false,
    usingFallback: true,
  }),
}));

// Mock settings API
vi.mock('@/api/settings', () => ({
  getSetupStatus: () => Promise.resolve({ completed: true }),
  dismissSetupWizard: () => Promise.resolve(),
  completeSetup: () => Promise.resolve(),
}));

// Mock useFeatures
vi.mock('@/hooks/useFeatures', () => ({
  useFeatures: () => ({
    enabled: new Set<string>(),
    loaded: true,
  }),
}));

// Register required platform pages
import { registerPage, clearPages } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';

beforeEach(() => {
  clearPages();
  registerPage({ route: 'products', component: () => <div data-testid="products-page">Products</div>, label: 'Products' });
  registerNavItem({ route: 'products', label: 'Products', i18nKey: 'nav-products', icon: null as unknown as React.ReactNode });
});

describe('Workspace entry', () => {
  it('renders workspace content without Localized multiple-child errors', async () => {
    // Dynamically import AppShell to pick up mocks
    const { default: AppShell } = await import('@/frontend/shell/AppShell');

    expect(() =>
      render(
        <LocalizationProvider l10n={testL10n}>
          <AppShell />
        </LocalizationProvider>,
      ),
    ).not.toThrow('Expected to receive a single React element to localize');
  });
});
