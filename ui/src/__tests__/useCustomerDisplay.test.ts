import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHookInAct } from '@/test-utils/renderInAct';
import { useCustomerDisplay } from '@/features/sales/useCustomerDisplay';
import type { Money } from '@/types/domain';

const mocks = vi.hoisted(() => ({
  listDisplays: vi.fn(),
  displayShow: vi.fn(),
  displayClear: vi.fn(),
}));

vi.mock('@/api/hardware', () => ({
  listDisplays: (...args: unknown[]) => mocks.listDisplays(...args),
  displayShow: (...args: unknown[]) => mocks.displayShow(...args),
  displayClear: (...args: unknown[]) => mocks.displayClear(...args),
}));

function makeTotal(overrides: Partial<Money> = {}): Money {
  return { minor_units: 1250, currency: 'USD', ...overrides };
}

beforeEach(() => {
  mocks.listDisplays.mockResolvedValue(['display-1']);
  mocks.displayShow.mockResolvedValue(undefined);
  mocks.displayClear.mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('useCustomerDisplay', () => {
  describe('display detection', () => {
    it('auto-detects the first display on mount', async () => {
      await renderHookInAct(() => useCustomerDisplay({ lines: [], total: null }));

      expect(mocks.listDisplays).toHaveBeenCalled();
    });

    it('returns the detected display id', async () => {
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ lines: [], total: null }),
      );

      expect(result.current.displayId).toBe('display-1');
    });

    it('returns null displayId when no displays are registered', async () => {
      mocks.listDisplays.mockResolvedValue([]);

      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ lines: [], total: null }),
      );

      expect(result.current.displayId).toBeNull();
    });

    it('returns null displayId when listDisplays throws', async () => {
      mocks.listDisplays.mockRejectedValue(new Error('no backend'));

      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ lines: [], total: null }),
      );

      expect(result.current.displayId).toBeNull();
    });
  });

  describe('cart state updates', () => {
    it('shows total and item count on display when items are in cart', async () => {
      await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(mocks.displayShow).toHaveBeenCalledWith(
        expect.objectContaining({ displayId: 'display-1' }),
      );
    });

    it('clears display when cart is empty', async () => {
      await renderHookInAct(() =>
        useCustomerDisplay({ lines: [], total: null }),
      );

      expect(mocks.displayClear).toHaveBeenCalledWith('display-1');
    });

    it('clears display when item count reaches zero', async () => {
      const { rerender } = await renderHookInAct(
        ({ lines, total }: { lines: { qty: number }[]; total: Money | null }) =>
          useCustomerDisplay({ lines, total }),
        { initialProps: { lines: [{ qty: 3 }], total: makeTotal() } },
      );

      mocks.displayClear.mockClear();
      rerender({ lines: [], total: makeTotal() });

      expect(mocks.displayClear).toHaveBeenCalledWith('display-1');
    });

    it('does not display when no display is connected', async () => {
      mocks.listDisplays.mockResolvedValue([]);

      await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(mocks.displayShow).not.toHaveBeenCalled();
    });

    it('uses singular "item" when count is 1', async () => {
      // The hook passes line2 as padded text to displayShow — we verify
      // the raw string passed to the API contains the singular form.
      await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 1 }], total: makeTotal() }),
      );

      expect(mocks.displayShow).toHaveBeenCalledWith({
        displayId: 'display-1',
        line1: expect.any(String),
        line2: expect.stringContaining('1 item'),
      });
    });

    it('uses plural "items" when count is not 1', async () => {
      await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(mocks.displayShow).toHaveBeenCalledWith({
        displayId: 'display-1',
        line1: expect.any(String),
        line2: expect.stringContaining('3 items'),
      });
    });

    it('skips redundant display updates when content has not changed', async () => {
      const lines = [{ qty: 3 }];
      const total = makeTotal();

      const { rerender } = await renderHookInAct(
        ({ lines: l, total: t }: { lines: { qty: number }[]; total: Money | null }) =>
          useCustomerDisplay({ lines: l, total: t }),
        { initialProps: { lines, total } },
      );

      mocks.displayShow.mockClear();
      rerender({ lines, total });

      expect(mocks.displayShow).not.toHaveBeenCalled();
    });
  });

  describe('handlePaymentComplete', () => {
    it('clears the display', async () => {
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 3 }], total: makeTotal() }),
      );

      result.current.handlePaymentComplete();

      expect(mocks.displayClear).toHaveBeenCalledWith('display-1');
    });

    it('calls onPaymentComplete callback', async () => {
      const onPaymentComplete = vi.fn();
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 3 }], total: makeTotal(), onPaymentComplete }),
      );

      result.current.handlePaymentComplete();

      expect(onPaymentComplete).toHaveBeenCalled();
    });

    it('does not throw when onPaymentComplete is not provided', async () => {
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 3 }], total: makeTotal() }),
      );

      expect(() => result.current.handlePaymentComplete()).not.toThrow();
    });

    it('does not call displayClear when no display is connected', async () => {
      mocks.listDisplays.mockResolvedValue([]);
      const { result } = await renderHookInAct(() =>
        useCustomerDisplay({ lines: [{ qty: 3 }], total: makeTotal() }),
      );

      result.current.handlePaymentComplete();

      expect(mocks.displayClear).not.toHaveBeenCalled();
    });
  });
});
