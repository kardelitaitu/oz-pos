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
  { key: 'restaurant-pos', name: 'Restaurant POS', description: 'Cashier terminal for restaurant ordering', icon: 'restaurant' },
  { key: 'store-pos', name: 'Store POS', description: 'Cashier terminal for retail', icon: 'store' },
  { key: 'kds', name: 'Kitchen Display', description: 'Order queue display for the kitchen', icon: 'kds' },
  { key: 'inventory', name: 'Inventory Management', description: 'Manage products and stock', icon: 'inventory' },
  { key: 'admin', name: 'Admin', description: 'System settings and reports', icon: 'admin' },
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
        activeWorkspace: null,
        workspaceScreens: [],
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

    it('shows user display name and role in header', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText(/Test Owner/)).toBeInTheDocument();
        expect(screen.getByText(/owner/)).toBeInTheDocument();
      });
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
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Click the first workspace card (Restaurant POS)
      const firstCard = document.querySelectorAll('.workspace-card')[0] as HTMLButtonElement;
      await userEvent.click(firstCard);
      expect(mockSetActiveWorkspace).toHaveBeenCalledWith('restaurant-pos');
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
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const cards = document.querySelectorAll('.workspace-card');
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
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const cards = document.querySelectorAll('.workspace-card--disabled');
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
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      const badges = screen.getAllByText('Not available');
      expect(badges.length).toBe(3);
    });

    it('allows owner role to access all workspaces', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
      });

      await renderInAct(wrap(<WorkspaceHome />));

      await waitFor(() => {
        expect(screen.getByText('Restaurant POS')).toBeInTheDocument();
      });

      // Owner has access to all cards — none should be disabled
      const disabledCards = document.querySelectorAll('.workspace-card--disabled');
      expect(disabledCards.length).toBe(0);

      // Find the Admin card by its heading text and click it
      const adminCard = document.querySelectorAll('.workspace-card')[4] as HTMLButtonElement;
      await userEvent.click(adminCard);
      expect(mockSetActiveWorkspace).toHaveBeenCalledWith('admin');

      // Verify the KDS card is also clickable
      const kdsCard = document.querySelectorAll('.workspace-card')[2] as HTMLButtonElement;
      await userEvent.click(kdsCard);
      expect(mockSetActiveWorkspace).toHaveBeenCalledWith('kds');
    });
  });

  // ── Keyboard navigation ────────────────────────────────────

  describe('keyboard navigation', () => {
    it('moves focus with arrow right and left keys', async () => {
      mockWorkspaceValue.mockReturnValue({
        availableWorkspaces: sampleWorkspaces,
        loading: false,
        error: null,
        retry: vi.fn(),
        setActiveWorkspace: mockSetActiveWorkspace,
        activeWorkspace: null,
        workspaceScreens: [],
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
