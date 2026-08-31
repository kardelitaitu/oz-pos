import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, waitFor, act } from '@testing-library/react';
import { CurrencyProvider } from '@/contexts/CurrencyContext';
import CurrencyWorkspaceSync from '@/contexts/CurrencyWorkspaceSync';

// CurrencyContext reload (workspace half): the provider sits ABOVE
// WorkspaceProvider (pre-session bootstrap), so it can never see the
// store switch itself. CurrencyWorkspaceSync is the bridge rendered
// below WorkspaceProvider: on every session-token change it pushes the
// token into refresh(), so per-store scoped defaults (CUR-03) reach
// every useCurrency consumer without a page reload.

const wsState = vi.hoisted(() => ({ current: null as string | null }));
const mockGetDefaultCurrency = vi.hoisted(() => vi.fn());
const mockGetDefaultCurrencyScoped = vi.hoisted(() => vi.fn());

vi.mock('@/api/currency', () => ({
  getDefaultCurrency: () => mockGetDefaultCurrency(),
  getDefaultCurrencyScoped: (t: string) => mockGetDefaultCurrencyScoped(t),
  setDefaultCurrency: vi.fn(),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ sessionToken: wsState.current }),
}));

beforeEach(() => {
  mockGetDefaultCurrency.mockReset();
  mockGetDefaultCurrencyScoped.mockReset();
  mockGetDefaultCurrency.mockResolvedValue('USD');
  mockGetDefaultCurrencyScoped.mockResolvedValue('IDR');
  wsState.current = null;
});

function Tree() {
  return (
    <CurrencyProvider>
      <CurrencyWorkspaceSync />
    </CurrencyProvider>
  );
}

describe('CurrencyWorkspaceSync', () => {
  it('does not call the scoped getter without a session token', async () => {
    render(<Tree />);
    await waitFor(() => expect(mockGetDefaultCurrency).toHaveBeenCalled());
    expect(mockGetDefaultCurrencyScoped).not.toHaveBeenCalled();
  });

  it('refreshes with the scoped default when a session token appears', async () => {
    const { rerender } = render(<Tree />);
    await waitFor(() => expect(mockGetDefaultCurrency).toHaveBeenCalled());

    act(() => {
      wsState.current = 'tok-1';
    });
    rerender(<Tree />);

    await waitFor(() => expect(mockGetDefaultCurrencyScoped).toHaveBeenCalledWith('tok-1'));
  });

  it('re-refreshes when the workspace switches to a new token', async () => {
    wsState.current = 'tok-1';
    const { rerender } = render(<Tree />);
    await waitFor(() => expect(mockGetDefaultCurrencyScoped).toHaveBeenCalledWith('tok-1'));

    act(() => {
      wsState.current = 'tok-2';
    });
    rerender(<Tree />);

    await waitFor(() => expect(mockGetDefaultCurrencyScoped).toHaveBeenCalledWith('tok-2'));
    expect(mockGetDefaultCurrencyScoped).toHaveBeenCalledTimes(2);
  });
});
