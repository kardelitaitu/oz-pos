// ── AppShell tests: KDS workspace navigation ──────────────────────
//
// Covers KDS rendering within store-pos (F12), restaurant-pos
// (chef button), and the standalone kds workspace, plus back-button
// navigation returning to the correct landing route.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { renderInAct } from '@/test-utils/renderInAct';
import userEvent from '@testing-library/user-event';
import { ToastProvider } from '@/frontend/shared/Toast';
import AppShell from '@/frontend/shell/AppShell';
import { withFluent } from '@/locales/test-utils';

// ── Mock sub-screens ─────────────────────────────────────────────

vi.mock('@/features/kds/KdsScreen', () => ({
  default: () => <div data-testid="kds-screen">Kitchen Display System</div>,
}));

vi.mock('@/features/retail/RetailPosScreen', () => ({
  default: ({ onNavigate }: { onNavigate?: (route: string) => void }) => (
    <div data-testid="retail-pos-screen">
      <button
        data-testid="trigger-kds-store"
        onClick={() => onNavigate?.('kds')}
      >
        Open KDS
      </button>
    </div>
  ),
}));

vi.mock('@/features/sales/PosScreen', () => ({
  default: ({ onNavigate }: { onNavigate?: (route: string) => void }) => (
    <div data-testid="pos-screen">
      <button
        data-testid="trigger-kds-restaurant"
        onClick={() => onNavigate?.('kds')}
      >
        Open KDS
      </button>
    </div>
  ),
}));

// ── Mock API modules used by AppShell ────────────────────────────

