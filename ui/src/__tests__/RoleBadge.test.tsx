import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import RoleBadge from '@/components/RoleBadge';

// Mock AuthContext — same pattern used by all other test files.
vi.mock('@/contexts/AuthContext', () => ({
  useAuth: vi.fn(),
}));

import { useAuth } from '@/contexts/AuthContext';

// ── Setup ──────────────────────────────────────────────────────────

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(`
role-badge-logged-in-aria = Logged in as { $displayName } ({ $roleName })
role-badge-logout-aria = Log out { $displayName }
role-badge-logout-title = Log out
`));
const l10n = new ReactLocalization([bundle]);

function renderRoleBadge() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <RoleBadge />
    </LocalizationProvider>,
  );
}

const mockUseAuth = useAuth as ReturnType<typeof vi.fn>;

// ── Tests ──────────────────────────────────────────────────────────

describe('RoleBadge', () => {
  beforeEach(() => {
    mockUseAuth.mockReset();
  });

  it('renders null when no session', () => {
    mockUseAuth.mockReturnValue({
      session: null,
      logout: vi.fn(),
      isManager: false,
      isOwner: false,
    });
    const { container } = renderRoleBadge();
    expect(container.innerHTML).toBe('');
  });

  it('renders display name from session', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Alice', role_name: 'cashier' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    expect(screen.getByText('Alice')).toBeInTheDocument();
  });

  it('renders role name from session', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Alice', role_name: 'cashier' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    expect(screen.getByText('cashier')).toBeInTheDocument();
  });

  it('renders first letter of display name as avatar', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Alice', role_name: 'cashier' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    expect(screen.getByText('A')).toBeInTheDocument();
  });

  it('renders different avatar letter for another user', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Bob', role_name: 'cashier' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    expect(screen.getByText('B')).toBeInTheDocument();
  });

  it('has aria-label from Fluent', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Alice', role_name: 'cashier' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    // Fluent adds invisible Unicode Bidi markers around interpolated vars.
    const badge = screen.getByRole('generic', { name: /logged in as/i });
    expect(badge).toBeInTheDocument();
  });

  it('has logout button with correct aria-label', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Alice', role_name: 'cashier' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    // Fluent wraps interpolated variables in Unicode Bidi markers.
    const btn = screen.getByRole('button', { name: /log out alice/i });
    expect(btn).toBeInTheDocument();
  });

  it('calls logout when logout button is clicked', async () => {
    const logout = vi.fn();
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Alice', role_name: 'cashier' },
      logout,
    });
    renderRoleBadge();
    await userEvent.click(screen.getByRole('button', { name: /log out alice/i }));
    expect(logout).toHaveBeenCalledTimes(1);
  });

  it('applies role variant class for cashier', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Alice', role_name: 'cashier' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    const roleEl = screen.getByText('cashier');
    expect(roleEl.classList.contains('role-badge-role--cashier')).toBe(true);
  });

  it('applies role variant class for manager', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Bob', role_name: 'manager' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    const roleEl = screen.getByText('manager');
    expect(roleEl.classList.contains('role-badge-role--manager')).toBe(true);
  });

  it('applies role variant class for owner', () => {
    mockUseAuth.mockReturnValue({
      session: { display_name: 'Carol', role_name: 'owner' },
      logout: vi.fn(),
    });
    renderRoleBadge();
    const roleEl = screen.getByText('owner');
    expect(roleEl.classList.contains('role-badge-role--owner')).toBe(true);
  });
});
