import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { usePosState } from '@/features/sales/usePosState';
import type { Product } from '@/types/domain';

vi.mock('@/utils/interaction', () => ({
  triggerInteraction: vi.fn(),
}));

function makeProduct(overrides: Partial<Product> = {}): Product {
  return {
    sku: 'LATTE' as Product['sku'],
    name: 'Caffè Latte',
    category: 'Hot Drinks',
    price: { minor_units: 450, currency: 'USD' },
    barcode: '4901234567890',
    inStock: true,
    stockQty: 50,
    productType: 'restaurant',
    ...overrides,
  };
}

afterEach(() => {
  vi.clearAllMocks();
});

describe('usePosState', () => {
  describe('cart operations', () => {
    it('starts with an empty cart', () => {
      const { result } = renderHook(() => usePosState());

      expect(result.current.lines).toEqual([]);
      expect(result.current.subtotal).toBeNull();
      expect(result.current.total).toBeNull();
    });

    it('adds a product to the cart', () => {
      const { result } = renderHook(() => usePosState());
      const product = makeProduct();

      act(() => { result.current.addProduct(product); });

      expect(result.current.lines).toHaveLength(1);
      expect(result.current.lines[0].sku).toBe('LATTE');
      expect(result.current.lines[0].qty).toBe(1);
    });

    it('increments quantity when adding the same product twice', () => {
      const { result } = renderHook(() => usePosState());
      const product = makeProduct();

      act(() => { result.current.addProduct(product); });
      act(() => { result.current.addProduct(product); });

      expect(result.current.lines).toHaveLength(1);
      expect(result.current.lines[0].qty).toBe(2);
    });

    it('adds separate lines for different products', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ sku: 'LATTE' as Product['sku'] })); });
      act(() => { result.current.addProduct(makeProduct({ sku: 'BAGEL' as Product['sku'], name: 'Bagel', price: { minor_units: 250, currency: 'USD' } })); });

      expect(result.current.lines).toHaveLength(2);
    });

    it('adds custom quantity for a product', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct(), 5); });

      expect(result.current.lines[0].qty).toBe(5);
    });

    it('removes a line from the cart', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct()); });
      const lineId = result.current.lines[0].id;

      act(() => { result.current.removeLine(lineId); });

      expect(result.current.lines).toHaveLength(0);
    });

    it('updates quantity of a line', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct()); });
      const lineId = result.current.lines[0].id;

      act(() => { result.current.updateQty(lineId, 3); });

      expect(result.current.lines[0].qty).toBe(3);
    });

    it('ignores updateQty when quantity is less than 1', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct()); });
      const lineId = result.current.lines[0].id;

      act(() => { result.current.updateQty(lineId, 0); });

      expect(result.current.lines[0].qty).toBe(1);
    });
  });

  describe('computed totals', () => {
    it('computes subtotal from line quantities and prices', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ price: { minor_units: 450, currency: 'USD' } }), 3); });

      expect(result.current.subtotal).toEqual({ minor_units: 1350, currency: 'USD' });
    });

    it('returns subtotal as null when cart is empty', () => {
      const { result } = renderHook(() => usePosState());

      expect(result.current.subtotal).toBeNull();
    });

    it('computes discount amount as percentage of subtotal', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ price: { minor_units: 1000, currency: 'USD' } })); });
      act(() => { result.current.setDiscount(10, 'Loyalty'); });

      expect(result.current.discountPercent).toBe(10);
      expect(result.current.discountAmount).toEqual({ minor_units: 100, currency: 'USD' });
    });

    it('clamps discount to 0-100 range', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct()); });
      act(() => { result.current.setDiscount(150, 'Too much'); });

      expect(result.current.discountPercent).toBe(100);
    });

    it('applies service charge when enabled', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ price: { minor_units: 1000, currency: 'USD' } })); });
      act(() => { result.current.setServiceCharge(true, 10); });

      expect(result.current.serviceChargeEnabled).toBe(true);
      expect(result.current.serviceChargePercent).toBe(10);
      expect(result.current.serviceChargeAmount).toEqual({ minor_units: 100, currency: 'USD' });
    });

    it('adds tip to the total', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ price: { minor_units: 1000, currency: 'USD' } })); });
      act(() => { result.current.setTipPercent(15); });

      expect(result.current.tipPercent).toBe(15);
      expect(result.current.tipAmount).toEqual({ minor_units: 150, currency: 'USD' });
    });

    it('computes grand total with discount + service charge + tip', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ price: { minor_units: 1000, currency: 'USD' } }), 2); });
      act(() => { result.current.setDiscount(10, 'VIP'); });
      act(() => { result.current.setServiceCharge(true, 5); });
      act(() => { result.current.setTipPercent(10); });

      // subtotal = 2000, discounted = 1800, service charge = 90, tip = 180
      expect(result.current.total).toEqual({ minor_units: 2070, currency: 'USD' });
    });

    it('returns null discountAmount when discount is 0', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct()); });

      expect(result.current.discountAmount).toBeNull();
      expect(result.current.tipAmount).toBeNull();
    });
  });

  describe('resetCart', () => {
    it('clears all lines and resets surcharges', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct()); });
      act(() => { result.current.setDiscount(15, 'Sale'); });
      act(() => { result.current.setTipPercent(10); });
      act(() => { result.current.setServiceCharge(true, 5); });
      act(() => { result.current.resetCart(); });

      expect(result.current.lines).toEqual([]);
      expect(result.current.discountPercent).toBe(0);
      expect(result.current.discountLabel).toBe('');
      expect(result.current.tipPercent).toBe(0);
      expect(result.current.serviceChargeEnabled).toBe(false);
    });
  });

  describe('updateLinePrice', () => {
    it('overrides the unit price of a line', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ price: { minor_units: 450, currency: 'USD' } })); });
      const lineId = result.current.lines[0].id;

      act(() => { result.current.updateLinePrice(lineId, { minor_units: 350, currency: 'USD' }); });

      expect(result.current.lines[0].unit_price).toEqual({ minor_units: 350, currency: 'USD' });
    });
  });

  describe('course management', () => {
    it('assigns a course to a line', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct()); });
      const lineId = result.current.lines[0].id;

      act(() => { result.current.assignCourse(lineId, 'course-1' as never); });

      expect(result.current.lines[0].courseId).toBe('course-1');
      expect(result.current.lines[0].coursingStatus).toBe('hold');
    });

    it('fires all lines on hold for a specific course', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ sku: 'LATTE' as Product['sku'] })); });
      act(() => { result.current.addProduct(makeProduct({ sku: 'BAGEL' as Product['sku'], name: 'Bagel' })); });
      const latteId = result.current.lines[0].id;
      const bagelId = result.current.lines[1].id;

      act(() => { result.current.assignCourse(latteId, 'course-1' as never); });
      act(() => { result.current.assignCourse(bagelId, 'course-2' as never); });
      act(() => { result.current.fireCourse('course-1' as never); });

      const latteLine = result.current.lines.find((l) => l.id === latteId)!;
      const bagelLine = result.current.lines.find((l) => l.id === bagelId)!;
      expect(latteLine.coursingStatus).toBe('fired');
      expect(bagelLine.coursingStatus).toBe('hold');
    });

    it('fires all courses at once', () => {
      const { result } = renderHook(() => usePosState());

      act(() => { result.current.addProduct(makeProduct({ sku: 'LATTE' as Product['sku'] })); });
      act(() => { result.current.addProduct(makeProduct({ sku: 'BAGEL' as Product['sku'], name: 'Bagel' })); });
      const latteId = result.current.lines[0].id;
      const bagelId = result.current.lines[1].id;

      act(() => { result.current.assignCourse(latteId, 'course-1' as never); });
      act(() => { result.current.assignCourse(bagelId, 'course-2' as never); });
      act(() => { result.current.fireAllCourses(); });

      expect(result.current.lines.every((l) => l.coursingStatus === 'fired')).toBe(true);
    });
  });
});
