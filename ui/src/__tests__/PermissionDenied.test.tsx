import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import PermissionDenied from '@/components/PermissionDenied';

// ── mock AuthContext ───────────────────────────────────────────────────
const mockSession = vi.fn();

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({ session: mockSession() }),
}));

vi.mock('@/components/PermissionDenied.css', () => ({}));

// ── FTL ────────────────────────────────────────────────────────────────
const ftl = `
permission-denied-title = Access Denied
permission-denied-desc = { $action } requires a { $requiredRole } role.
permission-denied-current = You are logged in as { $displayName } ({ $roleName }).
permission-denied-go-back = Go back
`;

const bundle = new FluentBundle('en');
bundle.addResource(new FluentResource(ftl));
const l10n = new ReactLocalization([bundle]);

function renderPerm(props: Parameters<typeof PermissionDenied>[0]) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <PermissionDenied {...props} />
    </LocalizationProvider>,
  );
}

// ── tests ──────────────────────────────────────────────────────────────
describe('PermissionDenied', () => {
  it('renders the "Access Denied" title', () => {
    mockSession.mockReturnValue(null);
    renderPerm({ action: 'void orders', requiredRole: 'Manager' });
    expect(screen.getByText('Access Denied')).toBeTruthy();
  });

  it('renders the description with action and requiredRole', () => {
    mockSession.mockReturnValue(null);
    renderPerm({ action: 'void orders', requiredRole: 'Manager' });
    const desc = document.querySelector('.permission-denied-desc');
    expect(desc).toBeTruthy();
    expect(desc!.textContent).toContain('void orders');
    expect(desc!.textContent).toContain('Manager');
  });

  it('renders current user info when session exists', () => {
    mockSession.mockReturnValue({
      display_name: 'Alice',
      role_name: 'cashier',
    });
    renderPerm({ action: 'edit settings', requiredRole: 'Manager' });
    const current = document.querySelector('.permission-denied-current');
    expect(current).toBeTruthy();
    expect(current!.textContent).toContain('Alice');
    expect(current!.textContent).toContain('cashier');
  });

  it('does not render current user info when no session', () => {
    mockSession.mockReturnValue(null);
    renderPerm({ action: 'void orders', requiredRole: 'Manager' });
    expect(document.querySelector('.permission-denied-current')).toBeNull();
  });

  it('renders a dismiss button when onDismiss is provided', () => {
    mockSession.mockReturnValue(null);
    renderPerm({ action: 'edit', requiredRole: 'Owner', onDismiss: vi.fn() });
    expect(screen.getByText('Go back')).toBeTruthy();
  });

  it('calls onDismiss when the button is clicked', () => {
    mockSession.mockReturnValue(null);
    const onDismiss = vi.fn();
    renderPerm({ action: 'edit', requiredRole: 'Owner', onDismiss });
    fireEvent.click(screen.getByText('Go back'));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('does not render dismiss button when onDismiss is absent', () => {
    mockSession.mockReturnValue(null);
    renderPerm({ action: 'void orders', requiredRole: 'Manager' });
    expect(document.querySelector('.permission-denied-btn')).toBeNull();
  });

  it('renders the lock icon with aria-hidden', () => {
    mockSession.mockReturnValue(null);
    renderPerm({ action: 'test', requiredRole: 'Admin' });
    const icon = document.querySelector('.permission-denied-icon');
    expect(icon).toBeTruthy();
    expect(icon!.getAttribute('aria-hidden')).toBe('true');
  });
});
