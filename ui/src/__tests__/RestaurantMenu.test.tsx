import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, act } from '@testing-library/react';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import userEvent from '@testing-library/user-event';
import RestaurantMenu from '@/features/restaurant/RestaurantMenu';
import type { Product } from '@/types/domain';
import sharedFtl from '@/locales/shared.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';

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
  // LOAD-05: the loading label resolves via requiredLocalized, so the
  // products bundle (restaurant-menu-loading) must be present.
  return renderWithFluentSync(<RestaurantMenu onAddProduct={onAddProduct!} />, sharedFtl, productsFtl);
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
    // Real bundle value (products.ftl) now that the bundle is loaded.
    expect(screen.getByText('Menu is empty')).toBeTruthy();
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
    expect(screen.getByText('Menu is empty')).toBeTruthy();
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

  // ── A11Y-06: context menu keyboard operability (WAI-ARIA menu pattern) ──

  it('opens the context menu via Shift+F10 and moves focus to the first menuitem', async () => {
    renderMenu();
    const user = userEvent.setup();
    const card = screen.getByText('Nasi Goreng').closest('button')!;

    card.focus();
    await user.keyboard('{Shift>}{F10}{/Shift}');

    await waitFor(() => {
      expect(screen.getByText('Pin to top')).toBeTruthy();
    });
    // Keyboard-open must move focus into the menu (first menuitem).
    expect(document.activeElement?.textContent).toContain('Pin to top');
  });

  it('opens the context menu via the ContextMenu key', async () => {
    renderMenu();
    const user = userEvent.setup();
    const card = screen.getByText('Nasi Goreng').closest('button')!;

    card.focus();
    await user.keyboard('{ContextMenu}');

    await waitFor(() => {
      expect(screen.getByText('Mark unavailable')).toBeTruthy();
    });
  });

  it('supports ArrowUp/ArrowDown roving focus between menuitems', async () => {
    renderMenu();
    const user = userEvent.setup();
    const card = screen.getByText('Nasi Goreng').closest('button')!;

    card.focus();
    await user.keyboard('{Shift>}{F10}{/Shift}');
    await waitFor(() => {
      expect(screen.getByText('Pin to top')).toBeTruthy();
    });

    // First menuitem is focused after open; ArrowDown moves to the second.
    await user.keyboard('{ArrowDown}');
    expect(document.activeElement?.textContent).toContain('Mark unavailable');

    // ArrowUp wraps back to the first.
    await user.keyboard('{ArrowUp}');
    expect(document.activeElement?.textContent).toContain('Pin to top');
  });

  it('closes the context menu via Escape and restores focus to the card', async () => {
    renderMenu();
    const user = userEvent.setup();
    const card = screen.getByText('Nasi Goreng').closest('button')!;

    card.focus();
    await user.keyboard('{Shift>}{F10}{/Shift}');
    await waitFor(() => {
      expect(screen.getByText('Pin to top')).toBeTruthy();
    });

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByText('Pin to top')).toBeNull();
    });
    // Focus must return to the triggering card (not the body).
    expect(document.activeElement).toBe(card);
  });

  it('closes the pointer-opened context menu via Escape without moving focus into the menu', async () => {
    renderMenu();
    const user = userEvent.setup();
    const card = screen.getByText('Nasi Goreng').closest('button')!;
    const focusedBefore = document.activeElement;

    await act(async () => {
      card.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 100, clientY: 200 }));
    });
    await waitFor(() => {
      expect(screen.getByText('Pin to top')).toBeTruthy();
    });

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByText('Pin to top')).toBeNull();
    });
    // Pointer-open must not steal focus into the menu or force-restore it —
    // the element focused before opening stays focused after Escape.
    expect(document.activeElement).toBe(focusedBefore);
  });
});
