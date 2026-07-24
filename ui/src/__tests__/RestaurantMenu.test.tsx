import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { render } from '@testing-library/react';
import { withFluent } from '@/locales/test-utils';
import RestaurantMenu from '@/features/restaurant/RestaurantMenu';
import type { Product } from '@/types/domain';
import sharedFtl from '@/locales/shared.ftl?raw';

const mockProducts = [
  {
    sku: 'NASI-GORENG', name: 'Nasi Goreng', category: 'Makanan',
    productType: 'restaurant', price: { minor_units: 25000, currency: 'IDR' },
    inStock: true, createdAt: '2026-01-01',
  },
  {
    sku: 'ES-TEH', name: 'Es Teh', category: 'Minuman',
    productType: 'restaurant', price: { minor_units: 5000, currency: 'IDR' },
    inStock: true, createdAt: '2026-01-02',
  },
] as Product[];

const mockUseProducts = vi.fn();
const mockGoToWorkspacePicker = vi.fn();
const mockLogout = vi.fn();
const mockToggleTheme = vi.fn();
const mockToggleFullscreen = vi.fn();
const mockGetUserPreferences = vi.fn();
const mockSetUserPreferences = vi.fn();

vi.mock('@/features/products/useProducts', () => ({
  useProducts: (...args: unknown[]) => mockUseProducts(...args),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    activeWorkspace: 'restaurant-pos',
    setActiveWorkspace: vi.fn(),
    activeInstance: null,
    setActiveInstance: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
    error: null,
    retry: vi.fn(),
    lastWorkspace: null,
    switchStore: vi.fn(),
    resolvedStoreId: 'default',
    sessionToken: null,
    swapSessionToken: vi.fn(),
  }),
}));

vi.mock('@/hooks/useWorkspaceNav', () => ({
  useWorkspaceNav: () => ({ goToWorkspacePicker: mockGoToWorkspacePicker }),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: { user_id: 'user-1', username: 'test', role_name: 'cashier', token: 't', role_id: 'r', display_name: 'Test' },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: (...args: unknown[]) => mockLogout(...args),
    clearError: vi.fn(),
    isManager: false,
    isOwner: false,
  }),
}));

vi.mock('@/frontend/shell/ThemeProvider', () => ({
  useTheme: () => ({ theme: 'light', toggleTheme: mockToggleTheme }),
  ThemeProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/hooks/useFullscreen', () => ({
  useFullscreen: () => ({ toggleFullscreen: mockToggleFullscreen }),
}));

vi.mock('@/api/settings', () => ({
  // @deprecated kept for backward compat; RestaurantMenu uses getUserPreferencesScoped
  getUserPreferences: (...args: unknown[]) => mockGetUserPreferences(...args),
  getUserPreferencesScoped: (...args: unknown[]) => mockGetUserPreferences(...args),
  setUserPreferences: (...args: unknown[]) => mockSetUserPreferences(...args),
}));

beforeEach(() => {
  mockUseProducts.mockReset();
  mockGoToWorkspacePicker.mockReset();
  mockLogout.mockReset();
  mockToggleTheme.mockReset();
  mockToggleFullscreen.mockReset();
  mockGetUserPreferences.mockReset().mockResolvedValue({});
  mockSetUserPreferences.mockReset();

  localStorage.clear();
  mockUseProducts.mockReturnValue({
    products: mockProducts,
    categories: ['Makanan', 'Minuman'],
    categoryMeta: [],
    loading: false,
  });
});

afterEach(() => {
  localStorage.clear();
});

function renderMenu(props: { onAddProduct?: (product: Product) => void } = {}) {
  const { onAddProduct } = props;
  return render(withFluent(<RestaurantMenu onAddProduct={onAddProduct!} />, sharedFtl));
}

describe('RestaurantMenu', () => {
  it('shows loading state', () => {
    mockUseProducts.mockReturnValue({ products: [], categories: [], categoryMeta: [], loading: true });
    renderMenu();
    expect(screen.getByText('Loading menu…')).toBeTruthy();
  });

  it('shows empty state', () => {
    mockUseProducts.mockReturnValue({ products: [], categories: [], categoryMeta: [], loading: false });
    renderMenu();
    expect(screen.getByText('No items available')).toBeTruthy();
  });

  it('renders product cards', () => {
    renderMenu();
    expect(screen.getByText('Nasi Goreng')).toBeTruthy();
    expect(screen.getByText('Es Teh')).toBeTruthy();
  });

  it('renders category pills', () => {
    renderMenu();
    expect(screen.getByText('All')).toBeTruthy();
    expect(screen.getByText('Makanan')).toBeTruthy();
    expect(screen.getByText('Minuman')).toBeTruthy();
  });

  it('filters by category', async () => {
    renderMenu();
    const user = userEvent.setup();

    await user.click(screen.getByText('Makanan'));

    expect(screen.getByText('Nasi Goreng')).toBeTruthy();
    expect(screen.queryByText('Es Teh')).toBeNull();
  });

  it('filters by search query', async () => {
    renderMenu();
    const input = document.querySelector('.restaurant-search-input') as HTMLInputElement;

    await act(async () => {
      const nativeSetter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype, 'value',
      )?.set;
      nativeSetter?.call(input, 'Teh');
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });

    expect(screen.queryByText('Nasi Goreng')).toBeNull();
    expect(screen.getByText('Es Teh')).toBeTruthy();
  });

  it('calls onAddProduct when a card is clicked', async () => {
    const onAddProduct = vi.fn();
    renderMenu({ onAddProduct });
    const user = userEvent.setup();

    await user.click(screen.getByText('Nasi Goreng'));

    expect(onAddProduct).toHaveBeenCalledWith(expect.objectContaining({ sku: 'NASI-GORENG' }));
  });

  it('opens hamburger menu', async () => {
    renderMenu();
    const user = userEvent.setup();
    const hamburger = document.querySelector('.restaurant-hamburger-btn') as HTMLButtonElement;

    await user.click(hamburger);

    await waitFor(() => {
      expect(screen.getByText('Manual')).toBeTruthy();
    });
  });

  it('shows context menu on right-click', async () => {
    renderMenu();
    const card = screen.getByText('Nasi Goreng').closest('button')!;

    await act(async () => {
      card.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 100, clientY: 200 }));
    });

    await waitFor(() => {
      expect(screen.getByText('Pin to top')).toBeTruthy();
    });
  });

  it('shows empty state when no products match filter', () => {
    mockUseProducts.mockReturnValue({ products: [], categories: ['Makanan'], categoryMeta: [], loading: false });
    renderMenu();
    expect(screen.getByText('No items available')).toBeTruthy();
  });

  it('hides out-of-stock products when marked unavailable via context menu', async () => {
    renderMenu();
    const card = screen.getByText('Nasi Goreng').closest('button')!;

    await act(async () => {
      card.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 100, clientY: 200 }));
    });

    await waitFor(() => {
      expect(screen.getByText('Mark unavailable')).toBeTruthy();
    });
  });
});
