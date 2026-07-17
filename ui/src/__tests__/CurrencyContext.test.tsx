import { describe, expect, it, vi, beforeEach } from 'vitest';
import { renderHook, waitFor, act, render } from '@testing-library/react';
import { CurrencyProvider, useCurrency } from '@/contexts/CurrencyContext';
import type { ReactNode } from 'react';

const mockGetDefaultCurrency = vi.fn();
const mockSetDefaultCurrency = vi.fn();

vi.mock('@/api/currency', () => ({
  getDefaultCurrency: (...args: unknown[]) => mockGetDefaultCurrency(...args),
  setDefaultCurrency: (...args: unknown[]) => mockSetDefaultCurrency(...args),
}));

beforeEach(() => {
  mockGetDefaultCurrency.mockReset();
  mockSetDefaultCurrency.mockReset();
});

function wrapper({ children }: { children: ReactNode }) {
  return <CurrencyProvider>{children}</CurrencyProvider>;
}

describe('CurrencyContext', () => {
  describe('useCurrency', () => {
    it('throws when used outside CurrencyProvider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const preventJsdomError = (e: ErrorEvent) => e.preventDefault();
      window.addEventListener('error', preventJsdomError);
      expect(() => renderHook(() => useCurrency())).toThrow(
        'useCurrency must be used within a CurrencyProvider',
      );
      window.removeEventListener('error', preventJsdomError);
      spy.mockRestore();
    });

    it('starts with fallback currency and loading=true', () => {
      mockGetDefaultCurrency.mockImplementation(() => new Promise(() => {}));
      const { result } = renderHook(() => useCurrency(), { wrapper });

      expect(result.current.currency).toBe('USD');
      expect(result.current.loading).toBe(true);
    });

    it('uses custom fallback prop', () => {
      mockGetDefaultCurrency.mockImplementation(() => new Promise(() => {}));
      const customWrapper = ({ children }: { children: ReactNode }) => (
        <CurrencyProvider fallback="EUR">{children}</CurrencyProvider>
      );
      const { result } = renderHook(() => useCurrency(), { wrapper: customWrapper });

      expect(result.current.currency).toBe('EUR');
    });

    it('loads currency from backend on mount', async () => {
      mockGetDefaultCurrency.mockResolvedValue('IDR');
      const { result } = renderHook(() => useCurrency(), { wrapper });

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      expect(result.current.currency).toBe('IDR');
      expect(mockGetDefaultCurrency).toHaveBeenCalledTimes(1);
    });

    it('falls back to fallback when backend returns null', async () => {
      mockGetDefaultCurrency.mockResolvedValue(null);
      const { result } = renderHook(() => useCurrency(), { wrapper });

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      expect(result.current.currency).toBe('USD');
    });

    it('falls back to fallback on API error', async () => {
      mockGetDefaultCurrency.mockRejectedValue(new Error('IPC error'));
      const { result } = renderHook(() => useCurrency(), { wrapper });

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      expect(result.current.currency).toBe('USD');
    });

    it('setCurrency calls the API and updates state', async () => {
      mockGetDefaultCurrency.mockResolvedValue('USD');
      mockSetDefaultCurrency.mockResolvedValue(undefined);
      const { result } = renderHook(() => useCurrency(), { wrapper });

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });

      await act(async () => {
        await result.current.setCurrency('EUR');
      });

      expect(mockSetDefaultCurrency).toHaveBeenCalledWith({ code: 'EUR' });
      expect(result.current.currency).toBe('EUR');
    });

    it('does not update state after unmount', async () => {
      let resolvePromise!: (value: string | null) => void;
      mockGetDefaultCurrency.mockImplementation(
        () => new Promise<string | null>((resolve) => {
          resolvePromise = resolve;
        }),
      );

      const { result, unmount } = renderHook(() => useCurrency(), { wrapper });

      expect(result.current.loading).toBe(true);

      unmount();

      await act(async () => {
        resolvePromise('IDR');
      });
    });
  });

  describe('CurrencyProvider rendering', () => {
    it('renders children', () => {
      const { container } = render(
        <CurrencyProvider>
          <div data-testid="child">hello</div>
        </CurrencyProvider>,
      );
      expect(container.querySelector('[data-testid="child"]')).toHaveTextContent('hello');
    });
  });
});
