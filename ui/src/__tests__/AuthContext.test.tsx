import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AuthProvider, useAuth } from '@/contexts/AuthContext';

// ── mock staff login API ──────────────────────────────────────────────
const mockStaffLogin = vi.fn();

vi.mock('@/api/staff', () => ({
  staffLogin: (...args: unknown[]) => mockStaffLogin(...args),
}));

// ── test consumer ─────────────────────────────────────────────────────
function TestConsumer() {
  const { session, loading, error, login, logout, clearError, isManager, isOwner } = useAuth();
  return (
    <div>
      <span data-testid="loading">{String(loading)}</span>
      <span data-testid="error">{error ?? 'no-error'}</span>
      <span data-testid="session">{session ? session.display_name : 'no-session'}</span>
      <span data-testid="role">{session?.role_name ?? 'none'}</span>
      <span data-testid="isManager">{String(isManager)}</span>
      <span data-testid="isOwner">{String(isOwner)}</span>
      <button data-testid="login-btn" onClick={() => login('alice', '1234')}>
        Login
      </button>
      <button data-testid="logout-btn" onClick={logout}>
        Logout
      </button>
      <button data-testid="clear-btn" onClick={clearError}>
        Clear error
      </button>
    </div>
  );
}

function renderProvider() {
  return render(
    <AuthProvider>
      <TestConsumer />
    </AuthProvider>,
  );
}

describe('AuthContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts with no session, no loading, no error', () => {
    renderProvider();
    expect(screen.getByTestId('session').textContent).toBe('no-session');
    expect(screen.getByTestId('loading').textContent).toBe('false');
    expect(screen.getByTestId('error').textContent).toBe('no-error');
  });

  it('isManager and isOwner are false with no session', () => {
    renderProvider();
    expect(screen.getByTestId('isManager').textContent).toBe('false');
    expect(screen.getByTestId('isOwner').textContent).toBe('false');
  });

  it('sets loading=true during login and resolves with session', async () => {
    mockStaffLogin.mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve({
        session: { display_name: 'Alice', role_name: 'manager', user_id: 'u1', role_id: 'r1' },
      }), 100)),
    );

    renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    // Loading should be true immediately after click
    expect(screen.getByTestId('loading').textContent).toBe('true');

    await waitFor(() => {
      expect(screen.getByTestId('session').textContent).toBe('Alice');
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });
  });

  it('sets error on login failure', async () => {
    mockStaffLogin.mockRejectedValue(new Error('Invalid PIN'));
    renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('error').textContent).toBe('Invalid PIN');
      expect(screen.getByTestId('loading').textContent).toBe('false');
      expect(screen.getByTestId('session').textContent).toBe('no-session');
    });
  });

  it('clears error on clearError call', async () => {
    mockStaffLogin.mockRejectedValue(new Error('Invalid PIN'));
    renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('error').textContent).toBe('Invalid PIN');
    });

    fireEvent.click(screen.getByTestId('clear-btn'));
    expect(screen.getByTestId('error').textContent).toBe('no-error');
  });

  it('logs out and clears session and error', async () => {
    mockStaffLogin.mockResolvedValue({
      session: { display_name: 'Alice', role_name: 'cashier', user_id: 'u1', role_id: 'r1' },
    });
    renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('session').textContent).toBe('Alice');
    });

    fireEvent.click(screen.getByTestId('logout-btn'));
    expect(screen.getByTestId('session').textContent).toBe('no-session');
    expect(screen.getByTestId('error').textContent).toBe('no-error');
  });

  it('isManager=true for manager role', async () => {
    mockStaffLogin.mockResolvedValue({
      session: { display_name: 'Bob', role_name: 'manager', user_id: 'u2', role_id: 'r2' },
    });
    renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('isManager').textContent).toBe('true');
      expect(screen.getByTestId('isOwner').textContent).toBe('false');
    });
  });

  it('isOwner=true and isManager=true for owner role', async () => {
    mockStaffLogin.mockResolvedValue({
      session: { display_name: 'Eve', role_name: 'owner', user_id: 'u3', role_id: 'r3' },
    });
    renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('isOwner').textContent).toBe('true');
      expect(screen.getByTestId('isManager').textContent).toBe('true');
    });
  });

  it('throws when useAuth is used outside AuthProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => {
      render(<TestConsumer />);
    }).toThrow('useAuth must be used within an <AuthProvider>');
    spy.mockRestore();
  });
});