vi.mock('@/api/settings', () => ({
  getSetupStatus: vi.fn(() => Promise.resolve({ completed: true })),
  completeSetup: vi.fn(),
  dismissSetupWizard: vi.fn(),
  getEnabledFeatures: vi.fn(() => Promise.resolve({ features: [] })),
  getStoreSettings: vi.fn(() =>
    Promise.resolve({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' }),
  ),
  getReceiptSettings: vi.fn(() =>
    Promise.resolve({
      showCurrency: true, decimalSeparator: 'dot', showTax: true,
      footer: '', paperWidth: 'standard', showTableNumber: false,
      marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
    }),
  ),
  listCreditSales: vi.fn(() => Promise.resolve([])),
  settleCredit: vi.fn(),
}));

// ── Mock other hooks and APIs ───────────────────────────────────

vi.mock('@/hooks/useIdleTimer', () => ({
  useIdleTimer: vi.fn(),
}));

vi.mock('@/hooks/useFeatures', () => ({
  useFeatures: vi.fn(() => ({
    enabled: new Set<string>(),
    loading: false,
    isEnabled: () => true,
    loaded: true,
    filterRoutes: (routes: string[]) => routes,
    error: null,
  })),
  FEATURES: {
    KITCHEN_DISPLAY: 'kitchen-display',
    TABLE_MANAGEMENT: 'table-management',
    USB_SCALE: 'usb-scale',
    QUICK_RETURN: 'quick-return',
    SERIAL_TRACKING: 'serial-tracking',
  } as const,
}));

// ── Auth context mock (dynamic per test) ────────────────────

const mockAuthSession = vi.fn(() => ({
  session: {
    user_id: 'user-1',
    username: 'testuser',
    role_name: 'cashier',
    token: 'mock-token',
    role_id: 'role-1',
    display_name: 'Test User',
  },
  loading: false,
  error: null,
  login: vi.fn(),
  logout: vi.fn(),
  clearError: vi.fn(),
  isManager: false,
  isOwner: false,
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => mockAuthSession(),
}));

// ── Workspace context mock (dynamic per test) ─────────────────

const mockWorkspace = vi.fn();

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => mockWorkspace(),
  WorkspaceProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

// ── page-registry: register the kds route so handleNavigate works ──
import { registerPage, clearPages } from '@/platform/ui/page-registry';

// ── Test wrapper ─────────────────────────────────────────────

function wrap(children: React.ReactNode) {
  return withFluent(<ToastProvider>{children}</ToastProvider>);
}

// ── Helpers ───────────────────────────────────────────────────

function mockStorePos() {
  mockWorkspace.mockReturnValue({
    activeWorkspace: 'store-pos',
    setActiveWorkspace: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
  });
}

function mockRestaurantPos() {
  mockWorkspace.mockReturnValue({
    activeWorkspace: 'restaurant-pos',
    setActiveWorkspace: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
  });
}

function mockKdsWorkspace() {
  mockWorkspace.mockReturnValue({
    activeWorkspace: 'kds',
    setActiveWorkspace: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
  });
}

// ── Helper: set Kitchen role on the auth mock ───────────────

function mockKitchenRole() {
  mockAuthSession.mockReturnValue({
    session: {
      user_id: 'user-1',
      username: 'kitchen-staff',
      role_name: 'Kitchen',
      token: 'mock-token',
      role_id: 'role-kitchen',
      display_name: 'Chef',
    },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager: false,
    isOwner: false,
  });
}

// ── Tests ────────────────────────────────────────────────────

describe('AppShell — KDS workspace navigation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset auth mock to default (cashier) before each test
    mockAuthSession.mockReset();
    mockAuthSession.mockReturnValue({
      session: {
        user_id: 'user-1',
        username: 'testuser',
        role_name: 'cashier',
        token: 'mock-token',
        role_id: 'role-1',
        display_name: 'Test User',
      },
      loading: false,
      error: null,
      login: vi.fn(),
      logout: vi.fn(),
      clearError: vi.fn(),
      isManager: false,
      isOwner: false,
    });
    clearPages();
    // Register the kds page so handleNavigate's accessibility check passes
    registerPage({ route: 'kds', component: () => null, label: 'KDS' });
  });

  // ── store-pos workspace ────────────────────────────────────

  describe('store-pos workspace', () => {
    it('renders RetailPosScreen when currentRoute is not kds', async () => {
      mockStorePos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('retail-pos-screen')).toBeInTheDocument();
      });
      expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
    });

    it('renders KdsScreen with back button when navigating to kds route', async () => {
      mockStorePos();
      await renderInAct(wrap(<AppShell />));

      // Retail POS renders first
      await waitFor(() => {
        expect(screen.getByTestId('retail-pos-screen')).toBeInTheDocument();
      });

      // Navigate to KDS
      await userEvent.click(screen.getByTestId('trigger-kds-store'));

      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });
      // Back button should be present
      expect(screen.getByRole('button', { name: /back/i })).toBeInTheDocument();
      // Retail POS should no longer be visible
      expect(screen.queryByTestId('retail-pos-screen')).not.toBeInTheDocument();
    });

    it('navigates back to products when back button is clicked from KDS', async () => {
      mockStorePos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('retail-pos-screen')).toBeInTheDocument();
      });

      // Navigate to KDS
      await userEvent.click(screen.getByTestId('trigger-kds-store'));
      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });

      // Click back button
      await userEvent.click(screen.getByRole('button', { name: /back/i }));
      await waitFor(() => {
        expect(screen.getByTestId('retail-pos-screen')).toBeInTheDocument();
      });
      expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
    });
  });

  // ── restaurant-pos workspace ───────────────────────────────

  describe('restaurant-pos workspace', () => {
    it('renders PosScreen when currentRoute is not kds', async () => {
      mockRestaurantPos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('pos-screen')).toBeInTheDocument();
      });
      expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
    });

    it('renders KdsScreen with back button when navigating to kds route', async () => {
      mockRestaurantPos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('pos-screen')).toBeInTheDocument();
      });

      // Navigate to KDS via chef button
      await userEvent.click(screen.getByTestId('trigger-kds-restaurant'));

      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });
      // Back button should be present
      expect(screen.getByRole('button', { name: /back/i })).toBeInTheDocument();
      expect(screen.queryByTestId('pos-screen')).not.toBeInTheDocument();
    });

    it('navigates back to sales when back button is clicked from KDS', async () => {
      mockRestaurantPos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('pos-screen')).toBeInTheDocument();
      });

      // Navigate to KDS
      await userEvent.click(screen.getByTestId('trigger-kds-restaurant'));
      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });

      // Click back button
      await userEvent.click(screen.getByRole('button', { name: /back/i }));
      await waitFor(() => {
        expect(screen.getByTestId('pos-screen')).toBeInTheDocument();
      });
      expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
    });
  });

  // ── Standalone KDS workspace ───────────────────────────────

  describe('standalone kds workspace', () => {
    it('renders KdsScreen standalone without a back button', async () => {
      mockKdsWorkspace();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });
      // No back button in standalone mode
      expect(screen.queryByRole('button', { name: /back/i })).not.toBeInTheDocument();
      // Neither POS screen should be visible
      expect(screen.queryByTestId('retail-pos-screen')).not.toBeInTheDocument();
      expect(screen.queryByTestId('pos-screen')).not.toBeInTheDocument();
    });
  });

  // ── Kitchen role ───────────────────────────────────────────

  describe('kitchen role', () => {
    it('renders KDS workspace with Kitchen role', async () => {
      mockKitchenRole();
      mockKdsWorkspace();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });
      // No back button in standalone mode
      expect(screen.queryByRole('button', { name: /back/i })).not.toBeInTheDocument();
    });

    it('can navigate to KDS from store-pos with Kitchen role', async () => {
      mockKitchenRole();
      mockStorePos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('retail-pos-screen')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByTestId('trigger-kds-store'));

      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });
      expect(screen.getByRole('button', { name: /back/i })).toBeInTheDocument();
    });

    it('can navigate back from KDS to store-pos with Kitchen role', async () => {
      mockKitchenRole();
      mockStorePos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('retail-pos-screen')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByTestId('trigger-kds-store'));
      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByRole('button', { name: /back/i }));
      await waitFor(() => {
        expect(screen.getByTestId('retail-pos-screen')).toBeInTheDocument();
      });
      expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
    });

    it('can navigate to KDS from restaurant-pos with Kitchen role', async () => {
      mockKitchenRole();
      mockRestaurantPos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('pos-screen')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByTestId('trigger-kds-restaurant'));

      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });
      expect(screen.getByRole('button', { name: /back/i })).toBeInTheDocument();
    });

    it('can navigate back from KDS to restaurant-pos with Kitchen role', async () => {
      mockKitchenRole();
      mockRestaurantPos();
      await renderInAct(wrap(<AppShell />));

      await waitFor(() => {
        expect(screen.getByTestId('pos-screen')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByTestId('trigger-kds-restaurant'));
      await waitFor(() => {
        expect(screen.getByTestId('kds-screen')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByRole('button', { name: /back/i }));
      await waitFor(() => {
        expect(screen.getByTestId('pos-screen')).toBeInTheDocument();
      });
      expect(screen.queryByTestId('kds-screen')).not.toBeInTheDocument();
    });
  });
});
