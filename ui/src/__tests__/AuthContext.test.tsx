import { describe, it, expect, vi } from 'vitest';
// `render` is kept in the import below — the 'throws when used outside
// AuthProvider' test relies on a synchronous throw during render, so
// `renderInAct`'s async boundary cannot be used there.
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { renderInAct } from '@/test-utils/renderInAct';
import { AuthProvider, useAuth } from '@/contexts/AuthContext';

// ── mock staff login API ──────────────────────────────────────────────
const mockStaffLogin = vi.fn();

vi.mock('@/api/staff', () => ({
  staffLogin: (...args: unknown[]) => mockStaffLogin(...args),
}));

// ── test consumer ─────────────────────────────────────────────────────
function TestConsumer() {
  const { session, loading, error, login, logout, clearError, isManager, isOwner, swapSession } = useAuth();
  return (
    <div>
      <span data-testid="loading">{String(loading)}</span>
      <span data-testid="error">{error ?? 'no-error'}</span>
      <span data-testid="session">{session ? session.display_name : 'no-session'}</span>
      <span data-testid="role">{session?.role_name ?? 'none'}</span>
      <span data-testid="userId">{session?.user_id ?? 'none'}</span>
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
      <button
        data-testid="swap-btn"
        onClick={() => swapSession({ display_name: 'Bob', role_name: 'cashier', user_id: 'u2', role_id: 'r2' })}
      >
        Swap
      </button>
    </div>
  );
}

async function renderProvider() {
  await renderInAct(
    <AuthProvider>
      <TestConsumer />
    </AuthProvider>,
  );
}

describe('AuthContext', () => {
  it('starts with no session, no loading, no error', async () => {
    await renderProvider();
    expect(screen.getByTestId('session').textContent).toBe('no-session');
    expect(screen.getByTestId('loading').textContent).toBe('false');
    expect(screen.getByTestId('error').textContent).toBe('no-error');
  });

  it('isManager and isOwner are false with no session', async () => {
    await renderProvider();
    expect(screen.getByTestId('isManager').textContent).toBe('false');
    expect(screen.getByTestId('isOwner').textContent).toBe('false');
  });

  it('sets loading=true during login and resolves with session', async () => {
    mockStaffLogin.mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve({
        session: { display_name: 'Alice', role_name: 'manager', user_id: 'u1', role_id: 'r1' },
      }), 100)),
    );

    await renderProvider();
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
    await renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('error').textContent).toBe('Invalid PIN');
      expect(screen.getByTestId('loading').textContent).toBe('false');
      expect(screen.getByTestId('session').textContent).toBe('no-session');
    });
  });

  it('clears error on clearError call', async () => {
    mockStaffLogin.mockRejectedValue(new Error('Invalid PIN'));
    await renderProvider();
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
    await renderProvider();
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
    await renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('isManager').textContent).toBe('true');
      // A manager is NOT an owner — only 'owner' and 'role-owner' roles satisfy isOwner.
      expect(screen.getByTestId('isOwner').textContent).toBe('false');
    });
  });

  it('isOwner=true and isManager=true for owner role', async () => {
    mockStaffLogin.mockResolvedValue({
      session: { display_name: 'Eve', role_name: 'owner', user_id: 'u3', role_id: 'r3' },
    });
    await renderProvider();
    fireEvent.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('isOwner').textContent).toBe('true');
      expect(screen.getByTestId('isManager').textContent).toBe('true');
    });
  });

  describe('swapSession (ADR #6 hot-swap)', () => {
    it('replaces session without changing loading state', async () => {
      mockStaffLogin.mockResolvedValue({
        session: { display_name: 'Alice', role_name: 'cashier', user_id: 'u1', role_id: 'r1' },
      });
      await renderProvider();
      fireEvent.click(screen.getByTestId('login-btn'));

      await waitFor(() => {
        expect(screen.getByTestId('session').textContent).toBe('Alice');
        expect(screen.getByTestId('loading').textContent).toBe('false');
      });

      // Hot-swap to Bob — loading must NOT change
      fireEvent.click(screen.getByTestId('swap-btn'));

      expect(screen.getByTestId('session').textContent).toBe('Bob');
      expect(screen.getByTestId('userId').textContent).toBe('u2');
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    it('clears any existing error on swap', async () => {
      mockStaffLogin.mockRejectedValue(new Error('Invalid PIN'));
      await renderProvider();
      fireEvent.click(screen.getByTestId('login-btn'));

      await waitFor(() => {
        expect(screen.getByTestId('error').textContent).toBe('Invalid PIN');
      });

      // Swap to a different user — error must be cleared
      fireEvent.click(screen.getByTestId('swap-btn'));

      expect(screen.getByTestId('error').textContent).toBe('no-error');
      expect(screen.getByTestId('session').textContent).toBe('Bob');
    });

    it('does not call staffLogin API', async () => {
      mockStaffLogin.mockResolvedValue({
        session: { display_name: 'Alice', role_name: 'cashier', user_id: 'u1', role_id: 'r1' },
      });
      await renderProvider();
      fireEvent.click(screen.getByTestId('login-btn'));

      await waitFor(() => {
        expect(screen.getByTestId('session').textContent).toBe('Alice');
      });

      mockStaffLogin.mockClear();
      fireEvent.click(screen.getByTestId('swap-btn'));

      expect(mockStaffLogin).not.toHaveBeenCalled();
    });
  });

  it('throws when useAuth is used outside AuthProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const preventJsdomError = (e: ErrorEvent) => e.preventDefault();
    window.addEventListener('error', preventJsdomError);
    expect(() => {
      render(<TestConsumer />);
    }).toThrow('useAuth must be used within an <AuthProvider>');
    window.removeEventListener('error', preventJsdomError);
    spy.mockRestore();
  });
});
