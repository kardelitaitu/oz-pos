import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHookInAct } from '@/test-utils/renderInAct';
import { useCustomerDisplay } from '@/features/sales/useCustomerDisplay';
import type { Money } from '@/types/domain';

const mocks = vi.hoisted(() => ({
  listDisplaysScoped: vi.fn(),
  displayShowScoped: vi.fn(),
  displayClearScoped: vi.fn(),
}));

vi.mock('@/api/hardware', () => ({
  listDisplaysScoped: (...args: unknown[]) => mocks.listDisplaysScoped(...args),
  displayShowScoped: (...args: unknown[]) => mocks.displayShowScoped(...args),
  displayClearScoped: (...args: unknown[]) => mocks.displayClearScoped(...args),
}));

function makeTotal(overrides: Partial<Money> = {}): Money {
  return { minor_units: 1250, currency: 'USD', ...overrides };
}

const TOKEN = 'session-token';

beforeEach(() => {
  mocks.listDisplaysScoped.mockResolvedValue(['display-1']);
  mocks.displayShowScoped.mockResolvedValue(undefined);
  mocks.displayClearScoped.mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('useCustomerDisplay', () => {
  describe('display detection', () => {
    it('auto-detects the first display on mount', async () => {
      await renderHookInAct(() => useCustomerDisplay({ sessionToken: TOKEN, lines: [], total: null }));

      expect(mocks.listDisplaysScoped).toHaveBeenCalledWith(TOKEN);
    });

    it('returns the detected display id', async () => {
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [], total: null }),
      );

      expect(result.current.displayId).toBe('display-1');
    });

    it('returns null displayId when no displays are registered', async () => {
      mocks.listDisplaysScoped.mockResolvedValue([]);

      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [], total: null }),
      );

      expect(result.current.displayId).toBeNull();
    });

    it('returns null displayId when listDisplays throws', async () => {
      mocks.listDisplaysScoped.mockRejectedValue(new Error('no backend'));

      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [], total: null }),
      );

      expect(result.current.displayId).toBeNull();
    });
  });

  describe('cart state updates', () => {
    it('shows total and item count on display when items are in cart', async () => {
      await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(mocks.displayShowScoped).toHaveBeenCalledWith(
        TOKEN,
        expect.objectContaining({ displayId: 'display-1' }),
      );
    });

    it('clears display when cart is empty', async () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (renderHookInAct as any)(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [], total: null }),
      );

      expect(mocks.displayClearScoped).toHaveBeenCalledWith(TOKEN, 'display-1');
    });

    it('clears display when item count reaches zero', async () => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { rerender } = await (renderHookInAct as any)(
        ({ lines, total }: { lines: { qty: number }[]; total: Money | null }) =>
          useCustomerDisplay({ sessionToken: TOKEN, lines, total }),
        { initialProps: { lines: [{ qty: 3 }], total: makeTotal() } },
      );

      mocks.displayClearScoped.mockClear();
      rerender({ lines: [], total: makeTotal() });

      expect(mocks.displayClearScoped).toHaveBeenCalledWith(TOKEN, 'display-1');
    });

    it('does not display when no display is connected', async () => {
      mocks.listDisplaysScoped.mockResolvedValue([]);

      await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(mocks.displayShowScoped).not.toHaveBeenCalled();
    });

    it('uses singular "item" when count is 1', async () => {
      // The hook passes line2 as padded text to displayShow — we verify
      // the raw string passed to the API contains the singular form.
      await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 1 }], total: makeTotal() }),
      );

      expect(mocks.displayShowScoped).toHaveBeenCalledWith(TOKEN, {
        displayId: 'display-1',
        line1: expect.any(String),
        line2: expect.stringContaining('1 item'),
      });
    });

    it('uses plural "items" when count is not 1', async () => {
      await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(mocks.displayShowScoped).toHaveBeenCalledWith(TOKEN, {
        displayId: 'display-1',
        line1: expect.any(String),
        line2: expect.stringContaining('3 items'),
      });
    });

    it('skips redundant display updates when content has not changed', async () => {
      const lines = [{ qty: 3 }];
      const total = makeTotal();

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { rerender } = await (renderHookInAct as any)(
        ({ lines: l, total: t }: { lines: { qty: number }[]; total: Money | null }) =>
          useCustomerDisplay({ sessionToken: TOKEN, lines: l, total: t }),
        { initialProps: { lines, total } },
      );

      mocks.displayShowScoped.mockClear();
      rerender({ lines, total });

      expect(mocks.displayShowScoped).not.toHaveBeenCalled();
    });
  });

  describe('handlePaymentComplete', () => {
    it('clears the display', async () => {
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 3 }], total: makeTotal() }),
      );

      result.current.handlePaymentComplete();

      expect(mocks.displayClearScoped).toHaveBeenCalledWith(TOKEN, 'display-1');
    });

    it('calls onPaymentComplete callback', async () => {
      const onPaymentComplete = vi.fn();
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { result } = await (renderHookInAct as any)(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 3 }], total: makeTotal(), onPaymentComplete }),
      );

      result.current.handlePaymentComplete();

      expect(onPaymentComplete).toHaveBeenCalled();
    });

    it('does not throw when onPaymentComplete is not provided', async () => {
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(() => result.current.handlePaymentComplete()).not.toThrow();
    });

    it('does not call displayClear when no display is connected', async () => {
      mocks.listDisplaysScoped.mockResolvedValue([]);
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ sessionToken: TOKEN, lines: [{ qty: 3 }], total: makeTotal() }),
      );

      result.current.handlePaymentComplete();

      expect(mocks.displayClearScoped).not.toHaveBeenCalled();
    });
  });
});
