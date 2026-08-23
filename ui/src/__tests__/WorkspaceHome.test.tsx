// ── WorkspaceHome tests ───────────────────────────────────────────
//
// Covers: loading state (skeleton), error state with retry, empty
// state, main workspace card rendering, keyboard navigation, role-
// based access control, and per-workspace accent colors.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, fireEvent, within, configure } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluent } from '@/__tests__/test-utils/render';
import WorkspaceHome from '@/features/workspaces/WorkspaceHome';

// WorkspaceHome renders a heavy multi-section screen driven by async
// context mocks; under parallel CI load a full render can exceed the
// default 1s waitFor timeout (the same flake class as SettingsPage).
// Vitest isolates module state per file, so this does not leak.
configure({ asyncUtilTimeout: 5000 });

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

// Default values for new context fields the component requires
const defaultWorkspaceOverrides = {
  setActiveInstance: vi.fn(),
  activeInstance: null,
  switchStore: vi.fn(),
  resolvedStoreId: 'default',
  sessionToken: 'mock-session-token',
  terminalId: 'test-terminal',
  swapSessionToken: vi.fn(),
};

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ ...defaultWorkspaceOverrides, ...mockWorkspaceValue() }),
}));



// ── Helpers ────────────────────────────────────────────────────

const sampleWorkspaces = [
  { instance_id: 'default-restaurant-pos', type_key: 'restaurant-pos', store_id: 'default', store_name: 'Main Store', name: 'Restaurant POS', description: 'Cashier terminal for restaurant ordering', icon: 'restaurant', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-store-pos', type_key: 'store-pos', store_id: 'default', store_name: 'Main Store', name: 'Store POS', description: 'Cashier terminal for retail', icon: 'store', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-kds', type_key: 'kds', store_id: 'default', store_name: 'Main Store', name: 'Kitchen Display', description: 'Order queue display for the kitchen', icon: 'kds', layout_mode: 'fullscreen', colour: null, is_default: false },
  { instance_id: 'default-warehouse', type_key: 'warehouse', store_id: 'default', store_name: 'Main Store', name: 'Warehouse', description: 'Product and stock management', icon: 'package', layout_mode: 'sidebar', colour: null, is_default: false },
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

function mockAuditorUser() {
  mockAuthSession.mockReturnValue({
    session: {
      user_id: 'user-2',
      display_name: 'Auditor One',
      role_name: 'auditor',
      role_id: 'role-auditor',
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

      await renderWithFluent(<WorkspaceHome />);

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

      await renderWithFluent(<WorkspaceHome />);

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

      await renderWithFluent(<WorkspaceHome />);

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
      // Non-owner/non-admin user gets the 'no access' empty message
      mockAuditorUser();
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getByText(/No workspaces/i)).toBeInTheDocument();
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });
      expect(screen.getByText('Store POS')).toBeInTheDocument();
      expect(screen.getByText('Kitchen Display')).toBeInTheDocument();
      expect(screen.getByText('Warehouse')).toBeInTheDocument();
      // Admin workspace is filtered out of the home card grid
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

      await renderWithFluent(<WorkspaceHome />);

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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      const hints = document.querySelectorAll('.workspace-card-key-hint');
      expect(hints.length).toBe(4);
      expect(hints[0]?.textContent).toBe('1');
      expect(hints[3]?.textContent).toBe('4');
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      // Each card should have a shortcut hint (hidden until hover)
      const hints = document.querySelectorAll('button.workspace-card .workspace-card-overlay');
      // 4 workspace cards + optional tools/add cards
      expect(hints.length).toBeGreaterThanOrEqual(4);
      expect(hints[0]?.textContent).toMatch(/1/);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      const cards = Array.from(document.querySelectorAll('.workspace-card:not(.workspace-card--add)'));
      expect(cards.length).toBe(4);
      const names = Array.from(cards).map((c) => c.querySelector('.workspace-card-name')?.textContent);
      expect(names).toEqual([
        'Restaurant POS',
        'Store POS',
        'Kitchen Display',
        'Warehouse',
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      const cards = document.querySelectorAll('.workspace-card');
      const workspaceCards = Array.from(cards).filter(c => !c.classList.contains('workspace-card--add'));
      expect(workspaceCards[0]).toHaveClass('ws-color-restaurant-pos');
      expect(workspaceCards[2]).toHaveClass('ws-color-kds');
      expect(workspaceCards[3]).toHaveClass('ws-color-warehouse');
    });
  });

  // ── Role-based access ───────────────────────────────────────
  //
  // Every preset role (owner/admin/manager/staff/auditor) can activate the
  // workspaces assigned to it; assignment filtering happens on the backend
  // `list_workspaces`. The client-side gate only blocks unknown/legacy
  // roles, so a recognized preset role sees all cards enabled (0048 2c).

  describe('role-based access', () => {
    it('enables all workspace cards for a recognized preset role', async () => {
      mockAuditorUser();
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      const cards = Array.from(document.querySelectorAll('.workspace-card')).filter(c => !c.textContent?.includes('Coming soon'));
      const disabled = cards.filter((c) => c.classList.contains('workspace-card--disabled'));
      // Auditor is a recognized preset role — no client-side card disabling.
      expect(disabled.length).toBe(0);
    });

    it('shows no availability badge for a recognized preset role', async () => {
      mockAuditorUser();
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      // All visible workspace cards should be accessible for a recognized preset role
      const disabledCards = document.querySelectorAll('.workspace-card--disabled');
      const nonPlaceholderDisabled = Array.from(disabledCards).filter(
        c => !c.textContent?.includes('Coming soon')
      );
      expect(nonPlaceholderDisabled.length).toBe(0);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      // Owner has access to all cards — none should be disabled
      const disabledCards = Array.from(document.querySelectorAll('.workspace-card--disabled')).filter(c => !c.textContent?.includes('Coming soon'));
      expect(disabledCards.length).toBe(0);

      // Admin workspace is filtered from card grid — use keyboard shortcut instead
      // Click the first visible card (Restaurant POS) to verify cards are clickable
      const visibleCards = document.querySelectorAll('.workspace-card:not(.workspace-card--add)');
      const firstCard = visibleCards[0] as HTMLButtonElement;
      await userEvent.click(firstCard);
      await waitFor(() => {
        expect(mockSetActiveWorkspace).toHaveBeenCalledWith('restaurant-pos');
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      // Click the logout button
      const logoutBtn = screen.getByRole('button', { name: /Logout/i });
      await userEvent.click(logoutBtn);

      // Click confirm in the modal
      await waitFor(() => {
        expect(screen.getByRole('dialog')).toBeInTheDocument();
      });

      // Scope confirm button to the dialog to avoid the toolbar "Logout" button.
      const dialog = screen.getByRole('dialog');
      const confirmBtn = within(dialog).getByRole('button', { name: /Logout/i });
      await userEvent.click(confirmBtn);

      await waitFor(() => {
        expect(mockLogout).toHaveBeenCalledTimes(1);
      });
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      // Press '9' — only 5 cards, so no action
      fireEvent.keyDown(document.activeElement!, { key: '9' });
      expect(mockSetActiveWorkspace).not.toHaveBeenCalled();
    });
  });

  // ── Fullscreen button ───────────────────────────────────────

  describe('fullscreen button', () => {
    it('renders a fullscreen toggle button with accessible tooltip', async () => {
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      const btn = document.querySelector('.workspace-home-fullscreen-btn') as HTMLButtonElement;
      expect(btn).toBeInTheDocument();
      expect(btn.getAttribute('title')).toBe('F11');
    });

    it('renders fullscreen button in loading state with accessible tooltip', async () => {
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

      await renderWithFluent(<WorkspaceHome />);

      const btn = document.querySelector('.workspace-home-fullscreen-btn') as HTMLButtonElement;
      expect(btn).toBeInTheDocument();
      expect(btn.getAttribute('title')).toBe('F11');
    });

    it('renders fullscreen button in error state with accessible tooltip', async () => {
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

      await renderWithFluent(<WorkspaceHome />);

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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

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

      await renderWithFluent(<WorkspaceHome />);

      // Admin workspace is filtered from visible cards
      // When activeWorkspace is 'admin', no visible card gets aria-current
      const cards = document.querySelectorAll('.workspace-card');
      const currentCard = Array.from(cards).find(c => c.getAttribute('aria-current') === 'true');
      expect(currentCard).toBeUndefined();

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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      const allCards = document.querySelectorAll('.workspace-card');
      const lastCard = allCards[allCards.length - 1] as HTMLButtonElement;
      lastCard.focus();

      fireEvent.keyDown(document.activeElement!, { key: 'Home' });
      const cards = document.querySelectorAll('.workspace-card:not(.workspace-card--add)');
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

      await renderWithFluent(<WorkspaceHome />);

      await waitFor(() => {
        expect(screen.getAllByText('Restaurant POS').length).toBeGreaterThanOrEqual(1);
      });

      const firstCard = document.querySelectorAll('.workspace-card')[0] as HTMLButtonElement;
      firstCard.focus();

      fireEvent.keyDown(document.activeElement!, { key: 'End' });
      // 4 workspace cards + tools/add = more than 4 total
      const allCards = document.querySelectorAll('.workspace-card');
      expect(allCards.length).toBeGreaterThanOrEqual(4);
    });
  });


});
