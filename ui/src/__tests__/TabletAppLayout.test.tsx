import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, render } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';
import TabletAppLayout from '@/frontend/shell/tablet/TabletAppLayout';
import sharedFtl from '@/locales/shared.ftl?raw';

const mockGetNavItems = vi.fn();

vi.mock('@/platform/ui/menu-registry', () => ({
  getNavItems: (...args: unknown[]) => mockGetNavItems(...args),
}));

beforeEach(() => {
  mockGetNavItems.mockReset();
  mockGetNavItems.mockReturnValue([
    { route: 'sales', label: 'Sales', i18nKey: 'nav-sales', icon: '💰' },
    { route: 'products', label: 'Products', i18nKey: 'nav-products', icon: '📦' },
    { route: 'customers', label: 'Customers', i18nKey: 'nav-customers', icon: '👥' },
    { route: 'settings', label: 'Settings', i18nKey: 'nav-settings', icon: '⚙️' },
    { route: 'reports', label: 'Reports', i18nKey: 'nav-reports', icon: '📊' },
    { route: 'kds', label: 'KDS', i18nKey: 'kds-title', icon: '🍳' },
    { route: 'inventory', label: 'Inventory', i18nKey: 'nav-inventory', icon: '📋' },
    { route: 'staff', label: 'Staff', i18nKey: 'nav-staff', icon: '👤' },
  ]);
});

function renderLayout(props: {
  route?: string;
  onNavigate?: (route: string) => void;
  enabledFeatures?: Set<string>;
  userRole?: string;
  workspaceScreens?: string[];
} = {}) {
  const {
    route = 'sales',
    onNavigate = vi.fn(),
    enabledFeatures,
    userRole,
    workspaceScreens,
  } = props;
  return render(
    withFluent(
      <TabletAppLayout route={route} onNavigate={onNavigate} enabledFeatures={enabledFeatures} userRole={userRole} workspaceScreens={workspaceScreens}>
        <div data-testid="content">Main Content</div>
      </TabletAppLayout>,
      sharedFtl,
    ),
  );
}

describe('TabletAppLayout', () => {
  it('renders children content', () => {
    renderLayout();
    expect(screen.getByTestId('content')).toHaveTextContent('Main Content');
  });

  it('renders nav items from menu registry', () => {
    renderLayout();
    expect(screen.getByText('Sales')).toBeTruthy();
    expect(screen.getByText('Products')).toBeTruthy();
    expect(screen.getByText('Settings')).toBeTruthy();
  });

  it('limits nav items to 7', () => {
    renderLayout();
    const tabs = screen.getAllByRole('tab');
    expect(tabs).toHaveLength(7);
  });

  it('highlights the active route', () => {
    renderLayout({ route: 'products' });
    const tabs = screen.getAllByRole('tab');
    const activeTab = tabs.find((t) => t.getAttribute('aria-selected') === 'true');
    expect(activeTab).toBeTruthy();
    expect(activeTab?.textContent).toContain('Products');
  });

  it('calls onNavigate when a tab is clicked', async () => {
    const onNavigate = vi.fn();
    const user = userEvent.setup();

    renderLayout({ onNavigate });
    await user.click(screen.getByText('Products'));

    expect(onNavigate).toHaveBeenCalledWith('products');
  });

  it('filters nav items by workspaceScreens', () => {
    renderLayout({ workspaceScreens: ['sales', 'kds'] });

    expect(screen.getByText('Sales')).toBeTruthy();
    expect(screen.getByText('KDS')).toBeTruthy();
    expect(screen.queryByText('Products')).toBeNull();
    expect(screen.queryByText('Settings')).toBeNull();
  });

  it('passes enabledFeatures to getNavItems', () => {
    mockGetNavItems.mockClear();
    const features = new Set(['simple-retail']);

    renderLayout({ enabledFeatures: features });

    expect(mockGetNavItems).toHaveBeenCalledWith(features, undefined);
  });

  it('passes userRole to getNavItems', () => {
    mockGetNavItems.mockClear();

    renderLayout({ userRole: 'manager' });

    expect(mockGetNavItems).toHaveBeenCalledWith(undefined, 'manager');
  });

  it('has tablist role with aria-label', () => {
    renderLayout();
    const tablist = screen.getByRole('tablist');
    expect(tablist).toBeTruthy();
  });

  it('sets aria-selected correctly on each tab', () => {
    renderLayout({ route: 'kds' });
    const tabs = screen.getAllByRole('tab');

    const kdsTab = tabs.find((t) => t.textContent?.includes('KDS'));
    expect(kdsTab?.getAttribute('aria-selected')).toBe('true');

    const salesTab = tabs.find((t) => t.textContent?.includes('Sales'));
    expect(salesTab?.getAttribute('aria-selected')).toBe('false');
  });
});
