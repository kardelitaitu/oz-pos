// ── WorkspaceHome tests ───────────────────────────────────────────
//
// Covers: loading state (skeleton), error state with retry, empty
// state, main workspace card rendering, keyboard navigation, role-
// based access control, and per-workspace accent colors.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderInAct } from '@/test-utils/renderInAct';
import { withFluent } from '@/locales/test-utils';
import WorkspaceHome from '@/features/workspaces/WorkspaceHome';

// ── Hoisted mocks ──────────────────────────────────────────────

const mockSetActiveWorkspace = vi.fn();
const mockAuthSession = vi.fn(() => ({
  session: {
    user_id: 'user-1',
    display_name: 'Test Owner',
    role_name: 'owner',
    role_id: 'role-owner',
  },
  loading: false,
  error: null,
  login: vi.fn(),
  logout: vi.fn(),
  clearError: vi.fn(),
  isManager: false,
  isOwner: true,
}));

const mockWorkspaceValue = vi.fn();

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => mockAuthSession(),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => mockWorkspaceValue(),
}));

// ── Test wrapper ───────────────────────────────────────────────

function wrap(children: React.ReactNode) {
  return withFluent(children);
}

// ── Helpers ────────────────────────────────────────────────────

const sampleWorkspaces = [
  { instance_id: 'default-restaurant-pos', type_key: 'restaurant-pos', store_id: 'default', store_name: 'Main Store', name: 'Restaurant POS', description: 'Cashier terminal for restaurant ordering', icon: 'restaurant', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-store-pos', type_key: 'store-pos', store_id: 'default', store_name: 'Main Store', name: 'Store POS', description: 'Cashier terminal for retail', icon: 'store', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-kds', type_key: 'kds', store_id: 'default', store_name: 'Main Store', name: 'Kitchen Display', description: 'Order queue display for the kitchen', icon: 'kds', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-inventory', type_key: 'inventory', store_id: 'default', store_name: 'Main Store', name: 'Inventory Management', description: 'Manage products and stock', icon: 'inventory', layout_mode: 'sidebar', colour: null, is_default: false },
  { instance_id: 'default-admin', type_key: 'admin', store_id: 'default', store_name: 'Main Store', name: 'Admin', description: 'System settings and reports', icon: 'admin', layout_mode: 'sidebar', colour: null, is_default: false },
];

function mockDefaultUser() {
  mockAuthSession.mockReturnValue({
    session: {
      user_id: 'user-1',
      display_name: 'Test Owner',
      role_name: 'owner',
      role_id: 'role-owner',
    },
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    isManager: false,
    isOwner: true,
  });
}

function mockCashierUser() {
  mockAuthSession.mockReturnValue({
    session: {
      user_id: 'user-2',
      display_name: 'Cashier One',
      role_name: 'cashier',
      role_id: 'role-cashier',
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

// ── Tests ──────────────────────────────────────────────────────

describe('WorkspaceHome', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockDefaultUser();
  });

  // ── Loading state ──────────────────────────────────────────

  describe('loading state', () => {
    it('shows skeleton grid while loading', async () => {
      mockWorkspaceValue.mockReturnValue({
  availableWorkspaces: [],
  loading: true,
  error: null,
  retry: vi.fn(),
  setActiveWorkspace: mockSetActiveWorkspace,
  setActiveInstance: vi.fn(),
  activeInstance: null,
  activeWorkspace: null,
  workspaceScreens: [],
  lastWorkspace: null,
  switchStore: vi.fn(),
  resolvedStoreId: 'default',
      });

      await renderInAct(wrap(<WorkspaceHome />));

      const skeletonGrid = document.querySelector('.workspace-skeleton-grid');
      expect(skeletonGrid).toBeInTheDocument();
      const skeletonCards = document.querySelectorAll('.workspace-skeleton-card');
      expect(skeletonCards.length).toBe(3);
    });
  });

  // ── Error state ────────────────────────────────────────────

  describe('error state', () => {
    it('shows error with retry when error is set and no workspaces', async () => {
      const mockRetry = vi.fn();
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: [],
        loading: false,
        error: 'Failed to load workspaces',
        retry: mockRetry,
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Connection Error')).toBeInTheDocument();
      });
      expect(screen.getByText(/Could not load your workspaces/)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument();
    });

    it('calls retry when retry button is clicked', async () => {
      const mockRetry = vi.fn();
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: [],
        loading: false,
        error: 'Failed to load workspaces',
        retry: mockRetry,
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Connection Error')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByRole('button', { name: /try again/i }));
      expect(mockRetry).toHaveBeenCalledTimes(1);
    });
  });

  // ── Empty state ────────────────────────────────────────────

  describe('empty state', () => {
    it('shows empty message when no workspaces available', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: [],
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('No workspaces available')).toBeInTheDocument();
      });
      expect(screen.getByText(/Contact an administrator/)).toBeInTheDocument();
    });
  });

  // ── Main render ────────────────────────────────────────────

  describe('main render', () => {
    it('renders all workspace cards with names and descriptions', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });
      expect(screen.getByText('Store POS')).toBeInTheDocument();
      expect(screen.getByText('Kitchen Display')).toBeInTheDocument();
      expect(screen.getByText('Inventory Management')).toBeInTheDocument();
      expect(screen.getByText('Admin')).toBeInTheDocument();
    });

    it('shows user display name in greeting', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        // Name appears in both the user profile and the greeting
        const nameElements = screen.getAllByText(/Test Owner/);
        expect(nameElements.length).toBeGreaterThanOrEqual(2);
      });
    });

    it('shows number key hint badges on each card', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const hints = document.querySelectorAll('.workspace-card-key-hint');
      expect(hints.length).toBe(5);
      expect(hints[0]?.textContent).toBe('1');
      expect(hints[4]?.textContent).toBe('5');
    });

    it('shows keyboard shortcut hint text on cards', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Each card should have a shortcut hint (hidden until hover)
      const hints = document.querySelectorAll('button.workspace-card .workspace-card-overlay');
      expect(hints.length).toBe(5);
      expect(hints[0]?.textContent).toMatch(/1/);
      expect(hints[4]?.textContent).toMatch(/5/);
    });

    it('calls setActiveWorkspace when a card is clicked', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Click the first workspace card (Restaurant POS)
      const firstCard = document.querySelectorAll('.workspace-card')[0] as HTMLButtonElement;
      await userEvent.click(firstCard);
      await waitFor(() => {
        expect(mockSetActiveWorkspace).toHaveBeenCalledWith('restaurant-pos');
      });
    });

    it('renders workspace cards in the correct sort order', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: [...sampleWorkspaces].reverse(), // Pass in reverse order
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const cards = Array.from(document.querySelectorAll('.workspace-card')).filter(c => !c.textContent?.includes('Coming soon'));
      expect(cards.length).toBe(5);
      const names = Array.from(cards).map((c) => c.querySelector('.workspace-card-name')?.textContent);
      expect(names).toEqual([
        'Restaurant POS',
        'Store POS',
        'Kitchen Display',
        'Inventory Management',
        'Admin',
      ]);
    });

    it('applies per-workspace accent color classes', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const cards = document.querySelectorAll('.workspace-card');
      expect(cards[0]).toHaveClass('ws-color-restaurant-pos');
      expect(cards[2]).toHaveClass('ws-color-kds');
      expect(cards[4]).toHaveClass('ws-color-admin');
    });
  });

  // ── Role-based access ───────────────────────────────────────

  describe('role-based access', () => {
    it('disables workspace cards that are not accessible for cashier role', async () => {
      mockCashierUser();
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const cards = Array.from(document.querySelectorAll('.workspace-card--disabled')).filter(c => !c.textContent?.includes('Coming soon'));
      // Cashier can only access restaurant-pos and store-pos
      expect(cards.length).toBe(3);
      const disabledNames = Array.from(cards).map(
        (c) => c.querySelector('.workspace-card-name')?.textContent,
      );
      expect(disabledNames).toContain('Kitchen Display');
      expect(disabledNames).toContain('Inventory Management');
      expect(disabledNames).toContain('Admin');
    });

    it('shows badge on disabled workspace cards', async () => {
      mockCashierUser();
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const badges = Array.from(screen.getAllByText('Not available')).filter(
        (b) => !b.closest('.workspace-card')?.textContent?.includes('Coming soon')
      );
      expect(badges.length).toBe(3);
    });

    it('allows owner role to click Admin workspace', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Owner has access to all cards — none should be disabled
      const disabledCards = Array.from(document.querySelectorAll('.workspace-card--disabled')).filter(c => !c.textContent?.includes('Coming soon'));
      expect(disabledCards.length).toBe(0);

      // Find the Admin card by its heading text and click it
      const adminCard = document.querySelectorAll('.workspace-card')[4] as HTMLButtonElement;
      await userEvent.click(adminCard);
      await waitFor(() => {
        expect(mockSetActiveWorkspace).toHaveBeenCalledWith('admin');
      });
    });

    it('allows owner role to click KDS workspace', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Verify the KDS card is clickable
      const kdsCard = document.querySelectorAll('.workspace-card')[2] as HTMLButtonElement;
      await userEvent.click(kdsCard);
      await waitFor(() => {
        expect(mockSetActiveWorkspace).toHaveBeenCalledWith('kds');
      });
    });
  });

  // ── Logout confirmation ────────────────────────────────────

  describe('logout confirmation', () => {
    it('shows logout confirmation modal when logout is clicked', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Click the logout button
      const logoutBtn = screen.getByRole('button', { name: /Logout/i });
      await userEvent.click(logoutBtn);

      // Should show the logout confirmation modal
      await waitFor(() => {
        expect(screen.getByText(/Logout\?/i)).toBeInTheDocument();
      });
      expect(screen.getByText(/Any unsaved work will be lost/i)).toBeInTheDocument();
    });

    it('calls logout when confirmed in modal', async () => {
      const mockLogout = vi.fn();
      mockAuthSession.mockReturnValue({
        session: {
          user_id: 'user-1',
          display_name: 'Test Owner',
          role_name: 'owner',
          role_id: 'role-owner',
        },
        loading: false,
        error: null,
        login: vi.fn(),
        logout: mockLogout,
        clearError: vi.fn(),
        isManager: false,
        isOwner: true,
      });

      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Click the logout button
      const logoutBtn = screen.getByRole('button', { name: /Logout/i });
      await userEvent.click(logoutBtn);

      // Click confirm in the modal
      await waitFor(() => {
        expect(screen.getByText(/Logout\?/i)).toBeInTheDocument();
      });

      const confirmBtn = document.querySelector('.logout-confirm-confirm') as HTMLButtonElement;
      await userEvent.click(confirmBtn);

      expect(mockLogout).toHaveBeenCalledTimes(1);
    });

    it('does not call logout when cancelled in modal', async () => {
      const mockLogout = vi.fn();
      mockAuthSession.mockReturnValue({
        session: {
          user_id: 'user-1',
          display_name: 'Test Owner',
          role_name: 'owner',
          role_id: 'role-owner',
        },
        loading: false,
        error: null,
        login: vi.fn(),
        logout: mockLogout,
        clearError: vi.fn(),
        isManager: false,
        isOwner: true,
      });

      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Click the logout button
      const logoutBtn = screen.getByRole('button', { name: /Logout/i });
      await userEvent.click(logoutBtn);

      // Click cancel in the modal
      await waitFor(() => {
        expect(screen.getByText(/Logout\?/i)).toBeInTheDocument();
      });

      const cancelBtn = screen.getByRole('button', { name: /Cancel/i });
      await userEvent.click(cancelBtn);

      expect(mockLogout).not.toHaveBeenCalled();
    });
  });

  // ── Keyboard shortcuts (number keys) ────────────────────────

  describe('keyboard shortcuts', () => {
    it('selects workspace when number key is pressed', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Press '3' to select the third card (KDS)
      fireEvent.keyDown(document.activeElement!, { key: '3' });
      await waitFor(() => {
        expect(mockSetActiveWorkspace).toHaveBeenCalledWith('kds');
      });
    });

    it('pressing 1 selects the first workspace', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      fireEvent.keyDown(document.activeElement!, { key: '1' });
      await waitFor(() => {
        expect(mockSetActiveWorkspace).toHaveBeenCalledWith('restaurant-pos');
      });
    });

    it('does nothing for number keys beyond workspace count', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Press '9' — only 5 cards, so no action
      fireEvent.keyDown(document.activeElement!, { key: '9' });
      expect(mockSetActiveWorkspace).not.toHaveBeenCalled();
    });
  });

  // ── Fullscreen button ───────────────────────────────────────

  describe('fullscreen button', () => {
    it('renders a fullscreen toggle button with F11 tooltip', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const btn = document.querySelector('.workspace-home-fullscreen-btn') as HTMLButtonElement;
      expect(btn).toBeInTheDocument();
      expect(btn.getAttribute('title')).toBe('F11');
    });

    it('renders fullscreen button in loading state with F11 tooltip', async () => {
      mockWorkspaceValue.mockReturnValue({
  availableWorkspaces: [],
  loading: true,
  error: null,
  retry: vi.fn(),
  setActiveWorkspace: mockSetActiveWorkspace,
  setActiveInstance: vi.fn(),
  activeInstance: null,
  activeWorkspace: null,
  workspaceScreens: [],
  lastWorkspace: null,
  switchStore: vi.fn(),
  resolvedStoreId: 'default',
      });

      await renderInAct(wrap(<WorkspaceHome />));

      const btn = document.querySelector('.workspace-home-fullscreen-btn') as HTMLButtonElement;
      expect(btn).toBeInTheDocument();
      expect(btn.getAttribute('title')).toBe('F11');
    });

    it('renders fullscreen button in error state with F11 tooltip', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: [],
        loading: false,
        error: 'Failed',
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Connection Error')).toBeInTheDocument();
      });

      const btn = document.querySelector('.workspace-home-fullscreen-btn') as HTMLButtonElement;
      expect(btn).toBeInTheDocument();
      expect(btn.getAttribute('title')).toBe('F11');
    });
  });

  // ── Active workspace indicator ───────────────────────────────

  describe('active workspace indicator', () => {
    it('does not show active indicator when lastWorkspace is null', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const activeCards = document.querySelectorAll('.workspace-card--active');
      expect(activeCards.length).toBe(0);
      const activeDots = document.querySelectorAll('.workspace-card-active-dot');
      expect(activeDots.length).toBe(0);
    });

    it('shows active indicator on the last active workspace card', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: 'kds',
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Kitchen Display')).toBeInTheDocument();
      });

      // The KDS card (index 2) should be active
      const activeCards = document.querySelectorAll('.workspace-card--active');
      expect(activeCards.length).toBe(1);
      const activeCardName = activeCards[0]?.querySelector('.workspace-card-name')?.textContent;
      expect(activeCardName).toBe('Kitchen Display');

      // The active dot should be present on the KDS card
      const activeDots = document.querySelectorAll('.workspace-card-active-dot');
      expect(activeDots.length).toBe(1);
      expect(activeCards[0]?.contains(activeDots[0] as Node)).toBe(true);
    });

    it('sets aria-selected on the active workspace card', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: 'admin',
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Admin')).toBeInTheDocument();
      });

      const adminCard = document.querySelectorAll('.workspace-card')[4] as HTMLButtonElement;
      expect(adminCard.getAttribute('aria-current')).toBe('true');

      // Other cards should not have aria-current
      const firstCard = document.querySelectorAll('.workspace-card')[0] as HTMLButtonElement;
      expect(firstCard.getAttribute('aria-current')).toBeNull();
    });
  });

  // ── Arrow-key navigation ────────────────────────────────────

  describe('arrow-key navigation', () => {
    it('moves focus with arrow right and left keys', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Focus the first card
      const firstCard = document.querySelectorAll('.workspace-card')[0] as HTMLButtonElement;
      firstCard.focus();

      // Arrow right should move to next card
      fireEvent.keyDown(document.activeElement!, { key: 'ArrowRight' });
      const cards = document.querySelectorAll('.workspace-card');
      expect(document.activeElement).toBe(cards[1]);

      // Arrow left should move back
      fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
      expect(document.activeElement).toBe(cards[0]);
    });

    it('Home key focuses first card', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const lastCard = document.querySelectorAll('.workspace-card')[4] as HTMLButtonElement;
      lastCard.focus();

      fireEvent.keyDown(document.activeElement!, { key: 'Home' });
      const cards = document.querySelectorAll('.workspace-card');
      expect(document.activeElement).toBe(cards[0]);
    });

    it('End key focuses last card', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
        lastWorkspace: null,
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const firstCard = document.querySelectorAll('.workspace-card')[0] as HTMLButtonElement;
      firstCard.focus();

      fireEvent.keyDown(document.activeElement!, { key: 'End' });
      const cards = document.querySelectorAll('.workspace-card');
      expect(document.activeElement).toBe(cards[4]);
    });
  });


});
